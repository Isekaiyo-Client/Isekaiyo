# Version Architecture

Status: **Accepted** · Companion to [Architecture](architecture.md)

## Discovery, Never Hardcoding

Minecraft versions are discovered from Mojang's authoritative manifest — verified live (2026-08): `launchermeta.mojang.com/mc/game/version_manifest_v2.json`, per-version metadata on `piston-meta` with SHA1 integrity. The current release train has already moved beyond the historical `1.x` naming scheme, which is decisive evidence that **no version knowledge may be hardcoded** in architecture or UI copy.

- The manifest is fetched, ETag/TTL-cached, and rendered as a list of `MinecraftVersionId`s with type (`release`, `snapshot`, `old_beta`, `old_alpha`) and release time.
- Snapshots are opt-in per instance ("show snapshots" toggle).
- Version aliases/channels (e.g., "latest release") are launcher-side labels resolved at use-time.

## Capability Model

Instead of version→feature tables scattered in code:

```text
MinecraftVersionId
      │ resolve
VersionMetadata ──▶ CapabilitySet  ◀── declared by adapter for version range
      │                     │
ClientModule / Loader / Java requirement declares required capabilities
                            │
              available ✕ required = feature availability
```

Initial capabilities include rendering era, shader pipeline support, networking stack era, mapping availability, loader-support matrix, minimum/maximum Java version. Capability *evaluation* lives in one place (`ikk-minecraft` + adapters); everything else queries it.

## Compatibility Matrix

Living document: [`docs/compatibility.md`](compatibility.md). Initial support tiers:

| Tier | Meaning | Initial targets |
|---|---|---|
| **Tier 1** | Full first-party client modules + loaders | Modern releases (floor set by ADR-0010 spike) |
| **Tier 2** | Launcher-managed instances (vanilla/loaders), partial client modules | 1.7.x–1.12.x via legacy adapter |
| **Tier 3** | Launch-only (vanilla) | Everything the manifest serves, within reason |

Honesty rule: features degrade visibly ("not available on this version"), never silently misbehave.

## Version Adapters

Detailed in [client-architecture](client-architecture.md#version-adapter-layer). Rules:

1. Adapter crates are the **only** code allowed to know version ranges.
2. Each adapter declares implemented capabilities; the registry cross-references.
3. Adding a future Minecraft version should require, ideally: zero core changes; adapter updates only where game internals shifted; CI compatibility jobs run per supported range.
4. When Mojang ships something radical (new pipeline, new packaging): adapters absorb it; if a capability itself changes meaning, that's an ADR-worthy event.

## Java Requirements

Version metadata carries Java requirements [V]; `ikk-java` maps them to runtime provisioning policies (see [Java Runtime Architecture](java-runtime.md)) — never a global "one JVM fits all" assumption.
