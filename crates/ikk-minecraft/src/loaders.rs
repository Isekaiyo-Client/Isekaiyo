//! Loader provider architecture (Phase 5).
//!
//! Each loader is an independent provider behind one trait. The pipeline's
//! extension point is deliberately narrow: a provider turns
//! `(minecraft version, loader version)` into an *effective*
//! [`VersionMetadata`] document. Everything downstream — resolver,
//! downloader, Java, planner, process — already speaks that one shape, so
//! adding a loader never touches generic launcher code.
//!
//! Status matrix (honest):
//! - **Vanilla** — `VanillaProvider`, passes metadata through unchanged.
//! - **Fabric** — real: official `meta.fabricmc.net` v2 endpoints.
//! - **Quilt** — real: official `meta.quiltmc.org` v3 endpoints.
//! - **Forge/NeoForge** — foundation only: their installer-based model needs
//!   its own pipeline (installer execution, processors) and returns an
//!   explicit "not yet supported" error instead of pretending.

use ikk_core::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};

use crate::metadata::VersionMetadata;

/// Stable loader identifiers used across IPC, storage, and UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderId {
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl LoaderId {
    pub fn as_str(self) -> &'static str {
        match self {
            LoaderId::Vanilla => "vanilla",
            LoaderId::Fabric => "fabric",
            LoaderId::Quilt => "quilt",
            LoaderId::Forge => "forge",
            LoaderId::NeoForge => "neoforge",
        }
    }
}

/// One selectable loader version from a provider's meta service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderVersionEntry {
    /// e.g. "0.16.9" for fabric-loader.
    pub version: String,
    pub stable: bool,
}

/// What a provider resolves for a concrete (mc, loader) pair.
#[derive(Debug, Clone)]
pub struct ResolvedLoader {
    /// Effective version id for directories/classpath naming,
    /// e.g. "fabric-loader-0.16.9-1.20.4".
    pub profile_id: String,
    /// The merged metadata document (JSON text). Parsing + overlaying happens
    /// in the caller so providers stay pure data producers where practical.
    pub effective_metadata: VersionMetadata,
}

/// The one method every loader must implement.
pub trait LoaderProvider {
    fn id(&self) -> LoaderId;

    /// Available loader versions for a Minecraft version. Empty list = this
    /// loader does not support that Minecraft version (compatibility is
    /// data-driven from the meta services; no hand-built matrices).
    fn list_versions(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>>;

    /// Resolve the effective metadata for a concrete pair. Must NOT download
    /// game artifacts — only loader metadata + the loader's own libraries are
    /// declared inside the returned document.
    fn resolve(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
        loader_version: &str,
        vanilla_metadata_json: &str,
    ) -> Result<ResolvedLoader>;
}

// ---------------------------------------------------------------------------
// Vanilla — explicit no-loader strategy.
// ---------------------------------------------------------------------------

pub struct VanillaProvider;

impl LoaderProvider for VanillaProvider {
    fn id(&self) -> LoaderId {
        LoaderId::Vanilla
    }

    fn list_versions(
        &self,
        _agent: &ureq::Agent,
        _mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>> {
        Ok(Vec::new()) // nothing to select; UI hides the picker for vanilla
    }

    fn resolve(
        &self,
        _agent: &ureq::Agent,
        mc_version: &str,
        _loader_version: &str,
        vanilla_metadata_json: &str,
    ) -> Result<ResolvedLoader> {
        let meta = VersionMetadata::parse(vanilla_metadata_json)?;
        Ok(ResolvedLoader {
            profile_id: mc_version.to_owned(),
            effective_metadata: meta,
        })
    }
}

// ---------------------------------------------------------------------------
// Fabric — https://meta.fabricmc.net/v2
// ---------------------------------------------------------------------------

pub struct FabricProvider;

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";

#[derive(Debug, Deserialize)]
struct FabricLoaderVersion {
    #[serde(rename = "loader")]
    loader: FabricInner,
}

#[derive(Debug, Deserialize)]
struct FabricInner {
    #[serde(rename = "version")]
    version: String,
    stable: bool,
}

impl FabricProvider {
    fn parse_version_list(json: &str) -> Result<Vec<LoaderVersionEntry>> {
        let raw: Vec<FabricLoaderVersion> = serde_json::from_str(json).map_err(|e| {
            Error::with_source(
                ErrorCode::MetadataInvalid,
                "malformed Fabric loader list",
                e,
            )
        })?;
        Ok(raw
            .into_iter()
            .map(|v| LoaderVersionEntry {
                version: v.loader.version,
                stable: v.loader.stable,
            })
            .collect())
    }

