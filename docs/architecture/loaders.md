# Loader Architecture (Phase 5/8)

## Provider seam

One trait — `ikk_minecraft::loaders::LoaderProvider`:

```rust
provider.resolve(agent, mc_version, loader_version, &vanilla_json) -> ResolvedLoader
```

A provider's whole job is turning `(mc version, loader version)` into an
*effective* metadata document (loader profiles overlay vanilla via
`inheritsFrom`). Everything downstream — resolver, downloader, planner,
process — speaks that one shape, so adding a loader touches nothing generic.

## Status (honest, §86)

| Loader | Status | Notes |
|---|---|---|
| Vanilla | SUPPORTED path | the null provider; metadata passes through |
| Fabric | EXPERIMENTAL | official meta v2, merged profile |
| Quilt | EXPERIMENTAL | meta v3, same merged-profile mechanism |
| Forge / NeoForge | PLANNED | explicit not-implemented error; never faked |

## Forge is NOT Fabric (§26)

Historical Forge (≤1.12) ships installer jars and legacy `minecraftArguments`;
modern Forge uses its own version documents with different library layouts.
The strategy when implemented is a *version-aware* Forge provider (split
legacy/modern internally) behind the same trait — never `if version >= x`
branches in common code.

See [adding-a-loader](../development/adding-a-loader.md).
