# PRD: Release v0.5.0

## Goal

Cut and publish Eyezen v0.5.0 following the standard release SOP in
`docs/workflows/release.md`. Include all PRs merged to `main` after the
v0.4.0 tag.

## Scope (what's in v0.5.0)

PRs merged after v0.4.0 (`6e0abbc`, 2026-05-22), in `git log` order:

| Commit  | PR  | Type     | Title                                                              |
|---------|-----|----------|--------------------------------------------------------------------|
| 81ed092 | #19 | feat     | add statistics database export via VACUUM INTO                     |
| 52ae677 | #18 | feat     | add Pomodoro mode alongside 20-20-20                               |
| f9f79fd | #17 | perf     | defer echarts via dynamic import to slim main bundle               |
| 5817b03 | #16 | docs     | adopt chrome-devtools-mcp release note style                       |

Categorization for CHANGELOG (per `release.md` rules):

- 🎉 **Features**: #18 (Pomodoro), #19 (data export)
- 🔧 **Maintenance**: #17 (perf), #16 (docs)

No breaking changes, no security fixes. All four PRs already squashed into `main`.

## Pre-release state

- Current version: `0.4.0` (synced across package.json / Cargo.toml / tauri.conf.json)
- Branch: `main` @ `81ed092`, working tree clean
- CI on `main` after #19 squash: initially failed (Windows `STATUS_ACCESS_VIOLATION`
  in `services::sound` — flaky on headless Windows runner; same code passed in
  PR CI). Rerun triggered (`gh run rerun 26305417204 --failed`); waiting for green.
- macOS / Linux / Security Audit all green on 81ed092 first run.

## Release flow (from `docs/workflows/release.md`)

1. Wait for `main` CI to be green (rerun in progress)
2. `git checkout -b release/v0.5.0` from `main`
3. `node scripts/bump-version.mjs 0.5.0` → bumps package.json, Cargo.toml,
   Cargo.lock (eyezen entry), tauri.conf.json, and prepends `## [0.5.0]` stub to CHANGELOG
4. Fill CHANGELOG body for v0.5.0 (categorized H3 with emoji, one line per PR)
5. Local validation: `npm run ci` must pass (8/8 incl. version-sync gate)
6. Commit, push, `gh pr create --base main --title "release: v0.5.0"`
7. Wait for PR CI green; `gh pr merge --squash --delete-branch`
8. `git checkout main && git pull && git tag v0.5.0 && git push origin v0.5.0`
9. Release CI builds 4 targets + drafts release notes (auto-extracted from CHANGELOG)
10. Verify draft assets (expected: 10 artifacts as in v0.4.0); publish

## Risks

- **Windows CI flake** (sound test access violation). Mitigation: rerun first;
  if reproducible on second run, investigate `services::sound` test isolation
  before cutting release branch.
- **Cargo.lock sync** (regression from v0.2.0): `bump-version.mjs` is fixed,
  but `npm run ci` step 1 still verifies parity; trust the gate.

## Success criteria

- [ ] `main` CI green for `81ed092`
- [ ] Version 0.5.0 synced across all 4 source files
- [ ] CHANGELOG `## [0.5.0]` entry merged via PR
- [ ] Tag `v0.5.0` pushed; Release CI green; 10 artifacts attached
- [ ] GitHub Release published (not draft)
