# Competitive & Ecosystem Research

Status: **Accepted** · Phase 0 deliverable · Confidence labels: **[V]** verified against primary sources · **[I]** engineering inference from public behavior · **[U]** unknown / do not assume

## Method

Proprietary clients (Lunar, Feather/Dawn, SKlauncher) publish no internal architecture documentation. This document analyzes only publicly observable behavior and never invents implementation detail. Where internals matter, we mark `[U]` and design so that the unknown does not affect our architecture.

## Lunar Client

- **What is publicly known [V]:** closed-source launcher + client; ships a large built-in module set (FPS/keystrokes/CPS HUDs, cosmetics, performance tweaks); supports a curated window of Minecraft versions rather than every release; auto-updating; distributed for Windows/macOS/Linux.
- **Strengths to learn from:** enormous first-party feature surface — validates Isekaiyo's "install and it's complete" philosophy; tight version-window curation keeps QA tractable; polished out-of-box UX.
- **Weaknesses / risks:** closed source (no community contribution, no auditability); version support gated by the vendor's porting effort; users cannot self-extend beyond what ships.
- **Architectural implication for Isekaiyo:** a large first-party module library *is* viable and valued, but it demands the capability/adapter layer ([version-architecture](../version-architecture.md)) so module ports are isolated, not global forks of the client per version. `[U]` how Lunar implements per-version injection internally; we design our own mechanism (ADR-0010) without assuming theirs.

## Feather / Dawn Client

- **Publicly observable [V/I]:** Feather positioned as a lightweight client with built-in modules plus support for adding Fabric mods; Dawn continues the line. Voice/social features advertised. Closed source.
- **Strengths:** demonstrates the hybrid model — first-party modules *plus* standard loader mods coexisting — which Isekaiyo adopts deliberately.
- **Weaknesses:** proprietary; feature availability tied to company roadmap.
- **Implication:** the client must treat "Isekaiyo module" and "Fabric mod" as separate systems that can be installed into the same instance without conflict assumptions (see [client-architecture](../client-architecture.md)).

## SKlauncher

- **Publicly observable [V/I]:** launcher offering multi-version management, multiple loaders, Modrinth/CurseForge integration surfaces, import features, offline profile options.
- **Strengths:** breadth — proves demand for one launcher covering versions + loaders + marketplace sources + imports.
- **Risks:** offline-profile handling in third-party launchers frequently blurs "local profile" vs "authenticated account". Isekaiyo makes this distinction explicit end-to-end (see [Authentication Architecture](../security.md#authentication)).
- **Implication:** marketplace aggregation and import tooling are table stakes; provenance labeling per source is a differentiator we adopt from day one.

## Prism Launcher

- **Verified [V]:** open-source fork of MultiMC (GitHub: PrismLauncher/PrismLauncher); instance-centric multi-install management; broad platform distribution including Flatpak nightly repo and GitHub Actions dev builds; translations via Weblate; GPL-family licensing inherited from MultiMC lineage.
- **Strengths:** the strongest public example of **instance isolation**, portability, and separation of concerns; mature metadata handling; healthy contributor model (Weblate, PR workflow).
- **Weaknesses (as observed by users/community):** UI density and dated interaction patterns; not designed around a first-party client layer.
- **Adopt:** instance-as-directory contract, validation, import/export discipline, packaging matrix realism ("maintainable packaging strategy", not "every distro"). **Avoid:** treating the instance list UI as an afterthought; Isekaiyo invests in UI polish equally.

## Modrinth App (Theseus)

- **Verified [V]:** Modrinth's desktop app lives in the `modrinth/code` monorepo alongside the website (per-package licenses). The app has been publicly described/built in Rust with Tauri. Modrinth provides an official developer API (docs.modrinth.com).
- **Strengths:** validates Rust+Tauri for exactly this problem domain at production scale; first-party API integration instead of scraping; modpack-first UX.
- **Weaknesses:** app is tightly coupled to the Modrinth ecosystem by design.
- **Implications:** (1) our Rust/Tauri choice has a working precedent in-domain; (2) marketplace architecture must be provider-plural from day one (`MarketplaceProvider` port), never hardwired to Modrinth even though Modrinth is our first integration; (3) respect official APIs, rate limits, attribution — codified in [marketplace](../marketplace.md).

## Ecosystem facts grounding the architecture

- **Mojang metadata [V, checked live 2026-08]:** `https://launchermeta.mojang.com/mc/game/version_manifest_v2.json` serves the authoritative version list with per-version JSON on `piston-meta.mojang.com`, SHA1-hashed, including `complianceLevel`. Notably the current release train has moved past the old `1.x` naming — direct proof that **hardcoding version knowledge is fatal**; discovery must be dynamic.
- **Fabric [V]:** lightweight, actively maintained loader ecosystem; official installer/maven artifacts; de-facto default for modern lightweight modding.
- **NeoForge [V]:** NeoForged project active (currently shipping 21.x-era APIs); forked from Forge in 2023 after licensing/governance disputes — loaders can and do fragment; our loader layer must treat each loader as an independent adapter behind a trait.
- **Forge [V]:** long-lived loader with deep legacy-version coverage; heavier install pipeline (installers, processors).
- **Quilt [V]:** Fabric-compatible community fork; treated as its own adapter that may reuse Fabric-adjacent logic where compatible.
- **CurseForge [U → OPEN DECISION]:** offers an API but access policies have changed repeatedly (keys restricted to approved applications). Status tracked in [Open Questions](../foundation-report.md#44-open-questions); Modrinth is first-party integrated, CurseForge is designed-for but gated on terms verification.

## Things Isekaiyo deliberately avoids (learned from all of the above)

1. Fork-based foundations inheriting unknown debt.
2. Vendor-gated extensibility (closed clients) — hence the plugin API.
3. Single-source marketplace coupling — hence the provider port.
4. Blurred local-vs-authenticated accounts — hence explicit account variants.
5. Version logic scattered globally — hence capabilities + adapters.
