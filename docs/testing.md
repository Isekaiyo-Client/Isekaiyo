# Testing Architecture

Status: **Accepted** · Companion to [architecture](architecture.md)

## Stages

| Stage | What | When | Where |
|---|---|---|---|
| Unit | pure domain logic (resolver, config migrations, error taxonomy) | every commit | inline `#[cfg(test)]` |
| Integration | subsystem against real fs/temp dirs (instance create/validate) | every commit | `tests/` per crate |
| Contract | external APIs (Mojang manifest, Modrinth) against recorded fixtures + optional live smoke | nightly | `tests/contracts/` |
| Launch E2E | install→login→instance→download→launch matrix per compatibility tier | nightly/manual | `tests/e2e/` (Phase 3+) |
| UI | critical flows in the webview | Phase 2+ | Playwright against the frontend |
| Regression | compatibility matrix rows ([compatibility.md](compatibility.md)) must have a green job to claim ✅ | per release | CI grid |

## Principles

- **No copyrighted Minecraft assets in the repo.** Fixtures use minimal synthetic files; real metadata is fetched at test time or stored as recorded responses.
- Tests never touch user data: temp dirs only.
- CI staging: Stage A (fast, Linux, every PR: fmt/clippy/test/frontend) vs Stage B (nightly tri-platform builds + audits). See `.github/workflows/ci.yml` and `nightly.yml`.
