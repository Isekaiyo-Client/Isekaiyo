# Repository Architecture

Status: **Accepted** · Monorepo, deliberately

## Why a monorepo

Launcher, client, shared crates, and docs must evolve atomically during the early phases (an IPC DTO change touches 3 trees at once). Multi-repo splitting is deferred until a boundary proves stable and independently versioned — the plugin API is the likely first candidate (Phase 12).

## Layout

```text
Isekaiyo/
├── apps/
│   ├── launcher/          # Tauri v2 app shell
│   │   └── src-tauri/     # Rust side of the app (workspace member)
│   └── launcher-ui/       # frontend (pnpm workspace package)
├── crates/                # core libraries (ikk-*) — see architecture.md §2
├── tools/
│   └── xtask/             # cargo xtask doctor / future task runner
├── scripts/               # setup.sh|.ps1, doctor.sh|.ps1
├── tests/                 # cross-crate integration & contract tests (grows per phase)
├── packaging/             # NSIS/AppImage/Flatpak/dmg (Phase 13)
├── assets/                # project-owned branding only; NO game assets ever
├── docs/                  # this tree (docs are part of the product)
├── .github/               # workflows, templates, CODEOWNERS
├── Cargo.toml             # workspace root
├── rust-toolchain.toml    # repo-controlled Rust toolchain
├── pnpm-workspace.yaml    # frontend packages
└── .nvmrc                 # Node 22
```

## Rules

- Only directories that earn their place exist; empty scaffolding is an anti-pattern.
- `client/` appears in Phase 9 — created when there's code, not before.
- Every crate answers "what breaks if I delete it?" with something real.
