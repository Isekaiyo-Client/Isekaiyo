# Licensing Considerations

Status: **Provisional — ADR-0009 open** · No LICENSE file committed until the maintainer ratifies it (stop-condition: legal commitments need explicit owner sign-off).

## Recommendation (not yet ratified)

- **Code:** dual-license `MIT OR Apache-2.0` — permissive, ecosystem-compatible, standard for Rust projects.
- **Docs:** CC-BY-4.0.
- **Assets/branding:** separate restricted license (trademark protection for the Isekaiyo name/logo).
- **Plugin API:** same as code; API compatibility is versioned, not licensed differently.

## Third-party dependency policy

Before adding any dependency, record in the PR: license (must be MIT/BSD/Apache/Zlib-family; LGPL only with dynamic-link analysis; GPL never linked into permissively-licensed binaries), maintenance status, and platform support. `cargo deny` + `cargo audit` run nightly. Third-party notices are generated into distribution bundles (Phase 13).

## Trademark & attribution

- Isekaiyo is an independent brand; **not affiliated with, endorsed by, or connected to Mojang Studios or Microsoft**. "Minecraft" is used only referentially.
- Third-party project names (Fabric, Forge, NeoForge, Quilt, Modrinth, Prism…) belong to their owners; used referentially only.
- Areas requiring professional legal review before stable: final license text, CurseForge terms, trademark registration jurisdictions, privacy-policy text.
