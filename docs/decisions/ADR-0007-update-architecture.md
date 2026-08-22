# ADR-0007: Update architecture

**Status:** Accepted (design); tooling Phase 13

**Decision.** All self-update and plugin-repository update flows use **signed manifests + signed artifacts (ed25519, pinned public keys)**. Verification failure = hard abort; nothing unverified is ever executed. Staged rollout percentages with automatic rollback to the previous signed manifest on anomaly. Channels (nightly→alpha→beta→stable) are quality gates, not just labels — a channel exists only when its gates can be honored.

**Reasoning.** The update channel is the single highest-value attack target in any desktop app (update hijack / compromised CDN threats). Signature pinning makes CDN compromise insufficient for code execution.

**Consequences.** Key management becomes an operational responsibility (rotation runbook required before Stable); unsigned "just replace the exe" shortcuts are permanently forbidden.
