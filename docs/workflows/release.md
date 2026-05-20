# Release Workflow

## Rules

- Release starts from green `dev`.
- `main` only changes through PR.
- Tag only on `main`.
- Normal PR CI runs checks only; `v*` tag CI builds installers.
- Version parity across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` is enforced by `scripts/check-version-sync.mjs` (runs as step 1 of `npm run ci`).
- Release notes are auto-extracted from `CHANGELOG.md` by `scripts/extract-changelog.mjs`; the body of `## [X.Y.Z]` becomes the GitHub release body.

## Flow

1. Validate `dev` locally and confirm CI is green.
2. Cut `release/vX.Y.Z` and bump versions with `scripts/bump-version.mjs`.
3. Fill the CHANGELOG stub and review version-bearing badges.
4. Open PR to `main`.
5. Squash merge after PR CI passes.
6. Tag on `main`; release workflow builds and drafts the release.
7. Verify draft assets and publish.
8. Back-merge `main` into `dev`.

## Commands

```bash
# 1. Validate dev
npm run ci
git push origin dev
gh run list --branch dev --limit 1 --json conclusion -q '.[0].conclusion'  # must print "success"

# 2. Cut release branch and bump
NEW_VERSION="0.2.0"
RELEASE_BRANCH="release/v$NEW_VERSION"
git checkout -b "$RELEASE_BRANCH"
node scripts/bump-version.mjs "$NEW_VERSION"
```

The bump script rewrites `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and prepends a dated `## [X.Y.Z]` stub to `CHANGELOG.md`. `AboutPage.svelte` reads `__APP_VERSION__` injected by Vite and does not need editing.

```bash
# 3. Fill CHANGELOG and (optionally) update README badges that name the version
$EDITOR CHANGELOG.md

# 4. Commit and PR
git add CHANGELOG.md package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
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

Back-merge:

```bash
git checkout dev
git merge main
git push origin dev
```

If GitHub repo Settings has "Automatically delete head branches" enabled (see `docs/workflows/branch-protection.md`), the `release/vX.Y.Z` branch is deleted on PR merge. Otherwise: `git push origin --delete "$RELEASE_BRANCH"`.

## Verify

- `npm run ci` passed locally (includes version sync gate).
- `dev` CI green (`gh run list` above).
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

After merge, tag on `main`, publish, then back-merge `main` to `dev`.

## Known CI Traps

| Trap | Constraint |
|------|------------|
| `tauri-action@v1` missing | Use `tauri-apps/tauri-action@v0`. |
| Linux audio build | Install `libasound2-dev`. |
| Clippy misses test warnings | Always use `--all-targets`. |
| Windows `Instant` underflow | Do not subtract duration from short-uptime `Instant::now()`. |
| ts-rs format drift | Pin ts-rs and exclude generated bindings from Prettier. |
| Version drift across files | `check-version-sync.mjs` runs in `npm run ci`; CHANGELOG must have a matching `## [X.Y.Z]` section before release CI starts. |
