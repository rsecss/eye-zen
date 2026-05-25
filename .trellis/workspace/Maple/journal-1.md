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


## Session 6: E2E acceptance + merge origin/main for stats branch

**Date**: 2026-05-21
**Task**: E2E acceptance + merge origin/main for stats branch
**Branch**: `feat/stats`

### Summary

E2E acceptance for SQLite rest statistics on feat/stats: ran 5 working->resting cycles with short-duration config (work=1min, rest=5s), verified 5 rows persisted to data.db (count=5, total=25s); restarted app and confirmed StatService re-reads same DB; confirmed cold-start auto-creates schema. Restored original config and cleaned test DB. Local npm run ci passed 8/8. Then origin/main had merged PR #10 (AFK detection); merged origin/main into feat/stats, resolved 7 conflicts (commands/mod.rs adopted #[cfg(not(test))] gating; context.rs kept apply_transition_and_collect_effects + added AFK skip log; capability/lib.rs/commands.ts unioned both new commands; spec docs kept both Statistics Trend IPC and AFK scenarios). Post-merge ci passed again.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `72ce247` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: Global hotkeys timer control

**Date**: 2026-05-21
**Task**: Global hotkeys timer control
**Branch**: `feat/hotkeys`

### Summary

Implemented configurable backend-owned global hotkeys for start rest, skip alert, and pause/resume; added transactional rebinding with rollback/status UI, macOS permission degradation guidance, TOML defaults, generated bindings, tests, and passing npm run ci.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `084fa63` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: Hotkeys per-action fallback (fix + 2x merge sync + spec align)

**Date**: 2026-05-21
**Task**: Hotkeys per-action fallback (fix + 2x merge sync + spec align)
**Branch**: `feat/hotkeys`

### Summary

Refactored HotkeyService apply_config from transactional all-or-nothing rollback to per-action best-effort: each shortcut succeeds or fails independently, partial conflicts no longer disable other working bindings or surface the 'kuaijiejian-weishengxiao' top banner; last_error reserved for macOS Accessibility Missing or all-fail. Settings hotkey input width tightened to 150px with text centered, raw shortcut tokens prettified to Cmd/Ctrl+Alt+X. Updated 1 renamed test, added 3 new tests (partial mixed, all-fail, edit-conflict-to-free). Then merged origin/main twice to sync PR #10 (AFK detection) and PR #11 (SQLite rest statistics trends): resolved 9 additive-union conflicts across IPC table, capabilities/commands/lib.rs, backend service DAG/start/shutdown orders, workspace journal; dropped obsolete get_daily_stats placeholder now that get_statistics_trends ships. Aligned ipc-and-state.md Global Hotkey Contracts scenario (Commands table, Scope, Validation matrix, Good/Base/Bad, Tests Required, Wrong vs Correct) with per-action semantics. All npm run ci stages green: 124 Rust tests + 34 vitest tests + svelte-check + prettier + vite build.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `fafa885` | (see git log) |
| `0de5bc5` | (see git log) |
| `7c3de40` | (see git log) |
| `60033b9` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 9: ECharts dynamic import + cleanup queue tidy

**Date**: 2026-05-22
**Task**: ECharts dynamic import + cleanup queue tidy
**Branch**: `main`

### Summary

Defer ECharts (echarts/{core,charts,components,renderers}) via onMount dynamic import in StatisticsPage.svelte; type-only import stays static. Main entry chunk drops 575.63 KB -> 40.97 KB (gzip 195.10 -> 12.78 KB, -93%); echarts now ships as 8 async chunks loaded on Statistics page open; vite chunks-larger-than-500-kB warning eliminated. disposed flag guards three unmount races. Tests untouched (hoisted vi.mock survives dynamic import). PR #17 squash-merged after 3-platform CI all green. Memory reconciliation: corrected post-v030-cleanup-queue.md — ECharts done, but hotkeys.rs 5 unused warnings still pending (cargo test compile output is the real signal, not cargo clippy).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f9f79fd` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 10: Data export + Pomodoro mode finish-work

**Date**: 2026-05-23
**Task**: Data export + Pomodoro mode finish-work
**Branch**: `feat/data-export`

### Summary

Implemented data-export (StatService::export_to via VACUUM INTO + export_statistics command + tauri-plugin-dialog + StatisticsPage Export Backup button + inline banner pattern reuse + 4 Rust tests + 3 Vitest cases + spec updates on VACUUM INTO exception and tokio::fs rule). Manually verified Pomodoro mode (commit from previous session) end-to-end and queued it alongside data-export. Two PRs opened with data-export depending on Pomodoro #18 merging first.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `dc7bca9` | (see git log) |
| `686ab6f` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 11: Phase 3 health-analysis v0.6: research-driven design + hybrid PRD + 2-stage implementation

**Date**: 2026-05-23
**Task**: Phase 3 health-analysis v0.6: research-driven design + hybrid PRD + 2-stage implementation
**Branch**: `feat/health-analysis`

### Summary

Researched 8 eye-care + 5 productivity competitors, identified SkipFlags 4-factor taxonomy as Eyezen's real differentiator. Codex review pushed back on score-theater risk; user picked hybrid route: P2-lite (rest_cycle_events + suppression breakdown) + honest Beta ECI (0.7 adherence + 0.3 longest, no skip_rate term) + P5 streak cards + one-shot trends migration. Two trellis-implement stages (backend / frontend) + trellis-check PASS, then live dev-server verification confirmed schema v1->v2 migration, backfill, and 6 schedule-suppressed emissions matched dev log timing. Branch ready for PR.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `b6f21cd` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 12: v0.6.0 release ship + post-v0.6.0 cleanup (bump-version stub + hotkeys cfg gates)

