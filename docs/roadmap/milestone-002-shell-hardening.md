# Milestone 002 — Application Shell Hardening & Launch Boundary

Status: **Active** · Builds directly on [Milestone 001](milestone-001-foundation.md). Scope discipline: still no Minecraft runtime, no downloads, no auth — this milestone hardens the shell and makes every unavailable capability *honest* instead of hidden.

## Goal

> The launcher looks and behaves like a real product: full design-token theming, schema-versioned settings with migration, a real launch command that refuses honestly, typed version classification, and an About page that states facts.

## Delivered

- **Design tokens (spec §9):** all colors/spacing/motion live as semantic CSS custom properties (`--ikk-*`); sakura-pink is the default AMOLED identity; `modern`/`sakura` themes are token swaps only. No component hard-codes colors.
- **Settings schema v2:** `confirm_before_delete`, `animations_enabled`; explicit migration path in `ikk-core::config::migrate` with round-trip tests. Old v1 files load cleanly and re-persist as v2.
- **Motion preference honored app-wide** via the `no-anim` shell class + `prefers-reduced-motion`.
- **Launch boundary:** `launch_instance` Tauri command validates the instance exists, then returns the stable `runtime.unavailable` error until the runtime milestone. Home's Play button calls it and renders the honest state — no fake progress.
- **Typed version model:** `ikk-core::version::MinecraftVersion` — release/snapshot/beta/alpha classification, path-safety validation (ids become directory names), IPC-shaped serialization.
- **Navigation:** Accounts added as a marked-planned section; About is a real page (build info, license, source link) — moved out of Settings.
- **Components:** `Switch`, `SettingRow`, `ConfirmDialog` join the shared primitives; delete confirmation respects the user setting.

## Acceptance criteria

```text
[x] cargo fmt / clippy -D warnings clean on all compilable crates
[x] cargo test: 31 core+api-types tests green (incl. v1→v2 config migration, version classification)
[x] pnpm typecheck / lint --max-warnings 0 / vitest / build all green
[ ] Full cargo check --workspace on Windows/Linux/macOS CI (needs WebKitGTK locally; CI covers)
[ ] Manual tri-platform smoke test of Play → honest refusal, settings persistence across restart
```

## Explicitly NOT in this milestone

Minecraft metadata discovery, downloads, Java management, loaders, accounts/auth, marketplace, client modules, packaging. Each has an architecture doc; none may be stubbed into existence.

## Next recommended milestone

**003 — Version metadata foundation:** Mojang manifest fetch through an `HttpService` port, cached per ADR-0008 storage policy, feeding a real version selector in the instance form. This is the first networked feature and unlocks everything downstream (install, launch).
