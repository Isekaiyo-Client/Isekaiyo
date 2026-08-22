# Marketplace Architecture

Status: **Accepted** (CurseForge gated on OQ-3) · Companion to [Architecture](architecture.md)

## Provider Abstraction

```rust
#[async_trait]
pub trait MarketplaceProvider {
    fn id(&self) -> ProviderId;                       // "modrinth", "curseforge", "ikk-plugins"
    async fn search(&self, q: &SearchQuery) -> Result<SearchPage>;
    async fn project(&self, id: &ProjectId) -> Result<ProjectDetails>;
    async fn versions(&self, id: &ProjectId) -> Result<Vec<ProjectVersion>>;
    async fn download(&self, v: &ProjectVersion) -> Result<DownloadTask>;
    fn capabilities(&self) -> ProviderCapabilities;    // modpacks? shaders? plugins?
}
```

The UI renders a unified experience with **visible source provenance** on every card, detail page, and installed item ("from Modrinth · CC-BY-NC…"). No blind scraping: only official APIs.

## Providers

| Provider | Status | Notes |
|---|---|---|
| **Modrinth** | First integration | Official API (docs.modrinth.com) [V]; `.mrpack` modpacks first-class |
| **CurseForge** | Designed-for, gated | API access terms have historically changed; requires approved key + terms review (OQ-3) before enabling |
| **Isekaiyo Plugin Repository** | Phase 12 | Signed manifests, our own review process |
| Future repos | Via trait | e.g., self-hosted/organization catalogs |

## Rules of Engagement

1. Respect API terms, licenses, attribution requirements, redistribution rules, and rate limits — enforced in the provider adapter (rate limiter per provider), not left to callers.
2. License/attribution metadata is stored with installed mods and surfaced in instance export.
3. Marketplace metadata is untrusted input: schema-validated, size-capped, never executed.
4. Downloads go through the standard integrity pipeline (checksums when the provider offers them; provider signature where available).

## Dependency Resolution

`ikk-mods` resolves a **deterministic installation plan** before anything touches disk:

```text
query: target mod + version
  ├─ check mc version compatibility
  ├─ check loader compatibility + environment (client/server)
  ├─ walk required deps (recurse), collect optional deps (user-selectable)
  ├─ detect conflicts/incompatibilities → present, never auto-resolve silently
  └─ emit ResolutionPlan: pinned versions, artifact hashes, install paths
```

The plan is displayed for review on non-trivial installs, cached with the instance, and re-runnable offline from cache. "Just drop files into /mods" is an explicit anti-pattern.
