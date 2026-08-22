# Roadmap

Status: **Accepted** · Dependency-driven; phases start only when their entry criteria hold.

## Phase 0 — Architecture & Foundation ✅
Research, ADRs, domain model, docs. **Complete.**

## Phase 1 — Repository + Workspace + UI Shell ← current
- [x] Toolchain pins (`rust-toolchain.toml`, `.nvmrc`), pnpm-only policy
- [x] Cargo workspace: `ikk-core`, `ikk-api-types`, launcher shell, xtask doctor
- [x] Tauri v2 shell + frontend skeleton proving IPC end-to-end
- [x] Setup/doctor scripts (Win/Linux/macOS), staged CI (PR + nightly)
- [ ] **M1.1** First green CI on GitHub (needs repo push by maintainer)
- [ ] **M1.2** Rust toolchain hard-pin after baseline; TS types generated from Rust (kill the hand-mirror)
- [ ] **M1.3** LICENSE decision finalized (ADR-0009) before any public binary ships

## Phases 2–8 (launcher spine)
2 Launcher core services · 3 Vanilla launching E2E · 4 Instances (+import/export) · 5 Microsoft auth · 6 Java/version management · 7 Loaders (Fabric→NeoForge→Forge→Quilt order by adapter cost) · 8 Marketplace (Modrinth first).

## Phases 9–14 (client & ecosystem)
9 Client foundation (ADR-0010 spike gates this) · 10 1.8.x-class client · 11 Modern client tiers · 12 Plugin API · 13 Cross-platform packaging · 14 Stable release.

Exit criteria per phase are defined as "another developer can use it without asking questions" — same bar as the foundation.
