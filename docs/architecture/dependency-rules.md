# Dependency Rules

Status: **Accepted** · Enforced mechanically by `cargo xtask arch`; violations fail CI. Any new edge requires an ADR.

## Workspace dependency direction

```text
                    ┌────────────────┐
                    │  ikk-launcher  │   (apps/launcher/src-tauri)
                    │  composition   │
                    └───┬────────┬───┘
                        │        │
             ┌──────────▼──┐  ┌──▼─────────────┐
             │ ikk-core    │◀─│ ikk-api-types  │
             │ foundation  │  │ (IPC DTOs)     │
             └─────────────┘  └────────────────┘

  tools/xtask: standalone (depends on nothing in the workspace)
```

| Dependent | May depend on | Must never depend on |
|---|---|---|
| `ikk-core` | external crates only | any workspace crate |
| `ikk-api-types` | `ikk-core` | apps, Tauri |
| `ikk-launcher` (app shell) | all libraries | — (it is the top) |
| `tools/xtask` | nothing internal | — |

## Rules

1. **Arrows point downward.** Libraries never know about the app; the app composes libraries. If `ikk-core` ever imports `ikk-launcher`, the architecture has failed.
2. **Core stays thin.** Only what every future crate genuinely needs (errors, IDs, task events). Domain logic for versions/instances/mods goes into its *own* future crates (`ikk-minecraft`, `ikk-instances`, … per [architecture.md](../architecture.md)), each allowed to depend on `ikk-core` and `ikk-api-types` — added when they earn their place.
3. **UI never reaches down.** The frontend communicates exclusively through typed Tauri commands (`apps/launcher/src-tauri/src/lib.rs`) using DTOs from `ikk-api-types`. No filesystem/network/process logic in `apps/launcher-ui`.
4. **Platform code is isolated** behind traits (ports); domain code performs no I/O ([architecture.md §1](../architecture.md)).
5. **Future client crates are separate** from launcher crates so the launcher can ship without client code linked ([client-architecture.md](../client-architecture.md)).

## Enforcement

`cargo xtask arch` parses each member manifest, extracts path-dependency edges, and compares them against an allowlist mirroring this document. CI runs it on every push. When you add a crate: update the allowlist **and this document in the same PR**, with an ADR if the edge crosses a layer boundary.
