//! Native library extraction: natives jars downloaded like any artifact are
//! unpacked into the instance's per-run natives directory before launch.
//!
//! Security: archive members are treated as untrusted. Only shared-library
//! files are extracted, `..`/absolute entries are rejected outright, and the
//! final path must stay inside the destination directory (zip-slip guard).

use ikk_core::error::{Error, ErrorCode, Result};
use std::{fs, path::Path};

const NATIVE_EXTENSIONS: [&str; 4] = [".dll", ".so", ".dylib", ".jnilib"];

fn is_native(name: &str) -> bool {
    NATIVE_EXTENSIONS
        .iter()
        .any(|ext| name.to_ascii_lowercase().ends_with(ext))
}

/// Extract native shared libraries from the given jars into `dest`.
/// Returns the number of files written. Metadata `exclude` prefixes
/// (typically `META-INF/`, `.git/`) are skipped.
pub fn extract_natives(jars: &[&Path], dest: &Path, exclude: &[String]) -> Result<usize> {
    fs::create_dir_all(dest).map_err(|e| {
        Error::with_source(
            ErrorCode::IoFailure,
            format!("cannot create natives dir {}", dest.display()),
            e,
        )
    })?;

    let dest_canonical = dest.canonicalize().map_err(|e| {
        Error::with_source(
            ErrorCode::IoFailure,
            format!("cannot resolve {}", dest.display()),
            e,
        )
    })?;

    let mut written = 0;
    for jar in jars {
        let file = fs::File::open(jar).map_err(|e| {
            Error::with_source(
                ErrorCode::IoFailure,
                format!("cannot open natives jar {}", jar.display()),
                e,
            )
        })?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| {
            Error::with_source(
                ErrorCode::MetadataInvalid,
                format!("{} is not a readable jar/zip", jar.display()),
                e,
            )
        })?;

        for i in 0..archive.len() {
            let mut member = archive.by_index(i).map_err(|e| {
                Error::with_source(
                    ErrorCode::MetadataInvalid,
                    format!("corrupt entry #{i} in {}", jar.display()),
                    e,
                )
            })?;
            let name = member.name().to_string();

            if !is_native(&name) {
                continue;
            }
            if exclude
                .iter()
                .any(|prefix| name.starts_with(prefix.as_str()))
            {
                continue;
            }
            // Zip-slip guard 1: no traversal components, not absolute.
            if name.contains("..") || name.starts_with('/') || name.contains('\\') {
                return Err(Error::new(
                    ErrorCode::MetadataInvalid,
                    format!(
                        "natives jar {} contains unsafe entry {name:?}",
                        jar.display()
                    ),
                ));
            }
            let out_path = dest.join(&name);
            // Zip-slip guard 2: resolved path must remain inside dest.
            let member_parent = out_path.parent().unwrap_or(dest);
            let _ = fs::create_dir_all(member_parent);
            let canonical_parent = member_parent.canonicalize().map_err(|e| {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot resolve {}", member_parent.display()),
                    e,
                )
            })?;
            if !canonical_parent.starts_with(&dest_canonical) {
                return Err(Error::new(
                    ErrorCode::MetadataInvalid,
                    format!("entry {name:?} escapes the natives directory"),
                ));
            }

            let mut out = fs::File::create(&out_path).map_err(|e| {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot create {}", out_path.display()),
                    e,
                )
            })?;
            std::io::copy(&mut member, &mut out).map_err(|e| {
                Error::with_source(
                    ErrorCode::IoFailure,
                    format!("cannot write {}", out_path.display()),
                    e,
                )
            })?;
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::{io::Write, path::PathBuf};

    fn make_zip(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join("natives.jar");
        let file = fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, data) in entries {
            zip.start_file(*name, zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn extracts_only_native_libraries() {
        let dir = std::env::temp_dir().join(format!("ikk-natives-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let jar = make_zip(
            &dir,
            &[
                ("lwjgl_opengl.so", b"ELF...".as_slice()),
                ("lwjgl.dll", b"MZ...".as_slice()),
                ("README.txt", b"not a native"),
                ("META-INF/MANIFEST.MF", b"manifest"),
            ],
        );
        let dest = dir.join("out");
        let n = extract_natives(&[&jar], &dest, &["META-INF/".to_owned()]).unwrap();
        assert_eq!(n, 2, "only .so and .dll extracted");
        assert!(dest.join("lwjgl_opengl.so").exists());
        assert!(dest.join("lwjgl.dll").exists());
        assert!(!dest.join("README.txt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn traversal_entries_are_rejected() {
        let dir = std::env::temp_dir().join(format!("ikk-natives-evil-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let jar = make_zip(&dir, &[("../../../../tmp/evil.so", b"bad".as_slice())]);
        let dest = dir.join("out");
        let err = extract_natives(&[&jar], &dest, &[]).unwrap_err();
        assert_eq!(err.code(), ErrorCode::MetadataInvalid);
        assert!(err.to_string().contains("unsafe entry"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_zips_report_metadata_invalid() {
        let dir = std::env::temp_dir().join(format!("ikk-natives-bad-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let jar = dir.join("bad.jar");
        fs::write(&jar, b"this is not a zip").unwrap();
        let dest = dir.join("out");
        assert_eq!(
            extract_natives(&[&jar], &dest, &[]).unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
