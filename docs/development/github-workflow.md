# GitHub Workflow

## Branch model (deliberately small)

```text
main        ← stable/releasable only; protected; PRs only; no force pushes
└── develop ← integration branch during active development
    ├── feature/launcher-core      (short-lived)
    ├── fix/download-cache
    └── chore/toolchain-pin
```

- **main**: release-ready. Direct pushes prohibited (branch protection: PR + review + CI required).
- **develop**: integration. Maintainers may push directly *while the team is tiny*; switch to PR-only when ≥ 3 regular contributors (documented migration, no silent change).
- **feature/fix/chore/docs/**: short-lived, deleted after merge.
- **release/**: only when cutting a stabilized release (see release-process.md).

## The standard loop

```sh
git fetch origin
git switch develop
git pull --ff-only
git switch -c feature/my-feature

# …work, test locally (cargo test, pnpm typecheck && pnpm lint)…

git add <files>                 # stage deliberately, not blindly
git commit -m "feat(instances): add instance manifest model"
git push -u origin feature/my-feature
# → open PR on GitHub (template fills automatically) → CI → review → merge
```

After merge:

```sh
git switch develop
git pull --ff-only
git branch -d feature/my-feature
git push origin --delete feature/my-feature   # optional cleanup
```

## Commits — Conventional Commits

```text
feat:      new capability        fix:       bug fix
refactor:  no behavior change    docs:      documentation
test:      tests only            build:     build/deps
ci:        CI config             chore:     maintenance
perf:      performance
```

Scope optional but encouraged: `feat(instances):`, `fix(downloads):`. A commit = one coherent change. `asdf`, `final2` will be asked to be rewritten before merge.

## Git concepts, concretely

| Term | Isekaiyo example |
|---|---|
| **commit** | one snapshot: "add instance manifest model" |
| **local branch** | your `feature/hud-shell` on your machine |
| **remote branch** | same name on GitHub after `git push -u origin feature/hud-shell` |
| **PR** | a *proposal* to merge your remote branch into `develop` — where review & CI happen |
| **merge vs rebase** | PRs merge into `develop` (merge commits OK). Rebase is for *updating your own branch*: `git fetch origin && git rebase origin/develop` |
| **conflict** | two PRs touched the same lines. Resolve locally: `git rebase origin/develop` → edit marked files → `git add` → `git rebase --continue` → re-run tests → `git push --force-with-lease` (never blind `--force`) |

## Roles & access

- **Owner/admin**: repository creator (administrative control retained; not granted to contributors).
- **Maintainers**: merge rights after review; triage; release cuts. GitHub team: `@Isekaiyo-Client/maintainers` (CODEOWNERS placeholder until handles assigned).
- **Contributors**: fork or branch + PR; no admin.
- **Reviewers/plugin devs/translators**: same PR flow; translation via issues labeled `localization` until a Weblate instance exists.

## Git identity & auth

```sh
git config --global user.name  "Your Name"
git config --global user.email "you@example.com"   # use the noreply email if you prefer
```

Authentication: **never** put credentials in repo files. Recommended order: `gh auth login` (GitHub CLI, handles HTTPS auth), SSH keys, or a fine-grained PAT in your credential manager. Check with `gh auth status`.

## Branch protection (configure in repo settings)

- `main`: require PR, ≥1 approval, CI (rust + frontend jobs) green, no force push, no deletions, linear history optional.
- `develop`: initially allow maintainer pushes; add same protections when the team grows.
