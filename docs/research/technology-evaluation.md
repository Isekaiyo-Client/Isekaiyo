# Technology Evaluation

Status: **Accepted** · Phase 0 deliverable · Decision records: ADR-0001 (Rust core), ADR-0002 (Tauri UI), ADR-0010 (client integration — OPEN)

Confidence labels: **[V]** verified · **[I]** inference · **[U]** unresolved

## Core Language

| Option | Assessment | Verdict |
|---|---|---|
| **Rust** | Memory safety without GC; fearless concurrency for download/process orchestration; cargo workspaces give enforceable crate boundaries; proven in-domain by Modrinth's desktop app [V]; single language spans core + native layer | **Selected** (ADR-0001) |
| C++ | Mature, but manual memory/UB risk across a large contributor base is a liability; build-system fragmentation raises CI cost | Rejected |
| C | No abstraction tooling for a domain model this large | Rejected |
| Kotlin/JVM | Excellent for Minecraft-adjacent tooling; but the *host* app would carry a JVM runtime dependency and GC pauses in a process-spawner/downloader are avoidable overhead; JVM needed anyway for the game itself, not for the launcher | Rejected for core; used inside client adapters where Minecraft-side code requires it |

## Desktop UI

| Option | Assessment | Verdict |
|---|---|---|
| **Tauri 2** | Verified docs [V]: Rust backend, system webview (small binaries), any frontend framework, `invoke()` IPC, security-audited major releases. Matches "Rust core + explicit UI boundary" requirement exactly. Risk: webview engine varies per OS (WebView2/WKWebView/WebKitGTK) → visual QA matrix needed [I] | **Selected** (ADR-0002) |
| Electron | Ships Chromium (~100+ MB) per app; contradicts startup/memory NFRs; JS main process weakens the typed-boundary story | Rejected |
| Qt (C++/Rust bindings) | Capable but licensing complexity (LGPL/commercial) for an open-source project with uncertain future monetization; heavier contributor on-ramp | Rejected |
| Native per-platform (WinUI/Swift/GTK) | 3× UI cost forever; kills cross-platform consistency | Rejected |
| egui/Iced (pure Rust) | Immature accessibility/localization stories vs web stack today [I] | Rejected for v1; revisited if requirements change |

Frontend: TypeScript + Vite + a component framework chosen at implementation start (React or Svelte — recorded as an open decision with a default of React for ecosystem/accessibility maturity). The UI talks to core **only** through generated typed command/event bindings.

## Minecraft Integration (client side)

The hard problem: injecting first-party modules into arbitrary game versions.

Options evaluated:

1. **Per-version full forks of the client** — what closed clients appear to do at scale [U]. Cost explodes linearly with supported versions. Rejected as primary strategy.
2. **Mixin-style bytecode transformation at launch time** (the approach popularized by Sponge/Fabric mixin ecosystems): declarative injections applied to obfuscated/mapped game bytecode during classloading. Well-understood, widely proven in open ecosystem [V]. Adopted as the *primary mechanism* for modern versions.
3. **Legacy-specific transformers** for pre-mapping-era versions (1.7/1.8 era lacks modern mappings infrastructure): isolated adapter crates with their own transformation logic. Accepted as necessary special cases, quarantined.
4. **Vanilla-injection via agent (`-javaagent`) attaching to unmodified vanilla**: candidate for early milestones because it avoids loader coupling; capability-gated.

**Decision shape:** ADR-0010 records this as the highest-risk open decision. Provisional direction: launch-time transformation with mixins for 1.16+ (exact floor TBD after spike), dedicated legacy adapters below it, all behind the version-capability model so no feature assumes one mechanism. A time-boxed technical spike (Phase 9 entry criterion) must validate: mapping application, transformation timing, classloading isolation, and anti-cheat compatibility posture before client work scales.

Mappings/tooling: use established community mapping chains rather than inventing one; exact chain selection is part of the ADR-0010 spike.

## Supporting Choices

- **Async runtime:** tokio.
- **HTTP:** reqwest + rustls (no dynamic OpenSSL).
- **Crypto/integrity:** SHA-256 verification everywhere; ed25519 signatures for updates/plugins manifests (ADR-0007).
- **Keyring:** OS credential stores via `keyring` crate with documented fallback.
- **Logging:** `tracing` structured events, per-subsystem targets.
- **Serialization:** serde; JSON for user-facing files, TOML only if human-editing dominates (per-dataset decisions in ADR-0008).

## Explicitly Not Chosen Yet

- Database layer (none until scale demands — see ADR-0008).
- CurseForge API integration (terms verification pending — Open Question OQ-3).
