//! Mod management domain (Phase 6).
//!
//! The five concepts the spec demands stay distinct — never one giant object:
//!
//! ```text
//! RemoteProject   what a source (e.g. Modrinth) knows about a mod
//! ProjectVersion  one release of that project
//! ModFile         one downloadable artifact of a version
//! InstalledMod    local, persisted installation state per instance
//! ModProfile      a named enabled/disabled configuration over installed mods
//! ```
//!
//! Hard rules enforced by this module's types:
//! - identity is `(source, project_id)` — NEVER a filename
//! - remote metadata and installation state share no struct
//! - `External` mods (user-dropped jars) are represented, not rejected
//!
//! Submodules: [`source`] (provider trait), [`modrinth`] (official API),
//! [`resolver`] (dependency solving), [`install`] (staged installs,
//! inventory reconciliation, profiles).

pub mod install;
pub mod modrinth;
pub mod resolver;
pub mod source;

use ikk_core::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Where a mod came from. First-party (`Isekaiyo`) and local sources are part
/// of the enum now so adding them later is an extension, not a rewrite — but
/// only Modrinth is implemented in this phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Modrinth,
    Local,
    Isekaiyo,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Modrinth => "modrinth",
            SourceKind::Local => "local",
            SourceKind::Isekaiyo => "isekaiyo",
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable project identity: `(source kind, source-local project id)`.
/// Filenames are explicitly NOT identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectRef {
    pub source: SourceKind,
    pub project_id: String,
}

impl ProjectRef {
    pub fn new(source: SourceKind, project_id: impl Into<String>) -> Self {
        Self {
            source,
            project_id: project_id.into(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.project_id.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "project id must not be empty",
            ));
        }
        if self.project_id.len() > 128 || self.project_id.contains(['/', '\\', '\0']) {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "project id contains unsafe characters",
            ));
        }
        Ok(())
    }
}

/// A remote mod project as a source describes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteProject {
    pub reference: ProjectRef,
    pub title: String,
    pub description: String,
    pub authors: Vec<String>,
    /// Icon URL; may be absent. The UI renders a deterministic placeholder.
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    pub game_versions: Vec<String>,
}

/// Dependency relation exactly as sources express it. `Optional` and
/// `Incompatible` are modeled so the resolver can honor them rather than
/// treating every edge as required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Required,
    Optional,
    Incompatible,
}

/// One dependency edge of a [`ProjectVersion`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source project id of the dependency (may be from any source).
    pub project_id: String,
    pub kind: DependencyKind,
}

/// One release of a remote project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectVersion {
    pub version_id: String,
    pub project: ProjectRef,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    /// "release" | "beta" | "alpha"
    pub release_type: String,
    pub published_unix: u64,
    pub dependencies: Vec<DependencyEdge>,
    pub files: Vec<ModFile>,
}

impl ProjectVersion {
    /// The primary jar: sources mark exactly one file primary; we take the
    /// first primary file, falling back to the first `.jar`. Never every file.
    pub fn primary_file(&self) -> Option<&ModFile> {
        self.files
            .iter()
            .find(|f| f.primary && f.filename.ends_with(".jar"))
            .or_else(|| self.files.iter().find(|f| f.filename.ends_with(".jar")))
    }

    /// Does this version serve the given mc version + loader pair?
    pub fn supports(&self, game_version: &str, loader: &str) -> bool {
        self.game_versions.iter().any(|v| v == game_version)
            && self.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader))
    }
}

/// One downloadable file attached to a [`ProjectVersion`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModFile {
    pub filename: String,
    pub url: String,
    pub primary: bool,
    pub size_bytes: u64,
    /// sha1 when the source provides it — verified at download time.
    pub sha1: Option<String>,
    /// sha512 (Modrinth provides it); used when sha1 is absent.
    pub sha512: Option<String>,
}

/// How an installed mod relates to the launcher's tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagedState {
    /// Installed through Isekaiyo; metadata tracked.
    Managed,
    /// Found in mods/ without metadata — user-placed. Never auto-deleted.
    External,
    /// Tracked metadata exists but the file vanished. Reported, not pruned.
    Missing,
}

/// Local installation record for ONE mod in ONE instance. This is the
/// persistence boundary between marketplace data (remote, cacheable,
/// disposable) and what the user actually has.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub project: ProjectRef,
    pub project_title: String,
    pub version_id: String,
    pub version_number: String,
    pub filename: String,
    pub sha1: Option<String>,
    pub dependencies: Vec<DependencyEdge>,
    pub enabled: bool,
    pub installed_at_unix: u64,
}

impl InstalledMod {
    pub fn managed_state(&self) -> ManagedState {
        ManagedState::Managed
    }
}

