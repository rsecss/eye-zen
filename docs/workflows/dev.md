# Development Workflow

## Rules

- Work on `dev`; merge to `main` only through PR.
- Keep changes scoped and atomic.
- Run `npm run ci` before pushing release-bound work.
- Normal push/PR CI does not package installers; release tags do.

## Hooks

| Hook | Runs | Purpose |
|------|------|---------|
| `pre-commit` | `lint-staged` + commitlint | Format staged frontend files and enforce Conventional Commits. |
| `pre-push` | `npm run ci` | Match local checks with GitHub Actions. |

`npm run ci` runs:

1. Rust format
2. Rust clippy with `--all-targets`
3. Rust tests
4. Svelte type check
5. Vitest
6. Prettier check
7. Rust check
8. Frontend build

## Flow

1. Define scope, non-goals, and acceptance criteria.
2. Read existing code and the relevant `.trellis/spec/` docs.
3. Implement the smallest change that satisfies the scope.
4. Add or update tests when behavior changes.
5. Run checks.
6. Commit with Conventional Commits.
7. Push and wait for CI.

## Branches

| Branch | Use |
|--------|-----|
| `dev` | Daily development. |
| `main` | Release branch; PR only. |
| `release/vX.Y.Z` | Short-lived release prep. |
| `fix/<name>` | Hotfix from `main`. |

## References

- Quality gate: `.trellis/spec/architecture/testing-quality.md`
- Change checklist: `.trellis/spec/architecture/change-management.md`
- PR flow: `docs/workflows/pr.md`
- Release flow: `docs/workflows/release.md`
