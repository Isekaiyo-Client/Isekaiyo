//! Dependency resolution and compatibility filtering (Phase 6 §14–§17).
//!
//! Pure functions over domain types: no HTTP, no filesystem. The resolver
//! takes the requested project, a source-backed version lookup, and the
//! instance context (mc version + loader + already-installed set) and produces
//! either an explicit installation plan or a precise reason it cannot.

use super::{
    DependencyKind, InstalledMod, ProjectRef, ProjectVersion,
};
use ikk_core::error::{Error, ErrorCode, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Instance context the solver needs. Loader string matches
/// [`crate::loaders::LoaderId::as_str`].
#[derive(Debug, Clone)]
pub struct InstanceContext {
    pub game_version: String,
    pub loader: String,
}

impl InstanceContext {
    pub fn new(game_version: impl Into<String>, loader: impl Into<String>) -> Self {
        Self {
            game_version: game_version.into(),
            loader: loader.into(),
        }
    }
}

/// What the solver decided.
#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    /// New versions to download+install (requested mod first).
    pub to_install: Vec<ProjectVersion>,
    /// Already satisfied — nothing to transfer.
    pub already_installed: Vec<ProjectRef>,
    /// Required dependencies that exist in NO compatible version anywhere.
    pub unsatisfiable: Vec<String>,
    /// Projects currently installed that the requested mod declares
    /// INCOMPATIBLE with.
    pub conflicts: Vec<InstalledMod>,
    /// Human-readable summary for confirmation dialogs.
    pub summary: String,
}

impl InstallPlan {
    pub fn total_files(&self) -> usize {
        self.to_install.iter().filter(|v| v.primary_file().is_some()).count()
    }
}

/// A source-agnostic way to ask "which versions of this project exist?" —
/// implemented by the application layer over any [`super::source::ModSource`]
/// (or an in-memory map in tests).
pub trait VersionLookup {
    fn versions_of(&mut self, project_id: &str) -> Result<Vec<ProjectVersion>>;
}

/// In-memory lookup for tests and cached metadata.
#[derive(Debug, Default)]
pub struct MapLookup {
    pub by_project: BTreeMap<String, Vec<ProjectVersion>>,
}

impl VersionLookup for MapLookup {
    fn versions_of(&mut self, project_id: &str) -> Result<Vec<ProjectVersion>> {
        Ok(self.by_project.get(project_id).cloned().unwrap_or_default())
    }
}

/// Pick the best [`ProjectVersion`] of a list for the instance context:
/// newest published among those supporting (game, loader), preferring
/// `release` over beta/alpha at equal recency rank.
pub fn select_compatible<'a>(
    versions: &'a [ProjectVersion],
    ctx: &InstanceContext,
) -> Option<&'a ProjectVersion> {
    let mut candidates: Vec<&ProjectVersion> = versions
        .iter()
        .filter(|v| v.supports(&ctx.game_version, &ctx.loader))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    // Stable sort: release-type first, then newest published.
    candidates.sort_by(|a, b| {
        let rank = |v: &ProjectVersion| match v.release_type.as_str() {
            "release" => 0,
            "beta" => 1,
            _ => 2,
        };
        rank(b)
            .cmp(&rank(a))
            .then(b.published_unix.cmp(&a.published_unix))
    });
    candidates.first().copied()
}

/// Resolve the full transitive closure needed to install `root` into the
/// instance described by `ctx`, given what's already installed.
///
/// Guarantees:
/// - cycle-safe: a project is expanded once even if dependencies loop back
/// - required edges only drive downloads; optional edges are recorded but
///   never fetched; incompatible edges against installed mods fail loudly
/// - every chosen version supports the instance's mc+loader pair; otherwise
///   the project lands in `unsatisfiable` with a precise message
pub fn resolve(
    root_project_id: &str,
    ctx: &InstanceContext,
    lookup: &mut dyn VersionLookup,
    installed: &[InstalledMod],
) -> Result<InstallPlan> {
    let mut plan = InstallPlan::default();

    // Conflicts first: declared-incompatible projects already present.
    check_root_conflicts(root_project_id, ctx, lookup, installed, &mut plan)?;

    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = vec![root_project_id.to_owned()];

    // Installed projects indexed by source-project id for satisfaction checks.
    let installed_by_id: BTreeMap<&str, &InstalledMod> =
        installed.iter().map(|m| (m.project.project_id.as_str(), m)).collect();

    while let Some(project_id) = queue.pop() {
        if !visited.insert(project_id.clone()) {
            continue; // cycle / duplicate — expand once
        }

        if let Some(existing) = installed_by_id.get(project_id.as_str()) {
            plan.already_installed.push(existing.project.clone());
            continue;
        }

        let versions = lookup.versions_of(&project_id)?;
        let Some(chosen) = select_compatible(&versions, ctx) else {
            plan.unsatisfiable.push(format!(
                "{project_id}: no version supports Minecraft {} on {}",
                ctx.game_version, ctx.loader
            ));
            continue;
        };

        // Enqueue required dependencies not yet handled.
        for edge in &chosen.dependencies {
            match edge.kind {
                DependencyKind::Required => queue.push(edge.project_id.clone()),
                DependencyKind::Optional => { /* recorded in the version doc only */ }
                DependencyKind::Incompatible => {
                    if let Some(other) = installed_by_id.get(edge.project_id.as_str()) {
                        plan.conflicts.push((*other).clone());
                    }
                }
            }
        }
        plan.to_install.push(chosen.clone());
    }

    plan.summary = build_summary(&plan);
    Ok(plan)
}

