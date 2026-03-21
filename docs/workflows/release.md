# Eyezen Release Workflow

Automate version release for the Eyezen Tauri v2 desktop app.

## Usage

```bash
/release [-p|--patch] [-mi|--minor] [-ma|--major] [<version>]
```

## Parameters

- `-p` or `--patch`: Patch version (default) — bug fixes, minor changes
- `-mi` or `--minor`: Minor version — new features, backward compatible
- `-ma` or `--major`: Major version — breaking changes
- `<version>`: Exact version number (e.g., 0.2.0, 1.0.0-beta.1)

## Context

- Tauri v2 app with three version files that must stay in sync
- Protected `main` branch — all releases go through PR
- GitHub Actions `release.yml` auto-builds cross-platform installers on `v*` tag
- No changeset/npm publish — version artifacts are GitHub Release binaries

## Version Files (Must Stay in Sync)

| File | Field | Example |
|------|-------|---------|
| `package.json` | `"version"` | `"0.1.0"` |
| `src-tauri/Cargo.toml` | `version` under `[package]` | `"0.1.0"` |
| `src-tauri/tauri.conf.json` | `"version"` | `"0.1.0"` |

### Additional Version References (update manually)

| File | Location | Notes |
|------|----------|-------|
| `README.md` | Version badge | Static shield.io badge |
| `README.zh-CN.md` | Version badge | Same as above |
| `src/pages/main/AboutPage.svelte` | `APP_VERSION` | Displayed in About page |

## Execution Flow

### 1. Parameter Parsing

Parse arguments to determine version bump type or exact version.

Default: `patch`.

### 2. Pre-flight Checks

```
- [ ] Working directory is clean (no uncommitted changes)
- [ ] On `dev` branch
- [ ] All tests pass: cargo test + npm test
- [ ] No Rust warnings: cargo clippy -- -D warnings
- [ ] Frontend type check: npx svelte-check
- [ ] Format check: npm run format:check + cargo fmt --check
- [ ] Production build succeeds: npm run tauri build
```

### 3. Analyze Changes Since Last Release

```bash
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [ -z "$LAST_TAG" ]; then
  COMMITS=$(git log --oneline)
else
  COMMITS=$(git log $LAST_TAG..HEAD --oneline)
fi
```

Display: commit history, file change statistics, change categories.

### 4. Calculate New Version

```bash
CURRENT_VERSION=$(node -p "require('./package.json').version")

# For patch/minor/major: use semver bump
# For custom: use provided version directly
```

### 5. Update Version in All Three Files

```bash
NEW_VERSION="<calculated>"

# package.json
node -e "
  const pkg = require('./package.json');
  pkg.version = '$NEW_VERSION';
  require('fs').writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
"

# src-tauri/tauri.conf.json
node -e "
  const conf = require('./src-tauri/tauri.conf.json');
  conf.version = '$NEW_VERSION';
  require('fs').writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n');
"

# src-tauri/Cargo.toml — sed replace version line under [package]
sed -i 's/^version = ".*"/version = "'$NEW_VERSION'"/' src-tauri/Cargo.toml

# Verify sync
echo "package.json:      $(node -p "require('./package.json').version")"
echo "tauri.conf.json:   $(node -p "require('./src-tauri/tauri.conf.json').version")"
echo "Cargo.toml:        $(grep '^version' src-tauri/Cargo.toml | head -1)"
```

### 6. Update CHANGELOG.md

Add new version section at the top following [Keep a Changelog](https://keepachangelog.com/) format.

Structure:
```markdown
## [X.Y.Z] - YYYY-MM-DD

### Features
- ...

### Fixes
- ...

### Other
- ...
```

Generate content by analyzing commits since last tag, categorizing by conventional commit prefix.

### 7. Create Release Branch and Commit

```bash
RELEASE_BRANCH="release/v$NEW_VERSION"
git checkout -b "$RELEASE_BRANCH"

# Commit version bump
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "chore: release v$NEW_VERSION"

# Push release branch
git push -u origin "$RELEASE_BRANCH"
```

### 8. Create Pull Request

```bash
gh pr create \
  --base main \
  --title "release: v$NEW_VERSION" \
  --body "$(cat <<'EOF'
## Release v$NEW_VERSION

### Changes

See [CHANGELOG.md](CHANGELOG.md) for full details.

### Pre-release Checklist

- [x] Version synced across package.json, Cargo.toml, tauri.conf.json
- [x] CHANGELOG.md updated
- [x] All tests pass (cargo test + npm test)
- [x] No warnings (clippy + svelte-check)
- [x] Production build verified

### Post-merge Actions (Automatic)

After this PR is merged to `main`:
1. Manually create and push tag: `git tag v$NEW_VERSION && git push origin v$NEW_VERSION`
2. GitHub Actions `release.yml` triggers automatically
3. Cross-platform builds (Windows, macOS ARM/Intel, Linux)
4. Draft GitHub Release created with all installers

### Platform Test Status

| Platform | Status |
|----------|--------|
| Windows | ✅ Tested |
| macOS | ⚠️ Untested |
| Linux | ⚠️ Untested |
EOF
)"
```

## Post-Merge Steps

After the PR is merged to `main`:

```bash
git checkout main
git pull origin main
git tag v$NEW_VERSION
git push origin v$NEW_VERSION
```

This triggers `release.yml` → cross-platform build → draft GitHub Release.

Review the draft release on GitHub, then click **Publish**.

## Important Notes

- **Three-file version sync** is critical. Never update only one file.
- **Tag after merge**, not before. The tag must be on `main`.
- **Draft release**: Review installer assets before publishing.
- **No npm publish**: This is a desktop app, not a library.
- Always update `CHANGELOG.md` before release.
- Always run full test suite before starting release.