**Date**: 2026-05-23
**Task**: v0.6.0 release ship + post-v0.6.0 cleanup (bump-version stub + hotkeys cfg gates)
**Branch**: `main`

### Summary

本 session 接续 health-analysis 合入后流程：(1) 推送 fix/bump-stub-and-windows-sound-flake + feat/health-analysis 双 PR (#22 #23) 到 main；(2) 发版 v0.6.0：bump-version + CHANGELOG (emoji 格式手填) + PR #24 + tag → release.yml 4 平台全 success，10 artifact published as latest；(3) 后续 chore PR #25：bump-version.mjs stub 改用项目 emoji 约定（避免每次发版手填），hotkeys.rs / timer/service.rs 5 个 dead_code warning 通过 cfg gate 收紧 + cfg_attr allow(dead_code) 处理（sub-agent 在 implement 阶段发现 PRD 误判为 dead code，实际是 v0.3.0 全局快捷键 feature-gated production code，被 lib.rs:59/77 在 #[cfg(feature="plugin-shortcuts")] 调用，删除即 regression）。两套 feature 配置 clippy 全 clean，172 tests 全过。Windows sound test cfg(not windows) 修复在 release.yml Windows job 上得到验证（无 STATUS_ACCESS_VIOLATION）。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `f44ccb8` | (see git log) |
| `10ea6be` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 13: Post-v0.7.0 docs housekeeping and stale branch cleanup

**Date**: 2026-05-24
**Task**: Post-v0.7.0 docs housekeeping and stale branch cleanup
**Branch**: `main`

### Summary

Relocated community files (README.zh-CN, CONTRIBUTING, CODE_OF_CONDUCT) to .github/ for GitHub recognition; updated all internal links and bump-version.mjs/update-docs/release/index.json references. Added Allow auto-merge row to branch-protection.md. Pruned old rsecs workspace (single-s, pre-Maple) and reassigned legacy task creator/assignee fields. Single PR #36 auto-merged via gh pr merge --auto. Cleaned up stale local branches: deleted worktree-agent-* and abandoned chore/archive-v0-7-0-epic (its 2 commits redone here).

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `ccc470b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 14: v0.7.x hardening epic: 10-commit single-PR closing P2/P3 audit findings + tray-panel UX bug

**Date**: 2026-05-25
**Task**: v0.7.x hardening epic: 10-commit single-PR closing P2/P3 audit findings + tray-panel UX bug
**Branch**: `chore/v0-7-x-hardening`

### Summary

Closed every P2/P3 finding from the v0.7.0 audit that was deferred into v0.7.x (F17 stat.rs split, F18 SettingsPage/StatisticsPage split, F19 locale canonical, F20 IPC event constants, F16 graded timeouts, F22 capability scope, F25 docs drift, F29 path caveats, F03+F28 macOS fullscreen real impl) plus the tray-panel focus-loss UX bug surfaced during v0.7.0 testing. F21 was dropped — re-audit showed shell:default is live for AboutPage external links. Followed up with a four-step README overhaul: v0.3->v0.7 catch-up, new screenshot taxonomy (3 manually-captured shots), reader-focused restructure (drop ProjectEye comparison/Download/Roadmap/Credits, add Quick Start + Architecture), slim Highlights to 7 emoji-anchored bullets. Working tree clean, npm run ci 8 steps green, 266 backend tests pass, frontend+backend coverage gates hold at 90%/85%.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `58f1eb4` | (see git log) |
| `b42269c` | (see git log) |
| `01c6af8` | (see git log) |
| `ba336b2` | (see git log) |
| `a72fce6` | (see git log) |
| `c8a1719` | (see git log) |
| `5723431` | (see git log) |
| `530ee6c` | (see git log) |
| `7709305` | (see git log) |
| `feffdda` | (see git log) |
| `ff55c7e` | (see git log) |
| `73ad575` | (see git log) |
| `c848fa0` | (see git log) |
| `2a0249a` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 15: release v0.7.1

**Date**: 2026-05-25
**Task**: release v0.7.1
**Branch**: `main`

### Summary

Patch release shipping v0.7.x hardening epic (PR #37). Fixed PR #37 macOS clippy errors (items-after-statements + doc-markdown in src-tauri/src/platform/macos.rs), merged epic, then cut release/v0.7.1 via trellis-implement sub-agent (Phase A: bump-version 0.7.0->0.7.1 across 4 version files + Cargo.lock + 2 README badges, promote [Unreleased] 10 finding bullets into [0.7.1] with shared (#37)(f62e2a1) tag, PR #38). Main session drove Phase B: monitor PR CI 6/6 green, squash merge with clean fast-forward, tag v0.7.1, push tag through GitHub 443 turbulence (3 retries, no --no-verify), release.yml built 10 artifacts (Win 3 / macOS 4 / Linux 3) in 6m21s, draft notes auto-extracted correctly, user verified Windows installer, published. Memory note release-v071-experience.md captures single-PR-epic CHANGELOG convention + tag push retry policy + Phase A/B split.

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `bf4c900` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
