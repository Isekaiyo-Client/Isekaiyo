# ADR-0009: Licensing strategy

**Status:** Proposed — **requires owner ratification before any public binary ships**

**Context.** Mixed components (code/docs/assets/branding) cannot share one license; distribution legality blocks on this decision.

**Proposal.**
- Code: `MIT OR Apache-2.0` dual-license
- Docs: `CC-BY-4.0`
- Branding/assets: restricted all-rights-reserved + trademark policy

**Dependency rule.** MIT/BSD/Apache/Zlib-family link freely; LGPL requires dynamic-link analysis; GPL never linked into permissively-distributed binaries. Enforced via `cargo deny` nightly.

**Open.** Professional legal review of final texts, CurseForge terms, trademark registration — flagged in [licensing.md](../licensing.md). Until ratified, workspace `license = "TBD"` and no binaries are distributed.
