# PRD: Fix bump-version stub position + Windows sound test flake

## Background

Two follow-ups surfaced during the v0.5.0 release flow:

1. **`scripts/bump-version.mjs` inserts the CHANGELOG stub in the wrong spot.**
   It prepends `## [X.Y.Z]` immediately after the marker
   `and this project adheres to [Semantic Versioning].\n`, but the marker is
   followed by a description block (categories table + line-format guidance)
   that should sit between the header and version sections. Result: every
   release requires manual cut-and-paste of the stub past the description
   block. Confirmed during v0.5.0.
2. **Windows CI flakes on `services::sound` tests with `STATUS_ACCESS_VIOLATION`
   (0xc0000005).** Same code passed in PR CI (3m24s) and on rerun (3m13s) but
   failed on the post-merge push CI for 81ed092. The crash occurs while
   running `services::sound::tests` (three tests, each constructs a
   `SoundService` which initializes a rodio `OutputStream`). Headless Windows
   runners have no audio device — rodio's Windows backend is the suspected
   root cause.

## Goals

- Bump script writes the stub at the correct position so future releases skip the manual fix.
- Windows CI stops flaking on sound tests; other platforms keep coverage.

## Non-goals

- Refactoring `SoundService` to be mock-friendly (would be a larger change).
- Backporting fixes to older CHANGELOG entries (v0.5.0 already manually fixed).
- Adding new tests for `bump-version.mjs` (script is small, manual verification on next release is enough).

## Approach

### Fix 1 — `scripts/bump-version.mjs::prependChangelogSection`

Find the **first existing `## [X.Y.Z]` heading** in the file and insert the
stub immediately before it. This places the new section after the description
block (which lives between the header and any version sections) and above
prior releases.

- Use a regex like `/\n## \[\d/m` to locate the first version heading.
- Fall back to "after the marker" only if no prior version exists (greenfield CHANGELOG).
- Keep the existing `if (text.includes(\`## [${version}]\`)) skip` early return.

### Fix 2 — `src-tauri/src/services/sound.rs::tests`

Annotate the whole `mod tests` with `#[cfg(not(target_os = "windows"))]`.
Rationale:

- All three tests call `SoundService::new()`, which constructs an
  `OutputStream`. The crash is inside that path on headless Windows runners.
- Per-test cfg would force every assertion to gate individually with no
  benefit since they all share the same root cause.
- Linux/macOS CI retains coverage of `new_creates_service`,
  `play_command_does_not_panic`, and `set_enabled_false_suppresses_play`.
- Document why with a one-line comment next to the cfg attr (per project
  comment guideline: explain *why*).

## Verification

- `node scripts/bump-version.mjs 9.9.9` on a clean working tree (then `git
  restore` everything) — verify `## [9.9.9]` lands above `## [0.5.0]` and
  below the description table. Cleanup any side effects.
- `npm run ci` 8/8 green locally (Windows local clippy still includes the
  warnings from main; Rust tests should now skip sound on this run if local
  OS is Windows, otherwise still run them).
- Cannot reproduce the Windows CI flake locally; trust the cfg gate.

## Out of scope risk acknowledgement

- Once cfg-guarded, a regression in `SoundService` initialization on Windows
  will not be caught by CI. Mitigation: manual smoke test on Windows still
  runs `cargo test` on the developer machine (unaffected by cfg since local
  test isn't headless), and the app itself exercises the path on launch.

## Deliverables

- `scripts/bump-version.mjs`: updated `prependChangelogSection`.
- `src-tauri/src/services/sound.rs`: cfg-gated `mod tests` + one-line WHY comment.
- One PR titled `fix: bump-version stub placement and Windows sound test flake`.

## Success criteria

- [ ] Local `npm run ci` passes after both fixes.
- [ ] Commit created on a feature branch (not pushed per user instruction).
- [ ] Two `feat`/`fix`-typed Conventional Commit messages OR one combined commit (TBD).
