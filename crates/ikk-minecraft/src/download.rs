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

use ikk_core::error::{Error, ErrorCode, Result};
use sha1::{Digest, Sha1};
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
    let mut file = fs::File::open(path).map_err(|e| {
        Error::with_source(
            ErrorCode::IoFailure,
            format!("cannot open {} for hashing", path.display()),
            e,
        )
    })?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("read error hashing {}", path.display()),
                e,
            )
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_encode(&hasher.finalize()))
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

/// Download `url` into `dest`, verifying against `expected_sha1` when given.
/// `on_bytes` receives cumulative byte counts *for this call* (0 for skips).
pub fn download_verified(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    opts: &DownloadOptions,
    on_bytes: &mut dyn FnMut(u64),
) -> Result<FileStatus> {
    // Fast path: already have a valid copy.
    if let Some(hash) = expected_sha1 {
        if dest.exists() && sha1_file(dest)? == hash {
            return Ok(FileStatus::Skipped);
        }
    } else if dest.exists() {
        // No hash to verify against (legacy metadata): presence is the best we
        // can do — documented limitation.
        return Ok(FileStatus::Skipped);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot create {}", parent.display()),
                e,
            )
        })?;
    }

    let tmp = dest.with_extension("part");
    let mut last_err: Option<Error> = None;

    for _attempt in 0..=opts.retries {
        if cancelled(&opts.cancel) {
            let _ = fs::remove_file(&tmp);
            return Err(Error::new(ErrorCode::Internal, "download cancelled"));
        }
        match fetch_once(agent, url, &tmp, expected_sha1, on_bytes) {
            Ok(()) => {
                // Atomic replace: the destination only ever holds a complete,
                // hash-verified file.
                fs::rename(&tmp, dest).map_err(|e| {
                    Error::with_source(
                        ErrorCode::IoFailure,
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
    }

    Err(last_err.unwrap_or_else(|| Error::new(ErrorCode::NetworkTimeout, "download failed")))
}

/// One transfer attempt.
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
    let mut file = fs::File::create(tmp).map_err(|e| {
        Error::with_source(
            ErrorCode::IoFailure,
            format!("cannot create {}", tmp.display()),
            e,
        )
    })?;

    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(map_io(url))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(map_io(url))?;
        hasher.update(&buf[..n]);
        total += n as u64;
        on_bytes(total);
    }
    file.flush().map_err(map_io(url))?;

    let actual = hex_encode(&hasher.finalize());
    if let Some(expected) = expected_sha1 {
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(Error::new(
                ErrorCode::ChecksumMismatch,
                format!("checksum mismatch downloading {url}: expected {expected}, got {actual}"),
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
    move |e| Error::with_source(ErrorCode::IoFailure, format!("I/O error during {url}"), e)
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
    fn cancel_flag_is_observed_type_safely() {
        let cancel = Arc::new(AtomicBool::new(false));
        assert!(!cancelled(&cancel));
        cancel.store(true, Ordering::Relaxed);
        assert!(cancelled(&cancel));
    }
}
