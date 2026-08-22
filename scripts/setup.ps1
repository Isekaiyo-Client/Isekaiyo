#!/usr/bin/env pwsh
# Isekaiyo development environment setup (Windows).
# Non-destructive, transparent, idempotent (mirrors scripts/setup.sh).
# Uses winget (built into Windows 10 1809+ / 11). Never disables security features.
$ErrorActionPreference = "Stop"

function Test-Cmd($name) { [bool](Get-Command $name -ErrorAction SilentlyContinue) }

Write-Host "`n== git" -ForegroundColor Cyan
if (-not (Test-Cmd "git")) {
  winget install --id Git.Git -e --accept-source-agreements --accept-package-agreements
} else { git --version }

Write-Host "`n== Node.js >= 22" -ForegroundColor Cyan
if (Test-Cmd "node") { node --version }
else {
  winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
}

Write-Host "`n== pnpm (only supported package manager)" -ForegroundColor Cyan
if (Test-Cmd "pnpm") { pnpm --version }
else {
  corepack enable pnpm
  if (-not (Test-Cmd "pnpm")) {
    Write-Host "corepack unavailable — see https://pnpm.io/installation" -ForegroundColor Yellow
    exit 1
  }
}

Write-Host "`n== Rust toolchain" -ForegroundColor Cyan
if ((Test-Cmd "cargo") -and (Test-Cmd "rustc")) { rustc --version }
else {
  winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
}
rustup component add rustfmt clippy

Write-Host "`n== Visual Studio Build Tools C++ workload (MSVC linker + Windows SDK)" -ForegroundColor Cyan
Write-Host "If missing, run:"
Write-Host '  winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
Write-Host "WebView2 ships with Windows 11; on Windows 10: https://developer.microsoft.com/microsoft-edge/webview2/"

Write-Host "`n== Frontend dependencies" -ForegroundColor Cyan
pnpm install

Write-Host "`nDone. Now run: pwsh ./scripts/doctor.ps1" -ForegroundColor Green
