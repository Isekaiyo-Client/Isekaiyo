//! Storage accounting with memoization (Phase 8 §61).
//!
//! A recursive walk of a large instance directory is far too expensive to run
//! every time the launcher UI opens, so results are cached per directory and
//! invalidated only when the directory's own metadata (mtime) changes. This is
//! a heuristic — file edits inside don't always bump the parent mtime — so
//! callers may force-refresh explicitly (e.g. after install/repair).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    bytes: u64,
    /// Directory mtime when measured; used as a cheap staleness signal.
    measured_at_mtime: Option<SystemTime>,
}

/// Memoized directory-size calculator. Cheap to clone-free share behind `&`.
#[derive(Default)]
pub struct StorageAccountant {
    cache: HashMap<PathBuf, CacheEntry>,
}

impl StorageAccountant {
    pub fn new() -> Self {
        Self::default()
    }

    /// Size of a directory tree in bytes, using the cache when fresh.
    pub fn size_of_dir(&mut self, dir: &Path) -> std::io::Result<u64> {
        let mtime = fs_metadata_time(dir)?;
        if let Some(entry) = self.cache.get(dir) {
            if entry.measured_at_mtime == mtime {
                return Ok(entry.bytes);
            }
        }
        let bytes = walk_size(dir)?;
        self.cache.insert(
            dir.to_path_buf(),
            CacheEntry {
                bytes,
                measured_at_mtime: mtime,
            },
        );
        Ok(bytes)
    }

    /// Force-drop the cached size for one directory (call after installs).
    pub fn invalidate(&mut self, dir: &Path) {
        self.cache.remove(dir);
    }

    /// Clear all cached sizes.
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    /// Cached-only peek (never walks): what the UI shows before a refresh.
    pub fn cached_size(&self, dir: &Path) -> Option<u64> {
        self.cache.get(dir).map(|e| e.bytes)
    }
}

fn fs_metadata_time(path: &Path) -> std::io::Result<Option<SystemTime>> {
    Ok(std::fs::metadata(path)?.modified().ok())
}

fn walk_size(dir: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            // Unreadable subtree counts as zero rather than failing the whole
            // accounting pass (spec: reporting must not crash on permissions).
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(ft) if ft.is_dir() => stack.push(path),
                Ok(_) => {
                    total += entry.metadata().map(|m| m.len()).unwrap_or(0);
                }
                Err(_) => continue,
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_support::unique_temp_dir;
    use std::fs;

    #[test]
    fn sizes_are_correct_and_memoized() {
        let dir = unique_temp_dir("store-acc");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.bin"), [0u8; 1000]).unwrap();
        fs::write(dir.join("sub").join("b.bin"), [0u8; 500]).unwrap();

        let mut acc = StorageAccountant::new();
        let first = acc.size_of_dir(&dir).unwrap();
        assert_eq!(first, 1500);

        // Cached second read agrees.
        assert_eq!(acc.size_of_dir(&dir).unwrap(), 1500);
        assert_eq!(acc.cached_size(&dir), Some(1500));

        // Invalidate forces a re-walk that sees new data.
        fs::write(dir.join("c.bin"), [0u8; 250]).unwrap();
        acc.invalidate(&dir);
        assert_eq!(acc.size_of_dir(&dir).unwrap(), 1750);
    }

    #[test]
    fn missing_directory_reports_zero_not_error_for_cached_reads() {
        let mut acc = StorageAccountant::new();
        let ghost = PathBuf::from("/definitely/not/a/dir-isekaiyo");
        assert_eq!(acc.cached_size(&ghost), None);
        // A direct size_of_dir on a missing dir yields 0 (walk finds nothing
        // readable) — accounting never hard-fails on absent trees.
        assert_eq!(acc.size_of_dir(&ghost).unwrap(), 0);
    }
}
