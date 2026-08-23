# Authentication (Phase 9)

Implementation: `ikk-minecraft::msauth`. Status: **implemented against the
officially documented endpoints; not yet verified end-to-end with a live
Microsoft account** — that requires an Azure app registration (`IKK_MS_CLIENT_ID`)
and a human completing the device flow.

## The flow (OAuth 2.0 Device Authorization Grant)

The user signs in at **microsoft.com/devicelogin** in THEIR browser. Isekaiyo
never renders a login form, never sees a password, never scrapes (spec §5/§52).

```
1. POST login.microsoftonline.com/consumers/oauth2/v2.0/devicecode
   → user_code + verification_uri + polling interval        (UI shows these)
2. POST .../oauth2/v2.0/token  (grant_type=device_code)
   → authorization_pending | slow_down | expired | denied | MSA tokens
3. POST user.auth.xboxlive.com/user/authenticate   (RPS ticket = MSA token)
4. POST xsts.auth.xboxlive.com/xsts/authorize      → XSTS token + user hash
   XErr 2148916233 → "no Xbox profile"; 2148916238 → child account blocked
5. POST api.minecraftservices.com/authentication/login_with_xbox
6. GET  api.minecraftservices.com/minecraft/profile → username + UUID
```

Every stage is validated; no stage may be skipped or faked. Step 6 doubles as
the entitlement check — an account without Java Edition fails here with an
honest message.

## Configuration

`IKK_MS_CLIENT_ID` must hold the project's Azure application (client) ID with
the `XboxLive.signin offline_access` scopes enabled for the device flow.
Missing configuration produces a clear `config.invalid` error — we never embed
or invent a client id.

## Refresh policy (§12)

One silent refresh attempt per operation (launch preparation or explicit
Refresh). Rejection flips the account to `reauth-required`; nothing retries
forever. Polling during initial sign-in is bounded (~5 minutes) server-expiry
honoring the mandated interval.

## Dependencies

No OAuth crate: the flow is plain HTTPS form/JSON calls over the existing
`ureq` agent, so there is nothing to trust beyond RustCrypto-adjacent TLS we
already ship. All parsers are pure functions unit-tested offline against
fixture bodies; all token-bearing types have redacting `Debug` impls.
