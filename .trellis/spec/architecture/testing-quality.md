# 测试与质量

> 测试工具表、变更类型测试要求、husky 质量门禁、CI 矩阵、Instant 陷阱、性能预算。
> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选。

---

## 测试工具

| 层级 | 工具 | 覆盖范围 | 入口命令 |
|------|------|---------|---------|
| Rust 单元测试 | `cargo test` | 状态机纯函数、配置解析、服务公开方法 | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Rust 集成测试 | `cargo test --test <name>` | 存储层、跨服务交互 | 同上 |
| 前端组件测试 | Vitest + `@testing-library/svelte` + `@testing-library/jest-dom` | 组件渲染、props 响应、交互回调、Store | `npm test -- --run` |
| 前端类型检查 | `svelte-check` | TS / Svelte 结构类型 | `npx svelte-check --tsconfig ./tsconfig.json` |
| ts-rs 绑定生成 | `cargo test` 副作用 | `src/lib/bindings/` 自动写出 | 跟随 Rust 单元测试 |
| E2E 测试 | Tauri Driver（后期）| 完整用户流程 | 待引入 |

ts-rs 输出 MUST 不进 Prettier（已在 `.prettierignore` 排除），格式由 ts-rs 版本决定；ts-rs 升级若改格式，MUST 重新跑 `cargo test` 并提交新绑定。

---

## 变更类型与测试要求

| 变更类型 | 测试要求 | 强制级别 |
|----------|---------|---------|
| `fix` | **先写**失败测试复现 bug，再写修复 | **MUST** |
| `feat` — Rust 服务 / 状态机 | 覆盖关键状态转换、边界值、错误路径 | **MUST** |
| `feat` — Tauri command | 至少一条边界测试 + 校验失败路径 | **MUST** |
| `feat` — Svelte 可复用组件 | 渲染 + props 响应 + 交互回调 + 边界值 | **MUST** |
| `feat` — Svelte 页面组件 | 关键交互路径 + store 集成 | **SHOULD** |
| `refactor` | 现有测试全部通过，无需新增 | **MUST** |
| 纯样式 / 文案 / 文档 | `npm run build` + `svelte-check` 通过 | **MUST** |

### 前端可复用组件测试维度

可复用组件（如 `Stepper`、`Toggle`、`Select`）MUST 覆盖以下四维：

| 维度 | 示例 |
|------|------|
| 默认渲染 | 传入 props 后正确显示初值 |
| Props 响应 | 外部 props 变化后 UI 同步更新（响应式订阅）|
| 用户交互 | 点击 / 输入触发 onchange 回调，回调参数符合契约 |
| 边界值 | min/max clamp、空值防御、非法输入 |

### 前端页面组件测试维度

页面组件（如 `SettingsPage`、`AboutPage`）SHOULD 覆盖：

| 维度 | 示例 |
|------|------|
| 关键交互 | 修改配置后调用正确的 `updateXxxConfig` |
| Store 集成 | mock store 后页面正确渲染配置值 |

详细前端测试约束见 [`../frontend/quality-guidelines.md`](../frontend/quality-guidelines.md)。

---

## 质量门禁

