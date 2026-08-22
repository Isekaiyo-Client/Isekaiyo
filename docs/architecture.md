# Architecture

Status: **Accepted** · Phase 0 deliverable · Deep-dive companions: [launcher](launcher-architecture.md) · [client](client-architecture.md) · [instances](instance-architecture.md) · [versions](version-architecture.md)

## 1. System Overview

Isekaiyo is a desktop application with an explicit three-tier boundary:

```text
┌───────────────────────────────────────────────────────┐
│ UI (web frontend: TypeScript, framework-agnostic)     │
│   No filesystem access. No launch logic. No HTTP.     │
└───────────────▲───────────────────────────────────────┘
                │ typed IPC commands + event stream
┌───────────────▼───────────────────────────────────────┐
│ Application layer (Rust): use-cases / services        │
│   instance_service, launch_service, auth_service, …   │
└───────────────▲───────────────────────────────────────┘
                │ domain traits
┌───────────────▼───────────────────────────────────────┐
│ Domain layer (Rust): entities, policies, invariants    │
│   resolver, capability model, config migrations        │
└───────────────▲───────────────────────────────────────┘
                │ ports
┌───────────────▼───────────────────────────────────────┐
│ Infrastructure (Rust): HTTP, fs, crypto, OS keyring,  │
│   process spawning, marketplace adapters, telemetry   │
└───────────────────────────────────────────────────────┘
```

Rules:
- The **UI never** touches the filesystem, spawns processes, or builds Minecraft arguments.
- The **domain layer never** performs I/O; it defines ports (traits) that infrastructure implements.
- Every cross-layer call is a named application service — no "UI → random util" calls.

## 2. Workspace Layout

A Cargo workspace of focused crates (boundaries enforced by `cargo-workspaces`-style dependency rules in CI):

```text
crates/
├── ikk-core/          # errors, config store, events, task orchestration, ids
├── ikk-minecraft/     # version metadata model + Mojang manifest client
├── ikk-instances/     # instance model, layout, validation, import/export
├── ikk-launch/        # classpath/JVM/game argument assembly, process mgmt
├── ikk-auth/          # account abstraction, MS OAuth flows, token storage
├── ikk-java/          # runtime discovery, provisioning, per-instance selection
├── ikk-download/      # async downloader: retries, checksums, cache, mirrors
├── ikk-loaders/       # loader trait + fabric/forge/neoforge/quilt adapters
├── ikk-mods/          # mod metadata, dependency resolver, install plans
├── ikk-marketplace/   # provider abstraction (Modrinth first)
├── ikk-content/       # resource packs, shader packs, worlds, servers.dat
├── ikk-config/        # versioned configuration w/ migrations
├── ikk-updater/       # signed self-update, rollback, channels
├── ikk-diagnostics/   # structured logs, redaction, crash parsing, bundles
├── ikk-telemetry/     # opt-in only; disabled at compile-time by default flag
├── ikk-plugins-host/  # plugin manifests, registry, lifecycle (host side)
├── ikk-platform/      # paths, OS integration, keyring, single-instance lock
apps/
├── launcher/          # Tauri app: thin shell wiring core services to IPC
└── launcher-ui/       # frontend source (TypeScript)
client/                # Isekaiyo client (see client-architecture.md)
tools/                 # CLI utilities (ikk-cli: headless instance ops for CI)
packaging/             # NSIS/MSI, AppImage, Flatpak manifest, dmg scripts
tests/                 # cross-crate integration & contract tests
docs/                  # this tree
```

Dependency rule: `apps → * ; everything → ikk-core ; nothing depends on ikk-platform except through traits` (enforced by a small CI check script). The **client** crate tree is intentionally separate from the launcher crates so the launcher can ship without any client code linked.

## 3. Domain Model

Strong types everywhere; no `HashMap<String, serde_json::Value>` as universal model. Core entities (full field tables in [instance](instance-architecture.md)/[version](version-architecture.md) docs):

