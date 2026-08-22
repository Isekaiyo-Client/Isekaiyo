# Team Setup Guide (for the founders)

## Person 1 — becoming the primary maintainer

1. Create the repo under the org (`Isekaiyo-Client/Isekaiyo` — exists). You keep **admin**; do not grant admin to anyone else.
2. Configure branch protection (Settings → Branches): `main` requires PR + 1 approval + CI green, no force pushes. `develop` stays pushable for now.
3. Create the `maintainers` team, add yourself; CODEOWNERS already references `@Isekaiyo-Client/maintainers`.
4. Push this foundation to a branch, open the first PR into `develop` so CI runs end-to-end before anyone else clones.
5. Add repo secrets later, only when a milestone needs them (never commit secrets).

## Person 2+ — joining as contributors

1. Accept the org invite (write access to `develop`, never admin).
2. Follow [getting-started](getting-started.md) fully: setup → doctor → run tests.
3. Work per [github-workflow](github-workflow.md): branch from `develop`, Conventional Commits, PR with template.

## Daily collaboration rhythm

```sh
git fetch origin && git switch develop && git pull --ff-only   # start of day
git switch -c feature/<topic>                                  # one topic per branch
# …work + tests…
git push -u origin feature/<topic>                             # PR time
```

Conflicts? Rebase your own branch (`git fetch origin && git rebase origin/develop`), resolve, re-test, `push --force-with-lease`. Never force-push shared branches.

Review etiquette: reviewers respond within ~48h; authors keep diffs small; CI is the arbiter of style.

When the team passes ~3 regulars: flip `develop` to PR-only too ([github-workflow](github-workflow.md) documents this migration).
