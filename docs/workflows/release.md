# Release Workflow

## Rules

- Release starts from green `dev`.
- `main` only changes through PR.
- Tag only on `main`.
- Normal PR CI runs checks only; `v*` tag CI builds installers.
- Version must match in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

## Flow

1. Validate `dev`.
2. Push `dev` and wait for CI.
3. Create `release/vX.Y.Z`.
4. Bump version and CHANGELOG.
5. Open PR to `main`.
6. Wait for PR CI and squash merge.
7. Tag on `main`.
8. Verify draft release assets and publish.
9. Sync `main` back to `dev`.

## Commands

```bash
npm run ci
git push origin dev

NEW_VERSION="0.2.0"
RELEASE_BRANCH="release/v$NEW_VERSION"
git checkout -b "$RELEASE_BRANCH"
```

Update:

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `CHANGELOG.md`
- `README.md` / `README.zh-CN.md` if badges or release text changed
- `src/pages/main/AboutPage.svelte` if the displayed version changed

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md README.md README.zh-CN.md src/pages/main/AboutPage.svelte
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

After release:

```bash
git checkout dev
git merge main
git push origin dev
git branch -d "$RELEASE_BRANCH"
git push origin --delete "$RELEASE_BRANCH"
```

## Verify

- `npm run ci` passed locally.
- `dev` CI passed.
- PR CI passed.
- Release CI produced all expected assets. See `docs/workflows/release-naming.md`.
- Draft release notes are accurate.
- Current-platform installer opens.

## Hotfix

```bash
git checkout main
git pull origin main
git checkout -b fix/<name>
# fix, test, commit
gh pr create --base main --head fix/<name> --title "fix: <summary>"
```

After merge, tag on `main`, publish, then merge `main` back to `dev`.

## Known CI Traps

| Trap | Constraint |
|------|------------|
| `tauri-action@v1` missing | Use `tauri-apps/tauri-action@v0`. |
| Linux audio build | Install `libasound2-dev`. |
| Clippy misses test warnings | Always use `--all-targets`. |
| Windows `Instant` underflow | Do not subtract duration from short-uptime `Instant::now()`. |
| ts-rs format drift | Pin ts-rs and exclude generated bindings from Prettier. |
