# Launch Engine (Phase 8)

The launch pipeline is a fixed sequence of phases (`ikk-core`/`ikk-minecraft::state`),
each with its own stable error category:

```
Preparing → ResolvingMetadata → ResolvingJava → BuildingPlan → Starting → Running
                                                                    ↘ Completed / Failed
```

## Steps

1. **Validate** — instance exists, metadata installed (dry-run can run here).
2. **Metadata** — the *effective* document from `cache/profiles/<id>.json`
   (vanilla JSON or loader-overlaid via `inheritsFrom`; see loaders.md).
3. **Java** — discover runtimes, select one meeting the metadata's
   `javaVersion.majorVersion` floor; `java.not_suitable` names what was found.
4. **Plan** — `planner::build_plan` is the ONLY place arguments are built.
   Modern rule-gated `arguments.*` and legacy `minecraftArguments` strings are
   both handled; placeholders substituted; classpath = client jar + libraries,
   joined with the platform separator.
5. **Spawn** — `process::spawn` pipes stdout/stderr into the instance log on a
   reader thread; argv is never logged. Exit classified completed / crashed /
   user-stopped.

## Dry run (§40–§41)

`dry_run_launch` performs steps 1–4 and returns the resolved plan — java,
main class, JVM/game args, classpath inputs — with `argv_redacted`: every
secret replaced by `[redacted]`. This redacted form is the ONLY argv shape
allowed to reach logs, crash reports, or the UI.

## Secrets policy

- access tokens live only inside the spawn call and `LaunchIdentity`
- `LaunchPlan::argv()` is internal; `argv_redated`/`redact_secrets` for anything observable
- process logs capture game output only

## Crash classification limits

Exit-code heuristics distinguish clean exit / user stop / non-zero exit. We do
NOT claim to detect the cause of a crash; the log tail plus exit category is
the diagnostic surface.
