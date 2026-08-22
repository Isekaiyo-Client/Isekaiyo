//! Deterministic artifact resolution: version metadata in, an exact list of
//! files to fetch out. No download decisions anywhere else in the codebase.
//!
//! Storage layout (shared content-addressed cache + per-instance state):
//! ```text
//! <cache>/versions/<id>/<id>.jar        client jar
//! <cache>/libraries/<metadata path>     classpath jars
//! <cache>/assets/indexes/<index>.json   asset index
//! <cache>/assets/objects/<h2>/<hash>    asset objects (shared, read-only)
//! <cache>/log-configs/<file id>         log4j configs
//! <instance>/game/natives-<os>/         extracted native libraries
//! ```

use ikk_core::error::{Error, ErrorCode, Result};
use std::path::{Path, PathBuf};

use crate::assets::AssetIndex;
use crate::metadata::VersionMetadata;
use crate::rules;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    ClientJar,
    Library,
    /// A natives jar: download like any artifact, then extract into the
    /// instance's natives dir.
    NativeJar,
    AssetIndex,
    Asset,
    LoggingConfig,
}

/// One file the installation needs. Hashes are verified by the downloader;
/// `None` hash means trust-on-transfer (only for legacy metadata without one).
#[derive(Debug, Clone)]
pub struct PlannedArtifact {
    pub url: String,
    pub sha1: Option<String>,
    pub size_hint: u64,
    pub dest: PathBuf,
    pub kind: ArtifactKind,
    /// Human label for progress display.
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub version_id: String,
    /// Stage 1: everything fetchable before the asset index is known.
    pub artifacts: Vec<PlannedArtifact>,
}

impl InstallPlan {
    pub fn total_bytes(&self) -> u64 {
        self.artifacts.iter().map(|a| a.size_hint).sum()
    }
}

/// Reject paths that escape their storage root (path traversal).
fn safe_relative(path: &str, what: &str) -> Result<PathBuf> {
    let p = Path::new(path);
    if p.is_absolute() || path.contains("..") || path.is_empty() {
        return Err(Error::new(
            ErrorCode::MetadataInvalid,
            format!("{what} contains an unsafe path: {path:?}"),
        ));
    }
    Ok(p.to_path_buf())
}

/// Build stage 1 of the plan: client jar, applicable libraries (+ native jars),
/// asset index, logging config.
pub fn plan_install(meta: &VersionMetadata, cache_root: &Path) -> Result<InstallPlan> {
    let ctx = rules::current_context();
    let mut artifacts = Vec::new();

    // --- client jar ---------------------------------------------------------
    let client = &meta.downloads.client;
    artifacts.push(PlannedArtifact {
        url: client.url.clone(),
        sha1: Some(client.sha1.clone()),
        size_hint: client.size,
        dest: cache_root.join("versions").join(&meta.id).join(format!("{}.jar", meta.id)),
        kind: ArtifactKind::ClientJar,
        label: format!("Minecraft {} client", meta.id),
    });

    // --- libraries ----------------------------------------------------------
    for lib in &meta.libraries {
        if !crate::resolve_lib_rules(lib, &ctx) {
            continue;
        }
        if let Some(artifact) = &lib.downloads.artifact {
            let dest = safe_relative(&artifact.path, "library artifact path")?;
            artifacts.push(PlannedArtifact {
                url: artifact.url.clone(),
                sha1: (!artifact.sha1.is_empty()).then(|| artifact.sha1.clone()),
                size_hint: artifact.size,
                dest: cache_root.join("libraries").join(dest),
                kind: ArtifactKind::Library,
                label: lib.name.clone(),
            });
        } else if lib.downloads.classifiers.is_none() && lib.natives.is_none() {
            // Very old metadata sometimes lacks `downloads`; we refuse rather
            // than guess Maven URLs (documented limitation for pre-1.7 era).
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                format!("library {} has no downloadable artifact", lib.name),
            ));
        }

        // Natives jar for this platform, when declared.
        if let Some(natives_map) = &lib.natives {
            if let Some(classifier) = natives_map.get(rules::os_name()) {
                let full_classifier = apply_native_suffix(classifier, &ctx);
                if let Some(native) =
                    lib.downloads.classifiers.as_ref().and_then(|c| c.get(&full_classifier))
                {
                    let dest = safe_relative(&native.path, "native artifact path")?;
                    artifacts.push(PlannedArtifact {
                        url: native.url.clone(),
                        sha1: (!native.sha1.is_empty()).then(|| native.sha1.clone()),
                        size_hint: native.size,
                        dest: cache_root.join("libraries").join(dest),
                        kind: ArtifactKind::NativeJar,
                        label: format!("{} ({})", lib.name, full_classifier),
                    });
                }
            }
        }
    }

    // --- asset index --------------------------------------------------------
    artifacts.push(PlannedArtifact {
        url: meta.asset_index.url.clone(),
        sha1: Some(meta.asset_index.sha1.clone()),
        size_hint: meta.asset_index.size,
        dest: cache_root
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", meta.asset_index.id)),
        kind: ArtifactKind::AssetIndex,
        label: format!("Asset index {}", meta.asset_index.id),
    });

    // --- logging config -----------------------------------------------------
    if let Some(file) = meta.logging_config() {
        artifacts.push(PlannedArtifact {
            url: file.url.clone(),
            sha1: Some(file.sha1.clone()),
            size_hint: file.size,
            dest: safe_relative(&file.id, "logging config id")?
                .file_name()
                .map_or_else(
                    || PathBuf::from("log-config.xml"),
                    |name| cache_root.join("log-configs").join(name),
                ),
            kind: ArtifactKind::LoggingConfig,
            label: format!("Logging config {}", file.id),
        });
    }

    Ok(InstallPlan {
        version_id: meta.id.clone(),
        artifacts,
    })
}

