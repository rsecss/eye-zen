# 测试与质量

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 测试工具

| 层级 | 工具 | 覆盖范围 |
|------|------|---------|
| Rust 单元测试 | `cargo test` | 状态机、配置、服务 |
| Rust 集成测试 | `cargo test --test` | 存储层、跨服务交互 |
| 前端组件测试 | Vitest + Testing Library | 组件渲染、交互、Store |
| 前端类型检查 | `svelte-check` | 类型安全、Svelte 结构 |
| E2E 测试 | Tauri Driver（后期） | 完整用户流程 |

## 变更类型与测试要求

| 变更类型 | 测试要求 | 强制级别 |
|----------|---------|---------|
| `fix` | 先写失败测试，再修复 | **MUST** |
| `feat` -- Rust 服务/状态机 | 覆盖关键状态转换、边界值、错误路径 | **MUST** |
| `feat` -- Tauri command | 至少一条边界测试 | **MUST** |
| `feat` -- Svelte 可复用组件 | 渲染 + props 响应 + 交互回调 + 边界值 | **MUST** |
| `feat` -- Svelte 页面组件 | 关键交互路径 + store 集成 | **SHOULD** |
| 纯样式/文案/文档 | 必须过构建 | **MUST** |
| `refactor` | 现有测试全部通过 | **MUST** |

### 前端组件测试标准

可复用组件（如 Stepper, Toggle, Select）MUST 覆盖：

| 测试维度 | 示例 |
|---------|------|
| 默认渲染 | 传入 props 后正确显示值 |
| Props 响应 | 外部 props 变化后 UI 同步更新 |
| 用户交互 | 点击按钮触发 onchange 回调，参数正确 |
| 边界值 | min/max clamp、空值防御 |

页面组件（如 SettingsPage, AboutPage）SHOULD 覆盖：

| 测试维度 | 示例 |
|---------|------|
| 关键交互 | 修改配置后调用正确的 update command |
| Store 集成 | mock store 后页面正确渲染配置值 |

## 质量门禁

项目使用 [husky](https://typicode.github.io/husky/) 管理 git hooks，自动执行分层检查。
详细说明见 `docs/workflows/dev.md`。

### Pre-commit（每次 `git commit`，MUST < 15 秒）

位置：`.husky/pre-commit` + `.husky/commit-msg`

```bash
npx lint-staged                    # Prettier 自动格式化暂存文件
npx commitlint --edit "$1"         # Conventional Commits 格式校验
```

- 仅处理暂存文件，不影响开发节奏
- `src/lib/bindings/` 已排除在 prettier 之外（ts-rs 输出格式优先）

### Pre-push（每次 `git push`，全量验证）

位置：`.husky/pre-push`

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test -- --run
npm run format:check
npm run build
```

- 耗时 1-3 分钟，确保推送的代码能通过 CI
- 日常开发可用 `git push --no-verify` 跳过（仅限 WIP 推送）
- 发版前 MUST NOT 跳过

### CI (GitHub Actions)

在 `push` / `pull_request` 到 `dev` / `main` 时触发（`.github/workflows/ci.yml`）：

- Rust: `check` → `clippy --all-targets` → `test` → `fmt --check`
- Frontend: `svelte-check` → `vitest run` → `format:check` → `vite build`
- 三平台矩阵：Windows / macOS / Linux
- 额外 Tauri build 验证（`tauri-action@v0`）
- Security: `cargo audit`（已配置，独立 job，仅 Linux 运行）

#### 平台特定注意事项

| 平台 | 注意事项 |
|------|---------|
| Linux | `libasound2-dev` 是 rodio/ALSA 必需依赖，MUST 包含在 `apt-get install` 中 |
| Linux | `#[cfg(target_os = "linux")]` 代码仅在 Linux CI 被 clippy 检查 |
| macOS | CI 仅构建单架构（ARM 或 Intel），非 universal binary |
| Windows | `Instant` 算术可能在短 uptime CI runner 上下溢（见下方） |
| 全平台 | `--all-targets` MUST 用于 clippy，否则 test-target 警告会被遗漏 |

#### Instant 测试陷阱

`std::time::Instant` 是单调时钟，从系统启动开始计时。CI runner 可能 uptime 极短，`Instant::now() - Duration::from_secs(large_value)` 会 panic。

```rust
// BAD: CI runner uptime 可能不足 1200 秒
let started = Instant::now() - Duration::from_secs(1200);

// GOOD: future_instant 模式，永远安全
let started = Instant::now() + Duration::from_secs(100);
```

**规则**：测试中构造 `Instant` 时，MUST 使用 `Instant::now() + Duration`（未来时间点），MUST NOT 使用 `Instant::now() - Duration`（过去时间点）。

## 性能预算（SHOULD 级别，Release 构建）

以下为 Release 构建下的目标值。不同平台可能有差异，以实际基线测量为准。

| 指标 | 目标 | 测量条件 |
|------|------|---------|
| 空闲 CPU | < 1% | Timer loop 使用 `tokio::time::interval`，非 busy loop |
| 内存占用（空闲） | < 50 MB | Windows Release，无 tip-window 显示 |
| 窗口创建延迟 | < 500ms | Tip window 从 Alerting 触发到可见 |
| Timer tick 精度 | < 100ms 偏差 | `tokio::time::interval` |
| 配置写入延迟 | < 500ms | 原子写入（tmp + rename） |
| 启动时间 | < 3s | 从 main 到托盘图标可见 |

- 性能预算 SHOULD 在关键路径变更时验证
- 首次达到 MVP 功能完整后 SHOULD 记录各平台基线数据
- 违反预算 SHOULD 记录为 issue 跟踪
