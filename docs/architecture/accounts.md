# Accounts Architecture (Phase 9)

## The separation rule

```
ACCOUNT  answers "who is launching?"        (ikk-core::accounts)
INSTANCE answers "what are they launching?" (ikk-core::instance)
```

Neither aggregate stores fields of the other. The same account can launch any
instance; the same instance can be launched by any account. Launch preparation
joins them through a typed `LaunchIdentity` — nothing else crosses.

## Models

- `Account` — PUBLIC metadata only (id, kind, display name, username, UUID,
  avatar URL, status, timestamps). This exact struct is safe to send to the
  frontend, cache, and log, because it cannot contain secrets.
- `AccountKind` — `microsoft` | `offline`. Future kinds (demo, custom…) are
  additive enum variants; no code hard-codes this list.
- `AccountStatus` — `signed-out`, `authenticated`, `refreshing`, `expired`,
  `reauth-required`, `error`. The frontend reads status; it never guesses.
- `AccountsFile` — versioned (`schema_version`) persistence document with the
  active-account selection.

## Lifecycle

```
add (offline | microsoft) → authenticated → [refresh] → authenticated
                    ↘ refresh rejected → reauth-required → sign-in again
logout → credentials deleted → signed-out (metadata kept)
remove → credentials deleted first → metadata deleted → active cleared
```

## Storage layout

| Data | Location | Notes |
|---|---|---|
| Public metadata | `<data>/accounts.json` | versioned; corrupt file backed up as `.corrupt-<stamp>` |
| MSA tokens | OS secure store, key `msa/<id>/tokens` | never on disk in plaintext |
| Cached MC identity | OS secure store, key `msa/<id>/mc` | username/uuid/token + expiry |
| Offline profiles | metadata only | no credential material exists |

## Launch integration (§29–§31)

`resolve_launch_identity(app)` — the ONLY place tokens leave secure storage:

1. No active account → structured error explaining what's required (§16).
2. Offline account → `LaunchIdentity::offline(username)`; token is vanilla's
   honest `"0"` placeholder.
3. Microsoft account → cached identity if unexpired; otherwise ONE silent
   refresh attempt, then the XBL→XSTS→MC chain again. Failure marks the
   account `reauth-required`; bounded, never retried forever.

The planner receives a one-shot `LaunchIdentity` and knows nothing about OAuth.
