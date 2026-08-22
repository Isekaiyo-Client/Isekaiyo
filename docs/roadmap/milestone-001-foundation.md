# Milestone 001 — Foundation Vertical Slice

Status: **Implemented (pending tri-OS CI confirmation)** · The first real implementation milestone. Scope discipline: nothing here proves Minecraft launching; everything here proves the *skeleton* works end-to-end on three OSes.

## Goal

> Clone → setup → doctor ✓ → app launches on Windows/Linux/macOS → UI talks to Rust → config loads/saves → structured logs exist → typed errors exist → instance model exists as data.

## Acceptance criteria

```text
[x] Repository builds: ikk-core, ikk-api-types, xtask compile + pass clippy -D warnings (verified locally, Rust 1.98)
[ ] Repository builds on Win/macOS CI (needs first pushed CI run)
[x] Tests execute: cargo test — 26 tests green (config recovery, instance CRUD, validation, IPC DTOs)
[ ] CI executes: full ci.yml pipeline green on a pushed branch
[x] Frontend checks green: pnpm typecheck && pnpm lint && pnpm test (vitest 7) && pnpm build
[ ] UI launches: tauri dev opens the shell window (blocked in the authoring sandbox: no WebKitGTK system deps; code reviewed, not launched)
[x] Core initializes: startup sequence logging → platform paths → config → instance store → UI (ikk-launcher/src/lib.rs)
[x] Configuration works: versioned JSON store, atomic saves, corrupt-file backup + recovery, serde defaults for forward compat
[x] Error handling exists: stable ErrorCode taxonomy surfaced as CommandError{code,message} to the UI; corrupt config never crashes
[x] Basic instance model exists: Instance + LoaderSpec with validation invariants, unit-tested (data only — no launching)
[x] Architecture enforcement runs: cargo xtask arch passes (verified live) and is in CI
[x] Docs match implementation: README/architecture/dependency-rules updated with the slice
[ ] Second developer reproduces: someone other than the author completes getting-started.md unassisted
```

## What was built (Milestone 001 implementation)

- `ikk-core`: `config` (versioned store), `instance` + `store` (domain + JSON persistence), `platform` (data dirs), extended error taxonomy
- `ikk-api-types`: `CommandError`, `ConfigLoadInfo`, re-exported domain types — single source of truth for IPC
- `ikk-launcher`: startup sequence + 8 typed commands (system info, startup info, config get/set, instances CRUD + select)
- `launcher-ui`: design-token stylesheet with 3 themes, shared UI primitives, real navigation, Home / Instances (full CRUD + dialogs + empty/loading/error states) / Settings, honest placeholders for Mods/Marketplace/Client
- Tests: 26 Rust unit tests + 7 vitest tests; CI runs both

## Explicitly OUT of scope

Version manifest fetching, Java management, authentication, any download, mod resolution, marketplace, plugins, theming beyond design tokens.

## Exit gate

All boxes checked + a tagged `v0.1.0-alpha.1` build produced by nightly.yml. Then Milestone 002 (version metadata + offline-first instance creation) may begin per [roadmap.md](../roadmap.md).
