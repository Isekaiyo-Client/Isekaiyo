//! Official Mojang version manifest (`version_manifest_v2.json`) — parsing
//! and a timestamped on-disk cache. Network access lives in the caller
//! ([`crate::fetch_text`]), so parsing and caching are fully offline-testable.

use ikk_core::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// The manifest document. Only fields Isekaiyo consumes are modeled; unknown
/// fields are ignored (forward compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: String,
    /// `release` | `snapshot` | `old_beta` | `old_alpha`
    #[serde(rename = "type")]
    pub kind: String,
    /// URL of this version's metadata JSON on piston-meta.
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

impl VersionManifest {
    pub fn parse(json: &str) -> Result<Self> {
        let manifest: Self = serde_json::from_str(json).map_err(|e| {
            Error::with_source(ErrorCode::MetadataInvalid, "malformed version manifest", e)
        })?;
        if manifest.versions.is_empty() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "version manifest contains no versions",
            ));
        }
        Ok(manifest)
    }

    pub fn find(&self, id: &str) -> Option<&ManifestEntry> {
        self.versions.iter().find(|entry| entry.id == id)
    }

    /// Stable releases, newest first (the manifest ships newest-first).
    pub fn releases(&self) -> impl Iterator<Item = &ManifestEntry> {
        self.versions.iter().filter(|e| e.kind == "release")
    }
}

/// A cached manifest with its fetch timestamp so the UI can say how stale it
/// is and stay usable offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedManifest {
    pub fetched_at_unix: u64,
    pub manifest: VersionManifest,
}

/// Default freshness window: refetch from the network when older than 1 hour
/// (matches upstream guidance for launchers; configurable by callers).
pub const MANIFEST_FRESHNESS: Duration = Duration::from_secs(60 * 60);

pub struct ManifestCache {
    path: PathBuf,
}

impl ManifestCache {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: cache_dir.into().join("version-manifest.json"),
        }
    }

    /// Load the cached manifest if present AND intact. Corrupt cache data is
    /// never returned silently: it is reported so the caller can delete +
    /// refetch, or surface an offline-with-no-cache state.
    pub fn load(&self) -> Result<Option<CachedManifest>> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(_) => return Ok(None), // no cache yet — not an error
        };
        let cached: CachedManifest = serde_json::from_str(&raw).map_err(|e| {
            Error::with_source(
                ErrorCode::MetadataInvalid,
                format!(
                    "cached version manifest at {} is corrupt; delete it to recover",
                    self.path.display()
                ),
                e,
            )
        })?;
        Ok(Some(cached))
    }

    pub fn is_fresh(&self, cached: &CachedManifest, max_age: Duration) -> bool {
        unix_now()
            .checked_sub(cached.fetched_at_unix)
            .map(|age| Duration::from_secs(age) <= max_age)
            .unwrap_or(false)
    }

    pub fn save(&self, manifest: &VersionManifest) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::with_source(
                    ikk_core::ErrorCode::IoFailure,
                    format!("cannot create cache dir {}", parent.display()),
                    e,
                )
            })?;
        }
        let cached = CachedManifest {
            fetched_at_unix: unix_now(),
            manifest: manifest.clone(),
        };
        let json = serde_json::to_string(&cached).map_err(|e| {
            Error::with_source(ErrorCode::Internal, "manifest cache failed to serialize", e)
        })?;
        // Atomic replace: a crash mid-write can never leave a half-manifest.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| {
            Error::with_source(
                ikk_core::ErrorCode::IoFailure,
                format!("cannot write {}", tmp.display()),
                e,
            )
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            Error::with_source(
                ikk_core::ErrorCode::IoFailure,
                format!("cannot finalize {}", self.path.display()),
                e,
            )
        })?;
        Ok(())
    }

    pub fn clear(&self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures;

    #[test]
    fn parses_realistic_manifest() {
        let manifest = VersionManifest::parse(test_fixtures::MANIFEST_JSON).unwrap();
        assert_eq!(manifest.latest.release, "1.21.4");
        assert_eq!(manifest.versions.len(), 4);
        assert_eq!(manifest.find("1.20.4").unwrap().kind, "release");
        assert!(manifest.releases().all(|e| e.kind == "release"));
    }

    #[test]
    fn malformed_manifest_is_rejected() {
        assert_eq!(
            VersionManifest::parse("{ not json").unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
        assert_eq!(
            VersionManifest::parse(r#"{"latest":{"release":"x","snapshot":"y"},"versions":[]}"#)
                .unwrap_err()
                .code(),
            ErrorCode::MetadataInvalid
        );
    }

    #[test]
    fn cache_roundtrips_and_reports_corruption() {
        let dir = tempfile_dir();
        let cache = ManifestCache::new(dir.clone());
        assert!(cache.load().unwrap().is_none(), "no cache yet");

        let manifest = VersionManifest::parse(test_fixtures::MANIFEST_JSON).unwrap();
        cache.save(&manifest).unwrap();
        let loaded = cache.load().unwrap().unwrap();
        assert_eq!(loaded.manifest.latest.release, "1.21.4");
        assert!(cache.is_fresh(&loaded, MANIFEST_FRESHNESS));

        std::fs::write(dir.join("version-manifest.json"), "{corrupt").unwrap();
        assert_eq!(
            cache.load().unwrap_err().code(),
            ErrorCode::MetadataInvalid,
            "corrupt cache must never be used silently"
        );

        cache.clear();
        assert!(cache.load().unwrap().is_none());
    }

    fn tempfile_dir() -> PathBuf {
        // Mirrors ikk-core::test_support but keeps this crate's tests standalone.
        let base = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = base.join(format!("ikk-mc-test-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
