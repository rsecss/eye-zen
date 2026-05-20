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
