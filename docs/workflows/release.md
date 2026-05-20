# Release Workflow

Complete release process from dev branch to published GitHub Release.

## Overview

```
dev branch ready → local validation → push & CI green → PR to main
    → squash merge → tag on main → Release CI builds → publish
```

## Prerequisites

- On `dev` branch
- Working directory clean
- All features for this release committed and pushed
- Shell commands below require Bash (Git Bash / WSL on Windows)

## Step-by-Step

### 1. Local Pre-flight Validation

MUST run ALL checks locally. Do NOT rely on CI to catch issues.

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test -- --run
npm run format:check
npm run build
```

> **v0.1.0 lesson**: Skipping pre-flight caused 9 CI fix-merge cycles.
> `--all-targets` is required for clippy — without it, test-target warnings are missed.
> `cargo test` also regenerates ts-rs bindings — verify no unexpected diffs.
> Use `npm run ci` as the single local/cloud parity entrypoint; Rust and Node are pinned by `rust-toolchain.toml` and `.nvmrc`.

### 2. Push Dev and Wait for CI Green

```bash
git push origin dev
```

Go to GitHub Actions and wait for **all three platforms** (Windows / macOS / Linux) to pass parity checks.

MUST NOT proceed until CI is fully green. If CI fails:
1. Fix the issue on `dev`
2. Push again
3. Wait for CI green again

### 3. Bump Version

Three files MUST stay in sync:

| File | Field |
|------|-------|
| `package.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` under `[package]` |
| `src-tauri/tauri.conf.json` | `"version"` |

Additional references to update manually:
- `README.md` / `README.zh-CN.md` — version badge
- `src/pages/main/AboutPage.svelte` — hardcoded version string in `<span class="version">`

```bash
NEW_VERSION="0.2.0"  # adjust as needed

# Update package.json
node -e "
  const pkg = require('./package.json');
  pkg.version = '$NEW_VERSION';
  require('fs').writeFileSync('package.json', JSON.stringify(pkg, null, 2) + '\n');
"

# Update tauri.conf.json
node -e "
  const conf = require('./src-tauri/tauri.conf.json');
  conf.version = '$NEW_VERSION';
  require('fs').writeFileSync('src-tauri/tauri.conf.json', JSON.stringify(conf, null, 2) + '\n');
"

# Update Cargo.toml
sed -i 's/^version = ".*"/version = "'$NEW_VERSION'"/' src-tauri/Cargo.toml

# Verify sync
echo "package.json:    $(node -p "require('./package.json').version")"
echo "tauri.conf.json: $(node -p "require('./src-tauri/tauri.conf.json').version")"
echo "Cargo.toml:      $(grep '^version' src-tauri/Cargo.toml | head -1)"
```

### 4. Update CHANGELOG.md

Add new version section at the top following [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- ...

### Fixed
- ...

### Changed
- ...
```

Generate content by analyzing commits since last tag:
```bash
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

### 5. Create Release Branch and PR

MUST use PR workflow. MUST NOT directly merge dev to main.

```bash
RELEASE_BRANCH="release/v$NEW_VERSION"
git checkout -b "$RELEASE_BRANCH"
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md \
      README.md README.zh-CN.md src/pages/main/AboutPage.svelte
git commit -m "chore: release v$NEW_VERSION"
git push -u origin "$RELEASE_BRANCH"
```

Create PR:
```bash
gh pr create \
  --base main \
  --title "release: v$NEW_VERSION" \
  --body "$(cat <<'EOF'
## Release v$NEW_VERSION

See [CHANGELOG.md](CHANGELOG.md) for full details.

### Pre-release Checklist

- [x] Version synced (package.json, Cargo.toml, tauri.conf.json)
- [x] CHANGELOG.md updated
- [x] All local checks pass
- [x] Dev branch CI green (3 platforms)
- [ ] PR CI green (3 platforms)

### Post-merge

1. Tag: `git tag v$NEW_VERSION && git push origin v$NEW_VERSION`
2. Release CI auto-triggers (4-target build)
3. Review draft release, then Publish
EOF
)"
```

### 6. Wait for PR CI Green, Then Merge

- CI runs automatically on the PR
- Use **Squash Merge** to keep main history clean
- This avoids the v0.1.0 problem of 9 merge commits polluting history

### 7. Tag on Main

```bash
git checkout main
git pull origin main
git tag v$NEW_VERSION
git push origin v$NEW_VERSION
```

Tag MUST be on `main`, MUST NOT be on `dev` or release branch.

### 8. Verify Release

- `release.yml` triggers automatically on `v*` tag
- Four-target build: Windows / macOS ARM / macOS Intel / Linux
- Windows portable zip is created automatically
- Wait for all builds to complete
- Verify Draft Release on GitHub:
  - All expected assets present (see `docs/workflows/release-naming.md`)
  - Download and spot-check at least the current platform's installer
  - Verify release notes are accurate
- Click **Publish** when satisfied

> Note: Code signing (macOS notarization, Windows SmartScreen) is not yet configured.
> When available, verify signing status before publishing.

### 9. Post-release

```bash
# Sync main back to dev
git checkout dev
git merge main
git push origin dev

# Clean up release branch
git branch -d release/v$NEW_VERSION
git push origin --delete release/v$NEW_VERSION
```

Write retrospective entry in `docs/devlog.md` (local file).

## Hotfix Process

When main has a bug but dev has unfinished features:

```bash
git checkout main
git checkout -b fix/critical-bug
# Fix + test + commit
# MUST create PR to main (same as release — no direct merge)
gh pr create --base main --title "fix: critical bug description" --body "..."
# After PR merged:
git checkout main && git pull
git tag v0.2.1
git push origin v0.2.1
# Sync back to dev
git checkout dev
git merge main
```

## CI Notes (v0.1.0 Lessons)

| Issue | Root Cause | Fix |
|-------|-----------|-----|
| `tauri-action@v1` not found | Upstream removed v1 tag | Use `@v0` |
| Linux build failure | Missing `libasound2-dev` | Add to apt-get |
| Clippy passes locally but fails CI | Didn't use `--all-targets` | Always use `--all-targets` |
| Linux-only clippy lints | `#[cfg(target_os)]` code only compiled on that platform | Review platform-gated code |
| Windows test panic | `Instant::now() - Duration` underflows on short-uptime CI | Use `future_instant` pattern |
| SVG import type error | Vite missing module declaration | Use PNG or add `vite-env.d.ts` |
| ts-rs format drift | Different versions produce different output | Pin ts-rs version + exclude from prettier |

## References

- Asset naming: `docs/workflows/release-naming.md`
- PR workflow: `docs/workflows/pr.md`
- Development workflow: `docs/workflows/dev.md`
- Change checklists: `.trellis/spec/architecture/change-management.md`
- Testing requirements: `.trellis/spec/architecture/testing-quality.md`