/// Legacy classifiers may contain `${arch}` — resolved to 64/32.
fn apply_native_suffix(classifier: &str, ctx: &rules::EvalContext) -> String {
    classifier.replace(
        "${arch}",
        if ctx.arch.starts_with("aarch64") || ctx.arch == "x86_64" {
            "64"
        } else {
            "32"
        },
    )
}

/// Stage 2: once the asset index is downloaded and verified, expand it into
/// per-object artifacts. Objects are skipped-by-hash on re-runs so this stays
/// cheap.
pub fn plan_assets(index: &AssetIndex, cache_root: &Path) -> Vec<PlannedArtifact> {
    index
        .unique_objects()
        .map(|(hash, size)| PlannedArtifact {
            url: AssetIndex::resource_url(hash),
            sha1: Some(hash.to_owned()),
            size_hint: size,
            dest: cache_root.join("assets").join("objects").join(&hash[..2]).join(hash),
            kind: ArtifactKind::Asset,
            label: format!("asset {hash}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures;

    fn fixture_meta() -> VersionMetadata {
        VersionMetadata::parse(test_fixtures::VERSION_METADATA_JSON).unwrap()
    }

    #[test]
    fn plan_is_deterministic_and_complete() {
        let meta = fixture_meta();
        let plan_a = plan_install(&meta, Path::new("/data")).unwrap();
        let plan_b = plan_install(&meta, Path::new("/data")).unwrap();
        assert_eq!(plan_a.artifacts.len(), plan_b.artifacts.len());

        // client + 3 always-applicable libs + 1 platform-native + asset index + logging config.
        // The disallow-linux lib applies everywhere except linux.
        let is_linux = rules::os_name() == "linux";
        let expected = if is_linux { 7 } else { 8 };
        assert_eq!(plan_a.artifacts.len(), expected);

        let client = plan_a
            .artifacts
            .iter()
            .find(|a| a.kind == ArtifactKind::ClientJar)
            .unwrap();
        assert_eq!(client.dest, PathBuf::from("/data/versions/1.20.4/1.20.4.jar"));
        assert_eq!(
            client.sha1.as_deref(),
            Some("3d50c9be6a2f0f1d0e0c9be6a2f0f1d0e0c9be6a")
        );
    }

    #[test]
    fn exactly_one_platform_native_per_os_rule_lib() {
        let meta = fixture_meta();
        let plan = plan_install(&meta, Path::new("/d")).unwrap();
        let natives: Vec<_> = plan.artifacts.iter().filter(|a| a.kind == ArtifactKind::NativeJar).collect();
        assert_eq!(natives.len(), 1, "fixture declares one native per OS");
        let os = rules::os_name();
        assert!(
            natives[0].label.ends_with(if os == "linux" { "linux" } else { "windows" }),
            "wrong native selected for {os}: {}",
            natives[0].label
        );
    }

    #[test]
    fn disallowed_library_is_excluded_on_linux_only() {
        let meta = fixture_meta();
        let plan = plan_install(&meta, Path::new("/d")).unwrap();
        let present = plan
            .artifacts
            .iter()
            .any(|a| a.label.contains("legacy-only"));
        assert_eq!(present, rules::os_name() != "linux");
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        let mut json = test_fixtures::VERSION_METADATA_JSON.to_string();
        json = json.replace(
            "com/mojang/brigadier/1.1.8/brigadier-1.1.8.jar",
            "../../../etc/passwd",
        );
        let meta = VersionMetadata::parse(&json).unwrap();
        let err = plan_install(&meta, Path::new("/data")).unwrap_err();
        assert_eq!(err.code(), ErrorCode::MetadataInvalid);
        assert!(err.to_string().contains("unsafe path"));
    }

    #[test]
    fn assets_expand_with_hash_layout() {
        let index = AssetIndex::parse(test_fixtures::ASSET_INDEX_JSON).unwrap();
        let assets = plan_assets(&index, Path::new("/data"));
        assert_eq!(assets.len(), 2);
        let first = &assets[0];
        assert!(first.dest.starts_with("/data/assets/objects"));
        let hash = first.sha1.as_ref().unwrap();
        let parent = first.dest.parent().unwrap().file_name().unwrap().to_str().unwrap();
        assert_eq!(parent, &hash[..2], "two-level hash layout");
        assert_eq!(
            first.url,
            AssetIndex::resource_url(hash),
            "URL derived from the same hash"
        );
    }
}
