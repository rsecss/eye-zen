# Development Workflow

## Rules

- Work on short-lived `feat/<scope>`, `fix/<scope>`, or `chore/<scope>` branches off `main`.
- All changes to `main` go through PR; no direct pushes.
- Keep changes scoped and atomic.
- Run `npm run ci` before pushing (the pre-push hook enforces this).
- Normal push/PR CI does not package installers; release tags do.

## Hooks

| Hook | Runs | Purpose |
|------|------|---------|
| `pre-commit` | `lint-staged` + commitlint | Format staged frontend files and enforce Conventional Commits. |
| `pre-push` | `npm run ci` | Match local checks with GitHub Actions. |

`npm run ci` runs:

1. Version sync check
2. Rust format
3. Rust clippy with `--all-targets`
4. Rust tests (`--no-default-features`)
5. Svelte type check
6. Vitest with `--coverage` (enforces threshold gate)
7. Prettier check
8. Rust check
9. Frontend build

GitHub Actions adds two extra jobs beyond `npm run ci`:

- `cargo-test-default-features` — Linux Rust tests with the default feature set (covers the production `plugin-shortcuts` path that `npm run ci` skips).
- `cargo-coverage` — Linux `cargo llvm-cov` with `--fail-under-lines 80`.

## Coverage gate

The frontend coverage threshold lives in `vite.config.ts` under
`test.coverage.thresholds` and is currently:

- lines: 90%
- functions: 85%
- branches: 80%
- statements: 90%

`npm run test:ci` (step 6 of `npm run ci`) enforces this — any PR that drops
overall coverage below the threshold fails locally and in CI.

To debug a local failure:

```bash
npm run test:coverage    # writes ./coverage/ HTML report
open coverage/index.html # browse uncovered lines per file
```

The threshold MUST NOT be lowered without an explicit ADR. New features that
add untested code SHOULD be paired with the corresponding tests in the same
PR. Files that genuinely cannot be unit-tested (e.g. thin Tauri-API bootstrap
shims) belong in `vite.config.ts`'s `coverage.exclude` array with a code
comment justifying the exclusion.

The backend coverage threshold is enforced in CI via
`cargo llvm-cov --fail-under-lines 90 --fail-under-functions 85`. Thin OS/IPC
shim files are excluded from the count via `--ignore-filename-regex`; see
`.github/workflows/ci.yml`'s `cargo-coverage` job for the exact regex and
rationale. Run it locally with:

```bash
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
cargo llvm-cov --manifest-path src-tauri/Cargo.toml --summary-only
```

## Flow

1. Define scope, non-goals, and acceptance criteria.
2. Read existing code and the relevant `.trellis/spec/` docs.
3. `git checkout main && git pull && git checkout -b <type>/<scope>`.
4. Implement the smallest change that satisfies the scope.
5. Add or update tests when behavior changes.
6. Run checks (or rely on the pre-push hook).
7. Commit with Conventional Commits.
8. Push the branch and open a PR to `main`.
9. Wait for PR CI to pass; squash merge; the head branch is auto-deleted.

## Branches

| Branch | Use |
|--------|-----|
| `main` | Single long-lived branch; always releasable; PR only. |
| `feat/<scope>` | New feature branch off `main`. |
| `fix/<scope>` | Bug fix or hotfix branch off `main`. |
| `chore/<scope>` | Tooling, docs, or maintenance branch off `main`. |
| `release/vX.Y.Z` | Short-lived release prep branch off `main`. |

## References

- Quality gate: `.trellis/spec/architecture/testing-quality.md`
- Change checklist: `.trellis/spec/architecture/change-management.md`
- PR flow: `docs/workflows/pr.md`
- Release flow: `docs/workflows/release.md`
