# Building

## Development

```sh
cargo check --workspace     # fastest compile validation
cargo build                 # debug binaries in target/debug/
pnpm exec tauri dev         # run the app shell wired to Vite HMR-free dev server
```

## Release

```sh
pnpm build                              # frontend → apps/launcher-ui/dist/
cargo build --release -p ikk-launcher   # optimized binary
```

Release profile (`Cargo.toml`): thin LTO, stripped debuginfo. Full bundling/installers (NSIS, AppImage, Flatpak, signed dmg) is Phase 13 — `packaging/` and the release workflow land then ([release-process](../release-process.md)).

## Application icons

Tauri **requires** `apps/launcher/src-tauri/icons/icon.ico` whenever the Windows target is built (`tauri-build` embeds it as a Win32 resource); the other formats cover macOS/Linux bundling. All assets are generated, reproducibly, by a dependency-free script:

```sh
python3 scripts/generate-icons.py        # regenerates every file under apps/launcher/src-tauri/icons/
```

The script renders the sakura mark at 1024px and writes `icon.png`, `32x32.png`, `128x128.png`, `128x128@2x.png`, a multi-size `icon.ico`, and `icon.icns`. Generated files are committed; re-run the script and commit the results after any design change. Do not hand-edit the binaries.

## Artifacts

Everything lands under `target/` and `dist/` — both git-ignored. Nothing generated is ever committed unless a milestone explicitly versions it (none do yet).
