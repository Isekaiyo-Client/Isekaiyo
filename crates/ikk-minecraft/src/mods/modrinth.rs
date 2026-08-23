//! Official Modrinth API (api.modrinth.com/v2) client.
//!
//! Rules honored here:
//! - documented REST API only, no scraping
//! - a User-Agent identifying the app (Modrinth asks for one)
//! - every response parsed through strict serde into domain types; unknown
//!   fields ignored so additive API changes don't break us
//! - all parse functions are pure → unit tests run offline on fixtures

use super::source::{ModSource, SearchHit, SearchQuery};
use super::{
    DependencyEdge, DependencyKind, ModFile, ProjectRef, ProjectVersion, RemoteProject,
    SourceKind,
};
use ikk_core::error::{Error, ErrorCode, Result};

const API_BASE: &str = "https://api.modrinth.com/v2";
/// Modrinth requests identifying UAs; keep in sync with the project.
pub const USER_AGENT: &str = concat!(
    "Isekaiyo/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/Isekaiyo-Client/Isekaiyo)"
);

// ---------------------------------------------------------------------------
// Wire shapes (serde) — field names match the v2 REST docs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RawSearchDoc {
    hits: Vec<RawHit>,
}

#[derive(Debug, Deserialize)]
struct RawHit {
    #[serde(rename = "project_id")]
    project_id: String,
    title: String,
    description: String,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default)]
    icon_url: Option<String>,
    downloads: u64,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    display_categories: Vec<String>,
    // Per-version loaders/game-versions come from /version; search hits carry
    // display hints only. Unknown fields are ignored by design.
    #[serde(default)]
    versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawVersion {
    id: String,
    #[serde(rename = "project_id")]
    project_id: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    version_type: String,
    date_published: Option<String>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(Debug, Deserialize)]
struct RawDependency {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    dependency_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFile {
    url: String,
    filename: String,
    primary: bool,
    size: u64,
    hashes: std::collections::BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Pure parsing — offline-testable.
// ---------------------------------------------------------------------------

/// Parse a `/v2/search` response body.
pub fn parse_search(json: &str) -> Result<Vec<SearchHit>> {
    let doc: RawSearchDoc = serde_json::from_str(json)
        .map_err(|e| Error::with_source(ErrorCode::MetadataInvalid, "malformed Modrinth search response", e))?;
    Ok(doc.hits.into_iter().map(hit_to_project).map(|project| SearchHit { project }).collect())
}

fn hit_to_project(h: RawHit) -> RemoteProject {
    // Prefer display categories (curated), fall back to raw ones; loader tags
    // arrive via `categories`/`display_categories` too but the authoritative
    // per-version loaders come from /version — these are display hints only.
    let mut categories = h.display_categories;
    if categories.is_empty() {
        categories = h.categories;
    }
    RemoteProject {
        reference: ProjectRef::new(SourceKind::Modrinth, h.project_id),
        title: h.title,
        description: h.description,
        authors: h.authors,
        icon_url: h.icon_url,
        downloads: h.downloads,
        categories,
        loaders: Vec::new(),
        game_versions: h.versions,
    }
}

/// Parse one `/v2/project/<id>` body.
pub fn parse_project(json: &str) -> Result<RemoteProject> {
    let raw: RawHit = serde_json::from_str(json).map_err(|e| {
        Error::with_source(ErrorCode::MetadataInvalid, "malformed Modrinth project", e)
    })?;
    Ok(hit_to_project(raw))
}

fn dependency_kind(s: &str) -> DependencyKind {
    match s {
        "optional" => DependencyKind::Optional,
        "incompatible" => DependencyKind::Incompatible,
        _ => DependencyKind::Required,
    }
}

fn file_from_raw(f: RawFile) -> ModFile {
    ModFile {
        filename: f.filename,
        url: f.url,
        primary: f.primary,
        size_bytes: f.size,
        sha1: f.hashes.get("sha1").cloned(),
        sha512: f.hashes.get("sha512").cloned(),
    }
}

/// Parse one `/v2/version` body (a JSON array).
pub fn parse_versions(json: &str) -> Result<Vec<ProjectVersion>> {
    let raw: Vec<RawVersion> = serde_json::from_str(json).map_err(|e| {
        Error::with_source(ErrorCode::MetadataInvalid, "malformed Modrinth versions", e)
    })?;
    Ok(raw
        .into_iter()
        .map(|v| ProjectVersion {
            version_id: v.id,
            project: ProjectRef::new(SourceKind::Modrinth, v.project_id),
            version_number: if v.version_number.is_empty() { v.id.clone() } else { v.version_number },
            game_versions: v.game_versions,
            loaders: v.loaders,
            release_type: v.version_type,
            published_unix: v
                .date_published
                .and_then(|d| iso8601_unix(&d))
                .unwrap_or(0),
            dependencies: v
                .dependencies
                .into_iter()
                .filter_map(|d| {
                    d.project_id.map(|pid| DependencyEdge {
                        project_id: pid,
                        kind: dependency_kind(d.dependency_type.as_deref().unwrap_or("required")),
                    })
                })
                .collect(),
            files: v.files.into_iter().map(file_from_raw).collect(),
        })
        .collect())
}

/// Minimal ISO-8601 (`2024-01-01T12:00:00Z`) → unix seconds. Returns `None`
/// for anything we can't parse rather than guessing.
fn iso8601_unix(s: &str) -> Option<u64> {
    let s = s.trim_end_matches('Z').trim_end_matches("+00:00");
    let mut parts = s.split(['-', 'T', ':']);
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    let hour: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minute: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let second: i64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    // Days-from-civil algorithm (Howard Hinnant); valid for all dates.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    u64::try_from(secs).ok()
}

// ---------------------------------------------------------------------------
// HTTP client implementing the source trait.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ModrinthSource;

impl ModSource for ModrinthSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Modrinth
    }

