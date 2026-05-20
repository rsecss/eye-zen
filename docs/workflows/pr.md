# Eyezen PR Workflow

Create pull request to `main` with standardized description.

## Usage

```bash
/pr [--draft] [--title <title>]
```

## Parameters

- `--draft`: Create as draft pull request
- `--title <title>`: Custom PR title (default: auto-generated from commits)

## Context

- Protected `main` branch — all changes go through PR (including releases and hotfixes)
- Development happens on `dev` branch
- Release uses `release/vX.Y.Z` branch (see `docs/workflows/release.md`)
- Hotfix uses `fix/<name>` branch from `main`
- CI runs automatically on PR creation (three-platform matrix)
- Conventional Commits format used throughout

## Execution Flow

### 1. Pre-flight Checks

```
- [ ] On `dev` branch (not `main`)
- [ ] Working directory is clean
- [ ] All tests pass locally
- [ ] Branch is up to date with remote
```

### 2. Analyze Branch Changes

```bash
MERGE_BASE=$(git merge-base main HEAD)
COMMITS=$(git log $MERGE_BASE..HEAD --oneline)
COMMIT_COUNT=$(echo "$COMMITS" | wc -l)
CHANGED_FILES=$(git diff --name-only $MERGE_BASE..HEAD)
```

### 3. Auto-categorize Changes

Analyze commits and files to detect:

| Category | Detection |
|----------|-----------|
| New feature | Commits with `feat` prefix |
| Bug fix | Commits with `fix` prefix |
| Breaking change | Commits with `!` or `BREAKING CHANGE` |
| Documentation | `.md` files changed |
| Tests | `test` or `spec` files changed |
| CI/CD | `.github/workflows/` files changed |
| Rust backend | `src-tauri/` files changed |
| Frontend | `src/` files changed (excluding bindings) |

### 4. Generate PR Title

Priority order:
1. Custom `--title` argument
2. If single feature: use its commit message
3. If multiple features: summarize scope (e.g., `feat: theme switching + autostart`)
4. Fallback: branch name

### 5. Generate PR Description

Template:

```markdown
## Summary

[1-3 sentence description of what this PR does]

### Changes

[Categorized list from commit analysis]

#### Backend (src-tauri/)
- ...

#### Frontend (src/)
- ...

#### Documentation
- ...

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] CI/CD

## Testing

- [ ] `cargo test` passes
- [ ] `npm test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `npx svelte-check` clean
- [ ] `npm run format:check` clean
- [ ] Manual smoke test on Windows

## Checklist

- [ ] Code follows project rules (see `.trellis/spec/`)
- [ ] Self-review completed
- [ ] No new warnings introduced
- [ ] CLAUDE.md updated (if architecture changed)
- [ ] CHANGELOG.md updated (if releasing)

## Commit History

```
[git log output]
```

---
Branch: `dev` → `main` | Commits: N | Files: N
```

### 6. Push and Create PR

```bash
git push origin dev

DRAFT_FLAG=""
if [ "$IS_DRAFT" = true ]; then
  DRAFT_FLAG="--draft"
fi

gh pr create \
  --base main \
  --head dev \
  --title "$PR_TITLE" \
  --body "$PR_DESCRIPTION" \
  $DRAFT_FLAG
```

## Important Notes

- **Always run tests before creating PR** — CI will catch failures, but it wastes runner minutes.
- **One PR per logical change set** — don't mix unrelated features.
- **Reference plan numbers** in description when implementing plans (e.g., "Implements Plan 012").
- **CI runs on three platforms** — if macOS or Linux fails, check platform-specific code.
- **Squash merge recommended** for feature branches to keep `main` history clean.
- **Release MUST go through PR** — MUST NOT 直接 `git merge dev` 到 main（v0.1.0 教训：9 次合并循环）。
- **Wait for CI green** — push dev 后等三平台 CI 全部通过再创建 PR。

## Pre-push Validation Checklist

在创建 PR 之前，MUST 在本地运行：

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test -- --run
npm run format:check
npm run build
```

> `--all-targets` 是必须的，否则 test-target 的 clippy 警告会被遗漏。
