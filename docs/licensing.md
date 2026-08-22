# Licensing

Status: **Decided (ADR-0009, Accepted)** · The repository `LICENSE` is **GPL-3.0**, committed by the project owner in the initial commit. All Cargo manifests carry `license = "GPL-3.0-or-later"` (see the ADR for the `-or-later` reading; flag an issue if version-3-only was intended).

## What applies to what

| Component | License |
|---|---|
| Source code (crates, apps, tools, client) | GPL-3.0-or-later |
| Documentation (`docs/`) | GPL for now; CC-BY-4.0 migration possible later |
| Assets / branding / logo | Restricted (all rights reserved) + trademark policy |

## Third-party dependency policy

Before adding any dependency, record in the PR: license, maintenance status, platform support.

- ✅ Freely usable under GPL: MIT, BSD, Apache-2.0, Zlib, MPL-2.0-family (with file-level copyleft respected), ISC.
- ⚠️ Review required: LGPL (combined-work terms, §13), AGPL, and anything with a network-use clause.
- ❌ Never: licenses incompatible with GPLv3, or unlicensed code.

`cargo deny` + `cargo audit` run nightly; lockfiles are committed. Third-party notices are generated into distribution bundles (Phase 13). Note the GPL consequence: anyone distributing Isekaiyo binaries must comply with GPL source-availability terms.

## Trademark & attribution

- Isekaiyo is an independent brand; **not affiliated with, endorsed by, or connected to Mojang Studios or Microsoft**. "Minecraft" is used only referentially.
- Third-party project names (Fabric, Forge, NeoForge, Quilt, Modrinth, Prism…) belong to their owners; used referentially only.
- Areas still requiring professional legal review before stable release: trademark registration jurisdictions, CurseForge marketplace terms, privacy-policy text, plugin-API derivative-work position ([open decisions](architecture/open-decisions.md)).
