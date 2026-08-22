# Plugin Architecture

Status: **Accepted (host-side), in-game surface OPEN** · Companion to [Architecture](architecture.md) · ADR-0005

## What a Plugin Is

A community extension built on the public Isekaiyo Plugin API — distinct from first-party modules and from Minecraft loader mods ([client-architecture](client-architecture.md#first-party-vs-third-party-vs-minecraft-mods)). Target use cases: HUD modules, UI extensions, keybinds, config screens, integrations, utilities, permitted automation, experiments.

## Manifest

```jsonc
{
  "schema_version": 1,
  "id": "com.example.waypoints-plus",
  "name": "Waypoints Plus",
  "version": "1.2.0",
  "api_version": "^1.0",            // semver range against ikk-client-api / plugin API
  "authors": ["..."],
  "dependencies": { "required": [], "optional": [] },
  "permissions": ["storage.scope:waypoints", "network.host:api.example.com"],
  "entry": "main.wasm"              // host-side; see trust model
}
```

## Trust Model — stated honestly

**A manifest is not security.** Plugins are executable code; the default posture is *trust-on-install with informed consent*, plus real technical boundaries where feasible:

| Surface | Mechanism | Honesty note |
|---|---|---|
| Host/launcher plugins | **WASM (wasmtime)** component model: capability-based imports, no ambient authority (fs/net only via granted capabilities) | Genuine sandboxing for pure-logic plugins [I]; performance-sensitive or native needs are out of scope by design |
| In-game client plugins | Phase 12+; likely same WASM host bridging to the module registry | OPEN until the client runtime stabilizes (ADR-0010 dependency) |
| Native plugins | **Not sandboxed. Period.** If ever supported, they require explicit "this can do anything" consent + signing review | We will never claim native code is contained |

Permission categories: `filesystem(scope)`, `network(host/port scope)`, `process` (host-only, heavily flagged), `game.read`, `game.hud`, `ui.panel`, `account.read` (never tokens). The install dialog shows requested capabilities in plain language; changes on update re-prompt.

## Lifecycle & Services

```text
install → verify signature/checksum → permission review → load → init
   → enable/disable at runtime → config persistence → reload (dev mode)
   → unload → uninstall
```

API services granted per permissions: logging, scoped storage, event bus (typed), settings UI registration, HUD element registration, keybind slots, HTTP client bound to granted hosts, scheduled tasks. Update flow uses the standard verified-update pipeline; plugin repository entries are signed.

## Compatibility Policy

- `api_version` semver; breaking API changes bump major and ship alongside the old major for one cycle where feasible.
- A published API compatibility test suite (`plugin-api-tests`) is part of CI; adapters claiming support must pass it.
