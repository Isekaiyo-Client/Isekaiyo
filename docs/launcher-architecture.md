# Launcher Architecture

Status: **Accepted** · Companion to [Architecture](architecture.md)

## Responsibilities

The launcher owns everything that happens *outside* the game process: instances, versions, loaders, mods, content, Java, accounts, downloads, updates, diagnostics, marketplace. It must be fully functional with zero Isekaiyo client code linked.

## Application Services (the only IPC surface)

UI ↔ core communication is exclusively these services, exposed as typed Tauri commands plus an event stream:

| Service | Commands (illustrative) | Emits |
|---|---|---|
| `InstanceService` | list, create, duplicate, delete, validate, import, export | `InstanceChanged` |
| `VersionService` | list_versions, refresh_manifest, version_details | `ManifestUpdated` |
| `LaunchService` | launch, cancel, last_launch_report | `LaunchProgress`, `GameOutput`, `GameExited` |
| `AccountService` | list, login_ms, add_local_profile, logout | `AuthStateChanged` |
| `JavaService` | list_runtimes, provision, set_instance_java | `JavaProvisionProgress` |
| `MarketplaceService` | search, project_details, install(plan) | `InstallProgress` |
| `ContentService` | packs/shaders/worlds/servers CRUD per instance | `ContentLoaded` |
| `DiagnosticsService` | logs, crash_reports, export_bundle | — |
| `SettingsService` | get/set global settings, theme, language | `SettingsChanged` |
| `UpdateService` | check, apply | `UpdateAvailable/Applied` |
| `PluginService` | list, enable/disable, permissions review | `PluginStateChanged` |

Rules: commands take/return DTOs defined in one shared crate (`ikk-api-types`) mirrored to TypeScript via code generation (e.g. `specta`/`ts-rs` — final tool pinned at implementation start). The UI holds no duplicate server state beyond ephemeral UI state; it subscribes to events.

## Launch Pipeline

Detailed flow in [Architecture §4](architecture.md#4-data-flow-launch). Failure semantics:

- Any step failure aborts before process spawn and produces a categorized `LaunchFailure` with a "Why didn't my game launch?" explanation and suggested remediations.
- Post-spawn failures stream game output into a `LogSession`; on non-zero exit, `ikk-diagnostics` attempts crash-report correlation (game-side crash reports + launcher-side log timeline).

## Instance Lifecycle (owned here, detailed in instance-architecture.md)

```text
create → configure → resolve → download → validate → ready
   ▲                                                    │
   └────────────── edit (re-validate diff) ◄────────────┘
import → validate → adopt (non-destructive; staged into new dir first)
delete → move to trash/staging with manifest receipt (recoverable window)
```

## Metadata & Caching Strategy

- Version manifest cached with ETag/TTL; refresh on demand + on launch of an uncached version.
- Per-version metadata immutable once fetched (content-hashed) — cache forever in cache dir.
- Marketplace search results cached briefly (minutes); project pages longer; never blocks UI.

## Launcher Without Client

CI enforces this literally: a release build configuration excludes client crates; integration tests exercise create-instance → launch vanilla with no client code present.

## Startup Performance Budgets (proposed benchmarks — measure, then enforce in CI)

| Metric | Target (proposed) | Method |
|---|---|---|
| Cold start → interactive shell | < 1.5 s on reference mid-range laptop | tracing span in CI perf job |
| Instance list render at 100 instances | < 300 ms | benchmark harness |
| Memory idle | < 250 MB RSS | CI measurement |

Numbers are *proposed* pending Phase 1 baselines; they exist so regressions are visible, not as marketing claims.