fn check_root_conflicts(
    root_project_id: &str,
    ctx: &InstanceContext,
    lookup: &mut dyn VersionLookup,
    installed: &[InstalledMod],
    plan: &mut InstallPlan,
) -> Result<()> {
    // The root itself may declare installed projects incompatible.
    let versions = lookup.versions_of(root_project_id)?;
    if versions.is_empty() {
        return Err(Error::new(
            ErrorCode::MetadataInvalid,
            format!("project {root_project_id} has no known versions"),
        ));
    }
    let installed_by_id: BTreeMap<&str, &InstalledMod> =
        installed.iter().map(|m| (m.project.project_id.as_str(), m)).collect();
    for v in &versions {
        for edge in &v.dependencies {
            if edge.kind == DependencyKind::Incompatible {
                if let Some(other) = installed_by_id.get(edge.project_id.as_str()) {
                    plan.conflicts.push((*other).clone());
                }
            }
        }
    }
    // And installed mods may declare the root incompatible.
    for m in installed {
        let vs = lookup.versions_of(&m.project.project_id)?;
        for v in &vs {
            for edge in &v.dependencies {
                if edge.kind == DependencyKind::Incompatible
                    && edge.project_id == root_project_id
                    && v.supports(&ctx.game_version, &ctx.loader)
                {
                    plan.conflicts.push(m.clone());
                }
            }
        }
    }
    plan.conflicts.sort_by(|a, b| a.project.project_id.cmp(&b.project.project_id));
    plan.conflicts.dedup_by(|a, b| a.project == b.project);
    Ok(())
}

fn build_summary(plan: &InstallPlan) -> String {
    let names: Vec<String> = plan
        .to_install
        .iter()
        .map(|v| format!("{} {}", v.project.project_id, v.version_number))
        .collect();
    let mut s = if plan.to_install.len() == 1 {
        format!("Install {}", names.join(", "))
    } else {
        format!("Install {} mods: {}", plan.to_install.len(), names.join(", "))
    };
    if !plan.already_installed.is_empty() {
        s.push_str(&format!(
            " · {} already satisfied",
            plan.already_installed.len()
        ));
    }
    if !plan.unsatisfiable.is_empty() {
        s.push_str(" · ⚠ unsatisfiable deps");
    }
    s
}

/// Reverse-dependency analysis for removal (§32/§33): which *installed* mods
/// still require `candidate`, excluding `also_removing`?
pub fn reverse_dependencies(
    candidate: &ProjectRef,
    installed: &[InstalledMod],
    also_removing: &[ProjectRef],
) -> Vec<ProjectRef> {
    installed
        .iter()
        .filter(|m| {
            m.project != *candidate
                && !also_removing.contains(&m.project)
                && m.dependencies.iter().any(|d| {
                    d.kind == DependencyKind::Required && d.project_id == candidate.project.project_id
                })
        })
        .map(|m| m.project.clone())
        .collect()
}

/// Orphan detection (§33): installed managed mods nobody requires and that
/// were not explicitly requested by the user.
pub fn orphans(installed: &[InstalledMod], explicitly_requested: &[ProjectRef]) -> Vec<ProjectRef> {
    let all: BTreeSet<&str> = installed
        .iter()
        .map(|m| m.project.project_id.as_str())
        .collect();
    let required: BTreeSet<&str> = installed
        .iter()
        .flat_map(|m| {
            m.dependencies
                .iter()
                .filter(|d| d.kind == DependencyKind::Required)
                .map(|d| d.project_id.as_str())
        })
        .collect();
    let explicit: BTreeSet<&str> =
        explicitly_requested.iter().map(|p| p.project_id.as_str()).collect();
    all.into_iter()
        .filter(|id| !required.contains(id) && !explicit.contains(id))
        .filter_map(|id| {
            installed
                .iter()
                .find(|m| m.project.project_id == id)
                .map(|m| m.project.clone())
        })
        .collect()
}

