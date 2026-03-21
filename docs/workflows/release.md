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
- [ ] No Rust warnings: cargo clippy --all-targets -- -D warnings
- [ ] Rust formatted: cargo fmt --all --check
- [ ] Frontend type check: npx svelte-check
- [ ] Frontend format check: npm run format:check
- [ ] Production build succeeds: npm run build
- [ ] ts-rs bindings up-to-date (cargo test regenerates them)
```

> **v0.1.0 教训**：本地跳过 pre-flight 导致 9 次 CI 修复循环。MUST 完整执行。
> clippy MUST 使用 `--all-targets` 以覆盖 test target 代码。

### 3. Push Dev and Wait for CI

```bash
git push origin dev
# MUST wait for three-platform CI to ALL pass before proceeding
# Check: https://github.com/<owner>/<repo>/actions
```

> **v0.1.0 教训**：MUST NOT 跳过此步。直接合并到 main 会在 CI 失败后导致反复修复循环。

### 4. Analyze Changes Since Last Release

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
- **MUST NOT 直接 merge dev → main**：MUST 使用 PR 工作流，等 CI 通过后再合并。

## v0.1.0 CI 踩坑记录

以下问题在首次发版中遇到，已修复并固化为规则，供后续参考：

| 问题 | 根因 | 修复 |
|------|------|------|
| `tauri-action@v1` 不存在 | 上游删除 v1 tag | 使用 `@v0` |
| Linux 编译失败 | 缺 `libasound2-dev` | 加入 apt-get install |
| clippy 通过但 CI 失败 | 本地未用 `--all-targets` | 统一使用 `--all-targets` |
| Linux clippy lint | `#[cfg(target_os)]` 代码仅在对应平台编译 | 跨平台代码需各平台验证 |
| Windows 测试 panic | `Instant::now() - Duration` 下溢 | 用 `future_instant` 模式 |
| SVG 导入类型错误 | Vite 缺 SVG 模块声明 | 切换为 PNG 或加 `vite-env.d.ts` |
| ts-rs 格式漂移 | 不同版本输出格式不同 | 注意 ts-rs 版本一致性 |
