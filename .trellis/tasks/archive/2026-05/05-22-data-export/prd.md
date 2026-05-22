# Data Export

## Goal

让用户能把自己积累的护眼数据"带走"——可以备份、跨设备迁移、给第三方工具分析，或者拿来证明给自己/医生看一段时间的休息执行情况。目前所有数据锁死在本地 SQLite 里，对用户不透明，也违背"数据归用户所有"的开源工具基本品德。

主用例锁定：**个人备份/迁移**（用户换机或重装时能保留全部历史休息数据）。

## Requirements

- 导出范围：`activity_segments` 全表（rest sessions），UTC 时间保留原样，无日期筛选（备份语义 = 全量）
- 导出格式：**SQLite 文件复制**，用 `VACUUM INTO` 原子写出
- 触发：StatisticsPage 右上角 "Export Backup" 按钮 → Tauri `dialog::save` 选保存路径
- 文件名默认：`eyezen-stat-YYYY-MM-DD.db`（dialog 中用户可改）
- 完成后 toast 反馈成功，失败时 toast 给出可读错误
- 用户取消 dialog → 静默关闭，无错误
- i18n zh-CN / en 双语补齐

## Acceptance Criteria

- [ ] StatisticsPage 右上角有 "Export Backup" 按钮
- [ ] 点按钮弹出系统 save dialog，默认文件名 `eyezen-stat-YYYY-MM-DD.db`
- [ ] 选定路径后产出一个有效 SQLite 文件
- [ ] 导出文件可被 `sqlite3` CLI / DB Browser for SQLite 打开
- [ ] 导出文件中 `activity_segments` 行数 = 源 DB 行数；`PRAGMA user_version` 一致
- [ ] 空 DB（无任何 session）能正常导出 schema-only db
- [ ] 用户取消 dialog 不报错
- [ ] 写盘失败（无权限/磁盘满/路径不存在）给出可读错误 toast
- [ ] 导出过程不阻塞 UI 主线程
- [ ] 导出期间如有新 session 写入不丢失（VACUUM INTO 原子语义）
- [ ] i18n zh-CN/en 全部就位

## Definition of Done

- Rust 单元/集成测试覆盖：基本导出、空 DB、覆盖已有路径、非法路径
- Vitest 覆盖：按钮交互、用户取消、错误展示
- `cargo fmt` / `cargo clippy --all-targets -D warnings` / `cargo test` 全绿
- `npx svelte-check` / `npm test` / `npm run build` 全绿
- 手动验证：跨平台至少在 Windows 上完整跑一次（触发 → 选路径 → 文件生成 → sqlite3 打开）
- `.trellis/spec/architecture/change-management.md` 变更清单核对完（新增 command + 新增 plugin）
- 若需要在 CLAUDE.md "模块索引"或 `ipc-and-state.md` 添加新条目，同步更新

## Technical Approach

### 后端

- **StatService 新增** `export_to(target_path: PathBuf) -> Result<()>`：
  - 用一句 SQL `VACUUM INTO ?1`，无需手动 WAL checkpoint（VACUUM 自动落盘 + 重建去碎片化）
  - 目标路径若已存在先 `fs::remove_file`（VACUUM INTO 要求目标不存在）
  - 不新增 Service：导出是 stat 数据的新出口，紧贴 StatService 单一职责（避免 YAGNI 的 ExportService）
- **新增 Tauri command** `export_statistics(target_path: String) -> Result<()>`（`commands/mod.rs`）：薄层转发 → `app_state.stat.export_to(...)`
- 错误统一通过现有 `AppError` 序列化到前端

### 前端

- **新增依赖** `@tauri-apps/plugin-dialog` (npm) + `tauri-plugin-dialog ~2.x` (Cargo)
- **lib.rs**：`.plugin(tauri_plugin_dialog::init())`
- **main-window.json**：加 `dialog:allow-save` + `allow-export-statistics`
- **StatisticsPage.svelte** 右上角添加 "Export Backup" 次要按钮
- 流程：点击 → `save({ defaultPath, filters: [{ extensions: ['db'] }] })` → 拿到 path → `invoke('export_statistics', { targetPath })` → toast
- 用户取消时 `save()` 返回 `null` → 静默关闭

