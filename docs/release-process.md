# Release Process

Status: **Accepted (design); tooling lands Phase 13** · ADR-0007

## Channels

| Channel | Audience | Exists from |
|---|---|---|
| Nightly | contributors/enthusiasts; unsigned or ad-hoc signed | Phase 2 |
| Alpha | invited testers; signed artifacts | Phase 4 |
| Beta | broad; update-channel tested | Phase 10 |
| Stable | everyone; full packaging + rollback | Phase 14 |

Not all channels exist at once — each is added only when its quality gates can be honored.

## Versioning

Independent version numbers, never coupled to Minecraft's:

- **Launcher/client app**: semver `MAJOR.MINOR.PATCH` from the single workspace version.
- **Plugin API**: separate semver (`api_version: ^1.0` in plugin manifests); breaking change ⇒ major bump + one-cycle overlap where feasible.
- **Config schemas**: per-file `schema_version` with migration chains (never "assume old files").

## Release flow

```text
develop → release/x.y.z branch → hardening fixes → merge to main + tag
→ CI builds tri-platform artifacts (nightly.yml matrix, extended)
→ artifacts signed (Windows: Authenticode; macOS: Developer ID + notarization;
   Linux: AppImage signing + repo metadata)
→ update manifest signed (ed25519, pinned keys) → staged rollout → monitor → full rollout
```

Rollback: previous signed manifest re-pointed; clients verify signatures before applying anything. An update that cannot be verified is never applied — hard abort, not best-effort.
