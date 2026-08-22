# ADR-0005: Plugin architecture & trust model

**Status:** Accepted (host side); in-game surface OPEN pending ADR-0010

**Context.** Community extensibility is a core promise; plugins are executable code, and a manifest is not security.

**Decision.** Two-tier honesty model:
1. Host/launcher plugins run as **WASM components (wasmtime)** with capability-based imports — genuine sandboxing for pure-logic plugins; no ambient filesystem/network authority.
2. **Native plugins are never sandboxed** and are only ever supported behind explicit "this can do anything" consent + signing review. We will never claim otherwise.
Permissions (`filesystem(scope)`, `network(host)`, `game.*`, `ui.*`) are shown in plain language at install and re-prompted on update.

**Reasoning.** Overstating sandboxing is a security lie with user harm; capability-scoped WASM is the only mechanism that lets us make *true* containment claims today.

**Consequences.** Performance-critical plugins limited until/unless a reviewed native path exists; API stability burden begins at Phase 12 (semver + compatibility test suite).
