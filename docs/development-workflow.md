# Eyezen 开发工作流程

> AI Agent 驱动的桌面应用开发全生命周期指南。
> 基于 Claude Opus + Codex + Gemini 三方圆桌讨论，结合行业最佳实践。
> 适用于：Rust + Tauri v2 + Svelte 5 + TailwindCSS v4 技术栈，单人/小团队 AI 辅助开发。

---

## 一、开发生命周期总览

```
需求成形 → 方案设计 → 原型设计 → 任务切片 → 实现+测试 → 自动门禁 → 多模型审查 → 合并 → 打 tag → 发布 → 复盘
```

每个阶段都有明确的输入、输出和不可跳过的检查点。

---

## 二、阶段详解

### 阶段 0：需求成形

**目标**：把模糊想法变成有边界的任务描述。

**必须做**：
- 写一份 Task Brief，按规模分级：
  - **小修**（typo/样式/文案/单文件 fix）：一句话说明改什么、为什么改
  - **中等**（单模块 feat/fix，≤3 文件）：简要 Brief（Problem + Acceptance + Non-goals）
  - **大功能**（跨模块/跨边界，≥4 文件）：完整 Brief：
  ```
  Problem:        要解决什么问题
  User impact:    用户可见的变化是什么
  Non-goals:      明确不做什么
  Touched modules: 涉及哪些模块
  Acceptance:     怎样算完成
  Risks:          可能的风险
  Tests required: 需要哪些测试
  ```
- 让 AI 反问 3-5 个澄清问题，暴露盲区
- 对齐关键意图：这个功能的核心价值是什么？用户最在意什么？

**不该做**：
- 直接让 AI "开始做"，没有边界约束
- 跳过 Non-goals，导致 AI 过度实现
- 把多个功能塞进一个需求

**输出物**：Task Brief（可以是 issue、markdown、甚至对话记录，但必须有）

---

### 阶段 1：方案设计与前期调研

**目标**：确认技术可行性，选定方案，画出关键架构图。

**必须做**：
- 调研阶段：
  - 用 `/brainstorming` skill 探索方案空间，不要直接跳到实现
  - 对不确定的库/API，写微型 PoC 验证（特别是跨平台能力）
  - 搜索同类产品的实现方式作为参考
- 设计阶段：
  - 画模块依赖图（Mermaid 即可，不需要 UML）
  - 明确前后端边界：哪些逻辑在 Rust，哪些在 Svelte
  - 定义 IPC 接口：command 签名 + event payload
  - 回答四个关键问题：
    1. 为什么这么做？（有没有更简单的方案）
    2. 会引入什么权限/状态/并发风险？
    3. 数据结构能撑到下个阶段吗？
    4. 失败时怎么降级？

**不该做**：
- 跳过设计直接编码（AI 会做出局部最优但全局不一致的决策）
- 过度设计（4 个服务不需要微服务架构）
- 不验证就假设跨平台 API 可用

**输出物**：方案说明（写入 `docs/.local/devlog.md` 或设计规格文档）

---

### 阶段 2：原型设计

**目标**：在写代码之前确定"用户看到什么、怎么交互"，避免实现阶段反复返工。

**为什么不能跳过**：
- AI 无法"看到"你脑中的界面，没有原型它只能猜测布局和交互
- 代码级迭代 UI 的成本远高于在草图上调整
- Eyezen 有 4 类窗口（main-window / tip-window / tip-window-minimal / tray-panel），每个窗口的交互模式不同，必须提前定义

**按规模分级**：

| 规模 | 原型要求 | 示例 |
|------|----------|------|
| 新窗口/新页面 | 完整线框图 + 交互流 + 组件拆解 | 统计页面、眼部运动引导页 |
| 已有页面新增区块 | 标注位置的草图 + 组件层级 | 设置页新增"快捷键"区块 |
| 纯后端/纯逻辑 | 跳过原型，直接进阶段 3 | DetectorService、配置原子写入 |

**完整原型流程**（新窗口/新页面）：