    fn parse_profile(json: &str) -> Result<ResolvedLoader> {
        // The profile document IS a version JSON with inheritsFrom set;
        // validate by parsing before handing it on. The concrete profile_id
        // is assigned in `resolve` where the loader version is known.
        let meta = VersionMetadata::parse(json)?;
        if meta.inherits_from.is_none() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "Fabric profile document lacks inheritsFrom",
            ));
        }
        Ok(ResolvedLoader {
            profile_id: meta.id.clone(),
            effective_metadata: meta,
        })
    }
}

impl LoaderProvider for FabricProvider {
    fn id(&self) -> LoaderId {
        LoaderId::Fabric
    }

    fn list_versions(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>> {
        let url = format!("{FABRIC_META}/versions/loader/{mc_version}");
        Self::parse_version_list(&crate::fetch_text(agent, &url)?)
    }

    fn resolve(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
        loader_version: &str,
        vanilla_metadata_json: &str,
    ) -> Result<ResolvedLoader> {
        let url =
            format!("{FABRIC_META}/versions/loader/{mc_version}/{loader_version}/profile/json");
        let mut resolved = Self::parse_profile(&crate::fetch_text(agent, &url)?)?;
        resolved.profile_id = format!("fabric-loader-{loader_version}-{mc_version}");
        finish_overlay(resolved, vanilla_metadata_json)
    }
}

// ---------------------------------------------------------------------------
// Quilt — https://meta.quiltmc.org/v3 (independent service, same doc shape).
// ---------------------------------------------------------------------------

pub struct QuiltProvider;

const QUILT_META: &str = "https://meta.quiltmc.org/v3";

impl QuiltProvider {
    fn parse_version_list(json: &str) -> Result<Vec<LoaderVersionEntry>> {
        // v3 shape: [{ "version": "...", "html": ..., "maven": ... }]
        #[derive(Debug, Deserialize)]
        struct Entry {
            version: String,
            #[serde(default)]
            stable: Option<bool>,
        }
        let raw: Vec<Entry> = serde_json::from_str(json).map_err(|e| {
            Error::with_source(ErrorCode::MetadataInvalid, "malformed Quilt loader list", e)
        })?;
        Ok(raw
            .into_iter()
            .map(|v| LoaderVersionEntry {
                version: v.version,
                stable: v.stable.unwrap_or(true),
            })
            .collect())
    }

    fn parse_profile(json: &str) -> Result<ResolvedLoader> {
        let meta = VersionMetadata::parse(json)?;
        if meta.inherits_from.is_none() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "Quilt profile document lacks inheritsFrom",
            ));
        }
        Ok(ResolvedLoader {
            profile_id: String::new(),
            effective_metadata: meta,
        })
    }
}

impl LoaderProvider for QuiltProvider {
    fn id(&self) -> LoaderId {
        LoaderId::Quilt
    }

    fn list_versions(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>> {
        let url = format!("{QUILT_META}/versions/loader/{mc_version}");
        Self::parse_version_list(&crate::fetch_text(agent, &url)?)
    }

    fn resolve(
        &self,
        agent: &ureq::Agent,
        mc_version: &str,
        loader_version: &str,
        vanilla_metadata_json: &str,
    ) -> Result<ResolvedLoader> {
        let url =
            format!("{QUILT_META}/versions/loader/{mc_version}/{loader_version}/profile/json");
        let mut resolved = Self::parse_profile(&crate::fetch_text(agent, &url)?)?;
        resolved.profile_id = format!("quilt-loader-{loader_version}-{mc_version}");
        finish_overlay(resolved, vanilla_metadata_json)
    }
}

// ---------------------------------------------------------------------------
// Forge / NeoForge — explicit foundation, not a fake implementation.
// ---------------------------------------------------------------------------

/// Forge's installation model (installer process + processors + binary-patch
/// data) requires its own pipeline that does not exist yet. This provider
/// exists so the registry/UI can enumerate it honestly; every operation
/// returns a stable "unsupported" error rather than pretending.
pub struct ForgeFoundationProvider {
    /// True for NeoForge, false for classic Forge.
    pub neo: bool,
}

impl LoaderProvider for ForgeFoundationProvider {
    fn id(&self) -> LoaderId {
        if self.neo {
            LoaderId::NeoForge
        } else {
            LoaderId::Forge
        }
    }