    fn search(&self, agent: &ureq::Agent, query: &SearchQuery) -> Result<Vec<SearchHit>> {
        let facets = build_facets(query);
        let sort = query.sort.clone().unwrap_or_else(|| "relevance".into());
        let index = match sort.as_str() {
            "downloads" => "downloads",
            "updated" => "updated",
            "newest" => "newest",
            _ => "relevance",
        };
        let url = format!(
            "{API_BASE}/search?limit=20&index={index}&offset={}&query={}&facets={facets}",
            query.page.saturating_sub(1) * 20,
            urlencode(&query.text),
        );
        let json = crate::fetch_text_with(agent, &url, USER_AGENT)?;
        parse_search(&json)
    }

    fn project(&self, agent: &ureq::Agent, reference: &ProjectRef) -> Result<RemoteProject> {
        let url = format!("{API_BASE}/project/{}", reference.project_id);
        let json = crate::fetch_text_with(agent, &url, USER_AGENT)?;
        parse_project(&json)
    }

    fn versions(
        &self,
        agent: &ureq::Agent,
        reference: &ProjectRef,
    ) -> Result<Vec<ProjectVersion>> {
        let url = format!("{API_BASE}/project/{}/version", reference.project_id);
        let json = crate::fetch_text_with(agent, &url, USER_AGENT)?;
        parse_versions(&json)
    }
}

/// Build the Modrinth `facets` query parameter from our filter struct.
/// Facets are ANDed groups of ORed terms: `[["versions:1.20.4"],["categories:fabric"]]`.
fn build_facets(query: &SearchQuery) -> String {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let push_group = |groups: &mut Vec<Vec<String>>, prefix: &str, values: &[String]| {
        if !values.is_empty() {
            groups.push(
                values
                    .iter()
                    .map(|v| format!("{prefix}:{}", v.to_ascii_lowercase()))
                    .collect(),
            );
        }
    };
    push_group(&mut groups, "versions", &query.game_versions);
    push_group(&mut groups, "categories", &query.loaders);
    push_group(&mut groups, "categories", &query.categories);
    if groups.is_empty() {
        return String::new();
    }
    serde_json::to_string(&groups).unwrap_or_default()
}