```
1. 用户旅程        → 用户从哪来、在这个界面做什么、做完去哪
2. 线框图          → 低保真草图（手绘 / Excalidraw / Figma）
3. 组件拆解        → 从线框图提取组件树（哪些是容器、哪些是叶子）
4. 交互定义        → 点击/悬停/键盘/状态切换时发生什么
5. 视觉规约        → 复用现有 CSS 变量（app.css / TailwindCSS v4），标注间距/字号/颜色
6. 响应式/多显示器  → tip-window 全屏覆盖需考虑多显示器；tray-panel 固定尺寸
```

**AI 辅助原型工具**：

| 工具 | 用途 | 何时用 |
|------|------|--------|
| `ui-ux-designer` agent | 生成页面结构、组件拆分、交互流程设计 | 新窗口/新页面 |
| `ui-ux-pro-max` skill | 67 种风格、96 种配色、布局方案生成 | 视觉方向不确定时 |
| Excalidraw / Figma | 手绘线框图 | 快速草图，给 AI 看截图 |
| HTML 静态原型 | 纯 HTML+CSS 快速验证布局 | 需要在浏览器中看效果时 |

**给 AI 实现 UI 时的关键输入**：
```
[线框图]   附截图或文字描述布局
[组件树]   列出组件层级和职责
[交互]     每个可交互元素的行为
[视觉]     复用哪些 CSS 变量，特殊样式标注
[数据源]   每个组件的数据从哪来（store / prop / invoke）
```

> 没有这些输入，AI 会按自己的"常见布局"生成代码，结果往往需要大量修改。提供线框图截图是最有效的约束手段。

**不该做**：
- 在 Figma 里做高保真设计稿（单人项目，投入产出比低）
- 跳过组件拆解直接让 AI 写一整个页面（会产出 200 行单组件巨石）
- 原型阶段讨论实现细节（"用 flex 还是 grid" 不是这个阶段的问题）

**输出物**：线框图（截图/链接）+ 组件树 + 交互说明（可以是一段文字描述）

---

### 阶段 3：技术栈确认与模块拆解

**目标**：锁定版本，拆分可独立实现的模块。

**技术栈锁定**：
- 明确写入 `CLAUDE.md`：
  - Rust edition、Tauri 版本、Svelte 版本、TailwindCSS 版本、关键 crate 版本
  - 不允许 AI 随意引入新依赖，必须先讨论
- 版本锁定策略：次要版本号锁定（如 `"@tauri-apps/api": "~2.0.0"`）
- 当前技术栈参考：`docs/.local/specs/2026-03-18-eyezen-rebuild-design.md` § 1.4

**模块拆解原则**：
- 每个切片满足：1 个目标、1 个主要路径、1 次提交主题
- 尽量不超过 5 个文件、1 小时内可完成并验证
- 拆解结果写入 `docs/plans/<NNN>-<scope>.md`，按 README.md 中的模板格式
- 示例（Phase 2 拆解）：
  ```
  statistics-data-model     → Rust struct + migration
  statistics-storage        → SQLite CRUD + tests
  statistics-ui             → Svelte 统计页面
  leave-detection-service   → DetectorService + platform impl
  leave-detection-exposure  → Timer 集成 + UI 状态
  i18n-infrastructure       → locale catalog + 加载机制
  i18n-seed-translations    → zh/en 翻译文件
  ```

**不该做**：
- 一次让 AI 把"统计+离席检测+i18n 全做完"
- 模块之间有隐式依赖但没有明确

---

### 阶段 4：实现（每个切片的固定流程）

**固定顺序，不可跳步**：

```
1. 读现有代码，理解模式        → 用 mcp__ace-tool 搜索相关代码
2. 列出影响面                  → 哪些文件要改，哪些接口受影响
3. 先定接口，再填实现          → Rust trait / TS interface 先行
4. 写实现代码                  → 遵循 CLAUDE.md 编码规范
5. 写对应测试                  → feat 必须有测试，fix 先写失败测试
6. 跑自动检查                  → fmt + clippy + svelte-check + test
7. 更新文档（如有 API 变更）    → IPC 接口、配置字段、权限
```

