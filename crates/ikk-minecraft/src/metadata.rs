//! Version metadata (`<version>.json` on piston-meta) — the full document a
//! launch needs: client download, libraries with platform rules, asset index,
//! Java requirement, arguments, logging config. Both modern (`arguments`) and
//! legacy (`minecraftArguments`) formats are modeled.

use ikk_core::error::{Error, ErrorCode, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::rules::Rule;

#[derive(Debug, Clone, Deserialize)]
pub struct VersionMetadata {
    pub id: String,
    #[serde(rename = "type", default)]
    pub version_type: Option<String>,
    #[serde(default)]
    pub release_time: Option<String>,
    pub main_class: String,
    /// Assets version id (e.g. "17") — names the asset index in game args.
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub java_version: Option<JavaRequirement>,
    pub downloads: Downloads,
    pub asset_index: AssetIndexRef,
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// Modern (1.13+) structured arguments.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Legacy (< 1.13) single-string game arguments.
    #[serde(rename = "minecraftArguments", default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub logging: Option<LoggingSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaRequirement {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    pub client: Downloadable,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloadable {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    /// Total size of all assets this index references.
    #[serde(rename = "totalSize", default)]
    pub total_size: u64,
}

/// A library coordinate `group:artifact:version[:classifier][:extra]`.
#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    /// Legacy natives map: os → classifier (pre-`downloads` era and some
    /// modern LWJGL entries).
    #[serde(default)]
    pub natives: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub extract: Option<ExtractRule>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<ArtifactRef>,
    #[serde(default)]
    pub classifiers: Option<BTreeMap<String, ArtifactRef>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArtifactRef {
    pub path: String,
    pub url: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
}

/// Extraction exclusions from metadata (e.g. META-INF, .git).
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractRule {
    #[serde(default, rename = "exclude")]
    pub exclude: Vec<String>,
}

/// Modern structured argument list; each entry is a plain string or a
/// rule-gated value (string or array of strings).
#[derive(Debug, Clone, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentValue>,
    #[serde(default)]
    pub jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Plain(String),
    Conditional { rules: Vec<Rule>, value: ArgList },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgList {
    One(String),
    Many(Vec<String>),
}

impl ArgumentValue {
    /// Expand to concrete strings if the rules permit on this platform.
    pub fn expand(&self, features: &dyn Fn(&str) -> bool) -> Option<Vec<String>> {
        match self {
            ArgumentValue::Plain(s) => Some(vec![s.clone()]),
            ArgumentValue::Conditional { rules, value } => {
                crate::rules::evaluate(rules, features).then(|| match value {
                    ArgList::One(s) => vec![s.clone()],
                    ArgList::Many(v) => v.clone(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingSection {
    pub client: LoggingClient,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingClient {
    /// JVM arg template containing `${path}`.
    pub argument: String,
    pub file: DownloadableFile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadableFile {
    pub id: String,
    pub url: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
}

impl VersionMetadata {
    pub fn parse(json: &str) -> Result<Self> {
        let meta: Self = serde_json::from_str(json).map_err(|e| {
            Error::with_source(
                ErrorCode::MetadataInvalid,
                format!("malformed version metadata: {e}"),
                e,
            )
        })?;
        if meta.id.trim().is_empty() || meta.main_class.trim().is_empty() {
            return Err(Error::new(
                ErrorCode::MetadataInvalid,
                "version metadata missing id or mainClass",
            ));
        }
        Ok(meta)
    }

    /// Required Java major version. Metadata before ~1.7.10 has none; those
    /// versions run on Java 8 — recorded as the floor, not a guess about the
    /// future.
    pub fn required_java_major(&self) -> u32 {
        self.java_version
            .as_ref()
            .map(|j| j.major_version)
            .unwrap_or(8)
    }

    /// The log4j config file reference, when the version ships one.
    pub fn logging_config(&self) -> Option<&DownloadableFile> {
        self.logging.as_ref().map(|l| &l.client.file)
    }

    /// The JVM argument template for the log4j config (`${path}` placeholder),
    /// when present.
    pub fn logging_argument(&self) -> Option<&str> {
        self.logging.as_ref().map(|l| l.client.argument.as_str())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::test_fixtures;

    #[test]
    fn parses_modern_metadata() {
        let meta = VersionMetadata::parse(test_fixtures::VERSION_METADATA_JSON).unwrap();
        assert_eq!(meta.id, "1.20.4");
        assert_eq!(meta.main_class, "net.minecraft.client.main.Main");
        assert_eq!(meta.required_java_major(), 17);
        assert_eq!(meta.libraries.len(), 5);
        assert!(meta.arguments.is_some());
        assert!(meta.minecraft_arguments.is_none());
        assert_eq!(
            meta.logging_config().map(|f| f.id.as_str()),
            Some("client-1.12.xml")
        );
    }

    #[test]
    fn malformed_metadata_is_rejected() {
        assert_eq!(
            VersionMetadata::parse("{").unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
        // id/mainClass are mandatory.
        assert!(VersionMetadata::parse(r#"{"id":"","mainClass":""}"#).is_err());
    }

    #[test]
    fn legacy_metadata_parses_without_arguments_object() {
        // Pre-1.13 shape: string args, no assetIndex totalSize etc.
        let json = r#"{
            "id": "b1.7.3",
            "mainClass": "net.minecraft.client.Minecraft",
            "minecraftArguments": "${auth_player_name} ${auth_session}",
            "assets": "legacy",
            "assetIndex": { "id": "legacy", "url": "https://x/index.json", "sha1": "abc", "size": 10 },
            "downloads": { "client": { "url": "https://x/client.jar", "sha1": "def", "size": 1 } },
            "javaVersion": null
        }"#;
        let meta = VersionMetadata::parse(json).unwrap();
        assert_eq!(meta.required_java_major(), 8);
        assert!(meta.arguments.is_none());
        assert_eq!(meta.minecraft_arguments.as_deref(), Some("${auth_player_name} ${auth_session}"));
    }
}
