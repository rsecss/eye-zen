# 编码规范

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 命名规范

| 对象 | 规范 | 示例 |
|------|------|------|
| Rust 模块/文件 | `snake_case` | `config.rs`, `timer.rs` |
| Rust 类型 (struct/enum/trait) | `PascalCase` | `TimerService`, `AppError` |
| Rust 函数/方法 | `snake_case` | `resolve_user_event()` |
| Rust 常量 | `UPPER_SNAKE_CASE` | `DEFAULT_WORK_MINUTES` |
| Tauri Command 名 | `snake_case`（即 Rust 函数名） | `get_state_snapshot` |
| Tauri Event 名 | `snake_case` | `state_changed` |
| Svelte 组件文件 | `PascalCase.svelte` | `MainApp.svelte` |
| Svelte 组件 prop | `camelCase` | `timerState` |
| TS 文件（lib/） | `kebab-case.ts` | `commands.ts`, `events.ts` |
| TS 文件（entries/） | `kebab-case.ts` | `tip-minimal.ts` |
| CSS 变量 | `--kebab-case` | `--bg-primary` |
| 分支名 | `<type>/短描述` | `feat/config-service` |

## Rust 规范

### 错误处理

- MUST NOT 使用 `unwrap()`、`expect()`（测试代码除外）
- I/O、service boundary、command 层 MUST 返回 `Result`，使用 `?` 传播
- 纯函数（状态机转换等）按语义返回 `Option` 或普通值，MUST NOT 强制 `Result`
- 错误类型 MUST 实现 `Serialize`（IPC 需要序列化到前端）

### 错误传播链

```
platform/ → PlatformError → service/ → AppError → command/ → IPC Result
```

| 层 | 错误类型 | 职责 |
|----|---------|------|
| `platform/` | 各平台原生错误 | 转为 `PlatformError` |
| `services/` | `PlatformError` → `AppError` | 添加业务上下文 |
| `commands/` | 透传 `AppError` | 不做转换，直接返回 |
| 前端 | `AppError` JSON | 根据 `kind` 字段显示用户提示 |

- 每层 MUST 只转换一次错误，MUST NOT 重复包装
- 日志 MUST 在错误发生层记录，上层 MUST NOT 重复记录同一错误

### 锁策略

- MUST 使用 reducer 模式：锁内收集 effects，锁外执行
- 锁持有时间 MUST 尽可能短（微秒级）
- MUST NOT 在持锁期间做 I/O、网络、channel 发送
- MUST NOT 嵌套锁（避免死锁）

### 异步

- MUST NOT 在 async 上下文中做同步 I/O，使用 `spawn_blocking`
- 后台 task MUST 通过 `CancellationToken` 或 channel 信号支持取消
- MUST NOT 使用 `tokio::time::sleep` 做精确计时，用 `tokio::time::interval`

### 类型

- 跨 Rust/TS 边界的 DTO 字段 SHOULD 使用 `u32` 替代 `u64`（避免 ts-rs 生成 `bigint`）
- 纯后端字段（时间戳、数据库主键、平台 API 句柄）MAY 使用 `u64`/`i64`
- 所有前端需要的类型 MUST 通过 ts-rs 导出
- MUST NOT 手写前端类型定义，以 ts-rs 生成为准

### Lint

```rust
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
```

- 在 `lib.rs` 顶部配置，全 crate 生效
- `cargo clippy` 产生的 warning MUST 全部修复后才能合入

## Svelte 5 规范

### Runes 语法

- 新代码 MUST 使用 `$state` / `$derived` / `$effect` / `$props` Runes 语法
- MUST NOT 在新代码中使用 Svelte 4 的 `$:` 语法和 `let` 响应式声明
- MUST NOT 使用 `createEventDispatcher()`（已弃用），使用 callback props 或 Svelte 5 事件系统

### 组件约束

- 单个组件 SHOULD NOT 超过 200 行
- 超过 200 行 SHOULD 按职责拆分为子组件，如有合理理由可保留
- 组件 MUST 职责单一

### 类型导入

- 前端类型 MUST 从 `$lib/bindings/` 导入 ts-rs 生成类型
- MUST NOT 在前端手动定义与后端重复的类型

## 注释规范

- MUST 只描述意图、约束、设计理由
- MUST NOT 复述代码逻辑
- MUST NOT 写"修改记录"（属于版本控制）
- 非显而易见的依赖关系或行为 SHOULD 添加设计理由注释
- 保持极度简洁

## 日志规范

| 级别 | 用途 | 示例 |
|------|------|------|
| `error` | 不可恢复错误，需要用户关注 | 配置文件损坏、数据库连接失败 |
| `warn` | 可恢复异常，不影响核心功能 | 平台能力降级、音频播放失败 |
| `info` | 关键业务事件 | 服务启动/关闭、状态转换 |
| `debug` | 开发调试信息 | 配置值、channel 消息 |
| `trace` | 高频细节 | 每次 tick、光标位置 |

- 每个平台能力降级 MUST 只记录一次 `warn`，MUST NOT 重复刷日志
- 敏感信息（路径中的用户名等）SHOULD 脱敏
- 使用 `tracing` crate + 每日轮转

## 通用规范

- 文件编码：UTF-8（无 BOM）
- 行尾：LF（`\n`）
- 提交：Conventional Commits（`feat:`、`fix:`、`docs:` 等）
- 分支：`feat/xxx`、`fix/xxx`、`refactor/xxx`、`docs/xxx`