**AI Agent 使用规则**：
- 凡是库 API、Tauri 权限、Svelte 5 语法，一律先查官方文档再写
- 要求 AI 输出时附带：
  ```
  Assumptions:       假设了什么
  Docs checked:      查了哪些文档
  Files touched:     改了哪些文件
  Tests to run:      需要跑哪些测试
  Open risks:        已知风险
  ```
- 如果 AI 给不出 assumptions 或 docs，这次输出不该直接采纳

**Skill 链式调用**：
- 新增 Tauri 功能：`tauri-v2 → configuring-tauri-permissions → rust-async-patterns → rust-best-practices`
- 新增/修改 Svelte 页面：`svelte-code-writer → svelte5-best-practices → 组件测试`
- 跨前后端功能：后端链一遍，前端链一遍，最后多模型 review

---

### 阶段 5：测试

**测试要求（不可模糊）**：

| 变更类型 | 测试要求 |
|----------|----------|
| `fix` | 先写失败测试，再修复，验证通过 |
| `feat` — Rust 服务/状态机 | 单元测试覆盖关键状态转换、边界值、错误路径 |
| `feat` — Tauri command | 至少一条边界测试覆盖调用条件 |
| `feat` — Svelte 组件（有交互/分支/事件） | 组件测试 |
| 纯样式/文案/文档 | 不强制自动测试，但必须过构建 |
| `refactor` | 现有测试全部通过，不改变外部行为 |

**测试层级**：

| 层级 | 工具 | 覆盖范围 |
|------|------|----------|
| Rust 单元测试 | `cargo test` | 状态机、配置、服务注册 |
| Rust 集成测试 | `cargo test --test` | 存储层、跨服务交互 |
| 前端组件测试 | Vitest + Testing Library | 组件渲染、交互、Store |
| 前端类型检查 | `svelte-check` | 类型安全、Svelte 结构 |
| E2E 测试 | Tauri Driver（后期） | 完整用户流程 |

---

### 阶段 6：代码审查（多模型审查模式）

**触发条件**（满足任一即必须做第二模型复核）：
- 跨前后端边界
- 新增权限/插件
- 新增异步任务/状态机状态
- 新增持久化/迁移
- 新增窗口/托盘/全局快捷键
- 改动超过 ~150 行或 3 个文件

**三模型分工**：

| 模型 | 角色 | 提示词要点 |
|------|------|-----------|
| Claude Code | 主实现器 | 长链路实现、小步迭代、按仓库上下文落地 |
| Codex | 代码级审查器 | "只找阻塞问题，不要重写；按严重级排序；列出缺失测试和回归点" |
| Gemini | 需求/边界审查器 | "这个方案漏了哪些用户场景？哪些状态转换没覆盖？哪些异常路径没写？" |

**审查反模式**：
- 不要让 3 个模型并行实现同一功能（制造 diff 噪音）
- 不要盲信任何单一模型的"没问题"结论
- 一个实用的提示词：*"不要解释为什么这段代码可能对；请列出 3 个它可能错的地方，并给出验证办法。"*

---

### 阶段 7：提交规范

**Conventional Commits（强制）**：

```
<type>(<scope>): <subject>

[body]

[footer]
```

**type 类型**：

| type | 用途 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(timer): add timed pause with auto-resume` |
| `fix` | 修复 bug | `fix(config): prevent TOML overwrite on parse failure` |
| `docs` | 文档变更 | `docs: add development workflow guide` |
| `refactor` | 重构（不改行为） | `refactor(service): extract effects from timer lock` |
| `test` | 测试 | `test(ui): add SettingsForm component tests` |
| `chore` | 构建/工具/依赖 | `chore: add pre-commit hooks for fmt and clippy` |
| `style` | 格式（不影响逻辑） | `style: apply rustfmt to platform module` |
| `perf` | 性能优化 | `perf(timer): reduce lock hold duration in on_tick` |
| `ci` | CI/CD 配置 | `ci: add cross-platform release workflow` |
| `build` | 构建系统 | `build: update Tauri to 2.1.0` |
| `revert` | 回退 | `revert: revert "feat(timer): ..."` |

**规则**：
- scope 可选但鼓励（如 `timer`, `config`, `ui`, `tray`, `platform`）
- subject 用祈使语气，首字母小写，不加句号
- 一次提交只做一件事（原子提交）
- Breaking change 在 footer 标注 `BREAKING CHANGE: <description>`
- 不要在提交中包含 secrets、API keys、tokens

**分支命名**：
```
feat/leave-detection
fix/config-atomic-write
refactor/timer-effects
docs/workflow-guide
chore/ci-pipeline
```

---
### 阶段 8：自动质量门禁

**分层门禁设计**（不要把所有检查塞进 pre-commit，否则你会关掉它）：

#### 8.1 Pre-commit（快，<15 秒）

```bash
# .husky/pre-commit 或 lefthook.yml（只做格式检查，不做类型检查）
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
```

#### 8.2 Pre-push（全量）

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test
npm run build
```

