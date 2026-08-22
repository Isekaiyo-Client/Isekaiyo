# Client Architecture

Status: **Accepted, with ADR-0010 OPEN** · Companion to [Architecture](architecture.md)

## Positioning

The Isekaiyo client is an optional enhancement layer injected into the game process. It is a *client enhancement*, not a cheat platform: it must respect server rules and vanilla server authority. It coexists with loader mods (Fabric/Forge/etc.) but does not depend on them.

## Structure

```text
client/
├── ikk-client-core/      # module registry, event bus, config binding, HUD framework
├── ikk-client-api/       # PUBLIC API surface for first-party modules & plugins
├── modules/              # first-party modules, one crate per category
│   ├── mod-pvp/          # CPS, keystrokes, hit effects/color, armor/item status,
│   │                     # potion status, togglesprint/sneak, zoom, combo, target info
│   ├── mod-hud/          # FPS, ping, coordinates, direction, configurable HUD chrome
│   ├── mod-performance/  # benchmark-gated optimizations (see below)
│   ├── mod-visual/       # animations, particles, nametags, crosshair, screen effects
│   ├── mod-social/       # friends/presence (opt-in service), Discord RPC integration
│   └── mod-utility/      # screenshots, replay hooks, waypoints, diagnostics overlay
└── adapters/
    ├── adapter-modern/   # 1.16+ (exact floor from ADR-0010 spike)
    └── adapter-legacy/   # 1.7.x–1.12.x transformation path
```

## Module Registry

Every module ships a static `ClientModuleDescriptor`:

```rust
pub struct ClientModuleDescriptor {
    pub id: ModuleId,                    // "ikk.keystrokes"
    pub name: LocalizedString,
    pub description: LocalizedString,
    pub category: ModuleCategory,        // PvP | Hud | Performance | Visual | Social | Utility
    pub required_capabilities: &'static [Capability],  // e.g. RENDER_HUD_OVERLAY
    pub dependencies: &'static [ModuleId],
    pub default_enabled: bool,
    pub settings_schema: SettingsSchema, // drives auto-generated config UI
    pub keybinds: &'static [KeybindSpec],
    pub hud: Option<HudElementSpec>,     // position/scale/anchor metadata
}
```

The registry is capability-checked at load: if the active version's adapter cannot supply `RENDER_HUD_OVERLAY`, dependent modules are listed as *unavailable on this version* in the UI — never silently broken.

## Version Adapter Layer

```text
Client Feature ──▶ Capability Interface (ikk-client-api)
                        │
        ┌───────────────┴───────────────┐
   AdapterModern                   AdapterLegacy
   (mixin-style launch-time         (dedicated transformers for
    transformation; mappings-        pre-mapping-era versions)
    based; see ADR-0010)
```

Capabilities (initial set): `RENDER_HUD_OVERLAY`, `INPUT_HOOKS`, `SCREEN_HOOKS`, `WORLD_ACCESS`, `ENTITY_RENDER_HOOKS`, `NETWORK_PACKET_OBSERVE`, `MODERN_SHADER_PIPELINE`, `CUSTOM_KEYBIND_SLOTS`. Adapters declare which capabilities they implement per version range. **No feature code contains version conditionals** — CI greps release code for raw version comparisons outside adapter crates.

## HUD Framework

Generalized element model: anchor + offset + scale + z-order, visibility rules (in-game only / always / specific screens), styling tokens from the theme system, optional entrance animations honoring reduced-motion. Elements are declarative descriptors rendered by the adapter — the same descriptor produces consistent behavior across versions.

## First-Party vs Third-Party vs Minecraft Mods

| System | Author | Runs where | Distribution |
|---|---|---|---|
| Official Isekaiyo modules | Isekaiyo team | In-game via client runtime | Shipped with client |
| Isekaiyo plugins | Community | Host-defined sandbox (launcher UI extensions; in-game via documented API when available) | Signed plugin repository |
| Minecraft mods (Fabric/Forge/…) | Community | Loader-managed | Marketplace sources |

These are never merged: different trust models, lifecycles, and APIs ([Plugin Architecture](plugin-api.md)).

## Performance Modules Policy

No FPS claims without benchmarks. Each performance module must ship with a reproducible benchmark scenario and results recorded in `docs/benchmarks/` before its marketing text may cite any number. Expensive systems (e.g. HUD redraw) are isolated off the render-critical path where the adapter permits.

## Server Compatibility Posture

Modules observe client-local state; they do not send packets the vanilla client wouldn't, do not automate gameplay beyond accessibility norms, and honor an optional per-server feature-policy file so communities can request disabling specific modules on their servers.
