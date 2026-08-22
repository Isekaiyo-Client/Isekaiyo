# Vision

Status: **Accepted** · Phase 0 deliverable

## Identity

**Isekaiyo** is an open-source, cross-platform ecosystem for Minecraft: Java Edition consisting of a first-party *launcher* and a first-party *client*, sharing a common core, plus a public plugin API. It ships from scratch: no forks of existing clients or launchers are architectural parents.

## Core Philosophy

> Install Isekaiyo and Minecraft should already feel complete.

A new user should be able to install, authenticate, pick a version, press Play — without ever needing to learn about Fabric, Forge, NeoForge, Quilt, JVM flags, asset indexes, or Java runtimes. Those systems are exposed **when useful** and hidden **when unnecessary**. The first-party team provides the large feature surface; the community extends it via plugins.

## Product Duality

```text
                       ISEKAIYO
                          │
            ┌─────────────┴─────────────┐
      ISEKAIYO CLIENT               LAUNCHER
   First-party modules          Instances / versions
   Client API                   Java runtime management
   HUD framework                Accounts / auth
   PvP / Performance            Loaders / mods / modpacks
   Visuals / Social             Worlds / servers
   Cosmetics                    Marketplace / downloads
            │                           │
            └─────────────┬─────────────┘
                     SHARED CORE
```

### Launcher (works standalone)

Versions, instances (vanilla, Isekaiyo client, Fabric, Forge, NeoForge, Quilt, future loaders where technically and legally appropriate), third-party mods, modpacks, resource packs, shader packs, worlds, servers, Java runtimes, accounts, downloads, updates, logs/crash diagnostics, import/export, configuration, profiles, marketplace browsing.

### Client (optional layer injected into the game)

PvP modules, generalized HUD framework, legitimate performance features, visual customization, social features, cosmetics, replay/screenshot tooling, server-aware behavior, extensive configuration, capability-gated version-specific features.

## Product Principles

1. **Duality over coupling.** The launcher must never require the client; the client must never require a specific launcher UI. Shared logic lives in the core, never in either side ad hoc.
2. **Progressive disclosure.** Simple by default, expert controls available but not in the way.
3. **Isolation over convenience.** Instances are sealed environments; one instance failing must never damage another.
4. **Capability-based compatibility.** Features declare what they need; versions expose what they provide. Never hardcode today's version list.
5. **Trust is explicit.** Executable code (plugins, mods) is treated as untrusted until provenance and permissions are understood. Sandboxing claims are never overstated.
6. **Privacy-first diagnostics.** Local diagnostics are rich; telemetry is minimal, documented, opt-in, and separable.
7. **Build the foundation first.** Research → requirements → architecture → then code. But analysis ends when another developer could implement Milestone 1 without guessing.

## User Personas

| Persona | Description | Primary needs | Explicitly *not* served |
|---|---|---|---|
| **Maya, the new player** | First Minecraft PC install, zero jargon tolerance | One-click install → play | Loader/Java/mod concepts |
| **Kenji, the PvP player** | Plays on 1.8.9-class and modern servers daily | Keystrokes/CPS/FPS HUD, low latency, fast instance switching | Heavy singleplayer tooling |
| **Alex, the modded player** | Runs several big modpacks across loaders | Reliable dependency resolution, imports, 2,000-mod scale | Hand-editing JSON |
| **Sam, the creator** | Records/screenshots/replays | Visual modules, replay tooling, stable captures | Cheating-adjacent features |
| **Devon, the plugin developer** | Builds community extensions | Stable documented API, dev mode, reload, clear permission model | Private API access |
| **Riya, the contributor** | Open-source developer on GitHub | Clean crates, tests, CI, ADRs explaining *why* | Tribal knowledge |
| **Operator, the parent/machine owner** | Cares about safety and privacy | Transparent telemetry, safe defaults, no dark patterns | Hidden background activity |

## Anti-Vision (deliberately avoided)

- A PvP-client-with-a-launcher-bolted-on.
- Forking a low-quality open-source client to save three months.
- Promising identical features on every Minecraft version ever released.
- Invasive analytics, dark-pattern upsells, or bundled advertising.
- Claiming FPS multipliers we cannot benchmark.
