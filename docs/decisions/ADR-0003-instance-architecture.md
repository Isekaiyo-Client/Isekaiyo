# ADR-0003: Isolated instance architecture

**Status:** Accepted

**Context.** Users run PvP 1.8.9, vanilla current, Fabric 1.21.x, and a Forge 1.12.2 modpack simultaneously; cross-contamination is the classic third-party-launcher failure mode.

**Decision.** Instances are self-describing directories (`isekaiyo-instance.json` + standard `.minecraft/` layout) with: per-instance write locks, validation-before-trust (corruption detected, never silently repaired), staged atomic mutation for imports/bulk installs, and pinned reproducible definitions for export. Shared read-only assets live in a content-addressed cache — instances never share mutable state.

**Consequences.** Slight disk overhead vs shared-dir designs (acceptable); import/export must respect foreign formats via adapters ([instance-architecture](../instance-architecture.md)); deletion is recoverable via trash/staging receipts.