/// Percent-encode the reserved set for query strings without a URL crate.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const SEARCH_JSON: &str = r#"{
        "hits": [
            {
                "project_id": "AANobbMI",
                "slug": "sodium",
                "title": "Sodium",
                "description": "Modern rendering engine",
                "authors": ["jellysquid3"],
                "icon_url": "https://cdn.modrinth.com/icon.png",
                "downloads": 5000000,
                "categories": ["fabric", "optimization"],
                "display_categories": ["optimization"],
                "versions": ["1.20.4", "1.20.1"]
            },
            {
                "project_id": "no-icon",
                "title": "No Icon Mod",
                "description": "",
                "downloads": 1
            }
        ],
        "offset": 0,
        "limit": 20,
        "total_hits": 2
    }"#;

    const VERSIONS_JSON: &str = r#"[
        {
            "id": "ver1",
            "project_id": "AANobbMI",
            "version_number": "0.5.0",
            "game_versions": ["1.20.4"],
            "loaders": ["fabric"],
            "version_type": "release",
            "date_published": "2024-01-05T10:30:00Z",
            "dependencies": [
                {"project_id": "P7dR8mSH", "dependency_type": "required"},
                {"project_id": "optX", "dependency_type": "optional"},
                {"dependency_type": "required"}
            ],
            "files": [
                {
                    "url": "https://cdn.modrinth.com/sodium.jar",
                    "filename": "sodium-0.5.0.jar",
                    "primary": true,
                    "size": 1024,
                    "hashes": {"sha1": "abc123", "sha512": "ff"}
                },
                {
                    "url": "https://cdn.modrinth.com/sodium-sources.jar",
                    "filename": "sodium-0.5.0-sources.jar",
                    "primary": false,
                    "size": 512,
                    "hashes": {}
                }
            ]
        }
    ]"#;

    #[test]
    fn search_parses_hits_into_projects() {
        let hits = parse_search(SEARCH_JSON).unwrap();
        assert_eq!(hits.len(), 2);
        let sodium = &hits[0].project;
        assert_eq!(sodium.reference.project_id, "AANobbMI");
        assert_eq!(sodium.title, "Sodium");
        assert_eq!(sodium.authors, vec!["jellysquid3"]);
        assert_eq!(sodium.categories, vec!["optimization"]);
        assert_eq!(sodium.game_versions.len(), 2);
        assert!(sodium.icon_url.is_some());
        // Missing optional fields must not fail parsing.
        assert!(hits[1].project.icon_url.is_none());
    }

    #[test]
    fn malformed_search_is_rejected() {
        assert_eq!(
            parse_search("{").unwrap_err().code(),
            ErrorCode::MetadataInvalid
        );
    }

    #[test]
    fn versions_parse_with_dependencies_and_files() {
        let versions = parse_versions(VERSIONS_JSON).unwrap();
        assert_eq!(versions.len(), 1);
        let v = &versions[0];
        assert_eq!(v.version_number, "0.5.0");
        assert_eq!(v.release_type, "release");
        // The dependency with no project_id is dropped, others survive.
        assert_eq!(v.dependencies.len(), 2);
        assert_eq!(v.dependencies[0].kind, DependencyKind::Required);
        assert_eq!(v.dependencies[1].kind, DependencyKind::Optional);
        // Files carry both hashes.
        assert_eq!(v.files[0].sha1.as_deref(), Some("abc123"));
        assert_eq!(v.files[0].sha512.as_deref(), Some("ff"));
        assert_eq!(v.published_unix, 1704450600);
        assert!(v.supports("1.20.4", "fabric"));
        assert!(!v.supports("1.19.2", "fabric"));
    }

    #[test]
    fn facets_encode_as_and_of_ors() {
        let q = SearchQuery {
            game_versions: vec!["1.20.4".into()],
            loaders: vec!["Fabric".into()],
            ..Default::default()
        };
        let facets = build_facets(&q);
        assert_eq!(facets, r#"[["versions:1.20.4"],["categories:fabric"]]"#);

        assert_eq!(build_facets(&SearchQuery::default()), "");
    }

    #[test]
    fn urlencoding_matches_rfc3986_unreserved() {
        assert_eq!(urlencode("sodium"), "sodium");
        assert_eq!(urlencode("a b&c=d"), "a%20b%26c%3Dd");
    }

    #[test]
    fn iso8601_parsing_is_conservative() {
        assert_eq!(iso8601_unix("2024-01-05T10:30:00Z"), Some(1704450600));
        assert_eq!(iso8601_unix("2024-01-05T10:30:00.123+00:00").is_some(), true);
        assert_eq!(iso8601_unix("not-a-date"), None);
        assert_eq!(iso8601_unix(""), None);
    }

    #[test]
    fn user_agent_identifies_the_app() {
        assert!(USER_AGENT.starts_with("Isekaiyo/"));
    }
}
