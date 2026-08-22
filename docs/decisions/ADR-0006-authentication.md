# ADR-0006: Authentication architecture

**Status:** Accepted

**Decision.**
- Microsoft accounts exclusively via official OAuth flows (system browser) → Xbox Live → XSTS → Minecraft services. The application never sees or stores the Microsoft password.
- Refresh tokens in the OS keyring (Windows Credential Manager / macOS Keychain / libsecret); access tokens session-memory only; encrypted-file fallback documented for headless cases.
- **Local profiles are a distinct account variant** (`Account::Local`), labeled honestly in the UI, never masquerading as authenticated accounts, and inherently unable to join online-mode servers. We build no circumvention of Mojang/Microsoft access controls.

**Reasoning.** Token theft and phishing are the dominant launcher security failures; the OS keyring + system-browser flow is the best available mitigation set. Blurred local-vs-authenticated accounts (common in third-party launchers) create legal and trust problems.

**Consequences.** Keyring unavailable edge cases need the documented fallback; account UI must carry the honest local-profile labeling even where users might prefer otherwise.