#### 8.3 PR CI（GitHub Actions，必须）

最小版只跑 Linux，给每个分支一个客观"可合并/不可合并"信号：

```yaml
# .github/workflows/ci.yml
name: CI
on:
  push:
    branches: [dev, main]
  pull_request:
    branches: [dev, main]

jobs:
  check:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Install frontend dependencies
        run: npm ci

      - name: Rust format check
        run: cargo fmt --all --check --manifest-path src-tauri/Cargo.toml

      - name: Rust clippy
        run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

      - name: Rust tests
        run: cargo test --manifest-path src-tauri/Cargo.toml

      - name: Svelte check
        run: npx svelte-check --tsconfig ./tsconfig.json

      - name: Frontend tests
        run: npm test

      - name: Build check
        run: npm run build

      - name: Security audit (advisory)
        run: cargo audit --manifest-path src-tauri/Cargo.toml
        continue-on-error: true

      - name: Dependency policy check
        run: cargo deny check --manifest-path src-tauri/Cargo.toml
```

#### 8.4 安全扫描（CI 中集成）

```bash
# Rust 依赖漏洞扫描
cargo install cargo-audit
cargo audit

# Rust 依赖许可证 + 来源 + 重复版本检查
cargo install cargo-deny
cargo deny check
```

- `cargo-audit`：扫描 `Cargo.lock` 中已知漏洞（RustSec 数据库）
- `cargo-deny`：检查许可证合规、禁用依赖、重复版本
- 建议 CI 中 `cargo audit` 设为 `continue-on-error: true`（告警不阻塞），`cargo deny` 设为硬门禁

---

### 阶段 9：发布流程

#### 9.1 版本策略

当前处于 `0.x.y` 阶段：
- `0.2.0`：Phase 2 新能力（统计/i18n/离席检测）
- `0.2.1`：兼容性 bug 修复
- `0.3.0`：下一批显著新功能

进入 `1.0.0` 的条件：
- 配置结构基本稳定
- 数据格式/迁移策略稳定
- 三平台发布流程稳定
- 至少一次真实用户升级路径验证过

版本号同步修改位置：
- `src-tauri/tauri.conf.json` → `version`
- `src-tauri/Cargo.toml` → `version`
- `package.json` → `version`

#### 9.2 Release CI（GitHub Actions）

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: 'macos-latest'
            args: '--target aarch64-apple-darwin'
          - platform: 'macos-latest'
            args: '--target x86_64-apple-darwin'
          - platform: 'ubuntu-22.04'
            args: ''
          - platform: 'windows-latest'
            args: ''
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - name: Install dependencies (Linux)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Install frontend dependencies
        run: npm ci

      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: v__VERSION__
          releaseName: 'Eyezen v__VERSION__'
          releaseBody: 'See CHANGELOG for details.'
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.args }}
```

#### 9.3 发布检查清单

每次打 tag 前必须确认：

```
[ ] 版本号已同步更新（tauri.conf.json, Cargo.toml, package.json）
[ ] CHANGELOG 已更新
[ ] dev 分支 CI 全绿
[ ] 本地三平台构建验证（至少当前平台）
[ ] 无未提交的变更
[ ] 数据库迁移兼容性确认（如有）
[ ] 配置文件向后兼容确认
```

#### 9.4 发布步骤

```bash
# 1. 确保 dev 分支干净且 CI 通过
git checkout dev
git pull

