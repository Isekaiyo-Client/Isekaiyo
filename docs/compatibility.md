# Compatibility Matrix

Status: **Living document** · Updated per release · Tiers defined in [version-architecture](version-architecture.md)

## Legend

✅ full support · 🟡 partial (see notes) · ⛔ not supported · ❔ untested/planned

## Launcher (instances, downloads, Java, auth)

| Version range | Vanilla | Fabric | NeoForge | Forge | Quilt |
|---|---|---|---|---|---|
| Current releases (manifest-served) | ✅ | ✅ | ✅ | ✅ | ✅ |
| 1.13 – recent | ✅ | ✅ | n/a→✅ | ✅ | ✅ |
| 1.7.2 – 1.12.2 | ✅ | ⛔¹ | ⛔ | ✅² | ⛔¹ |
| old_beta / old_alpha | 🟡³ | ⛔ | ⛔ | 🟡 | ⛔ |

¹ Fabric/Quilt do not target these versions.
² Via legacy install pipeline (installers/processors); adapter required.
³ Launch-only; assets servers for ancient versions are best-effort.

## Isekaiyo Client Modules

| Capability / module family | Modern (Tier 1 floor — pending ADR-0010 spike) | Legacy 1.7–1.12 (Tier 2) |
|---|---|---|
| HUD framework (FPS/ping/coords/direction) | ❔ → planned Tier 1 | ❔ legacy adapter target |
| PvP set (keystrokes/CPS/togglesprint/hit effects…) | ❔ | ❔ priority for 1.8.x |
| Performance modules | ❔ benchmark-gated | ⛔ initially |
| Visual modules | ❔ partial | ⛔ initially |

Cells move from ❔ to ✅/🟡 only with CI compatibility jobs passing on real launches.

## Platform Matrix

| Platform | Packaging | Support tier |
|---|---|---|
| Windows 10/11 x64 | NSIS installer + portable zip | Tier 1 |
| Linux x64 | AppImage + Flatpak + deb/rpm/AUR (community) | Tier 1 |
| macOS 13+ (Apple Silicon) | signed & notarized dmg | Tier 1 |
| macOS Intel | universal binary where practical | Tier 2 |

## Rules

- This matrix is generated-checked in CI against the actual test grid; a row claiming ✅ without a green job is a release blocker.
- Deprecation policy: a version row moves to ⛔ only via a documented release note with at least one minor release of warning.
