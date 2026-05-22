# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

Each version body uses the following categories (see
[`docs/workflows/release.md`](docs/workflows/release.md#changelog-entry-style)):

| Emoji | Category | Conventional Commit types |
|-------|----------|---------------------------|
| 🎉 | Features | `feat` |
| 🛠️ | Fixes | `fix` |
| 📃 | Documentation | `docs` |
| 🧪 | Refactor | `refactor` |
| 🔧 | Maintenance | `chore`, `ci`, `build`, `perf`, `style`, `test` |

Line format: `- imperative-lowercase description (#NN) (sha7)`. PR number is
omitted when the change predates the pull-request workflow.

## [0.5.0] - 2026-05-23

### 🎉 Features

- Pomodoro mode alongside 20-20-20 with configurable cycles and long break (#18) (52ae677)
- statistics database export via VACUUM INTO with native save dialog (#19) (81ed092)

### 🔧 Maintenance

- defer echarts via dynamic import to slim main bundle (#17) (f9f79fd)
- adopt chrome-devtools-mcp style for release notes (#16) (5817b03)

## [0.4.0] - 2026-05-22

### 🎉 Features

- cross-platform process whitelist skip next rest reminder (#14) (6e0abbc)

## [0.3.0] - 2026-05-22

### 🎉 Features

- AFK detection skips next rest reminder after idle threshold (#10) (58332ba)
- SQLite rest statistics trends with daily/weekly/monthly charts (#11) (5a68535)
- configurable global hotkeys for start/skip/toggle-pause (#12) (bc96fb5)

### 🔧 Maintenance

- migrate from dev/main to GitHub Flow and sync Cargo.lock in bump-version.mjs (#7) (b15a993)

## [0.2.0] - 2026-05-20

### 🎉 Features

- weekday scheduling suppresses alerts on configured days (#4) (49be514)

### 🛠️ Fixes

- sync Cargo.lock to v0.2.0 release (#5) (5b60ab8)

### 🔧 Maintenance

- switch license from MIT to GPL-3.0-or-later (#1) (bc4aa8f)
- resize main window from 520×640 to 880×560 (#4) (49be514)
- harden release pipeline with version sync gates (#2) (6e2e3ce)
- add cargo-deny security audit and lint-staged Rust formatting (#3) (99eaa80)
- adopt Trellis workflow and lock ts-rs to ~10.1 (#1) (bc4aa8f)

## [0.1.0] - 2026-03-21

### 🎉 Features

- 20-20-20 timer state machine with Working/PreAlert/Alerting/Resting (9d431b5)
- multi-monitor tip windows with glassmorphic design (f56cfeb)
- system tray panel with quick actions and tray-icon positioning (f75fb5e)
- in-app Settings UI with Timer/Behavior/Display cards (5748b27)
- About page with version info and update check (a8191a2)
- internationalization for zh-CN and en with hot switching (244a73b)
- dark/light theme switching with native title bar adaptation (69bd8b4)
- OS-level auto-start via tauri-plugin-autostart (e1d6f32)

### 🔧 Maintenance

- initial Tauri v2 + Svelte 5 + Vite scaffold (cd57866)
- ts-rs auto-generated TypeScript bindings (3d96503)
- three-platform CI matrix and four-target Release pipeline (b10533b)

[0.5.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.5.0
[0.4.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.4.0
[0.3.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.3.0
[0.2.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.2.0
[0.1.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.1.0
