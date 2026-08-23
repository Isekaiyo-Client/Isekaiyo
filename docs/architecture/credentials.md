# Credential Storage (Phase 9 §10–§11)

## The abstraction

`ikk-core::credentials::CredentialStore` — `store` / `retrieve` / `delete`,
keys namespaced like `msa/<account-id>/tokens`. Core contains the trait plus a
test-only in-memory impl; the production backend lives in the shell.

## Platform backends

The shell wires the `keyring` crate (keyring-rs org; MIT OR Apache-2.0 —
license-compatible, actively maintained, no transitive OAuth machinery):

| Platform | Backend | Feature |
|---|---|---|
| Windows | Windows Credential Manager | `windows-native` |
| macOS | Keychain | `apple-native` |
| Linux | Freedesktop Secret Service | `sync-secret-service` |

## Failure behavior

There is **no plaintext fallback**. If the platform store cannot open, write,
or read, operations fail with the stable `credentials.unavailable` category so
the UI can explain the limitation (e.g. "unlock your keyring"). A missing key
is `Ok(None)` — that's how a corrupted/removed credential becomes
`reauth-required` instead of a crash (§35).

## What never touches credentials

- `accounts.json` (public metadata only)
- instance JSON
- frontend state (no command returns tokens — there is no `getRawToken()`)
- logs, crash reports, argv debug views (`argv_redacted` only)
- Git (no fixtures carry real tokens)
