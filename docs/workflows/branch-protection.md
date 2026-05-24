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
| Allow merge commits | ❌ | Discourage merge-commit clutter. |
| Allow rebase merging | Optional | Useful for hotfix PRs. |
| Allow auto-merge | ✅ | Enables `gh pr merge --auto --squash --delete-branch`: PR auto-merges once required checks pass, eliminating manual wait. |
| Automatically delete head branches | ✅ | Removes the manual `git push origin --delete` step after PR merge. |

## Verifying

Push a deliberately broken commit to a throwaway branch and open a PR. The merge button should refuse until checks pass.
