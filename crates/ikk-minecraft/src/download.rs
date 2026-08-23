//! Verified streaming downloads.
//!
//! Contract (Phase 3 spec §7):
//! - existing file whose SHA-1 matches → SKIP (never re-download valid data)
//! - existing file whose hash mismatches → re-download (corrupt never survives)
//! - always download to `<dest>.part`, hash while streaming, then atomically
//!   rename — a crash or failed download can never leave a half file that
//!   pretends to be valid
//! - bounded retries; cooperative cancellation via an `AtomicBool`
//! - progress callbacks carry real byte counts, never estimates

use ikk_core::error::{classify_io, Error, ErrorCode, Result};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Hex-encode without pulling another crate.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

pub fn sha1_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

/// SHA-1 of an on-disk file, streamed (constant memory for multi-GB jars).
pub fn sha1_file(path: &Path) -> Result<String> {
    file_hash(path, HashKind::Sha1)
}

/// Which checksum algorithm the source metadata provides (spec §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashKind {
    Sha1,
    Sha256,
}

/// Incremental hasher over the supported algorithms.
enum Hasher {
    Sha1(Sha1),
    Sha256(Sha256),
}

impl Hasher {
    fn new(kind: HashKind) -> Self {
        match kind {
            HashKind::Sha1 => Hasher::Sha1(Sha1::new()),
            HashKind::Sha256 => Hasher::Sha256(Sha256::new()),
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        match self {
            Hasher::Sha1(h) => h.update(bytes),
            Hasher::Sha256(h) => h.update(bytes),
        }
    }

    fn finalize_hex(self) -> String {
        match self {
            Hasher::Sha1(h) => hex_encode(&h.finalize()),
            Hasher::Sha256(h) => hex_encode(&h.finalize()),
        }
    }
}

/// Hash of an on-disk file with the given algorithm, streamed (constant memory).
pub fn file_hash(path: &Path, kind: HashKind) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| {
        Error::with_source(
            classify_io(&e),
            format!("cannot open {} for hashing", path.display()),
            e,
        )
    })?;
    let mut hasher = Hasher::new(kind);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| {
            Error::with_source(
                classify_io(&e),
                format!("read error hashing {}", path.display()),
                e,
            )
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize_hex())
}

/// A checksum the source promised, e.g. from Mojang version metadata.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedHash<'a> {
    pub kind: HashKind,
    pub hex: &'a str,
}

impl<'a> ExpectedHash<'a> {
    pub fn sha1(hex: &'a str) -> Self {
        Self {
            kind: HashKind::Sha1,
            hex,
        }
    }

