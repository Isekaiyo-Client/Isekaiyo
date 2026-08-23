# Instance Architecture (Phase 8)

## Concepts

Three separate ideas — never merged:

```
MINECRAFT VERSION   what Mojang publishes (1.8.9, 1.20.1, …)
LOADER              how the game is modded (vanilla / Fabric / Forge / Quilt)
INSTANCE            a user's named configuration referencing both
```

An instance is a persistent typed object (`ikk-core::instance::Instance`), not
a launcher profile and not a Minecraft installation.

## Storage layout

Platform paths come from `ikk-core::platform` — no hard-coded `C:\Users\…` or
`/home/…` anywhere (`IKK_DATA_DIR` overrides for development/tests):

```
<data>/
├── config.json
├── instances/
│   ├── .trash/                      # safe-deleted instance files land here
│   └── <instance-id>.json           # the typed Instance document
├── instances/<instance-id>/game/    # per-instance game dir (--gameDir)
│   ├── mods/  ·  ikk/mods.json      # Phase 6 mod state lives with its instance
│   ├── logs/latest-launch.log       # merged stdout/stderr of the last run
│   └── launch-history.jsonl         # append-only: when we launched + PID
└── cache/
    ├── manifest.json                # cached Mojang version manifest
    ├── profiles/<id>.json           # resolved effective metadata per instance
    ├── versions/<ver>/<ver>.jar     # SHARED client jars — never duplicated
    ├── libraries/  ·  assets/       # shared resolution caches
    └── loader-meta/
```

Instance **metadata** is one file; instance **game data** is one directory.
Deleting metadata never touches game data without explicit confirmation.

## Typed model

`Instance` carries `LaunchSettings` — memory bounds, window size/fullscreen,
user JVM args, game args, local env overrides — all optional with serde
defaults so v1 files keep loading forever. Validation rejects impossible
values (512–32768 MiB, control characters in args) instead of clamping.

## CRUD surface

| Operation | Backend | Notes |
|---|---|---|
| create/list/get/update | `InstanceStore` | updates validate + bump `updated_at_unix` |
| rename | `store.rename` | metadata-only through the validated path |
| duplicate | `store.duplicate` | fresh id, settings copied; original untouched |
| delete | `store.trash_delete` | file → `.trash/`; game data needs separate confirm |

## Isolation rule

Every launch passes `--gameDir <data>/instances/<id>/game`, so mods, saves and
config can never cross between instances unless a user deliberately shares.

## Validation & repair

`validate_instance` runs pure filesystem checks (no network): game dir exists,
every planned artifact present + sha1-valid, java executable present. Findings
are structured (`severity/code/path/message`) and map to repair actions that
only ever re-download or re-create directories — nothing is deleted.
