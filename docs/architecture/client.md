# Isekaiyo Client Architecture

Status: **Foundation (Phase 7)** · The launcher (`apps/`) and the client
(`client/`) are separate products sharing one repository. The launcher launches
Minecraft; the client runs INSIDE Minecraft. No code path connects them at
runtime beyond Minecraft itself loading the client mod.

## 1. Layering

```text
┌──────────────────────────────────────────────────────────┐
│ :core  (pure Java 21, ZERO game imports)                 │
│   IsekaiyoClient · ClientContext · LifecycleState        │
│   EventBus/Events · Module/ModuleManager · Settings      │
│   ConfigManager/Migrations · KeybindManager              │
│   HudManager/HudElement/HudRenderer · ThemeTokens        │
│   NotificationManager · ProfileManager                   │
└──────────────────────▲───────────────────────────────────┘
                       │ implements VersionAdapter ONLY
┌──────────────────────┴───────────────────────────────────┐
│ Version adapters (the only projects importing the game)  │
│   :fabric-modern   ← primary development target          │
│   future: :fabric-legacy, :forge-*, :quilt-*             │
└──────────────────────────────────────────────────────────┘
```

The rule that keeps this sane: **common code never branches on Minecraft
versions**. It asks `VersionAdapter` for state or capabilities; adapters hide
every internal difference.

## 2. Capabilities

`Capability` (HUD, INPUT, KEYBINDS, PLAYER_STATE, …) is how old or limited
versions degrade safely. A module declares required capabilities at
construction; `ModuleManager.register()` marks it *Unavailable* when the
adapter can't provide them — visible in UI, never crashing.

## 3. Lifecycle

Deterministic order (spec §41), enforced by `IsekaiyoClient.initialize()`:

```text
BOOTSTRAPPING → adapter sanity → config load/migrate/recover
→ [modules registered by BuiltInModules] → persisted module state loaded
→ READY
shutdown: SHUTTING_DOWN → save/disable/unload modules → terminal
```

Failure during init ⇒ `FAILED`; the game continues unmodified.

## 4. Event bus

One statically-created `EventBus<T>` per event type in `Events`. Dispatch is a
copy-on-write array walk (allocation-free on hot paths); every listener is
attributed to an owner, exceptions are isolated and logged with that owner,
and `disable` tears down all of a module's subscriptions before `onDisable`.

## 5. How to create your first Isekaiyo module

1. Create a class extending `Module` in
   `client/core/src/main/java/net/isekaiyo/client/core/modules/builtin/`.
2. Fill metadata + requirements in the constructor:

```java
super("zoom", "Zoom", "Hold-to-zoom camera.",
      ModuleCategory.RENDER,
      Capability.of(Capability.HUD),   // capabilities needed
      new String[0]);                  // other modules needed
```

3. Add typed settings as final fields — the settings screen renders itself
   from them:

```java
private final Setting<Float> factor =
    StandardSettings.floating("factor", 3.0f, 1.0f, 10.0f);
```

4. Subscribe to events in `onEnable()`; clean up nothing by hand — disabling
   removes your subscriptions automatically:

```java
@Override protected void onEnable() {
    Events.CLIENT_TICK.subscribe(id(), e -> { /* cheap per-tick work */ });
}
```

5. For HUD output implement `HudElement` too and register both in
   `BuiltInModules.registerAll()`. One line each.
6. Write tests next to the others in `client/core/src/test/`.

Rules: no raw colors outside `ThemeTokens`; no allocations inside `render`;
no game imports anywhere in `:core`.

## 6. Adding a version adapter

1. New Gradle project under `client/`, included in `settings.gradle.kts`.
2. Implement `VersionAdapter` (+ input bridging) against that version's real
   APIs. List ONLY capabilities you actually implemented.
3. Register your entrypoint in the loader's manifest (see
   `fabric.mod.json`).
4. Add the target to the matrix below **only after its first green build**.

## 7. Version support matrix

Honesty rule: a row appears here only after a real build + manual launch.

| MC version | Loader | Adapter | Java | Status |
|---|---|---|---|---|
| 1.21.x | Fabric | `:fabric-modern` | 21 | foundation written; **not yet built/run** |
| others | — | — | — | not attempted |

## 8. Threading

- Game/render thread: everything touching `MinecraftClient`, HUD rendering,
  module `render()`.
- Anywhere else: mutate game state ONLY via
  `VersionAdapter.runOnGameThread(Runnable)`.
- Config I/O is atomic-write but still happens on the calling thread today;
  keep saves off the render loop (module state persists on toggle, which is
  user-frequency).

## 9. Configuration

`<config>/isekaiyo/{client,modules,hud,keybinds}.json` + `profiles/*.json`,
all version-stamped (`config_version`), migrated via pure transforms in
`Migrations`, atomically written, and corrupt-file backed up (never deleted)
with defaults loaded. Unknown fields are ignored for forward compatibility;
invalid setting values are rejected per-setting, never fatal.

## 10. Dependencies & licenses

| Dependency | License | Where |
|---|---|---|
| Gson | Apache-2.0 | `:core` config serialization |
| JUnit 5 | EPL-2.0 | `:core` tests (test scope) |
| Fabric Loader / API / Yarn | Apache-2.0 / CC0-1.0 (mappings) | `:fabric-modern` |

No proprietary client code is used or referenced; Lunar/Feather/Badlion are
UX inspiration only. The client makes no network connections of any kind in
this phase; none are planned without documented opt-in.

## 11. Testing

- `:core`: plain JUnit 5, no Minecraft required — bus isolation/ordering,
  module capability gating, settings validation, config recovery, keybind
  conflicts, profile round-trips. Run: `./gradlew :core:test`
- Adapters: manual acceptance (spec §64) once the first build exists.
- Launcher side is untouched and remains verified by cargo/pnpm suites.