    fn list_versions(
        &self,
        _agent: &ureq::Agent,
        _mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>> {
        Err(Error::new(
            ErrorCode::MetadataInvalid,
            "Forge support is not implemented yet — the installer-based \
             pipeline arrives in a later milestone",
        ))
    }

    fn resolve(
        &self,
        _agent: &ureq::Agent,
        _mc_version: &str,
        _loader_version: &str,
        _vanilla_metadata_json: &str,
    ) -> Result<ResolvedLoader> {
        Err(Error::new(
            ErrorCode::MetadataInvalid,
            "Forge support is not implemented yet — the installer-based \
             pipeline arrives in a later milestone",
        ))
    }
}

// ---------------------------------------------------------------------------
// Registry + shared overlay completion.
// ---------------------------------------------------------------------------

/// All known providers; the application layer picks by [`LoaderId`].
pub fn all_providers() -> Vec<Box<dyn LoaderProvider>> {
    vec![
        Box::new(VanillaProvider),
        Box::new(FabricProvider),
        Box::new(QuiltProvider),
        Box::new(ForgeFoundationProvider { neo: false }),
        Box::new(ForgeFoundationProvider { neo: true }),
    ]
}

pub fn provider_for(id: LoaderId) -> Result<Box<dyn LoaderProvider>> {
    all_providers()
        .into_iter()
        .find(|p| p.id() == id)
        .ok_or_else(|| Error::new(ErrorCode::Internal, "no such loader"))
}

/// Shared tail of Fabric/Quilt resolution: overlay the loader's profile onto
/// the vanilla metadata so the result is one self-contained document.
fn finish_overlay(
    mut resolved: ResolvedLoader,
    vanilla_metadata_json: &str,
) -> Result<ResolvedLoader> {
    let vanilla = VersionMetadata::parse(vanilla_metadata_json)?;
    let child = resolved.effective_metadata;
    resolved.effective_metadata = child.overlay_on(vanilla);
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures as fx;

    const FABRIC_LIST_JSON: &str = r#"[
        { "loader": { "version": "0.16.9", "stable": true } },
        { "loader": { "version": "0.16.8", "stable": true } },
        { "loader": { "version": "0.17.0-beta.1", "stable": false } }
    ]"#;

    const QUILT_LIST_JSON: &str = r#"[
        { "version": "0.26.0-beta.1", "stable": false },
        { "version": "0.25.0" }
    ]"#;

