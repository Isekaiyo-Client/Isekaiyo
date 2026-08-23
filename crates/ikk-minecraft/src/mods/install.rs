//! Staged mod installation, local state persistence, filesystem
//! reconciliation, enable/disable, profiles (Phase 6 §12–§28).
//!
//! Pipeline per mod file, reusing the Phase 3 download engine:
//! ```text
//! DOWNLOAD (streaming, .part + atomic rename, sha1 verified)
//!   → VERIFY (hash from source metadata; unverified is reported, never hidden)
//!   → COMMIT (file already in place atomically by the downloader)
//!   → RECORD (InstalledMod persisted in ikk/mods.json)
//! ```
//!
//! Guarantees:
//! - a failed download leaves no partial file and NO metadata row
//! - existing identical files are skipped (cache reuse across instances is
//!   free because mods land per-instance while remote metadata is cacheable)
//! - external jars are represented, never deleted; missing tracked mods are
//!   reported, never pruned silently
//! - disable/enable is a reversible `.disabled` suffix rename

use super::{InstalledMod, InventoryEntry, ManagedState, ModInventory, ModProfile, ProjectRef};
use crate::download::{self, DownloadOptions};
use ikk_core::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

const DISABLED_SUFFIX: &str = ".disabled";

// ---------------------------------------------------------------------------
// Local persistence: one JSON document per instance.
// ---------------------------------------------------------------------------

/// Everything Isekaiyo tracks locally about an instance's mods. Remote
/// marketplace responses are deliberately NOT persisted here — only the
/// distilled [`InstalledMod`] rows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModsData {
    pub installed: Vec<InstalledMod>,
    pub profiles: Vec<ModProfile>,
    pub active_profile: Option<String>,
}

/// File-backed store at `<instance>/ikk/mods.json`. Corrupt documents are
/// backed up (never destroyed) and reported to the caller.
pub struct ModsStore {
    path: PathBuf,
}

impl ModsStore {
    pub fn new(instance_ikk_dir: &Path) -> Self {
        Self {
            path: instance_ikk_dir.join("mods.json"),
        }
    }

    /// Load, recovering from corrupt data by backing the file up first.
    /// Returns `(data, corrupt_backup_path)`.
    pub fn load(&self) -> Result<(ModsData, Option<PathBuf>)> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((ModsData::default(), None));
            }
            Err(e) => {
                return Err(Error::with_source(
                    ErrorCode::IoFailure,
                    "cannot read mod state",
                    e,
                ));
            }
        };
        match serde_json::from_str::<ModsData>(&raw) {
            Ok(data) => Ok((data, None)),
            Err(parse_err) => {
                // Preserve the user's bytes; start clean but say so.
                let backup = self.path.with_extension("json.corrupt");
                fs::rename(&self.path, &backup).map_err(|e| {
                    Error::with_source(ErrorCode::IoFailure, "cannot back up corrupt mod state", e)
                })?;
                Err(Error::with_source(
                    ErrorCode::ConfigInvalid,
                    format!(
                        "mod state was malformed; it was preserved at {}",
                        backup.display()
                    ),
                    parse_err,
                ))
            }
        }
    }

    pub fn save(&self, data: &ModsData) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                Error::with_source(ErrorCode::IoFailure, "cannot create instance ikk dir", e)
            })?;
        }
        let tmp = self.path.with_extension("json.part");
        fs::write(&tmp, serde_json::to_vec_pretty(data).map_err(|e| {
            Error::with_source(ErrorCode::Internal, "cannot serialize mod state", e)
        })?)
        .map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "cannot write mod state", e)
        })?;
        fs::rename(&tmp, &self.path).map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "cannot finalize mod state", e)
        })?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Staged installation.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct InstallOutcome {
    pub downloaded: Vec<String>,
    pub skipped: Vec<String>,
    /// Files whose source provided NO sha1 — installed but flagged.
    pub unverified: Vec<String>,
    pub failed: Vec<String>,
}

