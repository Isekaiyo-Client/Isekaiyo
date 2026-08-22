#!/usr/bin/env pwsh
# Isekaiyo environment doctor (Windows). Mirrors scripts/doctor.sh.
$ErrorActionPreference = "Stop"

if (Get-Command cargo -ErrorAction SilentlyContinue) {
  cargo run --quiet -p xtask -- doctor
  exit $LASTEXITCODE
}

Write-Host "(Rust not installed yet — running pre-Rust checks only)"
$fail = $false
foreach ($name in @("git", "node", "pnpm")) {
  if (Get-Command $name -ErrorAction SilentlyContinue) {
    Write-Host ("{0,-14} ✓ {1}" -f $name, (Get-Command $name).Source)
  } else {
    Write-Host ("{0,-14} ✗ missing" -f $name)
    $fail = $true
  }
}
if (-not $fail) { Write-Host "Environment ready (pre-Rust)." } else { exit 1 }
