# ADR-0002: Tauri 2 for the launcher UI

**Status:** Accepted

**Context.** Need a modern, themable, accessible cross-platform UI with a hard boundary to a Rust core; startup/memory budgets matter (NFRs).

**Options.** Tauri 2 / Electron / Qt / per-platform native / pure-Rust GUI (evaluated in [technology-evaluation](../research/technology-evaluation.md)).

**Decision.** Tauri 2: system webview, Rust backend, framework-agnostic TypeScript frontend; UI ↔ core exclusively via typed commands/events.

**Reasoning.** Verified docs: security-audited releases, small binaries (no bundled Chromium), `invoke()` IPC matching our typed-boundary requirement; direct precedent in Modrinth's app.

**Consequences.** WebView engine differs per OS (WebView2/WKWebView/WebKitGTK) → visual QA matrix is a real cost, tracked in compatibility.md. WebKitGTK dev deps are a Linux onboarding hurdle (documented + scripted).

**Rejected.** Electron (ships Chromium, weakens typed boundary), Qt (licensing complexity), pure-Rust GUI (a11y/i18n immaturity today — revisitable).