# 2. 合并到 main
git checkout main
git merge dev

# 3. 打 tag（触发 Release CI）
git tag v0.2.0
git push origin main --tags

# 4. GitHub 上检查 draft release，补充 release notes，发布
```

#### 9.5 热修复与回滚

**热修复**（main 有 bug，dev 上有未完成的新功能）：
```bash
# 从 main 拉出 hotfix 分支
git checkout main
git checkout -b fix/critical-bug

# 修复 + 测试 + 提交
# ...

# 合并到 main 并打补丁 tag
git checkout main
git merge fix/critical-bug
git tag v0.2.1
git push origin main --tags

# 同步回 dev（避免下次合并冲突）
git checkout dev
git merge main
```

**回滚**（发布后发现严重问题）：
- 如果 GitHub Release 还是 draft：直接删除 draft 和 tag
- 如果已发布：发布新 patch 版本（v0.2.2）修复，而非删除旧版本
- 用户已安装的版本无法远程撤回，只能通过 updater 推送新版

#### 9.6 签名与公证（后期补充）

| 平台 | 要求 | 说明 |
|------|------|------|
| macOS | Developer ID + Notarization | 需要 Apple Developer 账号，secrets: `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_CERTIFICATE` |
| Windows | Code Signing | 非必需但强烈建议（SmartScreen 信任），需要代码签名证书 |
| Linux | 校验和 | 优先保证制品完整性，签名可后续增强 |

#### 9.7 Changelog 生成

- 保持 Conventional Commits 规范
- GitHub 自动生成 release notes（配置 `.github/release.yml`）
- 手动补充四段前言：
  ```
  ## Highlights        — 核心变化
  ## Breaking Changes  — 不兼容变更和迁移方法
  ## Known Issues      — 已知问题
  ## Checksums         — 制品校验和
  ```

---

### 阶段 10：发布后复盘

每次 release 后写一份极简复盘（5-10 分钟）：

```
## Release v0.x.y 复盘
Date: YYYY-MM-DD

### 本次改了什么
- ...

### 出了什么问题
- ...

### 下次要提前加什么检查
- ...

### 是否要补规则或测试
- ...
```

写入 `docs/.local/devlog.md`，积累项目经验。

---

## 三、AI Agent 驱动开发的关键注意点

### 3.1 最危险的 7 个坑

| 坑 | 表现 | 防御 |
|----|------|------|
| 幻觉 API | Tauri v1/v2 混用、Svelte 4/5 混用 | 强制先查文档再写，用 Context7 MCP |
| 版本错配 | 代码能写但与仓库真实版本不匹配 | 锁定版本写入 CLAUDE.md |
| 模式漂移 | 前几个文件遵守约定，后面"按模型习惯写" | 自动 linter 门禁（不靠提醒靠机器） |
| 上下文遗失 | 长会话后忘了约束和前面决策 | 每个切片写 6 行摘要，及时开新会话 |
| 过度重构 | 用户只要一个功能，AI 顺手改架构 | Task Brief 明确 Non-goals |
| 边界遗漏 | 改了 command 忘了权限，改了组件忘了测试 | Skill 链式调用覆盖全链路 |
| 假通过 | 代码能编译但运行路径不成立 | 最短路径验证 + 负向测试 |

### 3.2 有效的 Prompt 结构

```
[上下文] 项目使用 Tauri v2 + Svelte 5 Runes，当前在实现 X 模块。
[意图]   请实现 Y 功能。
[约束]   1. 只使用仓库已有依赖版本
         2. 错误处理不能用 unwrap，必须返回 Result
         3. 遵循 CLAUDE.md 编码规范
         4. 不确定的 API 先查文档，不要猜
