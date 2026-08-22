# Security Architecture

Status: **Accepted** · Phase 0 deliverable · See also [privacy](privacy.md), [plugin-api](plugin-api.md), ADR-0006/0007

## Threat Model (STRIDE-lite, per asset)

| Threat | Vector | Mitigation |
|---|---|---|
| Malicious mod/plugin | Marketplace or sideloaded code | Plugins: WASM capability sandbox where feasible, honest trust model otherwise; mods: loader-managed (documented risk to user at install); permission prompts; repository signing |
| Compromised download | MITM, CDN compromise | TLS everywhere; SHA-256 pinning from authoritative metadata [V]; signed update artifacts (ed25519); checksum failure = hard abort |
| Supply chain / dependency attack | crates/npm packages | `cargo audit`/`cargo deny` in CI; lockfiles committed; minimal dependency policy (ADR-0001); frontend deps audited |
| Token theft | Local malware, log leakage, memory scraping | OS keyring storage; tokens redacted from all logs by construction (redaction layer owns token patterns); short-lived refresh cycle; never serialized to disk plaintext by default |
| Credential misuse | Phishing-style auth flows | System-browser OAuth only ([Authentication](#authentication)); the app never sees the password |
| Path traversal / arbitrary file write | Crafted manifests, marketplace metadata, zip imports | All paths derived through a path-allowlist rooted at instance dirs; zip extraction with entry validation; metadata schema+size limits; fuzz tests on parsers |
| Update hijack | Compromised update channel | Signed manifests + artifacts, pinned keys, staged rollout with rollback ([Update Architecture](release-process.md)) |
| Malicious resource/shader packs | Content parsing exploits | Treated as data, loaded by the game not the launcher; documented risk; no launcher-side execution ever |
| Fake package / malicious marketplace metadata | Spoofed projects | Provenance display, official APIs only, report flow |
| DLL/shared-library injection into game | Local attacker | Out of scope for launcher defense; documented; we don't ship DLL search-path hacks |

## Authentication

- **Microsoft accounts only via the official OAuth 2.0 device/browser flow** into Xbox Live → XSTS → Minecraft services. The app never handles the user's Microsoft password.
- Tokens: access (short-lived) cached in session memory; refresh token in OS keyring. Logout wipes keyring entries.
- **Local/offline profiles are a distinct account type**, clearly labeled, never masquerading as authenticated Microsoft accounts, and cannot join online-mode servers — that's server-enforced anyway, and we don't build circumvention. This supports legitimate cases (offline LAN play, cracked-free testing of own content) without violating Mojang's EULA or access controls.

## Legal Constraints

No functionality designed to bypass authentication, pirate assets, or circumvent access controls. Game files are downloaded exclusively from Mojang's official endpoints. Areas flagged for professional legal review before stable release: license finalization, CurseForge terms, trademark registration, privacy-policy text (see [licensing](licensing.md)).

## Incident Response

SECURITY.md defines private reporting (GitHub security advisories), triage SLAs (48h acknowledgment target), coordinated disclosure, and pre-signed hotfix release path via the update channel.
