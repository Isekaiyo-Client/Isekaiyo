# ADR-0010: Client integration mechanism

**Status:** OPEN DECISION — the highest-risk unresolved item in the architecture

**What is unknown.** The exact transformation/classloading pipeline that lets first-party modules inject into arbitrary Minecraft versions without forking per version.

**Why it matters.** Gates Phase 9 entirely; wrong choice means either a per-version fork treadmill or broken anti-cheat compatibility.

**Information required.** A time-boxed technical spike must validate: mapping application across eras, transformation timing, classloader isolation, coexistence with Fabric/NeoForge, and behavior under common server anti-cheat tooling.

**Provisional direction (until spike concludes).** Launch-time mixin-style bytecode transformation for modern versions; dedicated quarantined legacy adapters for 1.7–1.12; `-javaagent` vanilla attach as an early-milestone fallback; everything behind the capability model so no feature assumes one mechanism.

**Deadline for revisiting.** Before Phase 9 begins. Spike results become the closing record of this ADR.
