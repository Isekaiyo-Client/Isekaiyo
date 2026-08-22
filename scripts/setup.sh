#!/usr/bin/env sh
# Isekaiyo development environment setup (Linux / macOS).
#
# Principles (spec §7):
# - Non-destructive: installs only what is missing, never removes anything.
# - Transparent: prints what it checks and why.
# - Idempotent: safe to re-run any time.
# - Never requires root; installers that need elevation ask on their own.
set -eu

say() { printf '\n== %s\n' "$1"; }
have() { command -v "$1" >/dev/null 2>&1; }

say "git"
have git || { echo "MISSING git — install from https://git-scm.com/downloads"; exit 1; }

say "Node.js >= 22 (.nvmrc pins the major)"
if have node; then
  echo "found: $(node --version)"
else
  echo "MISSING node — install https://nodejs.org or: nvm install $(cat .nvmrc)"
  exit 1
fi

say "pnpm (the only supported package manager)"
if have pnpm; then
  echo "found: $(pnpm --version)"
else
  echo "enabling pnpm via corepack (bundled with Node)"
  corepack enable pnpm
  have pnpm || { echo "corepack failed — see https://pnpm.io/installation"; exit 1; }
fi

say "Rust toolchain (rustup manages it; rust-toolchain.toml pins components)"
if have cargo && have rustc; then
  echo "found: $(rustc --version)"
else
  echo "rustup not found — installing via official installer (user-local, no root)"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
echo "ensuring components rustfmt + clippy"
rustup component add rustfmt clippy

say "Tauri system dependencies"
case "$(uname -s)" in
  Linux)
    if have apt-get; then
      echo "Debian/Ubuntu: install webkit2gtk build deps (needs sudo — run when prompted):"
      echo "  sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \\"
      echo "    libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config"
    elif have dnf; then
      echo "Fedora: see docs/development/linux.md (dnf group + webkit2gtk4.1-devel)"
    elif have pacman; then
      echo "Arch: see docs/development/linux.md (pacman -S webkit2gtk-4.1 base-devel)"
    else
      echo "Unknown distro — see docs/development/linux.md"
    fi
    ;;
  Darwin)
    have xcode-select || true
    xcode-select -p >/dev/null 2>&1 || echo "run: xcode-select --install"
    ;;
esac

say "Frontend dependencies"
pnpm install

say "Done. Now run: sh ./scripts/doctor.sh"
