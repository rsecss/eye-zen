# Journal - Maple (Part 1)

> AI development session journal
> Started: 2026-05-19

---



## Session 1: Bootstrap Trellis spec + migrate rules/

**Date**: 2026-05-19
**Task**: Bootstrap Trellis spec + migrate rules/
**Branch**: `dev`

### Summary

Migrated project conventions from rules/ (8 files) to .trellis/spec/{architecture,backend,frontend,guides}/ (17 files), reshaped layout for Tauri Rust+Svelte dual layer. Scaffolded Trellis workflow platform: hooks, sub-agents, skills, commands, codex mirror. Replaced superpowers Claude Code plugin with Trellis hooks. Cleaned all rules/ references in CLAUDE.md, CONTRIBUTING.md, docs/workflows/*, .claude/index.json; removed superpower entries from .gitignore. Two parallel general-purpose subagents wrote backend/+frontend/+architecture/ spec content concurrently. Codex MCP review was blocked by local .codex/config.toml:32 FeatureToml parse bug; fallback subagent review passed (10/10 paths, 3/3 code-fit samples, 0 placeholders).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e9e5275` | (see git log) |
| `2f8ce71` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 2: License switch: MIT -> GPL-3.0-or-later

**Date**: 2026-05-19
**Task**: License switch: MIT -> GPL-3.0-or-later
**Branch**: `dev`

### Summary

Switched Eyezen license from MIT to GPL-3.0-or-later to prevent closed-source forks. Replaced LICENSE with verbatim GNU GPL-3.0 text (674 lines), synced SPDX identifier across tauri.conf.json / Cargo.toml / package.json / .claude/index.json, updated copyright + publisher (Maple -> rsecss), refreshed README en/zh-CN badges + License sections, added inbound=outbound clause to CONTRIBUTING, recorded changelog entries in CLAUDE.md and docs/devlog.md. AGPL-3.0 evaluated and excluded (desktop app does not trigger its network clause). v0.1.0 binaries remain under MIT permanently. All 7 local quality gates passed.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `4cb8c2b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: CI hardening and docs slimming

**Date**: 2026-05-20
**Task**: CI hardening and docs slimming
**Branch**: `dev`

### Summary

Aligned local and GitHub CI checks, moved normal push and PR workflows to parity checks without packaging, pinned toolchains, updated PR description, and slimmed workflow and agent documentation into concise rules and maps.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `e3cf334` | (see git log) |
| `af98c41` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: workday-scheduling: ScheduleConfig + SkipFlags extension + UI polish + License consistency fix

**Date**: 2026-05-20
**Task**: workday-scheduling: ScheduleConfig + SkipFlags extension + UI polish + License consistency fix
**Branch**: `dev`

### Summary

Implemented Phase 2 starter slice: weekday scheduling that suppresses rest reminders on user-configured weekdays. Reused existing SkipFlags / any_active() OR aggregation so behavior only fires at the Working->PreAlert transition point (other states are never interrupted). Backend: pure is_schedule_active() + SkipFlags.schedule_inactive + chrono::Local each tick + update_schedule_config command. Frontend: ts-rs ScheduleConfig binding + Schedule SettingsCard (Toggle + 7 chip-styled native checkboxes) + zh-CN/en i18n (11 keys each). Bundled UI polish: main window 520x640 -> 880x560 (16:10), Active days row stacks vertically so English long descriptions no longer collide with the chip grid. Consistency fix: About page License MIT -> GPL-3.0-or-later. All gates green (cargo fmt + clippy --all-targets + test 80 passed, svelte-check 371/0/0, vitest 28 passed, prettier, vite build). Manual E2E verified.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `6e5fa9e` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


---

## 2026-05-20 — Migrate to GitHub Flow

Repo switched from dev+main double-branch to GitHub Flow (single main + short-lived feat/fix/chore branches). PR #7 merged, dev deleted. This commit is the trivial verification PR for the new flow.


## Session 5: SQLite Rest Statistics Trends

**Date**: 2026-05-21
**Task**: SQLite Rest Statistics Trends
**Branch**: `feat/stats`

### Summary

Implemented SQLite-backed rest statistics persistence, day/week/month aggregation, Statistics ECharts UI, tests, and updated Trellis specs.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `34db95d` | (see git log) |
| `a4ffb0b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
