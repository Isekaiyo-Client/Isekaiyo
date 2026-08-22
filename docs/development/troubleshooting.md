# Troubleshooting

Symptom-indexed; each entry links the fix.

## Environment

- **`cargo: command not found`** → Rust not installed. `sh ./scripts/setup.sh` or [rustup.rs](https://rustup.rs). After install, restart your shell (`source "$HOME/.cargo/env"`).
- **Wrong Node version** → `nvm install && nvm use` (`.nvmrc` pins 22). Verify `node --version`.
- **`pnpm` missing / wrong manager instinct** → we support pnpm only. Enable once: `corepack enable pnpm`.
- **Toolchain drift** → `rustup show` must report stable + rustfmt/clippy per `rust-toolchain.toml`.

## Build

- Windows linker errors → [windows.md](windows.md) (Build Tools C++ workload).
- Linux `webkit2gtk-4.1` errors → [linux.md](linux.md) distro tables.
- macOS header errors → `xcode-select --install`, see [macos.md](macos.md).
- Frontend type errors after pulling → stale build info: delete `apps/launcher-ui/*.tsbuildinfo` and rerun `pnpm typecheck`.
- **"icons/icon.ico not found" during `tauri-build`** → your checkout is missing `apps/launcher/src-tauri/icons/`. Regenerate: `python3 scripts/generate-icons.py` (see [building](building.md#application-icons)). Never "fix" this by removing icons from the Tauri config.

## Runtime

- Blank window → run `pnpm build` first, confirm `tauri.conf.json` `frontendDist` path exists.
- IPC "command not found" → command registered in `generate_handler![]`? Capability granted in `capabilities/default.json`?
- App won't start under Wayland → try X11 backend ([debugging](debugging.md)).

Still stuck: open a Discussion with your full `doctor.sh` output and OS details.
