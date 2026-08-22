# Open Architectural Decisions

Status: **Living document** · A decision listed here is **not final**. Nothing in this file may be cited as settled. Resolved items move to `docs/decisions/` as ADRs.

Format per question: Question → Candidates → Trade-offs → Research needed → Status.

---

## OD-1 · Minecraft client integration strategy

- **Question:** How does the Isekaiyo client inject into the game across version ranges (transformation framework, mappings pipeline, classloading)?
- **Candidates:** Mixin-style launch-time transformation; dedicated bytecode transformers per era; ASM direct.
- **Trade-offs:** Mixin ecosystems are mature but JVM-centric and pull Java tooling into a Rust-first project; hand-rolled transformers are controllable but expensive to maintain across versions.
- **Research needed:** Spike proving transformation + launch on one modern version (ADR-0010).
- **Status:** OPEN — blocks all client work; launcher work is unaffected.

## OD-2 · Plugin runtime model

- **Question:** Native Rust plugins vs. embedded scripting runtime (WASM/Lua) vs. process-isolated helpers?
- **Candidates:** dylib (no sandbox, trust-model-explicit); WASM (sandboxed, limited API surface, perf cost); sidecar processes (isolation, IPC complexity).
- **Research needed:** Capability/performance comparison spike before Phase 12.
- **Status:** OPEN — [plugin-api.md](../plugin-api.md) documents the trust model for whichever model lands.

## OD-3 · Plugin API derivative-work position under GPL

- **Question:** Are plugins that call the Isekaiyo API derivative works? What may the plugin repository distribute?
- **Research needed:** Legal review; precedent from GPL game modding ecosystems.
- **Temporary decision:** Plugins communicate over documented public APIs; we make no distribution claims beyond our own tree until reviewed.
- **Status:** OPEN — see [licensing.md](../licensing.md).

## OD-4 · License "or later" reading

- **Question:** Owner committed unmodified GPLv3 text — GPL-3.0-or-later (§14 default) or GPL-3.0-only?
- **Current state:** Manifests say `GPL-3.0-or-later`; flip to `-only` if the owner intends otherwise.
- **Status:** OPEN — needs one-line owner confirmation.

## OD-5 · Update signing infrastructure

- **Question:** Who holds release signing keys and what infrastructure signs update artifacts (ADR-0007 assumes signatures exist)?
- **Candidates:** minisign-style key held by maintainer; Sigstore; CI-secret-held keys.
- **Status:** OPEN — must resolve before any distributed build (Phase 13).

## OD-6 · Marketplace provider #2

- CurseForge access terms are unresolved upstream (official API requires approval); Modrinth is the committed first provider. Status: OPEN — revisit when CurseForge terms are confirmed; no scraping by policy.

## OD-7 · Telemetry backend

- Telemetry is opt-in-only by policy, but *where* opted-in data goes (self-hosted? none at all in v1?) is undecided. Temporary decision: no telemetry endpoint exists; diagnostics are local-only. Status: OPEN — lowest priority.

---

Resolved during the Foundation Audit (moved out of this file): data storage policy (ADR-0008), licensing (ADR-0009), UI framework (ADR-0002), dependency direction (dependency-rules.md + `cargo xtask arch`).