[输出]   请附带 Assumptions / Docs checked / Files touched / Tests to run / Open risks
```

### 3.3 会话管理

满足以下任一条件就开新会话：
- 改动超过 8 个文件或跨 3 个以上模块
- 会话超过 1 个主要功能
- AI 开始重复忘记前提或做出与前面决策矛盾的输出
- 上下文窗口已被压缩（Claude Code 会提示）

每完成一个切片，写 6 行摘要：
```
Goal:          做了什么
Decision:      选了什么方案
Files changed: 改了哪些文件
Tests added:   加了哪些测试
Known risks:   已知风险
Next step:     下一步
```

### 3.4 防止 AI 漂移

靠"提醒一下"没用，靠机器执行才有用。三层防线：

| 层级 | 机制 | 内容 |
|------|------|------|
| 仓库规则 | `CLAUDE.md` | 编码规范、技术栈约束、项目上下文 |
| 技能规则 | 6 个 project skills | 框架特定的最佳实践 |
| 自动门禁 | pre-commit / CI | fmt, clippy, svelte-check, tests |

Rust 侧建议在 `lib.rs` 或 `main.rs` 顶部加：
```rust
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
```

### 3.5 Skill 使用策略

| Skill | 触发条件 | 用途 |
|-------|----------|------|
| `/brainstorming` | 任何新功能/新模块/方案选型之前 | 探索方案空间，暴露盲区，避免直接跳到实现 |
| `tauri-v2` | 改 `tauri.conf.json`、新增 command/窗口/托盘/插件/打包 | 确认 Tauri v2 API 和配置 |
| `configuring-tauri-permissions` | 任何新 command/plugin/window | 收敛权限，最小授权 |
| `rust-async-patterns` | 定时器、后台服务、channel、tokio task、共享状态 | 避免阻塞 async、跨 await 持锁 |
| `rust-best-practices` | 任何 Rust 改动超过小修级别 | 错误类型、clone、unwrap、测试覆盖 |
| `svelte-code-writer` | 任何 `.svelte` 或 `.svelte.ts` 修改 | 查 Svelte 官方语法、结构问题 |
| `svelte5-best-practices` | `$state`/`$derived`/`$effect`/`$props`/事件处理 | Runes 正确用法、Svelte 4→5 迁移 |

链式调用模式：
```
新增 Tauri 功能:
  tauri-v2 → configuring-tauri-permissions → rust-async-patterns → rust-best-practices

新增 Svelte 页面:
  svelte-code-writer → svelte5-best-practices → 组件测试 review

跨边界功能（如统计）:
  后端链一遍 → 前端链一遍 → 多模型 review（不要混成一个大 prompt）
