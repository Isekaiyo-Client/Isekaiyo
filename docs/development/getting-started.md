# Getting Started

Goal: clone → setup → verify → run → test → contribute, on Windows/Linux/macOS, with zero guessing.

## 0. Prerequisites overview

| Tool | Version | Why | Installed by setup script? |
|---|---|---|---|
| Git | recent | obviously | checked |
| Node.js | **22.x** (`.nvmrc`) | frontend toolchain | checked |
| pnpm | ≥ 9 (**the only supported manager**) | JS deps | yes (corepack) |
| Rust stable | `rust-toolchain.toml` governs it | core + launcher | yes (rustup, user-local) |
| rustfmt + clippy | same channel | formatting/lint gate | yes (`rustup component add`) |
| Tauri system deps | per OS — see [windows](windows.md)/[linux](linux.md)/[macos](macos.md) | compile the shell | instructions only |

## 1. Clone & set up

```sh
git clone https://github.com/Isekaiyo-Client/Isekaiyo.git
cd Isekaiyo
sh ./scripts/setup.sh          # Windows: pwsh ./scripts/setup.ps1
```

The scripts are non-destructive and idempotent: they install only what's missing and explain every step.

## 2. Verify the environment

```sh
sh ./scripts/doctor.sh         # Windows: pwsh ./scripts/doctor.ps1
```

Expected:

```text
Isekaiyo Development Environment
----------------------------------------------
Git            ✓ git version 2.x
Rust toolchain ✓ rustc 1.x
Cargo          ✓ cargo 1.x
Node.js        ✓ v22.x
pnpm           ✓ x.y.z
----------------------------------------------
Environment ready.
```

Anything ✗ prints exact install instructions. The doctor never pretends health.

## 3. Run & test

```sh
pnpm dev                        # frontend shell in isolation
cargo run -p ikk-launcher       # full app (needs Tauri system deps); or:
pnpm exec tauri dev             # from apps/launcher-ui — same thing wired up
cargo test --workspace          # unit tests
cargo fmt --all -- --check      # format gate (CI enforces)
cargo clippy --workspace --all-targets -- -D warnings
```

## 4. Isolated dev data (never touch your real Minecraft)

Set `IKK_DEV_DATA_DIR=./IsekaiyoDev` (see the [env template](env-template.md)). All instance/cache/log paths derive from it during development. Automated tests **always** use temp dirs — never your production `.minecraft`.

## 5. First change

Follow [github-workflow.md](github-workflow.md): branch from `develop`, small PR, Conventional Commits, CI green.

## Checklist

```text
[ ] Git installed & GitHub authenticated (gh auth login OR SSH)
[ ] Rust installed, correct toolchain selected (rustup shows stable)
[ ] Cargo works
[ ] Node 22 active (node --version)
[ ] Frontend deps install (pnpm install)
[ ] Tauri prerequisites installed (doctor + OS guide)
[ ] Repo builds (pnpm build && cargo check --workspace)
[ ] Tests pass (cargo test --workspace)
[ ] fmt/clippy pass
[ ] Branch/commit/push/PR flow understood
```
