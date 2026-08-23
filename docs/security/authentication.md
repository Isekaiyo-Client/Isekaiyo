# Authentication Security (Phase 9)

## Threat model assumptions

- The user's own machine is trusted at rest; the OS secure store protects
  credentials from other processes and casual disk access.
- Network is untrusted: all auth traffic is TLS via the system-trusted store.
- Microsoft/Mojang endpoints are the authority for identity and entitlement;
  Isekaiyo never decides "premium" locally.

## Token handling policy

1. Tokens exist in exactly three places: inside `ikk_minecraft::msauth`
   transiently, in the OS credential store, and as the one-shot
   `LaunchIdentity.access_token` passed to the planner.
2. Every token-bearing struct implements a redacting `Debug`; unit tests
   assert secrets cannot appear in any `format!("{:?}")` output.
3. No command exposes a token. The IPC surface has list/select/logout/refresh
   only — there is no `getRawToken()` (§27).
4. Logs record *events* ("microsoft account added"), never payloads.
5. Launch argv reaches the UI only through `argv_redacted`, with every secret
   replaced by `[redacted]`.

## Offline profiles

Offline identities are honest by construction: token `"0"` (vanilla offline
convention), stable v3 UUID derived from the username, clearly labeled
`Offline` everywhere. Nothing fabricates entitlements; authenticated servers
reject such sessions by design.

## Audit checklist (run before release)

- `grep -rniE "(access_token|refresh_token|client_secret|bearer)" src/ logs/`
  → matches must be field names/redactions only, never values.
- Confirm no token-bearing type derives plain `Debug`.
- Confirm `SECURITY.md` reporting path stays accurate.

## Reporting

See [SECURITY.md](../../SECURITY.md) — private advisories only.
