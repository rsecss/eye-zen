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
  <img src="https://img.shields.io/badge/version-0.3.0-orange" alt="Version" />
  <img src="https://img.shields.io/badge/tauri-v2-blue" alt="Tauri v2" />
</p>

<p align="center">
  English | <a href=".github/README.zh-CN.md">简体中文</a>
</p>

> **Status**: v0.3.0 released. Download pre-built installers from [GitHub Releases](https://github.com/rsecss/eye-zen/releases/latest).

---

## What is the 20-20-20 Rule?

Every **20** minutes, look at something **20** feet (~6 meters) away for **20** seconds. This simple habit effectively reduces eye strain caused by prolonged screen time.

Eyezen automates this process — it quietly runs in the background and gently reminds you to rest when it's time.

## Features

- **20-20-20 Timer** — Customizable work/rest durations with full state machine (Working → PreAlert → Alerting → Resting)
- **Multi-monitor Support** — Break reminder windows appear on all connected displays
- **Fullscreen Detection** — Auto-skip reminders during fullscreen apps (Windows / Linux X11; limited on macOS & Wayland)
- **System Tray** — Persistent tray icon with status, quick actions, and glassmorphic panel
- **Dark / Light Theme** — Follows your preference including native title bar
- **Auto Start** — Launch at system startup
- **i18n** — Chinese / English with hot switching
- **Sound Alerts** — Gentle audio cue on break reminders
- **Lightweight** — Rust backend + native WebView via Tauri, minimal resource usage

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
| Maintenance | Active | Unmaintained |

## Screenshots

| Resting | Settings | About | Tip Window |
|:---:|:---:|:---:|:---:|
| ![Resting](docs/public/screenshots/resting.png) | ![Settings](docs/public/screenshots/settings.png) | ![About](docs/public/screenshots/about.png) | ![Tip Window](docs/public/screenshots/tip_window.png) |

## Download

### From Release (Recommended)

Go to [Releases](https://github.com/rsecss/eye-zen/releases/latest) to download the installer for your platform:

| Platform | Installer |
|----------|-----------|
| Windows | `.exe` (NSIS) or `.msi` |
| macOS | `.dmg` |
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

### Development

```bash
npx svelte-check --tsconfig ./tsconfig.json  # Type check
npm test                                      # Frontend tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run format:check                          # Prettier check
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
```

## Tech Stack

| Layer | Choice | Description |
|-------|--------|-------------|
| Framework | [Tauri v2](https://v2.tauri.app/) | Rust backend + native WebView |
| Frontend | [Svelte 5](https://svelte.dev/) | Runes reactivity, zero runtime |
| Build | [Vite 6](https://vite.dev/) | Fast HMR |
| Styling | [TailwindCSS v4](https://tailwindcss.com/) | Utility-first |
| Config | TOML | Human-readable |
| Audio | rodio | Rust-native, dedicated thread |
| Type Bridge | ts-rs | Rust → TypeScript auto-generation |

## Roadmap

- [x] **v0.1** — Core MVP (timer, tray, multi-monitor, fullscreen detection, settings, theme, auto-start, i18n)
- [x] **v0.2** — Workday scheduling
- [x] **v0.3** — Usage statistics + charts, AFK detection, configurable global hotkeys
- [ ] **v0.4** — Process whitelist
- [ ] **v1.0** — Feature complete + stable release

## Configuration

Config is stored as `config.toml` in the system app data directory:

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%\com.eyezen.app\config.toml` |
| macOS | `~/Library/Application Support/com.eyezen.app/config.toml` |
| Linux | `~/.config/com.eyezen.app/config.toml` |

All settings can be modified through the in-app Settings UI.

## Credits

Eyezen is inspired by **[ProjectEye](https://github.com/Jeremyyang920/ProjectEye)** — a smart eye protection tool for Windows that inspired Eyezen's core timer and break reminder design.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## License

[GNU General Public License v3.0 or later](LICENSE)
