//! ikk-api-types — the typed IPC contract between the launcher UI and core.
//!
//! These structs are the ONLY shapes that cross the boundary. Domain types are
//! re-exported from `ikk-core` rather than duplicated: one source of truth,
//! mirrored in TypeScript in `apps/launcher-ui/src/api.ts` (generation lands
//! in M2; until then the mirror is hand-written and covered by smoke checks).

use serde::{Deserialize, Serialize};

pub use ikk_core::config::{AppConfig, LoadSource, StartPage, Theme};
pub use ikk_core::instance::{Instance, LoaderKind, LoaderSpec};
pub use ikk_core::store::{InstanceListing, LoaderKindInput};

/// Serializable projection of [`ikk_core::Error`] for command failures.
/// `code` is the stable taxonomy string (`instance.invalid`, …) so the UI can
/// branch on category without parsing messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl From<ikk_core::Error> for CommandError {
    fn from(e: ikk_core::Error) -> Self {
        Self {
            code: e.code().as_str().to_owned(),
            message: e.to_string(),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CommandError {}

impl CommandError {
    /// Internal-category error for non-`ikk_core::Error` failure modes
    /// (e.g. poisoned locks).
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal.error".to_owned(),
            message: message.into(),
        }
    }
}

/// Response of the `get_system_info` shell command.
///
/// Proves: frontend -> Tauri -> Rust -> ikk-core, end to end.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemInfo {
    /// Workspace version of this build (from `CARGO_PKG_VERSION`).
    pub app_version: String,
    /// Target triple of the Rust core build.
    pub target: String,
    /// Stable build profile identifier ("debug" | "release").
    pub profile: String,
}

/// Result of the startup configuration load — lets the UI surface recovery
/// events instead of hiding them (spec §11).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigLoadInfo {
    pub source: LoadSourceDto,
    /// Set when the previous file was moved aside; shown to the user.
    pub corrupt_backup_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadSourceDto {
    Defaults,
    File,
    RecoveredCorrupt,
}

impl From<LoadSource> for LoadSourceDto {
    fn from(source: LoadSource) -> Self {
        match source {
            LoadSource::Defaults => Self::Defaults,
            LoadSource::File => Self::File,
            LoadSource::RecoveredCorrupt => Self::RecoveredCorrupt,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn system_info_is_json_shaped_for_the_frontend() {
        let info = SystemInfo {
            app_version: "0.1.0".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "debug".into(),
        };
        let json = serde_json::to_string(&info).expect("serialization is total here");
        let back: SystemInfo = serde_json::from_str(&json).expect("we just produced this shape");
        assert_eq!(back, info);
    }

    #[test]
    fn command_error_carries_stable_code() {
        let err: CommandError =
            ikk_core::Error::new(ikk_core::ErrorCode::InstanceInvalid, "bad name").into();
        assert_eq!(err.code, "instance.invalid");
    }

    #[test]
    fn app_config_roundtrips_through_json() {
        let json = serde_json::to_string(&AppConfig::default()).expect("total");
        let back: AppConfig = serde_json::from_str(&json).expect("we just produced this shape");
        assert_eq!(back, AppConfig::default());
    }

    #[test]
    fn instance_listing_serializes_for_ipc() {
        let listing = InstanceListing {
            instances: vec![],
            unreadable_files: 2,
        };
        let json = serde_json::to_string(&listing).expect("total");
        assert!(json.contains("\"unreadable_files\":2"));
    }
}
