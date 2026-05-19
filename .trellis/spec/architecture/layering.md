# 分层与可见性

> 跨层依赖图、`models/` 拆分、`pub` vs `pub(crate)` 边界、`mod.rs` 重导出规则。
> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选。

---

## 分层依赖（单向，禁止反向）

```
commands/  →  services/  →  platform/
    ↓            ↓
  models/     models/
```

- `commands/`（`src-tauri/src/commands/mod.rs`）MUST 只依赖 `services/` 与 `models/`，MUST NOT 直接调用 `platform/`。
- `services/`（`src-tauri/src/services/`）MUST 只依赖 `models/`、`platform/`、其他 `services/`（按 DAG）。
- `platform/`（`src-tauri/src/platform/{windows,macos,linux}.rs`）MUST NOT 依赖 `services/` 或 `commands/`。
- `models/`（`src-tauri/src/models/`）MUST NOT 依赖任何上层模块，纯数据定义 + 序列化。
- 前端（`src/`）MUST NOT 直接访问 SQLite、文件系统或系统 API，**唯一通道是 IPC**（见 [`ipc-and-state.md`](./ipc-and-state.md)）。

> 服务之间的依赖 DAG、四阶段生命周期、关闭顺序、锁策略由 [`../backend/service-pattern.md`](../backend/service-pattern.md) 负责，本文件不重复。

---

## `models/` 拆分

`models/` 统一存放跨层数据类型，按用途分文件。当前实现仅有 `config.rs` 和 `types.rs`（见 `src-tauri/src/models/mod.rs`）；后续新增类型 MUST 按下表归位：

| 文件 | 内容 | 可见性 | ts-rs |
|------|------|--------|-------|
| `config.rs` | `Config`、`TimerConfig`、`BehaviorConfig`、`DisplayConfig` | `pub` | `#[cfg_attr(test, derive(ts_rs::TS))]` |
| `timer.rs`（待新增）| 跨服务共享的 timer DTO（`StatePayload` 已在 `types.rs`，后续迁入） | `pub` | 同上 |
| `error.rs`（已实现于 `src-tauri/src/error.rs`，未来 MAY 迁入）| `AppError`、`Result<T>` | `pub` | 不导出（前端通过 invoke 错误捕获） |
| `events.rs`（待新增，当前位于 `src-tauri/src/events/mod.rs`）| IPC event 名称常量 + payload 类型 | `pub` + `#[derive(TS)]` | **必须**导出 |
| `channels.rs`（待新增）| 服务间内部 channel 消息类型 | `pub(crate)` | **不导出** |

- `models/events.rs` 中的类型 MUST 为 `pub` 且 `#[derive(TS)]`，前端通过 `src/lib/bindings/` 消费。
- `models/channels.rs` 中的类型 MUST 为 `pub(crate)`，**禁止**在前端导出。
- 两类 MUST NOT 混放，避免内部消息意外暴露到前端。
- 当前 `events/mod.rs` 仅有事件名常量字符串；新增 event payload 时 MUST 同步迁移到 `models/events.rs` + ts-rs 导出。

### ts-rs 导出模式

`config.rs` 与 `types.rs` 已采用 dev-dependency 模式：

```rust
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export, export_to = "../../src/lib/bindings/"))]
pub struct TimerConfig { ... }
```

- 所有跨 IPC 边界的类型 MUST 使用此模式，绑定文件由 `cargo test` 生成到 `src/lib/bindings/`。
- `src/lib/bindings/` 已加入 `.prettierignore`，禁止手编。

---

## `pub` vs `pub(crate)` 边界

| 对象 | 可见性 | 理由 |
|------|--------|------|
| Service struct（如 `ConfigService`、`TimerService`）| `pub(crate)` | 仅 `commands/`、`lib.rs` 需要持有 |
| Service 公开方法（供 command 调用） | `pub(crate)` | command 层是唯一调用方 |
| Service 内部方法（私有 helper） | `fn`（默认私有） | 实现细节，禁止外露 |
| 状态机纯函数（`step_time`、`collect_effects` 等） | `pub(crate)` | 单元测试可见，外部 crate 不可见 |
| `commands/` 中的 `#[tauri::command]` 函数 | `pub(crate)` | `generate_handler!` 宏注册即可，详见 `src-tauri/src/commands/mod.rs` |
| `models/` 中的 IPC 类型（events / 共享 DTO） | `pub` + `#[derive(TS)]` | ts-rs 跨 crate 导出需要 |
| `models/` 中的内部 channel 类型 | `pub(crate)` | 仅后端服务间使用 |
| `models/` 中的共享配置类型（`Config` 等） | `pub` | 可能被未来集成测试 crate 引用 |
| `platform/PlatformApi` trait + 默认实现 | `pub(crate)` | `services/` 是唯一消费者 |

实际代码示例：`src-tauri/src/services/timer/mod.rs` 全部使用 `pub(crate)` 重导出；`src-tauri/src/models/config.rs` 全部 `pub`。

---

## `mod.rs` 重导出规则

- `mod.rs` MUST 只重导出该模块的公开 API，MUST NOT 包含实现代码（实现写到子文件）。
  - 反例：把 100 行 struct 实现塞进 `services/timer/mod.rs`。
  - 正例：`src-tauri/src/services/timer/mod.rs` 仅做 `pub(crate) use` 重导出。
- MUST NOT 使用 `pub use foo::*` 通配重导出，必须显式列出每个名字。
  - 通配会让外部看到所有内部类型，破坏可见性边界。
- 跨子模块共享类型（如 `state::TimerState` 被 `effect.rs` 引用）SHOULD 通过 `super::state::TimerState` 引用，避免依赖 `mod.rs` 的重导出顺序。

---

## 前端与 Rust 的对应关系

| Rust 侧 | 前端侧 | 说明 |
|---------|--------|------|
| `src-tauri/src/commands/mod.rs` 的 `#[tauri::command]` | `src/lib/commands.ts` 的薄封装 | 一对一镜像；详见 [`ipc-and-state.md`](./ipc-and-state.md) |
| `src-tauri/src/events/mod.rs` 的事件名常量 | `src/lib/events.ts` 的 `listen<T>(...)` | 事件名字符串必须一致 |
| `src-tauri/src/models/*.rs` 中带 `#[derive(TS)]` 的类型 | `src/lib/bindings/*.ts`（自动生成） | 仅 `cargo test` 触发更新 |

跨边界类型变更 MUST 同时更新两侧并跑 `cargo test` 刷新绑定，否则前端编译失败。
