# Milestone 003 — Launcher Engine & Loader Architecture

Status: **Active** · Covers the Phase 3–5 engine work: Minecraft metadata pipeline, download engine, Java resolution, launch planning, process management, and the loader provider architecture (Fabric/Quilt real; Forge foundation).

## The pipeline that now exists

```text
Instance → manifest → version metadata → loader overlay (Fabric/Quilt)
        → install plan → verified downloads → natives extraction
        → Java discovery + compatibility → LaunchPlan → process spawn
        → stdout/stderr capture → exit classification → history
```

## Status matrix

| Capability | Status | Where |
|---|---|---|
| Version manifest fetch + timestamped cache + stale/offline fallback | **WORKING** (engine-tested) | `ikk-minecraft::manifest` |
| Version metadata parsing (modern `arguments` + legacy `minecraftArguments`, `inheritsFrom` overlay) | **WORKING** | `ikk-minecraft::metadata` |
| OS/arch/feature rule evaluation | **WORKING** | `ikk-minecraft::rules` |
| Install-plan resolution (client, libraries, natives, asset index, assets, log config) + Maven-coordinate fallback | **WORKING** | `ikk-minecraft::resolve` |
| Verified streaming downloads (SHA-1 skip/re-download, `.part` + atomic rename, retries, cancel) | **WORKING** (engine-tested) | `ikk-minecraft::download` |
| Asset index parsing + content-addressed object layout | **WORKING** | `ikk-minecraft::assets` |
| Natives extraction w/ zip-slip guards | **WORKING** | `ikk-minecraft::natives` |
| Java discovery/version parsing/compatibility floor | **WORKING** (parsing tested; discovery is plain fs+process) | `ikk-minecraft::java` |
| LaunchPlan builder (JVM/game args, substitutions, memory bounds) — the only place args are built | **WORKING** | `ikk-minecraft::planner` |
| Process spawn/output-capture/exit classification/user-stop | **WORKING** | `ikk-minecraft::process` |
| Launch state machine with validated transitions | **WORKING** | `ikk-minecraft::state` |
| Offline-profile identity (`OfflinePlayer:` UUID v3, token `"0"`, honest limits in UI) | **WORKING** | `ikk-minecraft::account` |
| LoaderProvider trait + registry | **WORKING** | `ikk-minecraft::loaders` |
| **Fabric** provider (meta.fabricmc.net v2: version lists + profile overlay) | **FOUNDATION COMPLETE, untested against live meta** | same |
| **Quilt** provider (meta.quiltmc.org v3) | **FOUNDATION COMPLETE, untested against live meta** | same |
| **Forge / NeoForge** | **NOT IMPLEMENTED** — provider returns explicit "not implemented yet"; installer-based pipeline is a future milestone | same |
| Microsoft accounts / secure token storage | **NOT IMPLEMENTED** — offline profiles only, clearly labeled | — |
| Tauri commands wiring the above | **WRITTEN** (`list_versions`, `list_loader_versions`, `install_instance`, `launch_instance`, `launch_status`, `stop_launch`, `read_launch_log`) — compile-gated on a machine with WebKitGTK/MSVC; sandbox cannot build the shell crate | `apps/launcher/src-tauri/src/lib.rs` |

## Testing

53 engine tests (offline fixtures only — no network in unit tests): manifest/metadata parsing incl. malformed inputs, rule semantics, deterministic plans + path-traversal rejection, hash/atomic download logic, Java version parsing/compatibility, planner substitutions for both metadata eras + memory bounds, process exit categories + real spawn test (unix), zip-slip rejection, loader list/profile parsing for Fabric & Quilt shapes, overlay merging into resolvable plans.

## Manual acceptance still required (needs a desktop)

1. Vanilla regression: create → Download → Play → game starts.
2. Fabric: create with fabric-loader latest stable → Play → game starts.
3. Quilt: same via quilt-loader.
4. Re-run Play → verify downloads are skipped (cache hits).
5. Console shows real game output; Stop marks "user-stopped".

## Known gaps / debt

- Sequential downloads (no concurrency/priority queue yet).
- Progress granularity is per-file counts, not bytes-in-flight in the UI.
- No profile system yet (memory/JVM overrides are engine-ready, UI-pending).
- Forge needs its own installer-execution pipeline (security-reviewed).
- `cargo tauri dev/build` verification must happen on Windows/macOS/Linux desktops.
