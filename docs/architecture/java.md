# Java Runtime Management (Phase 8)

Implementation: `ikk-minecraft::java`. One shared runtime pool under
`<data>/runtimes/` — instances *reference* runtimes; nothing duplicates a JVM
per instance (§21).

## Discovery (§18)

Candidate executables are probed across Windows / Linux / macOS conventions
(JAVA_HOME, PATH, platform-specific install locations). Each discovered runtime
records executable path and parsed major version.

## Compatibility (§17, §19)

Mojang metadata carries `javaVersion.majorVersion` per version. Selection
requires `runtime.major_version >= floor`; failure yields `java.not_suitable`
with the versions actually found — never a silent fallback to whatever `java`
happens to be on PATH. Before launch the validator confirms the selected
executable exists on disk (`java.not_found` otherwise).

## Provisioning strategy (§20)

Auto-provisioning is **planned, not implemented**. The design constraint is
already honored: a documented distribution source will feed the same verified
download engine (checksums + atomic rename) into `<data>/runtimes/java-<major>/`,
and instances reference runtimes by path — so adding it later touches no
instance model.
