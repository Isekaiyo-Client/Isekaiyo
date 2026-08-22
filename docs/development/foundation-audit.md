# Foundation Audit

Status: **Complete (Foundation Audit phase)** · Auditor: engineering validation pass over the entire repository before feature work. Companion: [Milestone 001](../roadmap/milestone-001-foundation.md).

## Repository overview (what actually exists)

```text
Isekaiyo/
├── apps/
│   ├── launcher/src-tauri/     # Tauri v2 app shell (ikk-launcher crate): 1 command, startup/shutdown logging
│   └── launcher-ui/            # React + TS + Vite frontend shell; typed invoke() wrapper; AMOLED token CSS
├── crates/
│   ├── ikk-core/               # ErrorCode taxonomy, typed IDs, TaskEvent — with unit tests
│   └── ikk-api-types/          # IPC DTO boundary (SystemInfo)
├── tools/xtask/                # cargo xtask doctor | arch
├── scripts/                    # setup.{sh,ps1}, doctor.{sh,ps1} — non-destructive, idempotent
├── .github/                    # ci.yml, nightly.yml, PR + issue templates, CODEOWNERS
├── docs/                       # architecture contract, ADRs 0000–0010, development guides, this audit
└── toolchain/config            # rust-toolchain.toml, .nvmrc (Node 22), pnpm-only policy, lockfile
```

Four workspace crates — each earns its place; no placeholder scaffolding.

## Architecture as represented (not imagined)

Three-tier boundary per [architecture.md](../architecture.md): UI → typed IPC → app shell (`ikk-launcher`) → `ikk-api-types` DTOs → `ikk-core`. Dependency direction is **machine-enforced** by `cargo xtask arch` against [dependency-rules.md](../architecture/dependency-rules.md). Launcher/client separation is preserved structurally: no client code exists in the launcher tree.

## Problems discovered & disposition

| # | Severity | Problem | Disposition |
|---|---|---|---|
| P1 | CRITICAL | `tools/xtask/src/main.rs` structurally broken — `fn doctor()` header lost in a prior edit; body orphaned after `mod arch`; would not compile | **Fixed** — function restored |
| P2 | CRITICAL | `ikk-core` tests use `serde_json` without declaring it; declared `tracing` dependency unused — test compile failure guaranteed | **Fixed** — `serde_json` added as dev-dependency, `tracing` removed |
| P3 | HIGH | License contradiction triangle: owner-committed GPL-3.0 `LICENSE` vs. `license = "TBD"` manifest vs. ADR-0009 proposing MIT/Apache vs. licensing.md claiming "no LICENSE committed" | **Fixed** — ADR-0009 Accepted as GPL-3.0-or-later; manifests updated; licensing.md rewritten; `-or-later` reading flagged as OD-4 |
| P4 | HIGH | README claimed "no implementation exists yet" while an application shell does | **Fixed** — status section reflects Phase 1 reality |
| P5 | MEDIUM | `docs/architecture/dependency-rules.md` cited by code but absent | **Fixed** — written to match the xtask allowlist exactly |
| P6 | MEDIUM | No open-decisions register despite spec §26 | **Fixed** — [open-decisions.md](../architecture/open-decisions.md), OD-1…OD-7 |
| P7 | MEDIUM | No milestone with concrete acceptance criteria for the vertical slice | **Fixed** — [milestone-001](../roadmap/milestone-001-foundation.md) |
| P8 | LOW | Rust-side compilation never executed anywhere yet (no cargo in the authoring environment) | **Accepted residual risk** — first CI run / local build will surface nits; gates assume that |

## What was verified vs. not

**Verified (executed):**
- `pnpm install` / `typecheck` / `lint` / `build` — all green after fixes
- Shell syntax of all `.sh` scripts; YAML parse of all workflows/templates; JSON parse of Tauri configs
- `scripts/doctor.sh` execution behavior incl. pre-Rust fallback
- xtask arch allowlist ↔ actual manifests cross-checked by hand (edges match)

**NOT VERIFIED (no Rust toolchain in this environment):**
- `cargo check/test/fmt/clippy --workspace`, `cargo xtask doctor|arch`, Tauri dev launch. The P1/P2 fixes are structural and reviewed but **uncompiled**. First pushed CI run is the real gate — see Milestone 001 criterion #1.

## Verdict

The foundation is coherent: boundaries documented *and* enforced, license settled, contradictions eliminated, first milestone concretely defined. Implementation-ready pending the first green Rust CI run.
