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

### Pre-commit（MUST < 15 秒）

```bash
npx lint-staged                    # Prettier 格式化暂存文件
npx commitlint --edit "$1"         # 提交信息校验
```

### Pre-push（全量）

```bash
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test
npm run build
```

### CI (GitHub Actions)

在 `push` / `pull_request` 到 `dev` / `main` 时触发：

- Rust: `fmt --check` → `clippy` → `test`
- Frontend: `svelte-check` → `vitest run` → `vite build`
- Security: `cargo audit`（SHOULD）

> CI 配置文件尚未创建。

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