    /// Mirrors meta.fabricmc.net's profile/json: inherits vanilla, adds the
    /// loader library, swaps mainClass to Knot.
    const FABRIC_PROFILE_JSON: &str = r#"{
        "id": "fabric-loader-0.16.9-1.20.4",
        "inheritsFrom": "1.20.4",
        "releaseTime": "2024-01-01T00:00:00+00:00",
        "time": "2024-01-01T00:00:00+00:00",
        "type": "release",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "libraries": [
            {
                "name": "net.fabricmc:fabric-loader:0.16.9",
                "url": "https://maven.fabricmc.net/"
            }
        ]
    }"#;

    #[test]
    fn fabric_list_parses_with_stability_flags() {
        let versions = FabricProvider::parse_version_list(FABRIC_LIST_JSON).unwrap();
        assert_eq!(versions.len(), 3);
        assert!(versions[0].stable);
        assert!(!versions[2].stable);
        assert_eq!(versions[0].version, "0.16.9");
    }

    #[test]
    fn quilt_list_is_independent_of_fabric_shape() {
        let versions = QuiltProvider::parse_version_list(QUILT_LIST_JSON).unwrap();
        assert_eq!(versions.len(), 2);
        assert!(!versions[0].stable, "beta flagged");
        assert!(versions[1].stable, "missing flag defaults to stable");
        assert_ne!(versions[0].version, versions[1].version);
    }

    #[test]
    fn malformed_loader_lists_are_rejected() {
        assert_eq!(
            FabricProvider::parse_version_list("{").unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
        assert_eq!(
            QuiltProvider::parse_version_list("[nope]")
                .unwrap_err()
                .code(),
            ErrorCode::MetadataInvalid
        );
    }

    #[test]
    fn fabric_profile_parses_and_requires_inherits_from() {
        let resolved = FabricProvider::parse_profile(FABRIC_PROFILE_JSON).unwrap();
        assert_eq!(
            resolved.effective_metadata.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert!(resolved.effective_metadata.client_download().is_none());

        let no_inherit = FABRIC_PROFILE_JSON.replace("\"inheritsFrom\": \"1.20.4\",", "");
        assert_eq!(
            FabricProvider::parse_profile(&no_inherit)
                .unwrap_err()
                .code(),
            ErrorCode::MetadataInvalid
        );
    }

    #[test]
    fn overlay_merges_child_over_vanilla() {
        let vanilla = VersionMetadata::parse(fx::VERSION_METADATA_JSON).unwrap();
        let child = VersionMetadata::parse(FABRIC_PROFILE_JSON).unwrap();
        let merged = child.overlay_on(vanilla);

        assert_eq!(merged.id, "fabric-loader-0.16.9-1.20.4");
        // Child mainClass wins.
        assert_eq!(
            merged.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        // Libraries concatenate: 1 fabric + 3 vanilla.
        assert_eq!(merged.libraries.len(), 4);
        // Inherited pieces arrive intact.
        assert!(merged.client_download().is_some());
        assert!(merged.asset_index().is_some());
        assert_eq!(merged.required_java_major(), 17);
        // Vanilla game args survive alongside the child's.
        assert!(merged.arguments.as_ref().unwrap().game.len() >= 7);

        // And the merged document resolves into a full install plan.
        let plan = crate::resolve::plan_install(&merged, std::path::Path::new("/data")).unwrap();
        let labels: Vec<_> = plan.artifacts.iter().map(|a| a.label.clone()).collect();
        assert!(
            labels.iter().any(|l| l.contains("fabric-loader")),
            "loader lib planned"
        );
    }

    #[test]
    fn forge_provider_refuses_honestly() {
        let forge = ForgeFoundationProvider { neo: false };
        let agent = ureq::AgentBuilder::new().build();
        let err = forge.list_versions(&agent, "1.20.4").unwrap_err();
        assert!(err.to_string().contains("not implemented yet"));
        let err = forge
            .resolve(&agent, "1.20.4", "49.0.30", "{}")
            .unwrap_err();
        assert!(err.to_string().contains("not implemented yet"));
    }

    #[test]
    fn registry_returns_each_known_provider() {
        assert_eq!(
            provider_for(LoaderId::Vanilla).unwrap().id(),
            LoaderId::Vanilla
        );
        assert_eq!(
            provider_for(LoaderId::Fabric).unwrap().id(),
            LoaderId::Fabric
        );
        assert_eq!(provider_for(LoaderId::Quilt).unwrap().id(), LoaderId::Quilt);
        assert_eq!(provider_for(LoaderId::Forge).unwrap().id(), LoaderId::Forge);
        assert_eq!(
            provider_for(LoaderId::NeoForge).unwrap().id(),
            LoaderId::NeoForge
        );
    }

    #[test]
    fn vanilla_passthrough_keeps_document() {
        let agent = ureq::AgentBuilder::new().build();
        let resolved = VanillaProvider
            .resolve(&agent, "1.20.4", "", fx::VERSION_METADATA_JSON)
            .unwrap();
        assert_eq!(resolved.profile_id, "1.20.4");
        assert_eq!(resolved.effective_metadata.id, "1.20.4");
    }
}
