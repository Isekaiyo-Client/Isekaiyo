# ADR-0008: Data storage policy

**Status:** Accepted

**Decision (per dataset, not one-size):**

| Dataset | Store | Why |
|---|---|---|
| Instances, configs | versioned JSON files | user-inspectable, portable, diffable, git-friendly for support |
| Metadata caches | JSON + TTL in cache dir | disposable by definition |
| Download cache | content-addressed blobs | dedupe across instances |
| Tokens/secrets | OS keyring (+ encrypted fallback) | never plaintext by default (ADR-0006) |
| Logs | structured tracing → files, redacted on export | diagnostics are product features |

**Explicitly deferred:** SQLite. No dataset currently needs queries/indexes/transactions; adding a DB now would be fashion, not need. **Revisit trigger:** >500 instances or a feature requiring indexed search (tracked as open item). Platform paths come from `directories` conventions — no hardcoded `%APPDATA%` anywhere.
