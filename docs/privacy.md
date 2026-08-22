# Privacy Architecture

Status: **Accepted** · Companion to [security](security.md)

## Model: privacy-first, telemetry opt-in

| Data class | Default | Notes |
|---|---|---|
| Local diagnostics (logs, crash reports) | On-device only | Rich, structured, redacted; export is user-initiated |
| Telemetry | **Off** | If enabled later: minimal counters (feature usage buckets, launch success/failure category), no identifiers beyond a random rotating ID, documented schema published in-repo, one-tap off |
| Crash reporting | Off for auto-upload; local always | Upload is per-crash consent with preview of exactly what's sent |
| Account information | OS keyring / session memory | Never in telemetry, never in diagnostics bundles |
| Marketplace queries | Sent to the queried provider only | No Isekaiyo middleman proxying search traffic |

## Diagnostics Bundle Redaction

`ikk-diagnostics` produces shareable bundles through a redaction pipeline that strips: auth tokens (pattern + value-based), account emails/UUIDs (unless user opts in), home-directory paths (rewritten to `~`-style), and third-party tokens seen in game logs. Redaction rules are unit-tested against known secret shapes; a bundle preview is shown before saving.

## Commitments

1. No advertising SDKs, no third-party analytics, no fingerprinting.
2. Data minimization: if a feature doesn't need data, it isn't collected.
3. Localization-ready privacy copy; plain-language permission prompts.
4. Privacy policy text ships in-repo (`docs/privacy-policy-draft.md`, legal review pending — see licensing).
