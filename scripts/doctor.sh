#!/usr/bin/env sh
# Isekaiyo environment doctor.
# Prefers `cargo xtask doctor` (tools/xtask) when a Rust toolchain exists;
# falls back to pure-shell checks so it works before Rust is installed.
set -eu

if command -v cargo >/dev/null 2>&1; then
  exec cargo run --quiet -p xtask -- doctor
fi

echo "(Rust not installed yet — running pre-Rust checks only)"
fail=0
check() {
  if command -v "$1" >/dev/null 2>&1; then
    printf '%-14s ✓ %s\n' "$1" "$(command -v "$1")"
  else
    printf '%-14s ✗ missing\n' "$1"
    fail=1
  fi
}
check git
check node
check pnpm
[ "$fail" -eq 0 ] && echo "Environment ready (pre-Rust)." || exit 1