/// Update detection (§24): compare an installed version against the best
/// compatible one now available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    Current,
    UpdateAvailable,
    /// No version supports the current mc+loader pair anymore.
    Incompatible,
    Unknown,
}

pub fn update_state(
    installed: &InstalledMod,
    available: &[ProjectVersion],
    ctx: &InstanceContext,
) -> UpdateState {
    let Some(best) = select_compatible(available, ctx) else {
        return UpdateState::Incompatible;
    };
    if best.version_id == installed.version_id {
        UpdateState::Current
    } else {
        UpdateState::UpdateAvailable
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::mods::{ModFile, SourceKind};

    fn ctx() -> InstanceContext {
        InstanceContext::new("1.20.4", "fabric")
    }

    fn pv(project_id: &str, number: &str, games: &[&str], loaders: &[&str], deps: &[(&str, DependencyKind)], published: u64) -> ProjectVersion {
        ProjectVersion {
            version_id: format!("{project_id}-{number}"),
            project: ProjectRef::new(SourceKind::Modrinth, project_id),
            version_number: number.into(),
            game_versions: games.iter().map(|s| s.to_string()).collect(),
            loaders: loaders.iter().map(|s| s.to_string()).collect(),
            release_type: "release".into(),
            published_unix: published,
            dependencies: deps
                .iter()
                .map(|(id, k)| DependencyEdge { project_id: id.to_string(), kind: *k })
                .collect(),
            files: vec![ModFile {
                filename: format!("{project_id}.jar"),
                url: format!("https://cdn/{project_id}.jar"),
                primary: true,
                size_bytes: 100,
                sha1: Some("aa".into()),
                sha512: None,
            }],
        }
    }

    fn installed(project_id: &str, deps: &[(&str, DependencyKind)]) -> InstalledMod {
        InstalledMod {
            project: ProjectRef::new(SourceKind::Modrinth, project_id),
            project_title: project_id.into(),
            version_id: format!("{project_id}-1"),
            version_number: "1".into(),
            filename: format!("{project_id}.jar"),
            sha1: None,
            dependencies: deps
                .iter()
                .map(|(id, k)| DependencyEdge { project_id: id.to_string(), kind: *k })
                .collect(),
            enabled: true,
            installed_at_unix: 0,
        }
    }

    fn lookup(entries: Vec<(&str, Vec<ProjectVersion>)>) -> MapLookup {
        MapLookup {
            by_project: entries
                .into_iter()
                .map(|(id, vs)| (id.to_string(), vs))
                .collect(),
        }
    }

    #[test]
    fn selects_newest_compatible_release() {
        let versions = vec![
            pv("a", "1.0", &["1.20.4"], &["fabric"], &[], 100),
            pv("a", "2.0", &["1.20.4"], &["fabric"], &[], 200),
            pv("a", "2.0-old-game", &["1.20.1"], &["fabric"], &[], 999),
            pv("a", "forge-only", &["1.20.4"], &["forge"], &[], 10_000),
        ];
        let picked = select_compatible(&versions, &ctx()).unwrap();
        assert_eq!(picked.version_number, "2.0");
        assert!(select_compatible(&versions, &InstanceContext::new("9.9.9", "fabric")).is_none());
    }

    #[test]
    fn resolves_transitive_chain_with_dedup_and_cycles() {
        // a → b → c → b (cycle back to b): must terminate, install 3 mods.
        let mut l = lookup(vec![
            ("a", vec![pv("a", "1", &["1.20.4"], &["fabric"], &[("b", DependencyKind::Required)], 1)]),
            ("b", vec![pv("b", "1", &["1.20.4"], &["fabric"], &[("c", DependencyKind::Required)], 1)]),
            ("c", vec![pv("c", "1", &["1.20.4"], &["fabric"], &[("b", DependencyKind::Required)], 1)]),
        ]);
        let plan = resolve("a", &ctx(), &mut l, &[]).unwrap();
        assert_eq!(plan.to_install.len(), 3);
        assert!(plan.unsatisfiable.is_empty());
        assert!(plan.summary.contains("3 mods"));
    }

    #[test]
    fn optional_deps_are_not_downloaded_but_required_are() {
        let mut l = lookup(vec![
            ("a", vec![pv("a", "1", &["1.20.4"], &["fabric"], &[
                ("opt", DependencyKind::Optional),
                ("req", DependencyKind::Required),
            ], 1)]),
            ("req", vec![pv("req", "1", &["1.20.4"], &["fabric"], &[], 1)]),
        ]);
        let plan = resolve("a", &ctx(), &mut l, &[]).unwrap();
        let ids: Vec<&str> = plan.to_install.iter().map(|v| v.project.project_id.as_str()).collect();
        assert!(ids.contains(&"req"));
        assert!(!ids.contains(&"opt"));
    }

    #[test]
    fn missing_compatible_dependency_is_named_precisely() {
        let mut l = lookup(vec![
            ("a", vec![pv("a", "1", &["1.20.4"], &["fabric"], &[("gone", DependencyKind::Required)], 1)]),
            ("gone", vec![pv("gone", "1", &["1.19.2"], &["forge"], &[], 1)]),
        ]);
        let plan = resolve("a", &ctx(), &mut l, &[]).unwrap();
        assert_eq!(plan.to_install.len(), 1);
        assert_eq!(plan.unsatisfiable.len(), 1);
        assert!(plan.unsatisfiable[0].contains("no version supports"));
    }

    #[test]
    fn already_installed_projects_are_satisfied_not_redownloaded() {
        let have = installed("b", &[]);
        let mut l = lookup(vec![
            ("a", vec![pv("a", "1", &["1.20.4"], &["fabric"], &[("b", DependencyKind::Required)], 1)]),
            ("b", vec![pv("b", "1", &["1.20.4"], &["fabric"], &[], 1)]),
        ]);
        let plan = resolve("a", &ctx(), &mut l, std::slice::from_ref(&have)).unwrap();
        assert_eq!(plan.to_install.len(), 1); // only a
        assert_eq!(plan.already_installed.len(), 1); // b satisfied
    }

    #[test]
    fn incompatible_declared_against_installed_conflicts() {
        let have = installed("badlib", &[]);
        let mut l = lookup(vec![
            ("a", vec![pv("a", "1", &["1.20.4"], &["fabric"], &[("badlib", DependencyKind::Incompatible)], 1)]),
        ]);
        let plan = resolve("a", &ctx(), &mut l, std::slice::from_ref(&have)).unwrap();
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].project.project_id, "badlib");
    }

    #[test]
    fn reverse_dependencies_exclude_co_removals() {
        let inst = vec![
            installed("api", &[]),
            installed("sodium", &[("api", DependencyKind::Required)]),
            installed("lithium", &[("api", DependencyKind::Required)]),
            installed("freestanding", &[]),
        ];
        let api = ProjectRef::new(SourceKind::Modrinth, "api");
        // Removing sodium too leaves lithium still needing api.
        let rdeps = reverse_dependencies(&api, &inst, &[ProjectRef::new(SourceKind::Modrinth, "sodium")]);
        assert_eq!(rdeps.len(), 1);
        assert_eq!(rdeps[0].project_id, "lithium");

        // Remove both dependents → no blockers.
        let rdeps = reverse_dependencies(
            &api,
            &inst,
            &[
                ProjectRef::new(SourceKind::Modrinth, "sodium"),
                ProjectRef::new(SourceKind::Modrinth, "lithium"),
            ],
        );
        assert!(rdeps.is_empty());
    }

    #[test]
    fn orphan_detection_ignores_required_and_explicit() {
        let inst = vec![
            installed("explicit-mod", &[]),
            installed("dep-lib", &[]),
            installed("user-mod", &[("dep-lib", DependencyKind::Required)]),
        ];
        let explicit = vec![ProjectRef::new(SourceKind::Modrinth, "explicit-mod")];
        let orphans = orphans(&inst, &explicit);
        assert!(orphans.is_empty(), "dep-lib is required by user-mod");

        let user_removed = vec![
            installed("explicit-mod", &[]),
            installed("dep-lib", &[]),
        ];
        let orphans = orphans(&user_removed, &explicit);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].project_id, "dep-lib");
    }

    #[test]
    fn update_states_are_classified() {
        let inst = installed("a", &[]);
        let same = vec![pv("a", "1", &["1.20.4"], &["fabric"], &[], 1)];
        let newer = vec![
            same[0].clone(),
            pv("a", "2", &["1.20.4"], &["fabric"], &[], 2),
        ];
        let none = vec![pv("a", "1", &["1.19.2"], &["forge"], &[], 5)];
        assert_eq!(update_state(&inst, &same, &ctx()), UpdateState::Current);
        assert_eq!(update_state(&inst, &newer, &ctx()), UpdateState::UpdateAvailable);
        assert_eq!(update_state(&inst, &none, &ctx()), UpdateState::Incompatible);
    }
}
