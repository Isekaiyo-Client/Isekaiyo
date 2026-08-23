//! The Minecraft instance domain model (Milestone 001 scope).
//!
//! This is deliberately *data only* — no launching, no downloading, no Java.
//! It establishes the aggregate shape that later milestones grow into a full
//! launcher domain ([instance-architecture](../../../docs/instance-architecture.md)).

use crate::error::{Error, ErrorCode, Result};
use crate::ids::{InstanceId, MinecraftVersionId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_NAME_LEN: usize = 64;

/// Per-instance launch preferences (Phase 8 §3). Typed — never an unstructured
/// blob. Every field optional so older persisted instances deserialize with
/// defaults (`#[serde(default)]` keeps v1 files readable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LaunchSettings {
    /// Maximum heap in MiB. None = engine picks a sane default from RAM.
    #[serde(default)]
    pub memory_mb: Option<u32>,
    /// Initial heap in MiB (clamped to ≤ memory_mb at validation).
    #[serde(default)]
    pub min_memory_mb: Option<u32>,
    #[serde(default)]
    pub window_width: Option<u32>,
    #[serde(default)]
    pub window_height: Option<u32>,
    #[serde(default)]
    pub fullscreen: bool,
    /// User JVM arguments appended AFTER safe defaults (spec §51).
    #[serde(default)]
    pub jvm_args: Vec<String>,
    /// Instance-specific game arguments (spec §52) — kept separate from JVM args.
    #[serde(default)]
    pub game_args: Vec<String>,
    /// Local environment overrides. Local user config only; remote data may
    /// NEVER populate this (spec §54).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// Bounds used by validation: values outside these are rejected as impossible
/// rather than clamped silently (spec §50).
pub const MIN_MEMORY_MB: u32 = 512;
pub const MAX_MEMORY_MB: u32 = 32_768;

impl LaunchSettings {
    fn validate(&self) -> Result<()> {
        for m in [self.memory_mb, self.min_memory_mb] {
            if let Some(mb) = m {
                if !(MIN_MEMORY_MB..=MAX_MEMORY_MB).contains(&mb) {
                    return Err(Error::new(
                        ErrorCode::InstanceInvalid,
                        format!("memory {mb} MiB outside {MIN_MEMORY_MB}–{MAX_MEMORY_MB}"),
                    ));
                }
            }
        }
        if let (Some(min), Some(max)) = (self.min_memory_mb, self.memory_mb) {
            if min > max {
                return Err(Error::new(
                    ErrorCode::InstanceInvalid,
                    "initial memory exceeds maximum memory",
                ));
            }
        }
        validate_arg_list("jvm", &self.jvm_args)?;
        validate_arg_list("game", &self.game_args)?;
        Ok(())
    }
}

fn validate_arg_list(kind: &str, args: &[String]) -> Result<()> {
    const MAX_ARGS: usize = 256;
    const MAX_ARG_LEN: usize = 4096;
    if args.len() > MAX_ARGS {
        return Err(Error::new(
            ErrorCode::InstanceInvalid,
            format!("{kind} argument list exceeds {MAX_ARGS} entries"),
        ));
    }
    for a in args {
        // Newlines/NULs would corrupt logs or spawn; reject, don't sanitize.
        if a.contains(['\n', '\r', '\0']) || a.len() > MAX_ARG_LEN {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                format!("{kind} argument contains control characters or is oversized"),
            ));
        }
    }
    Ok(())
}

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
    /// Bumped by the store on every successful update.
    #[serde(default)]
    pub updated_at_unix: u64,
    #[serde(default)]
    pub settings: LaunchSettings,
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
            updated_at_unix: 0,
            settings: LaunchSettings::default(),
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
        self.settings.validate()
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
