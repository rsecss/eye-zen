# Branch Protection (GitHub Settings)

These rules live in GitHub repo Settings and are not tracked in code. Configure once per fork.

## `main`

Settings → Branches → Add rule, pattern `main`:

| Setting | Value | Why |
|---------|-------|-----|
| Require a pull request before merging | ✅ | `release.md` rule: "main only changes through PR". |
| Require approvals | 1 (multi-maintainer) / off (solo) | Match team size. |
| Dismiss stale approvals on new commits | ✅ | Avoid stale rubber-stamps. |
| Require status checks to pass | ✅ | Enforce CI parity. |
| Required checks | `Security Audit`, `Windows`, `macOS`, `Linux` | Job names from `.github/workflows/ci.yml`. |
| Require branches to be up to date | ✅ | Force rebase or merge of base before merging. |
| Require linear history | Optional | Squash merge already produces a flat history. |

## Repository-wide (Settings → General → Pull Requests)

| Setting | Value | Why |
|---------|-------|-----|
| Allow squash merging | ✅ | `pr.md`: "Squash merge is preferred." |
| Allow merge commits | ❌ | Discourage merge-commit clutter for normal PRs. |
| Allow rebase merging | Optional | Useful for hotfix PRs. |
| Automatically delete head branches | ✅ | Removes the manual `git push origin --delete` step after each release. |

## `dev`

`dev` is the daily branch. Protection is **optional**.

If enabled, mirror `main`'s required status checks. Without protection, CI failures on `dev` are surfaced via email or workflow notifications rather than blocking pushes — pick based on team discipline.

## Verifying

Push a deliberately broken commit to a throwaway branch and open a PR. The merge button should refuse until checks pass.