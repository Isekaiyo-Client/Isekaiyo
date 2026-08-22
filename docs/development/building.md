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

## Artifacts

Everything lands under `target/` and `dist/` — both git-ignored. Nothing generated is ever committed unless a milestone explicitly versions it (none do yet).
