# ADR-0004: Capability-based version abstraction

**Status:** Accepted

**Context.** Mojang's manifest already outlived its `1.x` naming era (verified live 2026-08); hardcoded version knowledge rots immediately, and scattered `if version == "1.8.9"` checks are unmaintainable at 15+ version lines.

**Decision.** Versions resolve to a **CapabilitySet** (rendering era, shader pipeline, mapping availability, loader support, Java requirement). Client modules/loaders declare required capabilities; availability = set intersection. Adapter crates are the only code permitted to know version ranges (CI-grepped).

**Reasoning.** Adding future versions becomes an adapter concern, not a codebase-wide hunt; features degrade visibly instead of silently misbehaving.

**Consequences.** Slight indirection cost when writing modules; requires honest capability declarations and CI jobs per supported range to keep the compatibility matrix truthful.
