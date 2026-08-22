# ADR-0000: Foundation before code

**Status:** Accepted

**Decision.** Phase 0 produces research, ADRs, the domain model, and the repository/tooling foundation **before** feature implementation. No placeholder-code theater; no infinite analysis either — each phase ends when "another developer can proceed without guessing".

**Reasoning.** The costliest failures in projects of this scale are architectural (version handling, plugin trust, launcher/client coupling). They are cheap to fix on paper and brutal to fix in code.

**Consequence.** Implementation milestones (roadmap Phases 1–14) inherit explicit contracts and CI enforcement instead of folklore.