项目使用 [husky](https://typicode.github.io/husky/) 管理 git hooks，自动执行分层检查。详细说明见 [`../../../docs/workflows/dev.md`](../../../docs/workflows/dev.md)。

### Pre-commit（`.husky/pre-commit` + `.husky/commit-msg`，MUST < 15 秒）

```bash
npx lint-staged                  # Prettier 自动格式化暂存文件
npx --no -- commitlint --edit "$1"  # Conventional Commits 校验
```

- 仅处理暂存文件，不影响开发节奏。
- `src/lib/bindings/` 已在 `.prettierignore` 中排除（ts-rs 输出格式优先于 Prettier）。
- Commit message 不符合 Conventional Commits MUST 失败。

### Pre-push（`.husky/pre-push`，全量验证）

`.husky/pre-push` MUST call `npm run ci`. The `npm run ci` script is the single local parity entrypoint shared with GitHub Actions and currently runs eight checks:

```bash
[1/8] cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
[2/8] cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
[3/8] cargo test --manifest-path src-tauri/Cargo.toml
[4/8] npx svelte-check --tsconfig ./tsconfig.json
[5/8] npm test -- --run
[6/8] npm run format:check
[7/8] cargo check --manifest-path src-tauri/Cargo.toml
[8/8] npm run build
```

- 耗时约 1-3 分钟，确保推送代码能过 CI。
- 日常 WIP 推送 MAY 用 `git push --no-verify` 跳过。
- 发版分支 / `main` push MUST NOT 跳过。
- 本地 hook 是前置反馈，**不是**最终安全边界；最终门禁依赖 GitHub branch protection + required status checks。
- `rust-toolchain.toml` and `.nvmrc` pin the Rust and Node versions used locally and in GitHub Actions. CI MUST read those files instead of floating `stable` / `lts/*` aliases.

---

## CI（GitHub Actions）

### 触发条件

- `push` 到 `main` / `dev` 分支
- `pull_request` 到 `main` 分支
- 详细配置：`.github/workflows/ci.yml`

### Job 矩阵

| Job | Runner | 内容 |
|-----|--------|------|
| `audit` | `ubuntu-22.04` | `cargo audit --file src-tauri/Cargo.lock`，独立 job，仅 Linux |
| `check` (Windows) | `windows-latest` | `npm run ci` local/cloud parity checks |
| `check` (macOS) | `macos-latest` | `npm run ci` local/cloud parity checks |
| `check` (Linux) | `ubuntu-22.04` | `npm run ci` local/cloud parity checks；需 `libwebkit2gtk-4.1-dev`、`libappindicator3-dev`、`librsvg2-dev`、`patchelf`、`libasound2-dev` |

`check` job 步骤顺序：npm run ci。`npm run ci` owns the shared local/cloud parity checks. Full Tauri packaging belongs to `release.yml` on `v*` tags, not every push or PR.

### 平台特定注意事项

| 平台 | 注意事项 |
|------|---------|
| Linux | `libasound2-dev` 是 rodio / ALSA 必需依赖，MUST 包含在 `apt-get install` 中 |
| Linux | `#[cfg(target_os = "linux")]` 代码**仅**在 Linux runner 被 clippy 检查；其它平台改 Linux 代码 MUST 本地交叉验证或等 CI |
| macOS | 普通 CI 不打包；release.yml 才会双架构打包（ARM + Intel） |
| Windows | `Instant` 算术可能在短 uptime CI runner 上下溢，见下方陷阱 |
| 全平台 | `--all-targets` MUST 用于 clippy，否则 test-target 警告会被遗漏（v0.1.0 实际踩过）|

### 第三方 Action 版本

- `tauri-apps/tauri-action` MUST 使用 `@v0`（v1 tag 已被上游删除）
- 升级 Action 版本 MUST 单独 PR 验证

---

## Instant 测试陷阱

`std::time::Instant` 是单调时钟，从系统启动开始计时。CI runner 可能 uptime 极短，`Instant::now() - Duration::from_secs(large_value)` 会 panic（underflow）。

```rust
// BAD: CI runner uptime 可能不足 1200 秒
let started = Instant::now() - Duration::from_secs(1200);

// GOOD: future_instant 模式，永远安全
let started = Instant::now() + Duration::from_secs(100);
```

**规则**：测试中构造 `Instant` 用于模拟过去时点 MUST 使用 `Instant::now() + Duration`（未来锚点 + 反向算 elapsed），MUST NOT 使用 `Instant::now() - Duration`。timer 状态机相关测试尤其要注意（见 `src-tauri/src/services/timer/machine.rs` 的测试结构）。

---

## 性能预算（SHOULD 级别，Release 构建）

以下为 Release 构建目标值。不同平台可能有差异，以实际基线测量为准。

| 指标 | 目标 | 测量条件 |
|------|------|---------|
| 空闲 CPU | < 1% | Timer loop 使用 `tokio::time::interval`，非 busy loop |
| 内存占用（空闲）| < 50 MB | Windows Release，无 tip-window 显示 |
| 窗口创建延迟 | < 500ms | Tip window 从 Alerting 触发到可见 |
| Timer tick 精度 | < 100ms 偏差 | `tokio::time::interval` |
| 配置写入延迟 | < 500ms | 原子写入（tmp + rename） |
| 启动时间 | < 3s | 从 main 到托盘图标可见 |

- 性能预算 SHOULD 在关键路径变更时验证
- 首次达到 MVP 后 SHOULD 记录各平台基线数据
- 违反预算 SHOULD 记录为 issue 跟踪，MUST NOT 静默放过
