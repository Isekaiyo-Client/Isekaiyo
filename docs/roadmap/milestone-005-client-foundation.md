# Milestone 005 — Client Runtime Foundation (Phase 7)

## What exists now

`client/` — a Gradle workspace with two projects:

- **`:core`** (`client/core/`) — the entire version-agnostic client API in
  pure Java 21: lifecycle state machine, `ClientContext`, `VersionAdapter`
  contract, `Capability` gating, typed `EventBus` with exception isolation,
  `Module`/`ModuleManager`, typed `Setting`s with metadata, versioned
  `ConfigManager` with migration chain + corrupt-backup recovery,
  `KeybindManager` with conflict detection, `HudManager`/`HudElement`/
  `HudRenderer` abstraction, `ThemeTokens` (sakura/midnight/minimal),
  `NotificationManager`, `ProfileManager`, and four built-in modules proving
  the full chain: FPS HUD, Coordinates HUD, Keystrokes HUD, ToggleSprint.
- **`:fabric-modern`** (`client/fabric-modern/`) — the primary adapter:
  Fabric entrypoint, `FabricVersionAdapter` (honest capability set),
  `FabricHudRenderer` over 1.21's `DrawContext`, event bridging
  (tick/HUD/shutdown).

Launcher untouched; `cargo`/`pnpm` suites unaffected.

## Verification status (honest)

Delivered **without compile/test runs** per the user's standing instruction.
Before any claim of "works in Minecraft":

```sh
cd client && ./gradlew :core:test          # pure-JVM unit suite
./gradlew :fabric-modern:build             # first real Minecraft build
./gradlew runClient                        # manual acceptance, spec §64 A–I
```

Version pins in `gradle.properties` are **documented candidates** — pin exact
values after the first green build (policy stated in that file).

## Acceptance checklist (all pending first build)

1. `:core` tests green on a bare JDK — no Minecraft needed
2. `:fabric-modern` compiles against the pinned MC/Fabric versions
3. Client initializes inside Minecraft; debug text shows identity
4. FPS/Coordinates/Keystrokes toggle, render, persist positions
5. Keybind rebind persists; conflicts warn
6. Profile save/switch hot-swaps module state
7. Corrupt config → backup + defaults + notification

## Known gaps (deliberate)

- Input bridging (raw key/mouse → bus) is stubbed honestly in the adapter —
  wired during the first runnable build.
- ToggleSprint applies no movement flag yet (adapter `applyModuleEffects` is
  an explicit no-op) — nothing is faked.
- In-game settings screen (spec §27) is the next milestone; the typed-setting
  metadata that will auto-render it exists now.
