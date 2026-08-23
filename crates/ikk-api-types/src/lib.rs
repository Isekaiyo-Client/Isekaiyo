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

// ---------------------------------------------------------------------------
// Version & loader metadata DTOs (Phase 3/5)
// ---------------------------------------------------------------------------

/// One entry of the Mojang version manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntryDto {
    pub id: String,
    /// "release" | "snapshot" | "old_beta" | "old_alpha"
    pub kind: String,
}

/// The version list plus where it came from:
/// `cache` (fresh) · `network` (just refreshed) · `stale-cache` (offline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionListDto {
    pub source: String,
    pub entries: Vec<VersionEntryDto>,
}

/// One selectable loader version from a loader meta service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersionDto {
    pub version: String,
    pub stable: bool,
}

/// Result of an installation run. Every failed artifact is named so the UI
/// can offer repair instead of a generic failure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallReportDto {
    pub downloaded: u32,
    pub skipped: u32,
    pub total_files: u32,
    pub failed: Vec<String>,
}

/// How a game run ended (`process::GameExit` projection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameExitDto {
    pub exit_code: Option<i32>,
    pub user_stopped: bool,
    /// "completed" | "crashed" | "user-stopped"
    pub category: String,
}

/// Current launch pipeline state for the UI state machine display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchStatusDto {
    pub phase: String,
    pub pid: Option<u32>,
    pub exit: Option<GameExitDto>,
    pub log_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Mod management DTOs (Phase 6). Remote marketplace data and local
// installation state are separate shapes here too — mirroring the domain.
// ---------------------------------------------------------------------------

/// One search hit from a mod source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProjectDto {
    pub source: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub authors: Vec<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
}

/// One compatible version offered for installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModVersionDto {
    pub version_id: String,
    pub version_number: String,
    pub release_type: String,
    pub filename: String,
    pub size_bytes: u64,
    pub hash_verified_source: bool,
}

/// Result of resolving an install request BEFORE anything downloads —
/// the confirmation dialog's payload (§31).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModInstallPlanDto {
    /// Mods that will be newly downloaded.
    pub to_install: Vec<ModProjectDto>,
    /// Projects already present that satisfy dependencies.
    pub already_installed: Vec<String>,
    /// Required deps with no compatible version (blocks install).
    pub unsatisfiable: Vec<String>,
    /// Installed projects the requested mod declares incompatible with.
    pub conflicts: Vec<String>,
}

impl ModInstallPlanDto {
    pub fn is_installable(&self) -> bool {
        self.unsatisfiable.is_empty() && self.conflicts.is_empty()
    }
}

/// Report of a completed install run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModInstallReportDto {
    pub downloaded: Vec<String>,
    pub skipped: Vec<String>,
    /// Downloaded without a source-provided sha1 — surfaced honestly.
    pub unverified: Vec<String>,
    pub failed: Vec<String>,
}

impl ModInstallReportDto {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Local installation state of one mod row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledModDto {
    pub source: String,
    pub project_id: Option<String>,
    pub title: String,
    pub filename: String,
    pub version_number: Option<String>,
    pub enabled: bool,
    /// "managed" | "external" | "missing"
    pub state: String,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModInventoryDto {
    pub mods: Vec<InstalledModDto>,
}

/// A named mod configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProfileDto {
    pub id: String,
    pub name: String,
    pub enabled_count: u32,
    pub active: bool,
}

/// Per-mod update classification (§24).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModUpdateDto {
    pub project_id: String,
    pub installed_version: String,
    pub available_version: Option<String>,
    /// "current" | "update-available" | "incompatible" | "unknown"
    pub state: String,
}

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

    /// Stable category for capabilities whose domain layer exists but whose
    /// runtime implementation does not yet (launch, downloads, auth). The UI
    /// matches on `runtime.unavailable` to render honest "not built yet"
    /// states instead of generic failures.
    pub fn runtime_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "runtime.unavailable".to_owned(),
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
    fn runtime_unavailable_has_stable_code() {
        let err = CommandError::runtime_unavailable("not implemented yet");
        assert_eq!(err.code, "runtime.unavailable");
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
