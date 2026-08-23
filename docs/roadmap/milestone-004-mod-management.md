# Milestone 004 — Mod Management & Modrinth Integration (Phase 6)

## Scope delivered

- `ikk-minecraft::mods` — five distinct domain concepts
  (`RemoteProject → ProjectVersion → ModFile → InstalledMod → ModProfile`)
- `mods::source::ModSource` provider trait; `ModrinthSource` against the
  official api.modrinth.com/v2 REST API (identifying User-Agent, strict serde,
  offline-testable parsers)
- `mods::resolver` — transitive dependency resolution with cycle protection,
  mc-version + loader compatibility filtering, required/optional/incompatible
  edge semantics, reverse-dependency and orphan analysis, update-state
  classification
- `mods::install` — staged installs reusing the verified download engine;
  atomic-at-the-set metadata commits; corrupt-state backup recovery;
  managed/external/missing reconciliation; reversible `.disabled` toggling;
  profiles that rename files instead of re-downloading
- Tauri commands: `mods_search`, `mods_compatible_versions`,
  `mods_install_plan`, `mods_install`, `mods_inventory`, `mods_set_enabled`,
  `mods_remove` (reverse-dep guarded, force path), `mods_updates`,
  `mods_list_profiles`, `mods_create_profile`, `mods_switch_profile`
- UI: Mods page (Installed / Browse tabs), debounced stale-guarded search,
  dependency-aware install confirmation dialog, inventory with state badges,
  profiles

## Verification status

Per user instruction this milestone was delivered **without compile/test
runs**. Before shipping, run:

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p ikk-minecraft -p ikk-api-types
pnpm typecheck && pnpm lint --max-warnings 0 && pnpm test && pnpm build
```

Manual acceptance (TEST A–F of the Phase 6 spec) requires a Windows desktop
build; none have been performed yet.

## Honest limits

- Only Modrinth is implemented; Local/Isekaiyo sources are enum variants only.
- Hash verification uses sha1 when the source provides it; files without a
  source hash are installed but surfaced in the report's `unverified` list —
  never silently trusted.
- Search pagination is wired (page parameter) but the UI ships page 1 only.
- Update application ("apply this update") is not implemented; detection is.
