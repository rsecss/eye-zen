<p align="center">
  <img src="docs/public/logo.svg" alt="Eyezen" width="128" />
</p>

<h1 align="center">Eyezen</h1>

<p align="center">
  <strong>Cross-platform desktop eye care app based on the 20-20-20 rule</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platform" />
  <img src="https://img.shields.io/badge/version-0.7.0-orange" alt="Version" />
  <img src="https://img.shields.io/badge/tauri-v2-blue" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/coverage-93%25-brightgreen" alt="Coverage" />
</p>

<p align="center">
  English | <a href=".github/README.zh-CN.md">简体中文</a>
</p>

> **Status**: v0.7.0 released (2026-05-24). Download pre-built installers from [GitHub Releases](https://github.com/rsecss/eye-zen/releases/latest).

---

## What is the 20-20-20 Rule?

Every **20** minutes, look at something **20** feet (~6 meters) away for **20** seconds. This simple habit effectively reduces eye strain caused by prolonged screen time.

Eyezen automates this process — it quietly runs in the background and gently reminds you to rest when it's time.

## Features

### Core Timer
- **20-20-20 Mode** — Customizable work/rest durations with a full state machine (Working → PreAlert → Alerting → Resting)
- **Pomodoro Mode** — Alternative work-rest rhythm with configurable cycle length and long-break interval
- **Multi-monitor Support** — Break reminder windows appear on all connected displays
- **Sound Alerts** — Gentle audio cue on break reminders

### Smart Skipping
- **Fullscreen Detection** — Auto-skip reminders during fullscreen apps on Windows / macOS / Linux X11 (Wayland not yet supported)
- **AFK Detection** — Skip the next reminder when you've been idle for a configurable threshold
- **Process Whitelist** — Skip reminders when specific applications are in the foreground (cross-platform basename matching)
- **Workday Schedule** — Suppress reminders entirely on configured weekdays

### Analytics & Insights
- **Statistics Dashboard** — Daily / weekly / monthly trends rendered via ECharts
- **Health Analysis** — Eye-Care Index (ECI), adherence rate, and rhythm tracking to quantify your eye-care habit
- **Data Export** — Export the SQLite statistics database to a user-chosen location for backup or external analysis

### Interface & Integration
- **System Tray Panel** — Persistent tray icon with glassmorphic quick-action panel that follows the tray position; auto-hides on focus loss like a native popover
- **Dark / Light Theme** — Follows your preference including native title-bar adaptation on Windows
- **Internationalization** — Simplified Chinese / English with hot switching, no restart needed
- **Global Hotkeys** — Configurable shortcuts for start-rest / skip-rest / toggle-pause
- **Auto Start** — Launch at system startup (OS-native via tauri-plugin-autostart)

### Engineering
- **Lightweight** — Rust backend + native WebView via Tauri, minimal resource usage (~15 MB)
- **Tested** — 90%+ line coverage on both frontend (Vitest) and backend (cargo-llvm-cov), enforced in CI
- **Audited** — `cargo deny` security gate + zero npm advisories at every release

### Why Eyezen over ProjectEye?

Eyezen is inspired by [ProjectEye](https://github.com/Jeremyyang920/ProjectEye), but built from scratch with a modern stack:

| | Eyezen | ProjectEye |
|---|--------|------------|
| Platform | Windows / macOS / Linux | Windows only |
| Tech Stack | Rust + Tauri v2 + Svelte 5 | C# + WPF |
| Memory Usage | ~15 MB | ~170 MB |
| Theme | Dark / Light + native title bar | Light only |
| i18n | Chinese / English hot switch | Chinese only |
| Multi-monitor | All displays show reminder | Primary display only |
| Statistics | ECharts trends + Health Index | None |
| Pomodoro Mode | Yes | No |
| Maintenance | Active | Unmaintained |

## Screenshots

### Core Experience

| Resting | Tip Window |
|:---:|:---:|
| ![Resting](docs/public/screenshots/resting.png) | ![Tip Window](docs/public/screenshots/tip_window.png) |

| Settings | About |
|:---:|:---:|
| ![Settings](docs/public/screenshots/settings.png) | ![About](docs/public/screenshots/about.png) |

### Statistics & Health Analysis (added in v0.6.0)

| Overview · Eye-Care Index · 24h Ribbon | Daily / Weekly / Monthly Trend |
|:---:|:---:|
| ![Statistics Overview](docs/public/screenshots/statistics-overview.png) | ![Trend Chart](docs/public/screenshots/statistics-trend.png) |

### Pomodoro Mode (added in v0.5.0)

<p align="center">
  <img src="docs/public/screenshots/settings-pomodoro.png" alt="Pomodoro Settings" width="720" />
  <br/>
  <em>Configurable focus / short break / long break cycle alongside 20-20-20</em>
</p>

## Download

### From Release (Recommended)

Go to [Releases](https://github.com/rsecss/eye-zen/releases/latest) to download the installer for your platform:

| Platform | Installer |
|----------|-----------|
| Windows | `.exe` (NSIS) or `.msi` |
| macOS | `.dmg` (Intel & Apple Silicon) |
| Linux | `.deb` / `.AppImage` |

<details>
<summary>macOS Security Notice</summary>

macOS may block unsigned apps. After downloading, open Terminal and run:

```bash
xattr -cr /Applications/Eyezen.app
```

</details>

## Build from Source

### Prerequisites

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Platform-specific dependencies: see [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Steps

```bash
git clone https://github.com/rsecss/eye-zen.git
cd eye-zen

npm install
npm run tauri dev    # Development mode (hot reload)
npm run tauri build  # Production build
```

Build output: `src-tauri/target/release/bundle/`.

### Development checks

```bash
npm run ci  # All 8 local checks (fmt, lint, test, type-check, build)
```

Or individually:

```bash
npx svelte-check --tsconfig ./tsconfig.json                    # Type check
npm test                                                       # Frontend tests (Vitest)
cargo test --manifest-path src-tauri/Cargo.toml                # Backend tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run format:check                                           # Prettier
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
```

## Tech Stack

| Layer | Choice | Description |
|-------|--------|-------------|
| Framework | [Tauri v2](https://v2.tauri.app/) | Rust backend + native WebView |
| Frontend | [Svelte 5](https://svelte.dev/) | Runes reactivity, zero runtime |
| Build | [Vite 6](https://vite.dev/) | Fast HMR |
| Styling | [TailwindCSS v4](https://tailwindcss.com/) | Utility-first |
| Charts | [ECharts](https://echarts.apache.org/) | Tree-shaken, lazy-loaded |
| Database | SQLite via [sqlx](https://github.com/launchbadge/sqlx) | Statistics persistence |
| Config | TOML | Human-readable |
| Audio | rodio | Rust-native, dedicated thread |
| Type Bridge | ts-rs | Rust → TypeScript auto-generation |

## Roadmap

- [x] **v0.1** — Core MVP (timer, tray, multi-monitor, fullscreen detection, settings, theme, auto-start, i18n)
- [x] **v0.2** — Workday scheduling
- [x] **v0.3** — Usage statistics + charts, AFK detection, configurable global hotkeys
- [x] **v0.4** — Cross-platform process whitelist
- [x] **v0.5** — Pomodoro mode + statistics database export
- [x] **v0.6** — Health analysis (Eye-Care Index, adherence rate, rhythm tracking)
- [x] **v0.7** — Hardening release (90%+ coverage gate, security gate, port-trait architecture)
- [ ] **v1.0** — API freeze + stable release

## Configuration

Config is stored as `config.toml` in the system app data directory:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\com.eyezen.app\config.toml` |
| macOS | `~/Library/Application Support/com.eyezen.app/config.toml` |
| Linux | `~/.config/com.eyezen.app/config.toml` |

Statistics database (`data.db`) lives alongside `config.toml`. All settings can be modified through the in-app Settings UI; you should never have to edit the TOML by hand.

## Credits

Eyezen is inspired by **[ProjectEye](https://github.com/Jeremyyang920/ProjectEye)** — a smart eye protection tool for Windows that inspired Eyezen's core timer and break reminder design.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## License

[GNU General Public License v3.0 or later](LICENSE)
