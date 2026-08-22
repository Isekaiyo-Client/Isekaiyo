# Java Runtime Architecture

Status: **Accepted** · Companion to [Architecture](architecture.md)

## Requirements

Minecraft versions carry Java requirements in their official metadata [V]. One global JVM does not work across eras (8 → 21+). Therefore:

## Design

```text
ikk-java
├── discovery     # scan system JVMs (known install dirs, JAVA_HOME, PATH,
│                 #   macOS java_home -V, Linux update-alternatives)
├── provisioning  # download managed runtimes from an established distribution
│                 #   source into <data>/runtimes/<major>/, checksummed
├── selection     # per-instance: explicit override > instance requirement match
│                 #   > auto-provision best match
└── doctor        # "why did this fail" diagnostics: version mismatch, arch
                  #   mismatch, permissions, broken install
```

- Requirement resolution reads the version metadata's Java major requirement and maps to runtime policies; users may always pick any installed JVM explicitly (with a compatibility warning if it mismatches).
- Managed runtimes are shared read-only across instances; instances reference them by major-version policy, never by absolute path.
- Provisioned runtimes are verified (checksums at provision time, revalidated lazily).
- Removal/garbage collection of unused runtimes after a configurable idle period.

## Failure Modes

| Case | Behavior |
|---|---|
| No suitable Java installed | Auto-provision proposal with size/disclosure before downloading |
| Provision interrupted | Resume/restart; partial dirs discarded via temp-dir swap |
| User-selected incompatible JVM | Warning dialog; allow override ("I know what I'm doing") recorded in instance config |
| Runtime corrupted on disk | Detected at validation; offer re-provision |
