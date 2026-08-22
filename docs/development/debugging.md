# Debugging

## Rust

- Breakpoints: VS Code + CodeLLDB (see `.vscode/extensions.json` recommendations), or RustRover/CLion natively. Attach to `ikk-launcher`.
- Logs: `RUST_LOG=debug cargo run -p ikk-launcher` (tracing goes to stderr).
- Panics: `RUST_BACKTRACE=1` for a backtrace; workspace lints already push us away from panicking paths.

## Frontend

- In dev builds right-click → Inspect opens the WebView devtools; console, network, React DevTools all work.
- `pnpm dev` alone runs the UI without the backend — IPC calls will fail with "backend unreachable", which is expected in this mode.

## Full path (UI → Rust → fs/network)

1. `pnpm exec tauri dev` (from repo root or apps/launcher-ui).
2. Watch both consoles: Vite (frontend) and Cargo (backend tracing).
3. Set an IPC breakpoint in `src/api.ts` and a Rust breakpoint inside the command.

## Game-side (later milestones)

- Launcher logs: per-instance under `<data>/logs/`.
- Game stdout/stderr streams into the same session (`LogSession`).
- Crash reports land next to instance logs; `ikk-diagnostics` correlates them (Phase 2+).

## Common failures

| Symptom | Cause / fix |
|---|---|
| `failed to get cargo metadata` | rustup not installed → run setup script |
| Linker errors on Windows | VS Build Tools C++ workload missing → windows.md |
| `webkit2gtk-4.1 not found` | Linux dev headers missing → linux.md |
| Blank window | frontend dist missing → run `pnpm build`, check tauri.conf.json paths |
