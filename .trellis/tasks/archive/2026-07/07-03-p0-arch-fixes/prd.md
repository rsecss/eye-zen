# 修复 P0 架构缺陷：Service 依赖顺序对齐 + models 拆分

## Goal

修复 2026-07-03 深度架构审查识别的两个 P0 缺陷，确保代码与文档完全对齐，为 v1.0.0 SemVer 承诺扫清障碍。

## Requirements

### 1. Service 初始化顺序修正

**问题**：`lib.rs:169-178` 的 `init()` 顺序为 `detector` → `timer`，但 `service-pattern.md` DAG 显示 detector 依赖 timer 的 `current_skip_flags()` 拉取，顺序颠倒可能在未来引入初始化竞态。

**需求**：
- 调整 `lib.rs` 的 `init()` 顺序：config → i18n → sound → stat → **timer** → **detector** → window → tray → hotkeys
- 在 `service-pattern.md` 启动顺序章节显式记录：`init` 阶段的依赖可能与 `start` 阶段不同，detector 的 `init()` MUST 在 timer 之后
- 保持 `start()` 顺序不变（已正确：effect 执行器先启动，timer 最后启动）

### 2. `models/` 目录拆分兑现

**问题**：`layering.md:28-41` 承诺 `models/events.rs`（IPC event payload）和 `models/timer.rs`（timer DTO），但实际 `types.rs` 混合了所有类型，`events/mod.rs` 仅有字符串常量。

**需求**：
- 创建 `src-tauri/src/models/events.rs`：
  - 定义 `StateChangedPayload` / `PomodoroStatePayload`（从 `types.rs` 迁移）
  - Re-export `Config`（ConfigChangedPayload）/ `HotkeyStatus`（HotkeyStatusChangedPayload）/ `StatPersistenceErrorPayload`
  - 所有类型 MUST 带 `#[cfg_attr(test, derive(ts_rs::TS))]`
- 重命名 `src-tauri/src/models/types.rs` → `timer.rs`：
  - 保留 `pub type StatePayload = StateChangedPayload;`（向后兼容 alias）
  - 保留 `DetectorCapabilities`（command 返回类型，属于 timer/detector 交互 DTO）
- 更新 `models/mod.rs`：`pub mod events;` + `pub mod timer;`
- 更新全局引用：搜索 `models::types::`，替换为 `models::timer::` / `models::events::`
- 更新 `layering.md`：标记 `events.rs` / `timer.rs` 为"已实现"，删除"待新增"

## Acceptance Criteria

- [ ] `lib.rs` 的 `init()` 顺序调整为：config → i18n → sound → stat → timer → detector → window → tray → hotkeys
- [ ] `service-pattern.md` 新增"启动顺序"章节或在现有"启动顺序"章节补充 init/start 差异说明
- [ ] `models/events.rs` 创建完成，包含 `StateChangedPayload` / `PomodoroStatePayload` + re-export
- [ ] `models/types.rs` 重命名为 `timer.rs`，包含 `StatePayload` alias + `DetectorCapabilities`
- [ ] `models/mod.rs` 导出 `events` / `timer` 模块
- [ ] 全局引用从 `models::types::` 更新完毕
- [ ] `layering.md` 标记 `events.rs` / `timer.rs` 为"已实现"
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 全绿
- [ ] `npm run ci` 全绿
- [ ] `src/lib/bindings/` 包含 `StateChangedPayload.ts` / `PomodoroStatePayload.ts` / `DetectorCapabilities.ts`

## Constraints

- 不调整 `start()` 顺序（已正确）
- 不调整 shutdown 顺序（已正确）
- 不实现 `models/channels.rs`（当前无需求）
- 类型迁移 MUST 保持向后兼容（通过 `pub use` / `type` alias）

## Risks

- **风险 1**：调整 init 顺序可能触发启动期竞态
  - **缓解**：当前 detector/timer 的 `init()` 均仅缓存 `ServiceContext`，无跨服务调用
- **风险 2**：重命名 `types.rs` 可能遗漏引用
  - **缓解**：全局搜索 `models::types` + IDE 重构工具批量替换
