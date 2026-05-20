# Contributing to Eyezen

感谢你对 Eyezen 的关注！无论是修复一个 typo、报告一个 bug、补充一种语言翻译，还是在 macOS / Linux 上帮忙测试——每一份贡献都很有价值。

> **语言说明**：代码、commit message、PR 标题使用英文；issue 和讨论可以使用中文或英文。

---

## Table of Contents

- [行为准则](#行为准则)
- [项目概览](#项目概览)
- [开发环境搭建](#开发环境搭建)
- [开发工作流](#开发工作流)
- [贡献方向](#贡献方向)
- [提交规范](#提交规范)
- [Pull Request 指南](#pull-request-指南)
- [Bug 报告](#bug-报告)
- [功能建议](#功能建议)
- [编码规范](#编码规范)
- [许可证](#许可证)

---

## 行为准则

参与此项目即表示你同意遵守 [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md)。如需举报不当行为，请联系 awmple@outlook.com。

---

## 项目概览

Eyezen 是基于 20-20-20 规则的跨平台桌面护眼工具。

```
Frontend (Svelte 5 Runes)               Backend (Rust)
├── src/pages/          4 个窗口页面     ├── src-tauri/src/services/    7 个服务
├── src/lib/stores/     响应式状态       ├── src-tauri/src/commands/    9 个 IPC 命令
├── src/lib/i18n/       国际化           ├── src-tauri/src/platform/    跨平台抽象
├── src/lib/bindings/   ts-rs 类型桥接   ├── src-tauri/src/models/      共享类型
└── src/lib/commands.ts IPC 调用层       └── src-tauri/src/services/timer/  状态机
```

详细架构说明见 [CLAUDE.md](CLAUDE.md) 和 [`.trellis/spec/architecture/layering.md`](.trellis/spec/architecture/layering.md)。

---

## 开发环境搭建

### 前置条件

| 工具 | 最低版本 | 说明 |
|------|---------|------|
| [Node.js](https://nodejs.org/) | v18+ | 前端构建 |
| [Rust](https://www.rust-lang.org/tools/install) | stable | 后端编译 |
| 平台依赖 | — | 见 [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/) |

#### Linux 额外依赖

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

### 启动开发

```bash
git clone https://github.com/rsecss/eye-zen.git
cd eye-zen
npm install
npm run tauri dev
```

首次编译 Rust 后端需要较长时间（约 3-8 分钟），后续增量编译很快。

---

## 开发工作流

### 分支策略

```
main   ← 稳定发布分支，只接受来自 dev 的合并
dev    ← 主开发分支，PR 目标分支
feat/xxx  ← 功能分支，从 dev 创建
fix/xxx   ← 修复分支，从 dev 创建
```

**所有 PR 都应以 `dev` 为目标分支。**

### 质量门禁

提交前请确保以下检查全部通过：

```bash
# Rust
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

# Frontend
npm run format:check
npx svelte-check --tsconfig ./tsconfig.json
npm test

# 构建验证
npm run build
```

---

## 贡献方向

### 代码贡献

| 方向 | 涉及目录 | 技术栈 | 当前需求 |
|------|---------|--------|---------|
| 后端服务 | `src-tauri/src/services/` | Rust | Phase 2: 统计服务 / 离席检测 |
| 前端页面 | `src/pages/` | Svelte 5 | 统计页面 / 数据可视化 |
| 跨平台适配 | `src-tauri/src/platform/` | Rust | macOS / Wayland 全屏检测改进 |
| IPC 层 | `src/lib/commands.ts` + `src-tauri/src/commands/` | TS + Rust | 新功能的前后端接口 |

### 翻译贡献

Eyezen 目前支持中文 (zh-CN) 和英文 (en)。欢迎添加新语言！

**添加一种新语言的步骤：**

1. 复制 `src/lib/i18n/en.ts`，重命名为目标语言代码（如 `ja.ts`）
2. 翻译所有 value（保持 key 不变）
3. 在 `src/lib/i18n/index.svelte.ts` 中注册新语言
4. 在 `src-tauri/src/services/i18n.rs` 中添加对应的托盘菜单翻译
5. 在 `src-tauri/src/models/config.rs` 的 `Language` 枚举中添加新变体
6. 测试：切换语言后，所有界面文本（含托盘菜单）应立即更新

### 平台测试

以下平台构建通过但**缺少实际测试**，非常欢迎反馈：

| 平台 | 状态 | 已知限制 |
|------|------|---------|
| macOS (ARM) | 未测试 | 全屏检测返回保守 `false` |
| macOS (Intel) | 未测试 | 同上 |
| Linux (X11) | 未测试 | 应能正常工作 |
| Linux (Wayland) | 未测试 | 全屏检测不可用，提醒始终显示 |

如果你在这些平台上运行了 Eyezen，请在 [Issues](https://github.com/rsecss/eye-zen/issues) 中反馈结果，即使一切正常也很有帮助。

### 文档与其他

- 修正文档错误或过时信息
- 改进 README 或添加截图
- 完善开发文档
- 报告 bug 或提出功能建议

---

## 提交规范

使用 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
<type>(<scope>): <subject>
```

### type 类型

| type | 用途 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(timer): add timed pause with auto-resume` |
| `fix` | 修复 bug | `fix(config): prevent TOML overwrite on parse failure` |
| `docs` | 文档 | `docs: update contributing guide` |
| `refactor` | 重构 | `refactor(service): extract effects from timer lock` |
| `test` | 测试 | `test(ui): add SettingsForm component tests` |
| `chore` | 构建/工具 | `chore: update Tauri to 2.11` |
| `style` | 格式 | `style: apply rustfmt to platform module` |
| `ci` | CI/CD | `ci: add macOS ARM to release matrix` |

### 规则

- scope 可选但推荐（如 `timer`, `config`, `ui`, `tray`, `platform`, `i18n`）
- subject 使用英文祈使语气，首字母小写，不加句号
- 一次提交只做一件事

### 分支命名

```
feat/leave-detection
fix/config-atomic-write
docs/add-screenshots
```

---

## Pull Request 指南

### PR 提交前检查清单

- [ ] 以 `dev` 为目标分支
- [ ] 所有质量门禁通过（fmt / clippy / test / svelte-check / build）
- [ ] 新功能附带测试
- [ ] bug 修复先写失败测试，再修复
- [ ] 新增 Tauri Command 已配置对应权限（`src-tauri/capabilities/`）
- [ ] 如有 IPC 接口变更，前后端类型已同步
- [ ] 如有配置 schema 变更，保持向后兼容
- [ ] commit message 符合 Conventional Commits

### PR 标题格式

与 commit message 一致：`feat(scope): short description`

### PR 内容模板

```markdown
## Summary

简要说明改了什么，为什么改。

## Changes

- 具体变更列表

## Test Plan

- 怎样验证这些改动是正确的
```

### Review 流程

- 维护者会在 1-2 周内 review
- 请保持 PR 聚焦，单一目的——大 PR 更难 review
- 如果改动跨前后端边界或超过 ~150 行，请在 PR 描述中说明影响面

---

## Bug 报告

在 [Issues](https://github.com/rsecss/eye-zen/issues) 中创建，请包含：

```
**环境信息**
- OS: (如 Windows 11 23H2 / macOS 15.2 / Ubuntu 24.04)
- Eyezen 版本: (如 v0.1.0 或 commit hash)
- 显示器数量与分辨率:

**复现步骤**
1. ...
2. ...
3. ...

**期望行为**
...

**实际行为**
...

**日志/截图**（如有）
日志目录：
- Windows: %APPDATA%\com.eyezen.app\logs\
- macOS: ~/Library/Application Support/com.eyezen.app/logs/
- Linux: ~/.config/com.eyezen.app/logs/
```

---

## 功能建议

在 [Issues](https://github.com/rsecss/eye-zen/issues) 中创建，请说明：

- **问题**：你遇到了什么问题，或缺少什么能力？
- **方案**：你期望怎样解决？
- **替代方案**：是否考虑过其他方式？
- **适用场景**：这个功能对谁有用？

---

## 编码规范

项目有完整的编码规范文档，位于 [`.trellis/spec/`](.trellis/spec/) 目录：

| 分组 | 覆盖范围 |
|------|---------|
| [`architecture/layering.md`](.trellis/spec/architecture/layering.md) | 分层依赖、`models/` 拆分、可见性 |
| [`architecture/ipc-and-state.md`](.trellis/spec/architecture/ipc-and-state.md) | IPC 接口、Timer 状态机、错误类型 |
| [`architecture/change-management.md`](.trellis/spec/architecture/change-management.md) | 变更清单、破坏性协议、发版流程 |
| [`architecture/testing-quality.md`](.trellis/spec/architecture/testing-quality.md) | 测试要求、质量门禁 |
| [`backend/service-pattern.md`](.trellis/spec/backend/service-pattern.md) | 服务 DAG、四阶段生命周期、关闭顺序 |
| [`backend/coding-standards.md`](.trellis/spec/backend/coding-standards.md) | Rust 命名、错误传播、锁/异步 |
| [`backend/platform-storage.md`](.trellis/spec/backend/platform-storage.md) | PlatformApi、降级、TOML 配置、SQLite |
| [`backend/error-and-logging.md`](.trellis/spec/backend/error-and-logging.md) | AppError、tracing 级别 |
| [`frontend/`](.trellis/spec/frontend/index.md) | Svelte 5 + Vite 多入口 + store + IPC + 视觉 |

### 核心要点速查

**Rust**
- 禁止 `unwrap()` / `expect()`——使用 `?` 或 `Result`
- 锁内只做计算，Effect 在锁外执行
- `pub(crate)` 优先于 `pub`
- 新增服务必须添加到关闭顺序链

**Svelte 5**
- 使用 Runes（`$state` / `$derived` / `$effect`），不使用 Svelte 4 store 语法
- Store 是单一数据源——禁止乐观更新
- IPC 调用通过 `src/lib/commands.ts` 封装，不直接 `invoke()`

**通用**
- 所有依赖版本使用 tilde lock（`~`）
- 新增依赖须有明确理由，优先复用已有依赖

---

## 许可证

提交贡献即表示你同意以 [GPL-3.0-or-later](LICENSE) 许可证发布你的代码（入站许可证 = 出站许可证 / inbound = outbound）。