### i18n keys（zh-CN / en）

- `statistics.exportBackup.button` — "Export Backup" / "导出备份"
- `statistics.exportBackup.dialogTitle` — "Save Eyezen statistics" / "保存 Eyezen 统计数据"
- `statistics.exportBackup.defaultFilename` — `eyezen-stat-YYYY-MM-DD.db`（动态注入日期）
- `statistics.exportBackup.toastSuccess` — "Backup saved to {path}" / "已备份到 {path}"
- `statistics.exportBackup.toastError` — "Export failed: {reason}" / "导出失败：{reason}"

### 测试

- **Rust**：tempdir 调 `export_to` → 断言文件存在、`PRAGMA user_version` 一致、`activity_segments` 行数一致、可重新 open；空 DB 导出；目标路径已存在能覆盖；非法路径返回错误
- **Vitest**：mock invoke 验证按钮 → dialog → invoke 调用链；用户取消、错误显示

## Implementation Plan (small PRs)

- **PR1**：后端 `StatService::export_to` + command `export_statistics` + Rust 测试。无前端入口，可手动 invoke 验证。
- **PR2**：前端 `tauri-plugin-dialog` 接入（npm + Cargo + lib.rs + capability）+ StatisticsPage 按钮 + i18n + vitest。

## Decision (ADR-lite)

**Context**: 主用例是个人备份/迁移，需要决定导出格式。候选 SQLite 文件复制 / JSON 快照 / zip 双格式。

**Decision**: SQLite 文件复制，具体用 `VACUUM INTO` 而非 `fs::copy + WAL checkpoint`。

**Consequences**:
- 优：实现最简（一行 SQL）；schema 原样保留；未来"Data Import" task 直接 copy-over 即可，无需反序列化；文件最紧凑；VACUUM 自动处理 WAL/journal 状态；产物自动去碎片化
- 劣：人不可读，用户需用 DB Browser for SQLite / sqlite3 CLI 查看（本工具受众技术友好，可接受）
- 劣：schema 演化时需要 import 端做 SQLite migration，但 `PRAGMA user_version` 已经在 schema 里，天然支持版本识别
- 劣：VACUUM INTO 要求目标路径不存在 → 实现需先 `fs::remove_file`，对用户透明

## Out of Scope (explicit)

- **导入/恢复**：单独再做一个 task（"Data Import"）；本 task 仅产出能被未来 import 消费的格式
- **配置文件导出**：TOML 本身可读，用户自己复制；如确实需要，未来再加
- **自动定期备份**：MVP 仅手动触发
- **跨设备合并/去重**：用户假设新机器是干净环境
- **加密 / 密码保护**：MVP 不做（数据敏感度低，本地文件用户自己保管）
- **健康报告 / PDF**：已被主用例排除
- **last_export_at 记忆与展示**：MVP 不做，发版后收集反馈再决定
- **超期未备份提醒 banner**：MVP 不做

## Technical Notes

### 仓库现状（已确认）

- 唯一数据源：`src-tauri/src/services/stat.rs` 维护的 SQLite `activity_segments` 表
- Schema v1，字段：`id / state / started_at (RFC3339 UTC) / ended_at / duration_secs / date`
- 实际只写 `state = 'resting'`，其他 state 是预留位
- 数据库位置：app data dir，WAL 模式
- 已暴露 IPC 仅 `statistics_trends`（聚合后的 daily/weekly/monthly），原始 session 不出 IPC
- 前端 `src/pages/main/StatisticsPage.svelte` 用 ECharts 展示趋势
- `commands/mod.rs` 单文件承载所有 commands（新增直接加在此）
- `tauri-plugin-dialog` **未引入**，本 task 需新加 Cargo + npm + lib.rs init + capability 一整套
- 现有 capability 命名规范：`allow-<verb-noun>`（参考 `main-window.json`）

### 参考文档

- `.trellis/spec/architecture/ipc-and-state.md`（新增 command 契约）
- `.trellis/spec/architecture/change-management.md`（变更清单）
- `.trellis/spec/backend/service-pattern.md`（StatService 扩展）
- `.trellis/spec/frontend/store-and-ipc-patterns.md`（前端 invoke 包装）
