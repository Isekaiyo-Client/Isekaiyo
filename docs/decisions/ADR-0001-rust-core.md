# ADR-0001: Rust for the native/core layer

**Status:** Accepted · **Supersedes:** none

**Context.** The core orchestrates processes (JVMs), parallel downloads, filesystem state, crypto verification, and a long-lived desktop app across 3 OSes with a growing contributor base.

**Options.** Rust / C++ / C / Kotlin-JVM (see [technology-evaluation](../research/technology-evaluation.md)).

**Decision.** Rust, organized as a Cargo workspace with enforced crate boundaries.

**Reasoning.** Memory/thread safety without GC for a process-spawning, download-heavy core; workspace crates give machine-checkable architecture boundaries; proven in-domain (Modrinth's desktop app). JVM rejected for the *host* to avoid shipping a second runtime and GC jitter in orchestration paths.

**Consequences.** Slower compile loops than JS; learning curve for JVM-backgrounded contributors; strict `unsafe = deny`, clippy `-D warnings` from day one.

**Rejected.** C++ (UB risk across contributors), Kotlin/JVM host, C (no abstraction tooling for this domain).
