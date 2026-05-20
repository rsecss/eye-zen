# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-05-20

### Added

- **Workday scheduling** — toggle to keep Eyezen quiet on selected weekdays (typically weekends) without manually pausing. Suppression fires only at the Working → PreAlert boundary, so cycles already in progress are never interrupted mid-rest. Day granularity only (hour-of-day ranges and holidays are out of scope by design).

### Changed

- **License** — switched from **MIT** to **GPL-3.0-or-later**, effective for this release and onward. v0.1.0 binaries remain under MIT permanently. Applied across `LICENSE`, `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and the in-app About page.
- **Main window default size** — `520×640` → `880×560` (16:10) for a less cramped Settings/About layout.

## [0.1.0] - 2026-03-21

First public release. A fully functional 20-20-20 eye care desktop app.

### Added

- **20-20-20 Timer** — configurable work/rest durations with full state machine (Working → PreAlert → Alerting → Resting)
- **Multi-monitor support** — tip windows appear on all connected displays simultaneously
- **Fullscreen detection** — auto-skip reminders when fullscreen apps are running (Windows / Linux X11; macOS and Wayland limited)
- **System tray** — persistent tray icon with status tooltip, quick actions menu, and glassmorphic tray panel
- **Settings UI** — in-app settings for Timer, Behavior, and Display preferences
- **About page** — version info, platform details, GitHub links
- **Dark / Light theme** — CSS variable overrides + native title bar theme + WebView2 color-scheme for native controls
- **Auto-start** — OS-level launch at startup via `tauri-plugin-autostart`
- **Internationalization** — Chinese (zh-CN) / English (en) with hot switching, covers all UI and tray menu
- **Sound alerts** — gentle audio cue on rest reminder via rodio (dedicated audio thread)
- **Typed IPC** — ts-rs auto-generated TypeScript bindings, typed commands (5s timeout) and event listeners
- **Reactive stores** — Svelte 5 Runes stores with race condition protection (`version` counter + `loaded` flag)
- **Config persistence** — TOML config with arc-swap hot reload and file watcher
- **CI/CD** — GitHub Actions CI (three-platform matrix) + Release workflow (four-target build via `tauri-action@v0`)

### Known Limitations

- Fullscreen detection: macOS returns conservative `false`; Linux Wayland always shows reminders
- No usage statistics or charts yet (planned for v0.2)
- No away detection (planned for v0.3)
- Native `<select>` dropdown popup may not fully follow dark theme on some WebView2 versions

### Platform Support

| Platform | Status |
|----------|--------|
| Windows 10/11 | ✅ Tested |
| macOS (ARM/Intel) | ⚠️ Builds, untested |
| Linux (X11/Wayland) | ⚠️ Builds, untested |

[0.2.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.2.0
[0.1.0]: https://github.com/rsecss/eye-zen/releases/tag/v0.1.0
