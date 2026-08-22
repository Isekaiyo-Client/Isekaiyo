# Isekaiyo Architecture Foundation — Report Index

The complete foundation is distributed across this docs tree; this file indexes it and tracks cross-cutting open questions.

| Section (spec §67) | Document |
|---|---|
| 1–4 Summary, definition, principles, personas | [vision.md](vision.md) |
| 5–6 Requirements | [architecture.md](architecture.md), per-subsystem docs |
| 7 Competitive research | [research/competitive-analysis.md](research/competitive-analysis.md) |
| 8–9 Technology evaluation & stack | [research/technology-evaluation.md](research/technology-evaluation.md), ADR-0001/0002 |
| 10 System architecture, domain model | [architecture.md](architecture.md) |
| 11 Repository architecture | [repository.md](repository.md) |
| 13–14 Launcher / client | [launcher-architecture.md](launcher-architecture.md) · [client-architecture.md](client-architecture.md) |
| 12,15–17 Instances, versions, loaders | [instance-architecture.md](instance-architecture.md) · [version-architecture.md](version-architecture.md) |
| 18 Marketplace | [marketplace.md](marketplace.md) |
| 19 Plugins | [plugin-api.md](plugin-api.md), ADR-0005 |
| 20–22 Security, auth, privacy | [security.md](security.md) · [privacy.md](privacy.md) |
| 23–26 Config, updates, downloads, Java | ADR-0007/0008 · [java-runtime.md](java-runtime.md) |
| 28–32 UI/UX, themes, i18n | vision.md + theme tokens in shell CSS |
| 31 Testing | [testing.md](testing.md) |
| 33 Releases | [release-process.md](release-process.md) |
| 34–35 Contribution & docs | [CONTRIBUTING.md](../CONTRIBUTING.md) |
| 36 Licensing | [licensing.md](licensing.md) |
| 37 Threat model | [security.md](security.md#threat-model-stride-lite-per-asset) |
| 40 ADR index | [decisions/](decisions/) |
| 42 Roadmap · 43 MVP | [roadmap.md](roadmap.md) |

## Open Questions

| ID | Question | Why it matters | Revisit |
|---|---|---|---|
| OQ-1 | Exact mixin/transformation floor for the modern client adapter | gates Phase 9 | ADR-0010 spike |
| OQ-2 | Hard-pin Rust patch version | reproducibility vs breakage | after M1.1 CI baseline |
| OQ-3 | CurseForge API terms/approval for third-party launchers | marketplace coverage | before Phase 8 |
| OQ-4 | Final license (ADR-0009 provisional MIT OR Apache-2.0) | distribution legality | before first public binary |
