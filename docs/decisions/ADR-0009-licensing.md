# ADR-0009: Licensing strategy

**Status:** Accepted (updated during Foundation Audit) — the repository owner committed a **GPL-3.0** `LICENSE` in the initial commit; this ADR now records that decision instead of the earlier provisional MIT/Apache proposal.

**Context.** Mixed components (code/docs/assets/branding) cannot share one license, and distribution legality blocks on this decision. The owner resolved it by committing GPL-3.0 as the project license.

**Decision.**

- **Source code (crates, apps, tools, client):** `GPL-3.0-or-later` — the unmodified GNU GPL v3 text includes §14 ("or (at your option) any later version"); SPDX tag in all Cargo manifests is accordingly `GPL-3.0-or-later`. If the owner intends version-3-only, change manifests and this ADR together.
- **Documentation (`docs/`):** covered by the same GPL for now; a move to CC-BY-4.0 is possible later without affecting code.
- **Assets/branding:** restricted all-rights-reserved + trademark policy (unchanged from the original proposal).

**Dependency rule (reversed implication vs. the old permissive proposal).**
Under GPLv3, permissive dependencies (MIT/BSD/Apache/Zlib) are freely includable; LGPL works under §13 combined-work terms; the constraints now run the *other* direction:

- Downstream distributors of Isekaiyo must comply with GPL terms (source availability for distributed binaries).
- Any code we wish to accept into the tree must be GPL-compatible; do not accept code licensed only under terms incompatible with GPLv3.
- AGPL dependencies require explicit maintainer review before inclusion.

Enforcement via `cargo deny` + `cargo audit` nightly (already configured).

**Consequences.**
- Proprietary/embedded redistribution is excluded by design — acceptable for this project's goals.
- Some third-party SDKs with non-GPL-compatible licenses cannot be vendored; check before adding any dependency.
- Plugin API licensing: plugins calling the API over documented IPC boundaries are generally not derivative works, but the formal position is recorded as an open question in [open-decisions](../architecture/open-decisions.md).

**Rejected alternatives.** MIT/Apache dual-license (the original proposal — superseded by the owner's explicit choice); no-license/"TBD" limbo (blocks any public artifact).
