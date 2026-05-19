# 错误与日志

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

本文档约束后端错误类型设计、跨层传播链以及 `tracing` 日志使用方式。代码锚点：

- `src-tauri/src/error.rs`
- `src-tauri/src/logging.rs`
- `src-tauri/src/services/config.rs`（典型 IO/解析错误转换）
- `src-tauri/src/services/timer/service.rs`（典型业务错误返回）

## `AppError` 类型

实际定义（`src-tauri/src/error.rs`）：

```rust
pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    ConfigInvalid { field: String, reason: String },
    InvalidOperation { operation: String, reason: String },
    IoError { message: String },
}
```

规则：

- 错误变体 MUST 用结构化字段（`field` / `reason` / `operation`），MUST NOT 把信息塞到单一 `String`
- `#[serde(tag = "kind", content = "detail")]` 形态 MUST 保留，前端依此 discriminate
- 错误 MUST 实现 `Serialize`、`Display`、`std::error::Error`（当前已具备）
- 新增变体 MUST 同步：单元测试（参考 `src-tauri/src/error.rs` 中 `serialize_*` 测试）+ 前端类型映射（`src/lib/` 错误处理）

### `From` 实现

已有：

- `From<std::io::Error> for AppError` → `IoError`
- `From<toml::de::Error> for AppError` → `ConfigInvalid { field: "toml", reason }`

新外部错误源 MUST 通过 `From` 集中转换，MUST NOT 在每个调用点散落 `map_err(|e| AppError::IoError { ... })`。

## 错误传播链

```
platform/ → 平台原生错误 → service/ → AppError → command/ → IPC Result
```

| 层 | 错误类型 | 职责 |
|----|---------|------|
| `platform/` | 平台原生（`windows::core::Error` 等） | 在层内捕获并降级，对外返回 `bool` / `Option`，MUST NOT 把平台错误穿透到 service |
| `services/` | 内部用 `Result<T, AppError>`，外部错误经 `From` 转换 | 添加业务上下文（哪个 service、哪个操作） |
| `commands/` | 透传 `AppError` | 不做转换、不再加包装，`?` 直返 |
| 前端 | `AppError` JSON | 根据 `kind` 字段 discriminate，展示对应文案 |

规则：

- 每层 MUST 只转换一次错误，MUST NOT 重复包装（避免 `ConfigInvalid { reason: "IoError { message: ... }" }` 嵌套）
- 日志 MUST 在错误"发生层"记录，上层 MUST NOT 重复记录同一错误（避免日志噪声）
- `platform/` 降级日志使用 `warn!` 并加 `AtomicBool` 哨兵去重（详见 [`platform-storage.md`](./platform-storage.md)）

## 各层职责示例

`ConfigService` 范式（`src-tauri/src/services/config.rs`）：

```rust
// 1. I/O 错误自动 From 转换
let content = std::fs::read_to_string(path)?;

// 2. TOML 解析失败：写 .bak 并 warn，但仍返回默认 Config（非 Err）
warn!("invalid config at {}: {err}", path.display());

// 3. 显式构造 AppError 携带业务上下文
return Err(AppError::IoError {
    message: "config write lock poisoned".to_string(),
});
```

`TimerService` 范式（`src-tauri/src/services/timer/service.rs`）：

```rust
warn!("invalid timer event {event:?} in state {:?}", inner.state);
return Err(AppError::InvalidOperation {
    operation: format!("timer event {event:?}"),
    reason: format!("invalid in {:?}", inner.state),
});
```

观察：log 与 return 在同一处，MUST NOT 在 caller 那边重复 log。

## 日志：tracing 配置

实际初始化（`src-tauri/src/logging.rs`）：

```rust
let file_appender = rolling::daily(log_dir, "eyezen.log");
let env_filter =
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,eyezen=info"));

let subscriber = tracing_subscriber::registry()
    .with(env_filter)
    .with(fmt::layer().with_target(true))                 // 控制台带 ANSI
    .with(fmt::layer().with_ansi(false).with_writer(file_appender)); // 文件无 ANSI
```

规则：

- MUST 使用 `tracing` + `tracing-subscriber` + `tracing-appender`，MUST NOT 引入 `log` / `env_logger` 等替代方案
- 日志目录：`app_data_dir/eyezen/logs/eyezen.log`，每日轮转
- 默认 filter：第三方 crate `warn` 以上，本 crate `info` 以上；通过环境变量 `RUST_LOG` 可覆盖
- 全局只能初始化一次（`OnceLock<()>` 保证），MUST NOT 在 service 内重复调用

## 日志级别用法

| 级别 | 用途 | 示例 |
|------|------|------|
| `error!` | 不可恢复错误，需要用户关注 | 配置文件无法写入、数据库连接失败 |
| `warn!` | 可恢复异常 / 平台降级 | "windows fullscreen detection failed: ..."、"failed to back up invalid config" |
| `info!` | 关键业务事件 | "Eyezen starting up"、"config loaded from ..."、"timer service tick loop started" |
| `debug!` | 开发调试细节 | 当前配置值、channel 收到的消息内容 |
| `trace!` | 高频细节 | 每次 tick 的 elapsed、光标位置（P2） |

规则：

- 关键业务事件（服务启动 / 关闭、状态机转换、配置变更）MUST 用 `info!`
- 平台能力降级 MUST 用 `warn!` 且每个能力只 warn 一次（用 `AtomicBool` 哨兵）
- `error!` MUST 仅在确实需要用户介入时使用，普通可恢复错误用 `warn!`
- `debug!` / `trace!` MUST NOT 出现在主流程热路径（每秒 tick 内）的默认日志里

## 敏感信息脱敏

- 路径中的用户名（`C:\Users\xxx\...` / `/home/xxx/...`）SHOULD 在日志里保留，因调试需要；MUST NOT 上传到外部服务
- 配置内容包含密码 / token 时 MUST 脱敏（当前 schema 暂无此类字段）
- 日志 MUST NOT 包含完整剪贴板内容、键盘输入、网络请求 body
- 任何上报 / 崩溃日志（未来若引入）MUST 在收集前过滤上述类目

## 错误测试要求

- 每个 `AppError` 变体 MUST 有 `Serialize` 测试，验证 `kind` 与 `detail` 字段命名（参考 `src-tauri/src/error.rs` 中 `serialize_config_invalid`、`serialize_invalid_operation`、`serialize_io_error`）
- 每个 service 在错误路径上 MUST 有至少一个测试，验证返回的 `AppError` 变体与字段（如 `timer::service::tests::invalid_event_returns_error`）
- 新增 `From<外部错误>` MUST 附 `from_*` 测试，验证转换后的变体正确（如 `from_io_error`）
