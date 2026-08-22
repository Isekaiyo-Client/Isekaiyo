# Instance Architecture

Status: **Accepted** · Companion to [Architecture](architecture.md)

## Principle

An **instance** is an isolated, reproducible, self-describing Minecraft environment. Instances never share mutable state; corruption is detectable; deletion is recoverable within a grace window.

## On-Disk Contract

```text
<instances>/<instance-id>/
├── isekaiyo-instance.json     # manifest: schema_version, mc_version, loader spec,
│                              #   java requirement, profile settings, module state
├── .minecraft/                # game dir (saves/, resourcepacks/, shaderpacks/,
│                              #   screenshots/, servers.dat, options.txt, logs/)
├── mods/                      # loader-managed mods (present only if loader set)
├── ikk/
│   ├── client-state.json      # per-instance Isekaiyo client module configuration
│   └── plugin-data/           # plugin-scoped storage (permission-scoped)
└── locks/                     # runtime lock files (never user-facing)
```

The `.minecraft/` layout matches standard Minecraft directory conventions so external tools (and users) keep working. The `isekaiyo-instance.json` manifest is the source of truth for launcher metadata; it is validated (`ikk-instances validate`) on load and after import.

## What an Instance Defines

Minecraft version · loader + loader version (or none) · Java runtime selection or requirement · memory · JVM arguments (with safe defaults) · game arguments · installed mods (resolver-owned) · resource packs · shader packs · worlds/saves · server list (its own `servers.dat`) · screenshots · Isekaiyo client module state · instance-level settings overrides.

## Isolation Guarantees

1. **No shared mutable dirs.** Each instance has its own `.minecraft`; shared read-only assets/libraries live in the content-addressed cache.
2. **Per-instance write locks.** Concurrent launcher actions on one instance are serialized; different instances proceed in parallel.
3. **Validation before trust.** Manifest/schema mismatches mark the instance `NeedsAttention`, never silently repaired.
4. **Staged mutation.** Imports and bulk mod installs build into a staging directory, validate, then atomically swap in. Failures leave the original untouched.

## Reproducibility

An instance's *definition* (version, loader, locked mod list with versions/hashes) can be exported as a portable bundle. Re-creating it elsewhere resolves the same artifact set via checksums — not "latest compatible", but pinned versions.

## Import / Export

| Source | Approach |
|---|---|
| Standard Minecraft directory | Adopt-in-place copy (never move user data) |
| Prism Launcher / MultiMC instance | Read instance config + mods; map to our manifest; stage & validate |
| Modrinth App profile | Parse Modrinth modpack/index formats (documented) into resolution plan |
| CurseForge modpack export | Format-supported if terms permit (OQ-3); otherwise document manual path |
| Generic modpack zip (Modrinth `.mrpack`) | First-class: index → ResolutionPlan → download → instance |

All imports are non-destructive: new instance ID always created; collisions impossible by construction (IDs are generated, names are cosmetic).

## Scale

Design target: 500+ instances without UI degradation (list virtualization, lazy validation). Indexing/search beyond that is deferred (ADR-0008 open item).
