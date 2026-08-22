# Security Policy

## Reporting a vulnerability

**Do not open a public issue.** Use [GitHub private security advisories](https://github.com/Isekaiyo-Client/Isekaiyo/security/advisories/new) for all security reports.

- Acknowledgment target: **48 hours**
- Triage → fix → coordinated disclosure; credit reporters by default (opt out on request)

## Scope

In scope: the launcher, its update/auth/download mechanisms, plugin host, marketplace integrations, and build/CI infrastructure. Out of scope: vulnerabilities in Minecraft itself (report to Mojang), social engineering of users, and reports from automated scanners without a reproducible path (still welcome, lower priority).

## Rules baked into the project

- Secrets never enter the repository (see [security docs](docs/security.md)); CI secrets live in GitHub settings only.
- All downloads checksum-verified; update artifacts signature-verified (ADR-0007).
- Dependency audits (`cargo audit`/`deny`) run nightly; lockfiles are committed.

## Maintainer hotfix path

Security releases bypass the normal channel schedule: patch branch from the last release tag → review → signed release → staged rollout → disclosure advisory after users have the fix.
