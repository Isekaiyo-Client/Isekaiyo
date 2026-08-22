//! The Minecraft instance domain model (Milestone 001 scope).
//!
//! This is deliberately *data only* — no launching, no downloading, no Java.
//! It establishes the aggregate shape that later milestones grow into a full
//! launcher domain ([instance-architecture](../../../docs/instance-architecture.md)).

use crate::error::{Error, ErrorCode, Result};
use crate::ids::{InstanceId, MinecraftVersionId};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_NAME_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    #[default]
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

/// Which mod loader an instance uses. `version` is `None` for vanilla and
/// required for everything else — validated in [`Instance::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LoaderSpec {
    pub kind: LoaderKind,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instance {
    pub id: InstanceId,
    pub name: String,
    pub minecraft_version: MinecraftVersionId,
    pub loader: LoaderSpec,
    /// Unix seconds — u64 keeps JSON stable across platforms.
    pub created_at_unix: u64,
    pub last_played_unix: Option<u64>,
}

impl Instance {
    /// Create a new instance with generated timestamps; validates on construction.
    pub fn new(
        id: InstanceId,
        name: impl Into<String>,
        minecraft_version: MinecraftVersionId,
    ) -> Result<Self> {
        let instance = Self {
            id,
            name: name.into(),
            minecraft_version,
            loader: LoaderSpec::default(),
            created_at_unix: unix_now(),
            last_played_unix: None,
        };
        instance.validate()?;
        Ok(instance)
    }

    /// Invariants every persisted instance must satisfy. Called on creation
    /// AND before every update, so hand-edited files get caught at load+save
    /// boundaries too.
    pub fn validate(&self) -> Result<()> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "instance name must not be empty",
            ));
        }
        if self.name.len() > MAX_NAME_LEN {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("instance name exceeds {MAX_NAME_LEN} characters"),
            ));
        }
        if self.id.as_str().trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "instance id must not be empty",
            ));
        }
        if self.minecraft_version.as_str().trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "minecraft version must not be empty",
            ));
        }
        if let Some(version) = &self.loader.version {
            if version.trim().is_empty() {
                return Err(Error::new(
                    ErrorCode::InstanceInvalid,
                    "loader version set but blank (omit it for vanilla)",
                ));
            }
        }
        if self.loader.kind != LoaderKind::Vanilla && self.loader.version.is_none() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("loader {:?} requires a version", self.loader.kind),
            ));
        }
        Ok(())
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

    fn sample() -> Instance {
        Instance::new(
            InstanceId::new("inst-test-1"),
            "My Instance",
            MinecraftVersionId::new("1.21.x"),
        )
        .unwrap()
    }

    #[test]
    fn valid_instance_passes_validation() {
        sample().validate().unwrap();
    }

    #[test]
    fn blank_name_is_rejected() {
        assert!(Instance::new(
            InstanceId::new("i"),
            "   ",
            MinecraftVersionId::new("1.21.x")
        )
        .is_err());
    }

    #[test]
    fn oversized_name_is_rejected() {
        let long = "x".repeat(MAX_NAME_LEN + 1);
        let err = Instance::new(
            InstanceId::new("i"),
            long,
            MinecraftVersionId::new("1.21.x"),
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InstanceInvalid);
    }

    #[test]
    fn nonvanilla_loader_requires_version() {
        let mut inst = sample();
        inst.loader = LoaderSpec {
            kind: LoaderKind::Fabric,
            version: None,
        };
        assert_eq!(
            inst.validate().unwrap_err().code(),
            ErrorCode::InstanceInvalid
        );

        inst.loader = LoaderSpec {
            kind: LoaderKind::Fabric,
            version: Some("0.16.0".into()),
        };
        inst.validate().unwrap();
    }

    #[test]
    fn blank_loader_version_is_rejected() {
        let mut inst = sample();
        inst.loader = LoaderSpec {
            kind: LoaderKind::Forge,
            version: Some("  ".into()),
        };
        assert!(inst.validate().is_err());
    }
}
