# macOS Development

Tested baseline: macOS 13+ on Apple Silicon (aarch64). Intel builds compile with the same steps; CI covers aarch64 first.

## Required

1. **Xcode Command Line Tools** (full Xcode not needed for development):

   ```sh
   xcode-select --install
   ```

2. Homebrew packages:

   ```sh
   brew install pkg-config
   ```

3. Git, Node 22, pnpm, Rust — via `sh ./scripts/setup.sh`.

## Code signing & notarization (release only)

- Development builds run unsigned with zero setup.
- Release builds require an Apple Developer ID Application certificate and notarization (`codesign` + `notarytool`). Keys live in GitHub secrets; contributors never touch them. See `docs/release-process.md`.

## Verify

```sh
sh ./scripts/doctor.sh
cargo check --workspace
```
