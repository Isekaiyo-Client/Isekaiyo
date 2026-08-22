# Contributing to Isekaiyo

Thanks for contributing! This file is the short version; the complete guides live in [`docs/development/`](docs/development/getting-started.md).

## Quick start

```sh
git clone https://github.com/Isekaiyo-Client/Isekaiyo.git
cd Isekaiyo
sh ./scripts/setup.sh        # Windows: pwsh ./scripts/setup.ps1
sh ./scripts/doctor.sh       # everything ✓ before you continue
pnpm dev                     # frontend shell (or `cargo tauri dev` with Rust installed)
cargo test --workspace       # Rust unit tests
```

Full walkthrough (every step explained, nothing assumed): [docs/development/getting-started.md](docs/development/getting-started.md).

## Ground rules

1. **Read the architecture first.** [docs/architecture.md](docs/architecture.md) and the ADRs in [docs/decisions/](docs/decisions/) are contracts, not suggestions. Disagreements go through an Architecture Proposal issue → ADR, not drive-by refactors.
2. **Branches:** branch from `develop`. Names: `feature/<topic>`, `fix/<topic>`, `chore/<topic>`, `docs/<topic>`.
3. **Commits:** Conventional Commits — `feat(instances): add manifest model`, `fix(downloads): retry interrupted artifacts`. No "asdf", no "final2".
4. **PRs:** one coherent change per PR, template filled, CI green, at least one maintainer review. See the [PR workflow](docs/development/github-workflow.md).
5. **Quality gates are non-negotiable:** `cargo fmt --check`, `cargo clippy -D warnings`, tests pass. Don't suppress warnings globally — fix them or justify scoped allows.
6. **Secrets never enter the repo** ([SECURITY.md](SECURITY.md), [security docs](docs/security.md)).

## Ways to contribute

| Path | Start here |
|---|---|
| Code (Rust) | [development/architecture-overview](docs/development/architecture-overview.md), pick a roadmap item |
| Code (UI) | [development/debugging](docs/development/debugging.md), `apps/launcher-ui` |
| Plugins | [docs/plugin-api.md](docs/plugin-api.md) (API lands Phase 12) |
| Translations | open an issue labeled `localization` (i18n scaffolding Phase 13+) |
| Docs | any `docs/*` PR, template `.github/ISSUE_TEMPLATE/documentation.yml` |

## Review expectations

Reviewers check correctness, architecture fit, test coverage, and docs impact — not personal style (`rustfmt` settles that). Authors: keep PRs reviewable (< ~400 lines diff where feasible). Maintainers merge to `develop`; releases cut from `main` per the [release process](docs/release-process.md).
