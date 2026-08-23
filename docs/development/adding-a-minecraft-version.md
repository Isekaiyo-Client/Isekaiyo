# Adding a Minecraft Version (contributor guide)

Versions are not hand-registered: the launcher reads Mojang's official
`version_manifest_v2.json` (cached, stale-cache fallback offline). "Supporting"
a version therefore means **proving the pipeline on it**, not adding an entry.

## Checklist

1. **Create a test instance** for the target version (vanilla first).
2. **Install** — confirm metadata parses, libraries/natives/assets resolve.
   Legacy versions (<1.13) exercise the `minecraftArguments` path; modern ones
   the rule-gated `arguments.*` path. Both must work.
3. **Java floor** — check `javaVersion.majorVersion` resolves to an installed
   runtime (`java.not_suitable` means your Java story needs fixing, not the
   metadata).
4. **Dry run** — inspect `dry_run_launch`; verify main class, classpath and
   args look right before spawning.
5. **Launch** — real launch, clean exit classification in the log.
6. **Loaders** — repeat with each supported loader if applicable.

## Version matrix

A row appears here ONLY after step 6 passes. Nothing is claimed SUPPORTED
because the architecture exists.

| Minecraft | Loader | Java | Client adapter | Status |
|---|---|---|---|---|
| *(empty until real tests pass)* | | | | |

Known structural differences already handled by the engine:
- legacy string args vs modern rule-gated args
- pre-1.7.6 `resources` virtual asset dirs (`${game_assets}`)
- per-version natives classifiers
