# Isekaiyo

> **Isekaiyo should feel like a complete Minecraft operating environment rather than merely another launcher or another PvP client.**

Isekaiyo is an open-source, cross-platform ecosystem for Minecraft: Java Edition combining:

1. A full **launcher** (instances, versions, Java, loaders, mods, modpacks, marketplace)
2. A first-party **client** (a large built-in module library: PvP, HUD, performance, visual, social, utility)
3. A public **plugin API** so the community can extend both

The launcher is fully useful **without** the Isekaiyo client. The client is an optional enhancement layer. Neither requires the other — this boundary is the most important architectural invariant in the project. See [Product Definition](docs/vision.md#product-duality).

## Status

**Milestone 001 — Foundation vertical slice.** The application shell is real: it starts through an explicit initialization sequence (logging → platform paths → configuration → instance store → UI), persists configuration and instances as versioned JSON with safe recovery from corrupt files, and provides working Instances management (create / edit / delete / select) behind a typed IPC boundary. Launching Minecraft itself does **not** exist yet; unimplemented sections are marked honestly in the UI. Scope and acceptance: [Milestone 1](docs/roadmap/milestone-001-foundation.md).

**Milestone 002 — Shell hardening & launch boundary (in progress).** Semantic design-token theming (AMOLED black + sakura pink), settings schema v2 with migration, a real `launch_instance` command that refuses honestly with `runtime.unavailable`, a typed Minecraft version model, and an About page. Scope and acceptance: [Milestone 2](docs/roadmap/milestone-002-shell-hardening.md).

Verified locally: `cargo test --workspace` (26 tests) · `cargo clippy -D warnings` · `pnpm typecheck/lint/test/build`. See the [foundation audit](docs/development/foundation-audit.md) for exactly what was executed vs. reviewed-only.

Start reading here:

| Document | What it tells you |
|---|---|
| [Foundation Report](docs/foundation-report.md) | The complete architecture foundation, section-indexed |
| [Vision](docs/vision.md) | Product identity, philosophy, personas, principles |
| [Architecture](docs/architecture.md) | System layers, crate boundaries, domain model, data flows |
| [Roadmap](docs/roadmap.md) | Dependency-driven phases and the MVP |
| [Development](docs/development/getting-started.md) | How to build, test, and work in the repository |

## Non-negotiables

- Isekaiyo is **not affiliated with, endorsed by, or derived from Mojang/Microsoft**. Minecraft is © Mojang Studios / Microsoft. See [Licensing](docs/licensing.md) and [Legal Constraints](docs/security.md#legal-constraints).
- No systems designed to bypass authentication, pirate assets, or circumvent access controls.
- No secrets in logs, no unchecked downloads, no `unwrap()` culture in production paths.
- Version support is **capability-based**, never `if version == "1.8.9"` scattered through the codebase ([Version Architecture](docs/version-architecture.md)).

## Community

Contribution paths (code review, plugins, translation, docs, design) are defined in [CONTRIBUTING.md](CONTRIBUTING.md). Security reports go through [SECURITY.md](SECURITY.md) — never public issues.