```

---

## 四、文档策略

### 4.1 必需文档（7 类）

| 文档 | 内容 | 维护频率 |
|------|------|----------|
| `README.md` | 用户视角：是什么、平台、下载、功能、截图 | 每次 release |
| `CLAUDE.md` | AI 上下文：架构、模块索引、IPC 接口、开发命令、权限说明 | 每次架构变更 |
| `docs/development-workflow.md` | 本文档（含测试策略、发布流程） | 流程变更时 |
| `docs/plans/` | 实现计划，命名 `<NNN>-<scope>.md`（gitignored，本地保留） | 每个功能切片规划时 |
| `docs/.local/experience-review.md` | 经验复盘与重建指南 | 阶段复盘时 |
| `docs/.local/devlog.md` | 开发日志：关键决策、里程碑、会话摘要 | 每个切片/阶段完成时 |
| `docs/.local/specs/` | 设计规格文档 | 新功能设计时 |

> 注意：不要拆出独立的 `testing-strategy.md`、`release-runbook.md`、`security-and-permissions.md` — 这些内容已分别覆盖在本文档的阶段 4/8/7 和 `CLAUDE.md` 中。单人项目维护 8 份独立文档不现实，会迅速过期。等团队扩展到 3+ 人时再拆分。

### 4.2 调研/参考文档（`docs/.local/`）

| 文档 | 内容 |
|------|------|
| `docs/.local/projecteye-research.md` | ProjectEye 深度调研 |
| `docs/.local/blinkeye-research.md` | Blink Eye 分析 + 技术栈选型 |
| `docs/.local/style-comparison.html` | UI 风格对比 |

### 4.3 可选文档（按需创建）

- `docs/config-and-data.md` — 配置字段、默认值、迁移策略
- `docs/i18n-guidelines.md` — 翻译规范、key 命名、tray 菜单翻译
- `docs/troubleshooting.md` — 常见问题排查
- `CHANGELOG.md` — 版本变更记录

### 4.4 文档原则

- 凡是"隔两周你自己都可能忘"的内容，就应该文档化
- 不要写实现细节流水账，只写系统边界和决策理由
- AI 可以帮生成代码注释和接口文档，但宏观架构文档需要人工维护

---

## 五、依赖管理与长期维护

### 5.1 依赖更新

- 使用 Renovate 或 Dependabot（两者都支持分组更新）
  - Renovate：`packageRules` 分组灵活，可将 Cargo + NPM 打包
  - Dependabot：GitHub 原生集成，配置 `groups` 实现分组
- 配置为每月批量更新，避免天天轰炸
- 更新前让 AI 阅读 Changelog，评估迁移成本

### 5.2 处理 Breaking Changes

- Svelte 5 和 Tauri v2 都较新，breaking change 频率较高
- 策略：锁定次要版本号，定期（每月）评估是否升级
- 升级时让 AI 对比 Changelog 和当前代码，列出需要迁移的点

### 5.3 技术债管理

黄金法则：**如果没有 BUG，且近期不需要修改该模块，就不要因为"代码不够优雅"去重构它。**

将重构推迟到你需要触碰该代码的时刻（Boy Scout Rule：离开时比来时干净一点）。

### 5.4 崩溃监控（后期）

- Rust 端：`std::panic::set_hook` 捕获崩溃写入日志，下次启动提示用户反馈
- 考虑集成 `sentry-rust`（轻量级崩溃上报）
- 当前 tracing + 日志轮转已经是良好基础

---

## 六、完整流程速查表

```
┌─────────────────────────────────────────────────────────────┐
│                    一个功能的完整生命周期                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. Task Brief          写需求，明确边界和非目标              │
│         ↓                                                   │
│  2. /brainstorming      探索方案空间，不直接跳到代码          │
│         ↓                                                   │
│  3. 方案设计             画模块图，定 IPC 接口，记录决策       │
│         ↓                                                   │
│  4. 切片拆分             1 目标 / 1 提交 / ≤5 文件 / ≤1h     │
│         ↓                                                   │
│  ┌─── 每个切片循环 ───┐                                      │
│  │  5. 读现有代码       │                                    │
│  │  6. 定接口           │                                    │
│  │  7. 写实现 + 测试    │  ← Skill 链式调用                  │
│  │  8. 跑自动门禁       │  ← fmt + clippy + svelte-check     │
│  │  9. 多模型审查       │  ← Codex 代码审 + Gemini 边界审    │
│  │ 10. 提交（atomic）   │  ← Conventional Commits            │
│  └──────────────────────┘                                    │
│         ↓                                                   │
│ 11. PR / 合并到 dev     CI 全绿                              │
│         ↓                                                   │
│ 12. 合并到 main + tag   触发 Release CI                      │
│         ↓                                                   │
│ 13. 验收 draft release  补 release notes，发布               │
│         ↓                                                   │
│ 14. 发布复盘            5 分钟极简回顾                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 七、参考来源

- [Tauri v2 Official Docs — Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri v2 — GitHub Pipeline](https://v2.tauri.app/distribute/pipelines/github/)
- [Tauri v2 — macOS Code Signing](https://v2.tauri.app/distribute/sign/macos/)
- [Svelte 5 — What are runes?](https://svelte.dev/docs/svelte/what-are-runes)
- [Svelte 5 — Migration Guide](https://svelte.dev/docs/svelte/v5-migration-guide)
- [RustSec — cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [Embark Studios — cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [GitHub Docs — Automatically Generated Release Notes](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes)
- Codex 审查意见（2026-03-17 圆桌会议）
- Gemini 审查意见（2026-03-17 圆桌会议）
- METR 研究：AI 工具对资深开发者生产力的影响（2025 早期 RCT）
