//! Typed representation of a Minecraft version (Phase 2 foundation, spec §16).
//!
//! Versions are *not* free-form strings: they get classified by Mojang's
//! naming conventions, validated for filesystem safety (an id becomes part of
//! a directory name later), and travel across IPC as structured data.
//!
//! Deliberately out of scope here (later milestones): release dates, protocol
//! numbers, loader compatibility matrices — those come with real metadata
//! sources. This module only defines the shape and the invariants that never
//! change.

use crate::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};

/// Which channel a version belongs to, derived from its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionKind {
    /// Modern stable releases (`1.x`, `1.x.y`).
    Release,
    /// Development snapshots (`25w14a`, `1.21-pre4`, `1.21-rc1`).
    Snapshot,
    /// Legacy beta (`b1.7.3`).
    OldBeta,
    /// Legacy alpha (`a1.2.6`).
    OldAlpha,
}

/// A validated Minecraft version identifier plus its classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    pub kind: VersionKind,
}

impl MinecraftVersion {
    /// Parse and classify a version id. Fails on ids that are empty or
    /// unsafe as path components — the id will name directories and files
    /// once installation exists.
    pub fn parse(id: impl Into<String>) -> Result<Self> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "minecraft version must not be empty",
            ));
        }
        if id.len() > 32 {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "minecraft version exceeds 32 characters",
            ));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | 'w'))
        {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "minecraft version may only contain ASCII letters, digits, '.', '-', '_' \
                 (it becomes a directory name)",
            ));
        }
        Ok(Self {
            kind: classify(&id),
            id,
        })
    }

    /// Consume without validation — only for ids that came from a validated
    /// [`MinecraftVersion`] before (e.g. persisted instances).
    pub fn from_validated(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            kind: classify(&id),
            id,
        }
    }

    pub fn is_release(&self) -> bool {
        self.kind == VersionKind::Release
    }
}

/// Classify by Mojang naming conventions:
/// - `b1.*` → old beta, `a1.*` → old alpha
/// - pure `N.N` / `N.N.N` → release
/// - everything else (`25w14a`, `1.21-pre4`, `1.21-rc1`) → snapshot
pub fn classify(id: &str) -> VersionKind {
    let lower = id.to_ascii_lowercase();
    if lower.starts_with("b1.") {
        return VersionKind::OldBeta;
    }
    if lower.starts_with("a1.") {
        return VersionKind::OldAlpha;
    }
    let is_plain_release = lower
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    if is_plain_release && lower.contains('.') {
        VersionKind::Release
    } else {
        VersionKind::Snapshot
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn modern_releases_are_classified() {
        for id in ["1.21", "1.8.9", "1.16.5", "1.7.10"] {
            let v = MinecraftVersion::parse(id).unwrap();
            assert_eq!(v.kind, VersionKind::Release, "{id}");
            assert!(v.is_release());
        }
    }

    #[test]
    fn snapshots_and_pre_releases_are_classified() {
        for id in ["25w14a", "1.21-pre4", "1.21-rc1", "20w14infinite"] {
            let v = MinecraftVersion::parse(id).unwrap();
            assert_eq!(v.kind, VersionKind::Snapshot, "{id}");
            assert!(!v.is_release());
        }
    }

    #[test]
    fn legacy_channels_are_classified() {
        assert_eq!(
            MinecraftVersion::parse("b1.7.3").unwrap().kind,
            VersionKind::OldBeta
        );
        assert_eq!(
            MinecraftVersion::parse("a1.2.6").unwrap().kind,
            VersionKind::OldAlpha
        );
    }

    #[test]
    fn empty_version_is_rejected() {
        assert_eq!(
            MinecraftVersion::parse("   ").unwrap_err().code(),
            ErrorCode::InstanceInvalid
        );
    }

    #[test]
    fn path_unsafe_versions_are_rejected() {
        for bad in ["../../etc", "1.21/x", "a b", "1.21\n"] {
            assert!(
                MinecraftVersion::parse(bad).is_err(),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn oversized_version_is_rejected() {
        let long = "1".repeat(33);
        assert!(MinecraftVersion::parse(long).is_err());
    }

    #[test]
    fn serializes_with_kind_for_ipc() {
        let json = serde_json::to_string(&MinecraftVersion::parse("1.21").unwrap()).unwrap();
        assert!(json.contains(r#""kind":"release""#));
    }
}
