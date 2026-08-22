//! Asset index (`objects` map) parsing. The index is small; individual asset
//! objects are content-addressed by SHA-1 and stored under
//! `assets/objects/<hash[:2]>/<hash>` — the standard Mojang layout, so hashes
//! double as presence checks.

use ikk_core::error::{Error, ErrorCode, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub objects: BTreeMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    /// SHA-1 hex of the object — also its storage path.
    pub hash: String,
    pub size: u64,
}

impl AssetIndex {
    pub fn parse(json: &str) -> Result<Self> {
        let index: Self = serde_json::from_str(json).map_err(|e| {
            Error::with_source(ErrorCode::MetadataInvalid, "malformed asset index", e)
        })?;
        if index.objects.is_empty() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "asset index contains no objects",
            ));
        }
        Ok(index)
    }

    /// Iterate (hash, size) pairs; duplicate names sharing a hash are
    /// naturally deduplicated by the map key → value identity.
    pub fn unique_objects(&self) -> impl Iterator<Item = (&str, u64)> {
        self.objects
            .values()
            .map(|o| (o.hash.as_str(), o.size))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
    }

    /// Standard Mojang resource URL for a hash.
    pub fn resource_url(hash: &str) -> String {
        format!(
            "https://resources.download.minecraft.net/{}/{}",
            &hash[..2.min(hash.len())],
            hash
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures;

    #[test]
    fn parses_index_and_dedupes() {
        let index = AssetIndex::parse(test_fixtures::ASSET_INDEX_JSON).unwrap();
        assert_eq!(index.objects.len(), 2);
        assert_eq!(index.unique_objects().count(), 2);
    }

    #[test]
    fn resource_urls_follow_mojang_layout() {
        let url = AssetIndex::resource_url("4c8a5e7f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d");
        assert_eq!(
            url,
            "https://resources.download.minecraft.net/4c/4c8a5e7f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d"
        );
    }

    #[test]
    fn empty_or_corrupt_index_is_rejected() {
        assert_eq!(
            AssetIndex::parse("{}").unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
        assert!(AssetIndex::parse("nope").is_err());
    }
}