/// A named mod configuration over an instance's installed mods.
/// Profiles reference mods by [`ProjectRef`] — files are never duplicated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModProfile {
    pub id: String,
    pub name: String,
    /// Projects explicitly enabled while this profile is active. Mods not
    /// listed are disabled under this profile.
    pub enabled_projects: Vec<ProjectRef>,
    pub created_at_unix: u64,
}

impl ModProfile {
    pub fn enables(&self, project: &ProjectRef) -> bool {
        self.enabled_projects.contains(project)
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "mod profile name must not be empty",
            ));
        }
        if self.id.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::InstanceInvalid,
                "mod profile id must not be empty",
            ));
        }
        Ok(())
    }
}

/// Aggregate inventory state of one instance's mods directory, derived from
/// persistent metadata + filesystem reconciliation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModInventory {
    pub mods: Vec<InventoryEntry>,
}

/// One row of the inventory UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub project: Option<ProjectRef>,
    pub title: String,
    pub filename: String,
    pub version_number: Option<String>,
    pub enabled: bool,
    pub state: ManagedState,
    /// Set for `Missing`/warning rows: human-readable explanation.
    pub warning: Option<String>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn version(loaders: &[&str], games: &[&str], files: Vec<ModFile>) -> ProjectVersion {
        ProjectVersion {
            version_id: "v1".into(),
            project: ProjectRef::new(SourceKind::Modrinth, "sodium"),
            version_number: "0.5.0".into(),
            game_versions: games.iter().map(|s| s.to_string()).collect(),
            loaders: loaders.iter().map(|s| s.to_string()).collect(),
            release_type: "release".into(),
            published_unix: 0,
            dependencies: vec![],
            files,
        }
    }

    fn file(name: &str, primary: bool) -> ModFile {
        ModFile {
            filename: name.into(),
            url: format!("https://cdn.example/{name}"),
            primary,
            size_bytes: 10,
            sha1: Some("deadbeef".into()),
            sha512: None,
        }
    }

    #[test]
    fn primary_file_prefers_marked_primary_jar() {
        let mut v = version(&["fabric"], &["1.20.4"], vec![file("sources.jar", false), file("main.jar", true)]);
        assert_eq!(v.primary_file().unwrap().filename, "main.jar");

        // Fallback: no primary marked → first jar wins.
        v.files = vec![file("a.jar", false), file("b.zip", false)];
        assert_eq!(v.primary_file().unwrap().filename, "a.jar");

        // Non-jar files are never selected.
        v.files = vec![file("c.zip", true)];
        assert!(v.primary_file().is_none());
    }

    #[test]
    fn compatibility_is_case_insensitive_on_loader_exact_on_game() {
        let v = version(&["Fabric", "quilt"], &["1.20.4"], vec![]);
        assert!(v.supports("1.20.4", "fabric"));
        assert!(v.supports("1.20.4", "QUILT"));
        assert!(!v.supports("1.20.1", "fabric"));
        assert!(!v.supports("1.20.4", "forge"));
    }

    #[test]
    fn project_ref_rejects_unsafe_ids() {
        assert!(ProjectRef::new(SourceKind::Modrinth, "sodium").validate().is_ok());
        assert!(ProjectRef::new(SourceKind::Modrinth, "").validate().is_err());
        assert!(ProjectRef::new(SourceKind::Modrinth, "../etc").validate().is_err());
        assert!(ProjectRef::new(SourceKind::Modrinth, "a/b").validate().is_err());
    }

    #[test]
    fn profile_enablement_and_validation() {
        let p = ProjectRef::new(SourceKind::Modrinth, "sodium");
        let mut profile = ModProfile {
            id: "pvp".into(),
            name: "PvP".into(),
            enabled_projects: vec![p.clone()],
            created_at_unix: 0,
        };
        assert!(profile.enables(&p));
        assert!(!profile.enables(&ProjectRef::new(SourceKind::Modrinth, "lithium")));
        profile.name = "  ".into();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn installed_mod_serializes_round_trip() {
        let m = InstalledMod {
            project: ProjectRef::new(SourceKind::Modrinth, "sodium"),
            project_title: "Sodium".into(),
            version_id: "abc".into(),
            version_number: "0.5.0".into(),
            filename: "sodium-0.5.0.jar".into(),
            sha1: Some("aa".into()),
            dependencies: vec![DependencyEdge {
                project_id: "fabric-api".into(),
                kind: DependencyKind::Required,
            }],
            enabled: true,
            installed_at_unix: 1234567890,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"source\":\"modrinth\""));
        let back: InstalledMod = serde_json::from_str(&json).unwrap();
        assert_eq!(back.project, m.project);
        assert_eq!(back.enabled, true);
    }
}
