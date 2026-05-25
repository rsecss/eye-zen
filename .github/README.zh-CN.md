<p align="center">
  <img src="../docs/public/logo.svg" alt="Eyezen" width="128" />
</p>

<h1 align="center">Eyezen</h1>

<p align="center">
  <strong>跨平台桌面护眼工具 — 安静、聪明、不打扰</strong>
</p>

<p align="center">
  <a href="../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platform" />
  <img src="https://img.shields.io/badge/version-0.7.1-orange" alt="Version" />
  <img src="https://img.shields.io/badge/tauri-v2-blue" alt="Tauri v2" />
  <img src="https://img.shields.io/badge/coverage-93%25-brightgreen" alt="Coverage" />
</p>

<p align="center">
  <a href="../README.md">English</a> | 简体中文
</p>

---

## 👀 简介

Eyezen 是一个低噪音的桌面护眼工具。它按 **20-20-20 规则**（每 20 分钟看 6 米外 20 秒）温柔提醒你休息，并提供番茄模式适配深度专注节奏。全屏 / 离席 / 进程白名单 / 工作日多重智能跳过让提醒不打扰你的关键时刻；统计与健康分析帮你看清护眼习惯是否真正坚持。

## ✨ 亮点

- ⏱️ **双计时模式** — 20-20-20 与番茄模式并行，工作 / 休息时长可自定义
- 🎯 **智能跳过** — 全屏检测、AFK 离席、进程白名单、工作日调度
- 📊 **统计与健康分析** — 日 / 周 / 月 ECharts 趋势 + 量化护眼习惯的 Eye-Care Index
- 🖥️ **多显示器提醒** — 所有连接的显示器同时显示休息提醒
- 🌓 **Dark / Light + 国际化** — 简体中文 / English 热切换，Windows 平台原生标题栏适配
- ⌨️ **全局快捷键与玻璃拟态托盘** — 快捷键 start / skip / pause；托盘面板跟随图标、失焦自动隐藏
- ⚡ **轻量与测试** — Rust + Tauri 后端，~15 MB 内存；前后端均锁 90%+ 行覆盖率，CI 强制

## 📸 截图

### 核心交互

| 休息中 | 提醒窗口 |
|:---:|:---:|
| ![休息中](../docs/public/screenshots/resting.png) | ![提醒窗口](../docs/public/screenshots/tip_window.png) |

| 设置 | 关于 |
|:---:|:---:|
| ![设置](../docs/public/screenshots/settings.png) | ![关于](../docs/public/screenshots/about.png) |

### 统计与健康分析

| 总览 · 护眼指数 · 24 小时色带 | 日 / 周 / 月趋势 |
|:---:|:---:|
| ![统计总览](../docs/public/screenshots/statistics-overview.png) | ![趋势图](../docs/public/screenshots/statistics-trend.png) |

### 番茄模式

<p align="center">
  <img src="../docs/public/screenshots/settings-pomodoro.png" alt="番茄模式设置" width="720" />
  <br/>
  <em>与 20-20-20 并列的专注 / 短休 / 长休循环配置</em>
</p>

## 🚀 Quick Start

**环境要求**：[Node.js](https://nodejs.org/) v18+，[Rust](https://www.rust-lang.org/)（stable），各平台系统依赖请参考 [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)。

```bash
git clone https://github.com/rsecss/eye-zen.git
cd eye-zen
npm install
npm run tauri dev    # 开发模式（热重载）
npm run tauri build  # 生产构建 → src-tauri/target/release/bundle/
```

一键跑齐 8 步本地 CI 检查（fmt + clippy + cargo test + svelte-check + vitest + prettier + build + version sync）：

```bash
npm run ci
```

## 🛠️ 技术栈

| 层 | 选型 | 说明 |
|---|------|------|
| 框架 | [Tauri v2](https://v2.tauri.app/) | Rust 后端 + 原生 WebView |
| 前端 | [Svelte 5](https://svelte.dev/) | Runes 响应式，零运行时 |
| 构建 | [Vite 6](https://vite.dev/) | 极速 HMR，多入口窗口 |
| 样式 | [TailwindCSS v4](https://tailwindcss.com/) | 工具类优先 |
| 图表 | [ECharts](https://echarts.apache.org/) | Tree-shaken，按需加载 |
| 数据库 | SQLite via [sqlx](https://github.com/launchbadge/sqlx) | 统计数据持久化 |
| 配置 | TOML | 人类可读 |
| 音频 | rodio | Rust 原生，独立线程 |
| 类型桥接 | ts-rs | Rust → TypeScript 自动生成 |

## 🏗️ 项目架构

```
┌────────────────────────────────────────────────────────────┐
│  前端 (Svelte 5)                                           │
│  窗口: main · tray-panel · tip-window · tip-minimal        │
│    invoke()  ─→ Tauri Commands（薄层）                     │
│    listen()  ←─ Typed Events（ts-rs 桥接）                 │
└────────────────────────────────────────────────────────────┘
                              │
┌────────────────────────────────────────────────────────────┐
│  后端 Services（Tauri State · 9 个 Arc 共享服务）          │
│    Config · Timer · Detector · Window · Sound              │
│    Tray   · I18n  · Stat     · Hotkey                      │
│  通信: watch 通道 + EffectSink trait                       │
└────────────────────────────────────────────────────────────┘
                              │
┌────────────────────────────────────────────────────────────┐
│  平台抽象层 — PlatformApi trait                            │
│  Windows · macOS · Linux X11/Wayland                       │
│  能力: 全屏检测 · 空闲时长 · 前台进程                       │
└────────────────────────────────────────────────────────────┘
```

- **Services** 在启动时一次构造，通过 `Arc<AppServices>` 注入 Tauri State 共享；服务间用 `tokio::sync::watch` 通道与 `EffectSink` trait 通信，让 Timer 状态机保持纯函数。
- **平台抽象** 把 OS 相关 FFI 隔离在 `PlatformApi` trait 内，按能力维度暴露 degrade flag——Settings UI 会自动灰显当前不可用的能力（例如 Wayland 上的 AFK）。
- **IPC 契约** 由 `ts-rs` 自动生成并集中在 `src/lib/bindings/`，消除 Rust↔TS 漂移问题。
- **编码规范** 沉淀在 `.trellis/spec/`——分层、IPC、平台等约束的唯一来源，贡献前请先读。

## ⚙️ 配置文件

配置存储于系统应用数据目录下的 `config.toml`；统计数据库 `data.db` 同目录。

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\com.eyezen.app\` |
| macOS | `~/Library/Application Support/com.eyezen.app/` |
| Linux | `~/.config/com.eyezen.app/` |

所有设置均可通过应用内 Settings UI 修改，无需手动编辑 TOML。

## 🤝 贡献

欢迎任何形式的贡献！请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细的贡献指南和开发规范。

## 📄 许可证

[GNU General Public License v3.0 or later](../LICENSE)