impl InstallOutcome {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Install every version of a resolver plan into `mods_dir`.
///
/// All-or-nothing at the *metadata* level: files download individually
/// (each atomic), but `mods.json` is updated once, only when every file of
/// every requested version succeeded — so the UI never shows "installed"
/// for a half-fetched set. Already-present valid files are skipped.
/// Returns `(outcome, staged metadata rows)`. The caller commits the rows to
/// [`ModsData`] only when `outcome.ok()` — that is the transaction boundary.
pub fn install_plan(
    agent: &ureq::Agent,
    plan: &[super::ProjectVersion],
    mods_dir: &Path,
    opts: &DownloadOptions,
) -> Result<(InstallOutcome, Vec<InstalledMod>)> {
    fs::create_dir_all(mods_dir)
        .map_err(|e| Error::with_source(ErrorCode::IoFailure, "cannot create mods dir", e))?;

    let mut outcome = InstallOutcome::default();
    let mut staged_rows: Vec<InstalledMod> = Vec::new();

    for version in plan {
        let Some(file) = version.primary_file() else {
            outcome.failed.push(format!(
                "{} {}: no primary jar file",
                version.project.project_id, version.version_number
            ));
            continue;
        };
        let dest = safe_mod_path(mods_dir, &file.filename)?;
        let result = download::download_verified(
            agent,
            &file.url,
            &dest,
            file.sha1.as_deref(),
            opts,
            &mut |_| {},
        );
        match result {
            Ok(download::FileStatus::Skipped) => outcome.skipped.push(file.filename.clone()),
            Ok(download::FileStatus::Downloaded) => outcome.downloaded.push(file.filename.clone()),
            Err(e) => {
                outcome.failed.push(format!("{}: {e}", file.filename));
                continue;
            }
        }
        if file.sha1.is_none() {
            outcome.unverified.push(file.filename.clone());
        }
        staged_rows.push(InstalledMod {
            project: version.project.clone(),
            project_title: version.project.project_id.clone(),
            version_id: version.version_id.clone(),
            version_number: version.version_number.clone(),
            filename: file.filename.clone(),
            sha1: file.sha1.clone(),
            dependencies: version.dependencies.clone(),
            enabled: true,
            installed_at_unix: unix_now(),
        });
    }

    Ok((outcome, staged_rows))
}

/// Reject filenames that would escape the mods dir or smuggle our suffixes.
pub fn safe_mod_path(mods_dir: &Path, filename: &str) -> Result<PathBuf> {
    let name = Path::new(filename);
    if filename.is_empty()
        || name.file_name() != Some(name.as_os_str())
        || filename.contains(['/', '\\', '\0'])
        || filename.contains("..")
        || !filename.to_ascii_lowercase().ends_with(".jar")
    {
        return Err(Error::new(
            ErrorCode::InstanceInvalid,
            format!("unsafe or non-jar mod filename: {filename:?}"),
        ));
    }
    Ok(mods_dir.join(filename))
}

// ---------------------------------------------------------------------------
// Enable / disable — reversible rename, collision-safe.
// ---------------------------------------------------------------------------

fn enabled_path(disabled: &Path) -> PathBuf {
    let s = disabled.to_string_lossy();
    PathBuf::from(s.strip_suffix(DISABLED_SUFFIX).unwrap_or(&s).to_owned())
}

fn disabled_path(enabled: &Path) -> PathBuf {
    let mut s = enabled.as_os_str().to_os_string();
    s.push(DISABLED_SUFFIX);
    PathBuf::from(s)
}

/// Disable a mod jar (reversible). Returns the new path.
pub fn disable_file(path: &Path) -> Result<PathBuf> {
    let target = disabled_path(path);
    if target.exists() {
        return Err(Error::new(
            ErrorCode::InstanceInvalid,
            format!("a disabled copy already exists at {}", target.display()),
        ));
    }
    fs::rename(path, &target).map_err(|e| {
        Error::with_source(ErrorCode::IoFailure, "cannot disable mod", e)
    })?;
    Ok(target)
}

/// Re-enable a previously disabled mod.
pub fn enable_file(path: &Path) -> Result<PathBuf> {
    let target = enabled_path(path);
    fs::rename(path, &target).map_err(|e| {
        Error::with_source(ErrorCode::IoFailure, "cannot enable mod", e)
    })?;
    Ok(target)
}

// ---------------------------------------------------------------------------
// Removal — managed rows only; external jars are never touched here.
// ---------------------------------------------------------------------------

/// Remove a tracked mod's file + metadata. Refuses to delete anything not in
/// the tracked set (external mods are the user's property).
pub fn remove_managed(
    data: &mut ModsData,
    mods_dir: &Path,
    project: &ProjectRef,
) -> Result<bool> {
    let Some(pos) = data.installed.iter().position(|m| m.project == *project) else {
        return Ok(false);
    };
    let removed = data.installed.remove(pos);
    let p = mods_dir.join(&removed.filename);
    if p.exists() {
        fs::remove_file(&p).map_err(|e| {
            Error::with_source(ErrorCode::IoFailure, "cannot remove mod file", e)
        })?;
    }
    // Also clear its disabled twin if one exists.
    let d = disabled_path(&p);
    if d.exists() {
        let _ = fs::remove_file(&d);
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Filesystem reconciliation — inventory derivation (§20–§22).
// ---------------------------------------------------------------------------

/// Derive the UI inventory from tracked metadata + the actual directory.
/// - tracked + present          → Managed (enabled/disabled as recorded)
/// - tracked + absent           → Missing (warning; metadata preserved)
/// - present + untracked *.jar  → External (warning-free informational row)
pub fn reconcile(data: &ModsData, mods_dir: &Path) -> ModInventory {
    let mut entries: Vec<InventoryEntry> = Vec::new();

    for m in &data.installed {
        let present = mods_dir.join(&m.filename).exists()
            || disabled_path(&mods_dir.join(&m.filename)).exists();
        entries.push(if present {
            InventoryEntry {
                project: Some(m.project.clone()),
                title: m.project_title.clone(),
                filename: m.filename.clone(),
                version_number: Some(m.version_number.clone()),
                enabled: m.enabled,
                state: ManagedState::Managed,
                warning: None,
            }
        } else {
            InventoryEntry {
                project: Some(m.project.clone()),
                title: m.project_title.clone(),
                filename: m.filename.clone(),
                version_number: Some(m.version_number.clone()),
                enabled: false,
                state: ManagedState::Missing,
                warning: Some("tracked file is missing from the mods folder".into()),
            }
        });
    }

    // External jars: any *.jar / *.jar.disabled we don't track.
    let tracked: std::collections::BTreeSet<String> = data
        .installed
        .iter()
        .map(|m| m.filename.to_ascii_lowercase())
        .collect();
    if let Ok(read_dir) = fs::read_dir(mods_dir) {
        let mut externals: Vec<InventoryEntry> = read_dir
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let name = path.file_name()?.to_str()?.to_owned();
                let lower = name.to_ascii_lowercase();
                if !(lower.ends_with(".jar") || lower.ends_with(format!("{DISABLED_SUFFIX}").as_str()))
                {
                    return None;
                }
                let base = lower.strip_suffix(DISABLED_SUFFIX).unwrap_or(&lower).to_owned();
                if tracked.contains(&base) {
                    return None;
                }
                Some(InventoryEntry {
                    project: None,
                    title: base.trim_end_matches(".jar").to_owned(),
                    filename: name.clone(),
                    version_number: None,
                    enabled: !lower.ends_with(DISABLED_SUFFIX),
                    state: ManagedState::External,
                    warning: Some("placed manually — not managed by Isekaiyo".into()),
                })
            })
            .collect();
        externals.sort_by(|a, b| a.filename.cmp(&b.filename));
        entries.extend(externals);
    }

    ModInventory { mods: entries }
}

/// Apply enable/disable flags from `data` onto disk (used after profile
/// switches): enabled → plain `.jar`, disabled → `.jar.disabled`.
pub fn apply_enabled_state(data: &mut ModsData, mods_dir: &Path) -> Result<()> {
    for m in &mut data.installed {
        let plain = mods_dir.join(&m.filename);
        let disabled = disabled_path(&plain);
        if m.enabled && disabled.exists() {
            enable_file(&disabled)?;
        } else if !m.enabled && plain.exists() {
            disable_file(&plain)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Profile operations.
// ---------------------------------------------------------------------------

/// Create a profile snapshotting the currently-enabled managed set.
pub fn create_profile_from_current(data: &ModsData, id: String, name: String) -> Result<ModProfile> {
    if data.profiles.iter().any(|p| p.id == id || p.name == name) {
        return Err(Error::new(
            ErrorCode::InstanceInvalid,
            format!("profile {id:?} already exists"),
        ));
    }
    let profile = ModProfile {
        id,
        name,
        enabled_projects: data
            .installed
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.project.clone())
            .collect(),
        created_at_unix: unix_now(),
    };
    profile.validate()?;
    Ok(profile)
}

/// Switch the active profile: rewrites enabled flags (and renames files).
/// Passing `None` enables all managed mods (the implicit "everything" mode).
pub fn switch_profile(
    data: &mut ModsData,
    mods_dir: &Path,
    profile_id: Option<&str>,
) -> Result<()> {
    match profile_id {
        None => {
            for m in &mut data.installed {
                m.enabled = true;
            }
            data.active_profile = None;
        }
        Some(id) => {
            let profile = data
                .profiles
                .iter()
                .find(|p| p.id == id)
                .ok_or_else(|| Error::new(ErrorCode::InstanceNotFound, "no such mod profile"))?;
            for m in &mut data.installed {
                m.enabled = profile.enables(&m.project);
            }
            data.active_profile = Some(id.to_owned());
        }
    }
    apply_enabled_state(data, mods_dir)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests — offline; temp dirs only.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::mods::{DependencyEdge, DependencyKind, SourceKind};

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ikk-mods-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn row(project: &str, filename: &str, deps: &[&str]) -> InstalledMod {
        InstalledMod {
            project: ProjectRef::new(SourceKind::Modrinth, project),
            project_title: project.into(),
            version_id: format!("{project}-v1"),
            version_number: "v1".into(),
            filename: filename.into(),
            sha1: None,
            dependencies: deps
                .iter()
                .map(|d| DependencyEdge {
                    project_id: (*d).into(),
                    kind: DependencyKind::Required,
                })
                .collect(),
            enabled: true,
            installed_at_unix: 0,
        }
    }

    #[test]
    fn store_roundtrips_and_recovers_from_corruption() {
        let dir = tmp("store");
        let store = ModsStore::new(&dir);

        let (empty, backup) = store.load().unwrap();
        assert!(empty.installed.is_empty());
        assert!(backup.is_none());

        let mut data = ModsData::default();
        data.installed.push(row("sodium", "sodium.jar", &[]));
        store.save(&data).unwrap();
        let (back, _) = store.load().unwrap();
        assert_eq!(back.installed.len(), 1);
        assert_eq!(back.installed[0].project.project_id, "sodium");

        // Corrupt → backed up, error names the backup location.
        fs::write(dir.join("mods.json"), "{not json").unwrap();
        let err = store.load().unwrap_err();
        assert_eq!(err.code(), ErrorCode::ConfigInvalid);
        assert!(err.to_string().contains("corrupt"));
        assert!(dir.join("mods.json.corrupt").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsafe_filenames_are_rejected_before_any_download() {
        let dir = tmp("paths");
        assert!(safe_mod_path(&dir, "ok-mod.jar").is_ok());
        assert!(safe_mod_path(&dir, "../evil.jar").is_err());
        assert!(safe_mod_path(&dir, "sub/dir.jar").is_err());
        assert!(safe_mod_path(&dir, "script.exe").is_err());
        assert!(safe_mod_path(&dir, "").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disable_enable_rename_is_reversible_and_collision_checked() {
        let dir = tmp("toggle");
        let jar = dir.join("mod-a.jar");
        fs::write(&jar, b"x").unwrap();

        let disabled = disable_file(&jar).unwrap();
        assert_eq!(disabled, dir.join("mod-a.jar.disabled"));
        assert!(disabled.exists());
        assert!(!jar.exists());

        let back = enable_file(&disabled).unwrap();
        assert_eq!(back, jar);
        assert!(jar.exists());

        // Disabling twice without enabling → collision error, nothing lost.
        let _ = disable_file(&jar).unwrap();
        let dup = disable_file(&dir.join("mod-a.jar"));
        assert!(dup.is_err());
        assert!(dir.join("mod-a.jar.disabled").exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reconcile_classifies_all_three_states_precisely() {
        let dir = tmp("recon2");
        let mut data = ModsData::default();
        let mut off = row("lithium", "lithium.jar", &[]);
        off.enabled = false;
        data.installed.push(row("sodium", "sodium.jar", &[]));
        data.installed.push(off);
        data.installed.push(row("ghost", "ghost.jar", &[]));

        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("sodium.jar"), b"s").unwrap();
        fs::write(dir.join("lithium.jar.disabled"), b"l").unwrap();
        fs::write(dir.join("manual.jar"), b"m").unwrap();

        let inv = reconcile(&data, &dir);
        let find = |name: &str| inv.mods.iter().find(|e| e.filename == name).cloned();

        let sodium = find("sodium.jar").unwrap();
        assert_eq!(sodium.state, ManagedState::Managed);
        assert!(sodium.enabled);

        let lithium = find("lithium.jar").unwrap();
        assert_eq!(lithium.state, ManagedState::Managed);
        assert!(!lithium.enabled);

        let ghost = find("ghost.jar").unwrap();
        assert_eq!(ghost.state, ManagedState::Missing);
        assert!(ghost.warning.is_some());

        let manual = find("manual.jar").unwrap();
        assert_eq!(manual.state, ManagedState::External);
        assert!(manual.project.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_managed_touches_only_tracked_files() {
        let dir = tmp("rm");
        let mut data = ModsData::default();
        data.installed.push(row("mine", "mine.jar", &[]));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mine.jar"), b"m").unwrap();
        fs::write(dir.join("user-file.jar"), b"u").unwrap();

        // Removing something untracked is a no-op that returns false.
        assert!(!remove_managed(&mut data, &dir, &ProjectRef::new(SourceKind::Local, "nope")).unwrap());

        assert!(remove_managed(&mut data, &dir, &ProjectRef::new(SourceKind::Modrinth, "mine")).unwrap());
        assert!(!dir.join("mine.jar").exists());
        assert!(dir.join("user-file.jar").exists(), "external jar untouched");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn profiles_snapshot_and_switch() {
        let dir = tmp("profiles");
        let mut data = ModsData::default();
        data.installed.push(row("sodium", "sodium.jar", &[]));
        data.installed.push(row("lithium", "lithium.jar", &[]));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("sodium.jar"), b"s").unwrap();
        fs::write(dir.join("lithium.jar"), b"l").unwrap();

        // Snapshot current (both enabled) into a profile…
        let perf = create_profile_from_current(&data, "perf".into(), "Performance".into()).unwrap();
        data.profiles.push(perf);
        // …then disable lithium out-of-band and confirm switching restores it.
        data.installed[1].enabled = false;
        switch_profile(&mut data, &dir, Some("perf")).unwrap();
        assert!(data.installed.iter().all(|m| m.enabled));
        assert!(dir.join("lithium.jar").exists());
        assert!(!dir.join("lithium.jar.disabled").exists());

        // Unknown profile errors; None resets to all-enabled.
        assert!(switch_profile(&mut data, &dir, Some("nope")).is_err());
        switch_profile(&mut data, &dir, None).unwrap();
        assert_eq!(data.active_profile, None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_profiles_are_rejected() {
        let data = ModsData::default();
        assert!(create_profile_from_current(&data, "a".into(), "A".into()).is_ok());
        let mut data2 = ModsData::default();
        data2.profiles.push(ModProfile {
            id: "a".into(),
            name: "A".into(),
            enabled_projects: vec![],
            created_at_unix: 0,
        });
        assert!(create_profile_from_current(&data2, "a".into(), "B".into()).is_err());
        assert!(create_profile_from_current(&data2, "b".into(), "A".into()).is_err());
    }
}
