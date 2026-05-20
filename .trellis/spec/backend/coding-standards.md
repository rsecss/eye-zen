# Rust 编码规范

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

本文档约束 `src-tauri/` 下所有 Rust 代码的写法。Svelte / TS 规范见 [`../frontend/`](../frontend/)，跨层契约见 [`../architecture/`](../architecture/)。

## 命名规范

| 对象 | 规范 | 示例 |
|------|------|------|
| Rust 模块 / 文件 | `snake_case` | `config.rs`、`timer/service.rs` |
| 类型 (struct/enum/trait) | `PascalCase` | `TimerService`、`AppError` |
| 函数 / 方法 | `snake_case` | `resolve_user_event()`、`handle_user_event()` |
| 常量 | `UPPER_SNAKE_CASE` | `DEFAULT_WORK_MINUTES` |
| Tauri Command 名 | `snake_case`（即 Rust 函数名） | `get_state_snapshot` |
| Tauri Event 名 | `snake_case` | `state_changed`、`config_changed` |
| 分支名 | `<type>/短描述` | `feat/config-service` |

文件命名实例：`src-tauri/src/services/timer/{state,machine,effect,effect_executor,service}.rs` 按职责切分，单文件聚焦单一概念。

## 错误处理

- MUST NOT 使用 `unwrap()`、`expect()`（仅测试代码可用）
- I/O、service 边界、command 层 MUST 返回 `crate::error::Result<T>`，使用 `?` 传播
- 纯函数（如 `resolve_user_event`、`step_time`，见 `src-tauri/src/services/timer/machine.rs`）按语义返回 `Option<Transition>`，MUST NOT 强制 `Result`
- 错误类型 MUST 实现 `Serialize`（IPC 需要序列化到前端，见 `AppError` 的 `#[serde(tag = "kind", content = "detail")]`）

错误传播分层与日志策略详见 [`error-and-logging.md`](./error-and-logging.md)。

## 锁策略（reducer 模式）

锁内只做状态读写，副作用一律在锁外执行：

```rust
// 取自 src-tauri/src/services/timer/service.rs::handle_user_event
let effects = {
    let mut inner = self.inner.lock().await;
    if let Some(transition) = resolve_user_event(&inner.state, event, inner.paused_from) {
        let now = Instant::now();
        inner.apply_transition(transition);
        collect_effects(transition, &inner, now)
    } else {
        return Err(AppError::InvalidOperation { /* ... */ });
    }
}; // 锁在此释放

let app = self.app.lock().await.clone();
for effect in &effects {
    effect_executor::execute_effect(app.as_ref(), effect);
}
```

规则：

- MUST 锁内收集 effects，锁外执行副作用
- 锁持有时间 MUST 尽可能短（微秒级）
- MUST NOT 在持锁期间做 I/O、网络、channel 发送、`emit()`
- MUST NOT 嵌套锁（避免死锁）
- 同步锁中毒（`Mutex<T>` poisoned）MUST 显式处理而非 `unwrap`，参考 `ConfigService::emit_config_changed` 对 poisoned 锁记录 `warn!` 后早返

## 异步

- MUST NOT 在 async 上下文中做长时间同步 I/O；阻塞操作 SHOULD 用 `tokio::task::spawn_blocking` 或独立线程（如 `SoundService` 用 `std::thread` 跑 rodio）
- 后台 task MUST 通过显式 `JoinHandle::abort` 或 channel 信号支持取消（见 `TimerService::shutdown` 中 `handle.abort()`）
- 精确周期性任务 MUST 使用 `tokio::time::interval(...)`，MUST NOT 用 `tokio::time::sleep` 串联（漂移）
- 现有 timer loop 模板见 `src-tauri/src/services/context.rs::spawn_timer_loop`

## 跨 Rust / TS 类型边界

- 跨界 DTO 字段（前端要消费）SHOULD 使用 `u32` 替代 `u64`，避免 ts-rs 生成 `bigint`。`src-tauri/src/models/config.rs` 中 `work_minutes: u32` 即为此设计
- 纯后端字段（时间戳、SQLite 主键、平台 API 句柄）MAY 使用 `u64` / `i64`
- 所有前端需要的类型 MUST 通过 ts-rs 导出，约定模式：

  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
  #[cfg_attr(test, derive(ts_rs::TS))]
  #[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
  pub struct Config { /* ... */ }
  ```

- MUST NOT 在前端手写与后端重复的类型；以 ts-rs 生成为单一事实来源
- ts-rs 是 dev-dependency，`cfg_attr(test, ...)` 模式确保生产构建不引入

## Lint

`src-tauri/src/lib.rs` 顶部已配置：

```rust
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![cfg_attr(test, allow(unused_imports, dead_code, clippy::unused_self))]
```

- 全 crate 生效，MUST 保留
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` 产出的 warning MUST 全部修复后才能合入
- `--all-targets` MUST 始终带上，否则 test target 警告会漏报（CI 已踩过坑）
- 个别字段可用 `#[allow(clippy::module_name_repetitions)]` 等局部豁免，MUST 写清理由或限定到最小作用域

## 注释规范

- MUST 只描述意图、约束、设计理由
- MUST NOT 复述代码逻辑
- MUST NOT 写"修改记录"（属于 git log）
- 非显而易见的依赖关系或行为 SHOULD 添加简短设计理由
- 保持极度简洁；可参考 `src-tauri/src/services/context.rs` 中关于关闭顺序的内联注释

## 通用规范

- 文件编码：UTF-8（无 BOM）
- 行尾：LF（`\n`）
- 提交：Conventional Commits（`feat:`、`fix:`、`docs:` 等）
- 分支：`feat/xxx`、`fix/xxx`、`refactor/xxx`、`docs/xxx`

## 自检命令

写完代码后，MUST 在提交前本地运行：

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml
```

三条全部通过才能进入下一步前端检查。Pre-push hook 会强制再跑一遍。
