# Release Workflow

## Rules

- Release starts from a green `main`.
- `main` only changes through PR.
- Tag only on `main`.
- Normal PR CI runs checks only; `v*` tag CI builds installers.
- Version parity across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` is enforced by `scripts/check-version-sync.mjs` (runs as step 1 of `npm run ci`).
- Release notes are auto-extracted from `CHANGELOG.md` by `scripts/extract-changelog.mjs`; the body of `## [X.Y.Z]` becomes the GitHub release body.

## Flow

1. Validate `main` locally and confirm latest CI is green.
2. Cut `release/vX.Y.Z` from `main` and bump versions with `scripts/bump-version.mjs`.
3. Fill the CHANGELOG stub and review version-bearing badges.
4. Open PR to `main`.
5. Squash merge after PR CI passes.
6. Tag on `main`; release workflow builds and drafts the release.
7. Verify draft assets and publish.

## Commands

```bash
# 1. Validate main
git checkout main
git pull origin main
npm run ci
gh run list --branch main --limit 1 --json conclusion -q '.[0].conclusion'  # must print "success"

# 2. Cut release branch and bump
NEW_VERSION="0.3.0"
RELEASE_BRANCH="release/v$NEW_VERSION"
git checkout -b "$RELEASE_BRANCH"
node scripts/bump-version.mjs "$NEW_VERSION"
```

The bump script rewrites `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` (eyezen package), `src-tauri/tauri.conf.json`, and prepends a dated `## [X.Y.Z]` stub to `CHANGELOG.md`. `AboutPage.svelte` reads `__APP_VERSION__` injected by Vite and does not need editing.

```bash
# 3. Fill CHANGELOG and (optionally) update README badges that name the version
$EDITOR CHANGELOG.md

# 4. Commit and PR
git add CHANGELOG.md package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git add README.md README.zh-CN.md  # only if you actually changed a version-bearing badge
git commit -m "chore: release v$NEW_VERSION"
git push -u origin "$RELEASE_BRANCH"
gh pr create --base main --head "$RELEASE_BRANCH" --title "release: v$NEW_VERSION"
```

After merge:

```bash
git checkout main
git pull origin main
git tag v$NEW_VERSION
git push origin v$NEW_VERSION
```

If GitHub repo Settings has "Automatically delete head branches" enabled (see `docs/workflows/branch-protection.md`), the `release/vX.Y.Z` branch is deleted on PR merge. Otherwise: `git push origin --delete "$RELEASE_BRANCH"`.

## CHANGELOG entry style

Each `## [X.Y.Z]` body uses categorized H3 sections with emoji titles. Line
format is `- imperative-lowercase description (#NN) (sha7)`. PR number is
omitted only when the change predates the pull-request workflow (v0.1.0).

| Emoji | Category | Conventional Commit types | When to use |
|-------|----------|---------------------------|-------------|
| 🎉 | Features | `feat` | user-visible new capability |
| 🛠️ | Fixes | `fix` | bug fix users would notice |
| 📃 | Documentation | `docs` | doc-only PR worth surfacing in release notes |
| 🧪 | Refactor | `refactor` | internal restructuring with no behavior change |
| 🔧 | Maintenance | `chore`, `ci`, `build`, `perf`, `style`, `test` | infra, deps, CI, build, perf, formatting |

Conventions:

- one bullet per merged PR; squash commit's first-line subject is the source of truth
- omit trailing punctuation on bullet lines
- list features first; within a category, list higher-impact items first
- `release: vX.Y.Z` and `chore(task): archive *` PRs are excluded
- security-relevant fixes also call out the constraint inside the bullet (e.g. `(GHSA-...)`)
- breaking changes get their own section above Features: `### 🚨 Breaking Changes`

Example (v0.3.0):

```markdown
## [0.3.0] - 2026-05-22

### 🎉 Features

- AFK detection skips next rest reminder after idle threshold (#10) (58332ba)
- SQLite rest statistics trends with daily/weekly/monthly charts (#11) (5a68535)
- configurable global hotkeys for start/skip/toggle-pause (#12) (bc96fb5)

### 🔧 Maintenance

- migrate from dev/main to GitHub Flow and sync Cargo.lock in bump-version.mjs (#7) (b15a993)
```

`scripts/extract-changelog.mjs <version>` extracts the body between
`## [X.Y.Z]` markers — no rendering hooks, so anything you write here lands
verbatim in the GitHub release body.

## Verify

- `npm run ci` passed locally (includes version sync gate and coverage threshold).
- PR CI passed.
- Release CI produced all expected assets. See `docs/workflows/release-naming.md`.
- Draft release notes are accurate (auto-extracted from `CHANGELOG.md`).
- Current-platform installer opens.

## Hotfix

```bash
git checkout main
git pull origin main
git checkout -b fix/<name>
# fix, test, commit
gh pr create --base main --head fix/<name> --title "fix: <summary>"
```

After merge, tag on `main` and publish.

## Known CI Traps

| Trap | Constraint |
|------|------------|
| `tauri-action@v1` missing | Use `tauri-apps/tauri-action@v0`. |
| Linux audio build | Install `libasound2-dev`. |
| Clippy misses test warnings | Always use `--all-targets`. |
| Windows `Instant` underflow | Do not subtract duration from short-uptime `Instant::now()`. |
| ts-rs format drift | Pin ts-rs and exclude generated bindings from Prettier. |
| Version drift across files | `check-version-sync.mjs` runs in `npm run ci`; CHANGELOG must have a matching `## [X.Y.Z]` section before release CI starts. |