| Entity | Purpose | Key relationships |
|---|---|---|
| `MinecraftVersionId` | Newtype over version string | referenced by `VersionMetadata`, instances |
| `VersionManifest` | Cached snapshot of Mojang's manifest | many `VersionMetadata` |
| `VersionMetadata` | Per-version JSON (libraries, assets, mainClass, java) | belongs to manifest |
| `JavaRuntime` | A provisioned or discovered JVM | selected per instance |
| `MinecraftInstance` | The central aggregate | owns loader spec, mods, content, config ref |
| `InstanceProfile` | Launch-relevant settings (memory, JVM args, window) | part of instance |
| `ModLoaderSpec` | Which loader + version, if any | resolves via `ikk-loaders` |
| `Mod` / `ModVersion` | Installed artifact + its metadata | constrained by resolver |
| `Modpack` | Portable definition of an instance set | imports into instances |
| `ResolutionPlan` | Deterministic output of dependency resolver | executed by download system |
| `DownloadTask` / `Artifact` | Work items + verified results | checksummed, cached |
| `Account` enum | `Microsoft(MicrosoftAccount)` \| `Local(LocalProfile)` | never conflated |
| `LaunchConfiguration` | Fully resolved launch inputs | produced from instance + profile + account |
| `PluginManifest`, `Plugin` | Third-party extension identity/lifecycle | permission set attached |
| `ClientModuleDescriptor` | Registry entry for a first-party module | capability requirements |
| `Configuration<S>` | Versioned config with migration chain | global / per-instance scopes |
| `CrashReport`, `LogSession` | Diagnostics artifacts | redacted before sharing |

Invariants worth stating now:
- An instance's on-disk state must always be re-validatable (`ikk-instances validate`) — corruption is detected, not silently propagated.
- `Account::Local` is represented distinctly end-to-end (UI label, auth flow, server join behavior); it is never presented as an authenticated Microsoft account.
- Every persisted structure carries a `schema_version`.

## 4. Data Flow: Launch

```text
UI "Play" ──IPC──▶ launch_service.launch(instance_id)
                     ├─ load + validate instance
                     ├─ resolve version metadata (cache-first)
                     ├─ resolve loader chain (if any)
                     ├─ ensure Java runtime (provision if needed)
                     ├─ build ResolutionPlan → execute missing downloads
                     ├─ assemble LaunchConfiguration (args, classpath, natives)
                     ├─ acquire auth session (refresh if expired)
                     └─ spawn JVM, stream stdout/stderr into LogSession,
                        emit progress events to UI
```

Every step emits typed events (`TaskEvent`) so the UI renders real progress and every failure has a category ([Error Handling](#7-error-handling)).

## 5. Concurrency & Process Model

- One long-lived core; Tauri commands are async entry points onto it (tokio).
- Download pool with bounded parallelism, global cancellation tokens.
- Filesystem writes to an instance are serialized per instance (lock map) — concurrent modification of one instance is prevented without blocking unrelated instances.
- Single-instance lock at app level (second launch focuses the first).

## 6. Networking

One HTTP stack (`reqwest` + rustls), wrapped in an internal `HttpService` that enforces timeouts, retries with jittered backoff, cancellation, and per-provider rate limits. Marketplace providers implement a `MarketplaceProvider` port; no raw HTTP calls outside infrastructure. Proxy support inherits system settings where the platform exposes them; manual proxy is a documented setting.

## 7. Error Handling

- Crate-level error enums deriving a shared `ErrorCode` taxonomy (stable string categories like `network.timeout`, `auth.token_expired`, `download.checksum_mismatch`, `instance.corrupt`).
- User-facing messages carry recovery suggestions ("Java not found → Provision Java 21 automatically?").
- Production code bans `.unwrap()`/`.expect()` outside tests and truly-infallible spots (lint-enforced via `clippy::unwrap_used = deny` with scoped allows).

## 8. Storage Policy

Decision recorded in [ADR-0008](decisions/ADR-0008-data-storage-policy.md). Summary:

| Dataset | Store | Why |
|---|---|---|
| Instances, configs | JSON files on disk (per-file schema version) | user-inspectable, portable, diffable |
| Metadata caches | JSON in cache dir with TTL | disposable |
| Download cache | content-addressed blob dir | dedupe across instances |
| Tokens/secrets | OS keyring (Windows Credential Manager, macOS Keychain, libsecret) with encrypted file fallback | never plaintext by default |
| Search/indexes | none initially | revisit at 500+ instance scale (ADR-0008 open item) |

Platform paths via `directories` crate conventions; nothing hardcoded to `%APPDATA%`.

## 9. Feature Flags & Developer Mode

Flags live in config with an expiry date field; CI fails if a flag outlives two minor releases. Developer mode adds verbose logging, debug overlays, plugin hot-reload, test marketplace sources — gated behind a settings toggle and never weakening normal-user security defaults.
