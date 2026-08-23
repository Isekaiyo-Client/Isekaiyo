//! The mod-source provider trait — the seam that keeps the application from
//! hard-coding Modrinth (spec §2/§58). A source knows how to search, fetch
//! project details, and list versions; everything above this trait (resolver,
//! installer, UI) is source-agnostic.
//!
//! HTTP lives inside implementations, never in domain code or UI. Parsing
//! helpers are exposed as pure functions so tests run offline against
//! fixtures.

use crate::mods::{ProjectRef, ProjectVersion, RemoteProject};
use ikk_core::error::Result;

/// One search hit: the project plus its relevance rank from the source.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub project: RemoteProject,
}

/// Pagination + filter parameters for [`ModSource::search`].
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub text: String,
    /// Filter to versions serving these Minecraft versions (empty = any).
    pub game_versions: Vec<String>,
    /// Filter to these loaders ("fabric", "quilt", …) (empty = any).
    pub loaders: Vec<String>,
    pub categories: Vec<String>,
    /// Source-specific sort key ("relevance", "downloads", "updated", …).
    pub sort: Option<String>,
    pub page: u32,
}

impl SearchQuery {
    pub fn is_first_page(&self) -> bool {
        self.page == 0 || self.page == 1
    }
}

/// The provider seam. Implementations must:
/// - keep ALL HTTP inside themselves
/// - never scrape HTML; use documented APIs only
/// - treat every response as untrusted input (validate before returning)
pub trait ModSource {
    fn kind(&self) -> crate::mods::SourceKind;

    fn search(&self, agent: &ureq::Agent, query: &SearchQuery) -> Result<Vec<SearchHit>>;

    fn project(&self, agent: &ureq::Agent, reference: &ProjectRef) -> Result<RemoteProject>;

    /// All known versions of a project. Compatibility filtering against a
    /// concrete mc+loader pair happens in [`crate::mods::resolver`], not here,
    /// so callers can build their own views of the same list.
    fn versions(&self, agent: &ureq::Agent, reference: &ProjectRef)
        -> Result<Vec<ProjectVersion>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_page_defaults() {
        let q = SearchQuery::default();
        assert!(q.is_first_page());
        let q = SearchQuery {
            page: 3,
            ..Default::default()
        };
        assert!(!q.is_first_page());
    }
}