    pub fn sha256(hex: &'a str) -> Self {
        Self {
            kind: HashKind::Sha256,
            hex,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Already present and hash-valid — nothing was transferred.
    Skipped,
    Downloaded,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            FileStatus::Skipped => "skipped",
            FileStatus::Downloaded => "downloaded",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// Total attempts per file (first try + this many retries).
    pub retries: u32,
    pub cancel: Arc<AtomicBool>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            retries: 3,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// Download `url` into `dest`, verifying against `expected_sha1` when given
/// (SHA-1 convenience wrapper over [`download_verified_hash`]). `on_bytes`
/// receives cumulative byte counts *for this call* (0 for skips).
pub fn download_verified(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    opts: &DownloadOptions,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<FileStatus> {
    download_verified_hash(
        agent,
        url,
        dest,
        expected_sha1.map(ExpectedHash::sha1),
        opts,
        on_bytes,
    )
}

/// Generalized verified download: SHA-1 or SHA-256 per source metadata, skip
/// when a valid copy exists, bounded retries with exponential backoff, atomic
/// finalization, cooperative cancellation carrying [`ErrorCode::Cancelled`].
pub fn download_verified_hash(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    expected: Option<ExpectedHash<'_>>,
    opts: &DownloadOptions,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<FileStatus> {
    // Fast path: already have a valid copy.
    if let Some(hash) = expected {
        if dest.exists() && file_hash(dest, hash.kind)? == hash.hex {
            return Ok(FileStatus::Skipped);
        }
    } else if dest.exists() {
        // No hash to verify against (legacy metadata): presence is the best we
        // can do — documented limitation.
        return Ok(FileStatus::Skipped);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::with_source(classify_io(&e), format!("cannot create {}", parent.display()), e))?;
    }

    let tmp = dest.with_extension("part");
    let mut last_err: Option<Error> = None;

    for attempt in 0..=opts.retries {
        if cancelled(&opts.cancel) {
            let _ = fs::remove_file(&tmp);
            return Err(Error::new(ErrorCode::Cancelled, "download cancelled"));
        }
        match fetch_once(agent, url, &tmp, expected, on_bytes) {
            Ok(()) => {
                // Atomic replace: the destination only ever holds a complete,
                // hash-verified file.
                fs::rename(&tmp, dest).map_err(|e| {
                    Error::with_source(
                        classify_io(&e),
                        format!("cannot finalize {}", dest.display()),
                        e,
                    )
                })?;
                return Ok(FileStatus::Downloaded);
            }
            Err(e) => {
                // Never leave a partial `.part` behind pretending to be data.
                let _ = fs::remove_file(&tmp);
                last_err = Some(e);
            }
        }
        // Bounded exponential backoff between attempts (spec §63): 250ms,
        // 500ms, 1s… capped at 4s. Skipped after the final attempt and when
        // cancellation was requested during the failure.
        if attempt < opts.retries && !cancelled(&opts.cancel) {
            let delay = std::time::Duration::from_millis(250u64.saturating_mul(1 << attempt.min(4)));
            std::thread::sleep(delay);
        }
    }

    Err(last_err.unwrap_or_else(|| Error::new(ErrorCode::NetworkTimeout, "download failed")))
}

/// One transfer attempt.
fn fetch_once(
    agent: &ureq::Agent,
    url: &str,
    tmp: &Path,
    expected: Option<ExpectedHash<'_>>,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<()> {
fn fetch_once(
    agent: &ureq::Agent,
    url: &str,
    tmp: &Path,
    expected_sha1: Option<&str>,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<()> {
    let response = agent.get(url).call().map_err(map_ureq(url))?;
    if response.status() != 200 {
        return Err(Error::new(
            ErrorCode::NetworkTimeout,
            format!("HTTP {} fetching {url}", response.status()),
        ));
    }

    let mut reader = response.into_reader();
    let mut file = fs::File::create(tmp)
        .map_err(|e| Error::with_source(classify_io(&e), format!("cannot create {}", tmp.display()), e))?;

    let mut hasher = Hasher::new(expected.map_or(HashKind::Sha1, |h| h.kind));
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(map_io(url))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| Error::with_source(classify_io(&e), format!("I/O error during {url}"), e))?;
        hasher.update(&buf[..n]);
        total += n as u64;
        on_bytes(total);
    }
    file.flush()
        .map_err(|e| Error::with_source(classify_io(&e), format!("I/O error during {url}"), e))?;

    let actual = hasher.finalize_hex();
    if let Some(expected) = expected {
        if !actual.eq_ignore_ascii_case(expected.hex) {
            return Err(Error::new(
                ErrorCode::ChecksumMismatch,
                format!(
                    "checksum mismatch downloading {url}: expected {}, got {actual}",
                    expected.hex
                ),
            ));
        }
    }
    Ok(())
}

fn map_ureq(url: &str) -> impl Fn(ureq::Error) -> Error + '_ {
    move |e| match e {
        ureq::Error::Status(status, _) => Error::new(
            ErrorCode::MetadataInvalid,
            format!("HTTP {status} fetching {url}"),
        ),
        other => Error::with_source(
            ErrorCode::NetworkTimeout,
            format!("network error fetching {url}"),
            other,
        ),
    }
}

fn map_io(url: &str) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| {
        Error::with_source(
            classify_io(&e),
            format!("I/O error during {url}"),
            e,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn hex_and_hash_are_consistent() {
        assert_eq!(
            sha1_hex(b"hello"),
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
        assert_eq!(hex_encode(&[0xde, 0xad]), "dead");
    }

    #[test]
    fn sha1_file_streams_correctly() {
        let dir = std::env::temp_dir().join(format!("ikk-dl-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blob.bin");
        let payload = vec![42u8; 200_000]; // > buffer size, exercises streaming
        fs::write(&path, &payload).unwrap();
        assert_eq!(sha1_file(&path).unwrap(), sha1_hex(&payload));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_vectors_and_file_hash() {
        // Known vector: SHA-256("hello").
        let dir = std::env::temp_dir().join(format!("ikk-dl-sha2-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("blob.bin");
        fs::write(&path, b"hello").unwrap();
        assert_eq!(
            file_hash(&path, HashKind::Sha256).unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(file_hash(&path, HashKind::Sha1).unwrap(), sha1_hex(b"hello"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cancel_flag_is_observed_type_safely() {
        let cancel = Arc::new(AtomicBool::new(false));
        assert!(!cancelled(&cancel));
        cancel.store(true, Ordering::Relaxed);
        assert!(cancelled(&cancel));
    }
}
