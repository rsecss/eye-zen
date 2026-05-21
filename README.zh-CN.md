<p align="center">
  <img src="docs/public/logo.svg" alt="Eyezen" width="128" />
</p>

<h1 align="center">Eyezen</h1>

<p align="center">
  <strong>基于 20-20-20 规则的跨平台桌面护眼工具</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg" alt="License" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-brightgreen" alt="Platform" />
  <img src="https://img.shields.io/badge/version-0.3.0-orange" alt="Version" />
  <img src="https://img.shields.io/badge/tauri-v2-blue" alt="Tauri v2" />
</p>

<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

> **项目状态**: v0.3.0 已发布。从 [GitHub Releases](https://github.com/rsecss/eye-zen/releases/latest) 下载预编译安装包。

---

## 什么是 20-20-20 规则？

每工作 **20** 分钟，看向 **20** 英尺（约 6 米）外的物体，持续 **20** 秒。这个简单的习惯可以有效缓解长时间用屏带来的眼部疲劳。

Eyezen 帮你自动化这个过程——安静地在后台计时，时间到了温和提醒你休息。

## 功能

- **20-20-20 定时器** — 可自定义工作/休息时长，完整状态机（工作 → 预提醒 → 提醒 → 休息）
- **多显示器支持** — 在所有显示器上同时显示休息提醒窗口
- **全屏免打扰** — 检测全屏应用时自动跳过提醒（Windows / Linux X11；macOS 与 Wayland 有限支持）
- **系统托盘** — 常驻托盘，快捷查看状态和操作
- **Dark / Light 主题** — 跟随设置切换，包括原生标题栏
- **开机自启动** — 系统启动时自动运行
- **国际化** — 中文 / English 热切换
- **提示音** — 休息提醒时播放柔和音效
- **基于 Rust + Tauri 的轻量桌面栈** — 原生 WebView，资源消耗低

### 为什么选择 Eyezen 而不是 ProjectEye？

Eyezen 的灵感来自 [ProjectEye](https://github.com/Jeremyyang920/ProjectEye)，但使用现代技术栈从零构建：

| | Eyezen | ProjectEye |
|---|--------|------------|
| 平台 | Windows / macOS / Linux | 仅 Windows |
| 技术栈 | Rust + Tauri v2 + Svelte 5 | C# + WPF |
| 内存占用 | ~15 MB | ~170 MB |
| 主题 | Dark / Light + 原生标题栏适配 | 仅浅色 |
| 国际化 | 中文 / English 热切换 | 仅中文 |
| 多显示器 | 所有显示器同时提醒 | 仅主显示器 |
| 维护状态 | 活跃开发中 | 长期未维护 |

## 截图

| 休息中 | 设置 | 关于 | 提醒窗口 |
|:---:|:---:|:---:|:---:|
| ![休息中](docs/public/screenshots/resting.png) | ![设置](docs/public/screenshots/settings.png) | ![关于](docs/public/screenshots/about.png) | ![提醒窗口](docs/public/screenshots/tip_window.png) |

## 下载

### 从 Release 下载（推荐）

前往 [Releases](https://github.com/rsecss/eye-zen/releases/latest) 下载对应平台安装包：

| 平台 | 安装包 |
|------|--------|
| Windows | `.exe` (NSIS) 或 `.msi` |
| macOS | `.dmg` |
| Linux | `.deb` / `.AppImage` |

<details>
<summary>macOS 安全提示</summary>

macOS 可能会阻止未签名的应用。下载后打开终端执行：

```bash
xattr -cr /Applications/Eyezen.app
```

</details>

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) v18+
- [Rust](https://www.rust-lang.org/tools/install) (stable)
- 各平台系统依赖请参考 [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/rsecss/eye-zen.git
cd eye-zen

# 安装前端依赖
npm install

# 开发模式（热重载）
npm run tauri dev

# 生产构建
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

### 开发验证

```bash
# 前端类型检查
npx svelte-check --tsconfig ./tsconfig.json

# 前端测试
npm test

# Rust lint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# 代码格式化检查
npm run format:check
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
```

## 技术栈

| 层 | 选型 | 说明 |
|---|------|------|
| 框架 | [Tauri v2](https://v2.tauri.app/) | Rust 后端 + 原生 WebView |
| 前端 | [Svelte 5](https://svelte.dev/) | Runes 响应式，零运行时 |
| 构建 | [Vite 6](https://vite.dev/) | 极速 HMR |
| 样式 | [TailwindCSS v4](https://tailwindcss.com/) | 工具类优先 |
| 配置 | TOML | 人类可读 |
| 音频 | rodio | Rust 原生，独立线程 |
| 类型桥接 | ts-rs | Rust → TypeScript 自动生成 |

## 路线图

- [x] **v0.1** — 核心 MVP（定时器、托盘、多显示器、全屏检测、设置、主题、自启动、i18n）
- [x] **v0.2** — 工作日调度
- [x] **v0.3** — 使用统计 + 图表、离席检测、可配置全局快捷键
- [ ] **v0.4** — 进程白名单
- [ ] **v1.0** — 功能完整 + 稳定发布

## 配置文件

配置存储于系统应用数据目录下的 `config.toml`：

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\com.eyezen.app\config.toml` |
| macOS | `~/Library/Application Support/com.eyezen.app/config.toml` |
| Linux | `~/.config/com.eyezen.app/config.toml` |

所有设置均可通过应用内设置界面修改，无需手动编辑配置文件。

## 致谢

Eyezen 的灵感来自 **[ProjectEye](https://github.com/Jeremyyang920/ProjectEye)** — Windows 智能护眼工具，Eyezen 的核心计时与休息提醒设计深受其启发。

## 贡献

欢迎任何形式的贡献！请先阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详细的贡献指南和开发规范。

## 许可证

[GNU General Public License v3.0 or later](LICENSE)
