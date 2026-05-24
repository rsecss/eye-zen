# v0.7.x Hardening Epic

> 状态：planning（draft v1，brainstorm 收敛完成）
> 来源：v0.7.0 epic 遗留 candidates + 本会话截图实测发现的 tray-panel UX bug
> 范围：10 项 finding 单 PR + 按任务粒度多 commit；最后 `/trellis:finish-work` 收尾再开 PR

## 一、Goal

清掉 v0.7.0 epic 划到 v0.7.x 的全部 P2/P3 finding（除"覆盖率 95%"、"tip-window mini"、"F15 stat fetch 优化"三项），加上本次实测发现的 tray-panel 失焦不隐藏 UX bug。一次性把 v0.7 系列的 hardening 收尾，为后续选择"Phase 3 续集 vs v1.0.0 API 冻结"留下干净基线。

## 二、Scope（10 项纳入）

按 commit 顺序排列：

| # | Finding | 简述 | 类型 |
|---|---------|------|------|
| 1 | F17 | `stat.rs` (1439 行) 拆 6 mod | refactor |
| 2 | F18 | `SettingsPage` 拆 7 子组件 + `StatisticsPage` 拆 5 子组件 | refactor |
| 3 | F19 | i18n canonical 统一为 `en` | refactor |
| 4 | F20 | IPC event 名常量化 | refactor |
| 5 | F16 | IPC timeout 三档分级 | perf |
| 6 | F21/F22 | capability 收紧（删 dead permission + 限制 allow-emit） | chore |
| 7 | F25 | 文档漂移残留扫描 | docs |
| 8 | F29 | 跨平台路径"已知限制"文档化 | docs |
| 9 | F03+F28 | macOS fullscreen 真实现（CGWindowList + CGDisplayBounds） | fix |
| 10 | NEW | tray-panel 失焦自动隐藏 | fix |

## 三、Out of Scope（明确剔除）

- **覆盖率推至 95%** — 当前 90/85 边际效益递减
- **tip-window mini/角落通知模式** — 属于 Phase 3 UX 增强
- **F15 stat fetch 全表扫描优化** — 数据量评估 3 万行级别毫秒返回，非真实痛点，推迟到有用户反馈
- **F06/F07 API 重命名 + Beta 移除** — 留给 v1.0.0 SemVer 冻结一次性操作
- **新功能开发** — epic 期间冻结所有 feature

## 四、Brainstorm Decisions（已决议）

| # | Decision | 选择 |
|---|----------|------|
| D1 | Commit 顺序 | 大重构先（F17→F18→其余按独立性→末尾 tray-panel UX） |
| D2 | F17 stat.rs 拆分 | 6 mod 按职责（mod/writer/migration/export/trends/health） |
| D3 | F18 Settings/Statistics 拆分粒度 | 细：按功能块拆出子组件 |
| D4 | F19 canonical | `en` (短形式) |
| D5 | F20 常量化范围 | 只做 IPC events，tray menu id 不动 |
| D6 | F16 timeout 分档 | 3 档 (default 5s / io 10s / export 60s) |
| D7 | F15 处理 | 移出本 epic |
| D8 | F21/F22 范围 | 双修复 |
| D9 | F29 处理 | 文档化为"已知限制" |
| D10 | macOS fullscreen API | CGWindowListCopyWindowInfo + CGDisplayBounds（research 推荐） |
| D11 | tray-panel 失焦机制 | 仅 blur 隐藏（`Focused(false) → hide()`） |
| - | PR 策略 | 单 PR + 按任务粒度多 commit；完工后 `/trellis:finish-work` → PR |
| - | macOS 验证 | 完整代码 + 仅 CI mac runner 跑测试，本地不验 |

## 五、Requirements（按 commit 顺序）

### Commit 1: refactor(stat): split stat.rs into 6 modules (F17)

**Files**:
- 新建 `src-tauri/src/services/stat/` 目录
- `mod.rs` — `StatService` + `impl Service` + writer cmd（公开 API 入口）
- `writer.rs` — `run_writer_loop` + `persist_*` + `locked_pool` + `emit_persistence_error` + `map_send_error`
- `migration.rs` — `migrate` + `migrate_initial_to_v1` + `migrate_v1_to_v2`
- `export.rs` — `validate_export_path` + `resolve_timezone`
- `trends.rs` — `aggregate_sessions` + `add_bucket` + `into_buckets` + `parse_outcome` + `parse_reason`
- `health.rs` — `today_counts` + `adherence_rate` + `ribbon_entries` + `compute_eye_care_index` + `compute_rhythm` + `is_rest_day_today` + `longest_work_secs_today` + `median`
- 删除 `src-tauri/src/services/stat.rs`
- 更新 `src-tauri/src/services/mod.rs` 引用

**AC**:
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 全绿
- [ ] `cargo llvm-cov --fail-under-lines 90 --fail-under-functions 85` 通过
- [ ] 单文件 ≤ 400 行（mod.rs 最大）
- [ ] 旧 `stat.rs` 完全删除（不留转发 shim）

### Commit 2: refactor(ui): split SettingsPage and StatisticsPage into sub-components (F18)

**Files (Settings)**:
- 新建 `src/pages/main/settings/` 目录
- `TimerSection.svelte` — Timer 时长 + 模式（Pomodoro 切换）+ 短长休配置
- `BehaviorSection.svelte` — AFK + fullscreen skip + 提示音
- `DisplaySection.svelte` — theme + language
- `ScheduleSection.svelte` — workday schedule + weekday toggle
- `WhitelistSection.svelte` — process whitelist add/remove
- `HotkeySection.svelte` — global hotkeys + 状态显示
- `AutoStartSection.svelte` — autostart toggle + 错误提示
- `src/pages/main/SettingsPage.svelte` 改为 orchestrator（~150 行）

**Files (Statistics)**:
- 新建 `src/pages/main/statistics/` 目录
- `TrendChart.svelte` — ECharts wrap + lifecycle + renderChart
- `EciDisplay.svelte` — ECI ribbon + threshold colors + accent
- `RangeSwitcher.svelte` — daily/weekly/monthly tabs
- `ExportControls.svelte` — export 按钮 + 成功/错误提示
- `SuppressedDetails.svelte` — suppressed reasons expansion
- `src/pages/main/StatisticsPage.svelte` 改为 orchestrator（~200 行）

**AC**:
- [ ] `npm test` 全绿（含现有 SettingsPage / StatisticsPage 测试，必要时改测试 target 到子组件）
- [ ] `npx svelte-check` 全绿
- [ ] `vitest --coverage` 行 90% / 函数 85% 不下降
- [ ] 单文件 ≤ 300 行

### Commit 3: refactor(i18n): unify locale canonical to `en` (F19)

**Files**:
- `.trellis/spec/architecture/ipc-and-state.md` — `language` 取值由 `zh-CN` / `en` / `en-US` 改为 `zh-CN` / `en`
- `src/pages/tray/__tests__/TrayApp.test.ts` — `language: 'en-US' as const` → `'en' as const`
- `src/lib/stores/config.svelte.ts` 或 config 反序列化处 — 加 `en-US → en` 别名映射（向后兼容旧 config 1 行代码）
- 全代码扫一遍 `"en-US"` / `'en-US'` 字面值，UI/测试/工具中改为 `"en"`
- 不动 MSI bundle 命名（Tauri 自动产物）

**AC**:
- [ ] 全代码搜 `"en-US"` / `'en-US'` 仅剩 MSI 命名 + 注释
- [ ] 旧 config.toml 含 `language = "en-US"` 仍能加载（别名映射生效）
- [ ] `npm test` + `cargo test` 全绿

### Commit 4: refactor(ipc): extract IPC event names as shared constants (F20)

**Files**:
- 新建 `src/lib/events-constants.ts`（或同等位置）export 6 个事件名 const
- 新建 `src-tauri/src/events/names.rs`（如不存在）export 同样 6 个 `const &str`
- 改 `src/lib/events.ts` 使用 const 替换裸字符串
- 改 Rust 端 emit 处（`commands/mod.rs` + 各 service）使用 const
- 改 `capabilities/main-window.json` + `tray-panel.json` 中 listen allowlist 注释引用常量来源
- ts-rs 类型导出 *如果* 可行：通过 `#[ts(export)]` const 或单独维护双端 enum

**AC**:
- [ ] events.ts + Rust 端 emit 处不再有裸字符串 IPC event 名
- [ ] `npm test` + `cargo test` 全绿
- [ ] Tray menu id（"pause"/"settings"/"about"/"quit"）保持裸字符串不动

### Commit 5: perf(ipc): graded timeout for IPC calls (F16)

**Files**:
- `src/lib/commands.ts`:
  - `INVOKE_TIMEOUT_DEFAULT_MS = 5000`
  - `INVOKE_TIMEOUT_IO_MS = 10000`
  - `INVOKE_TIMEOUT_EXPORT_MS = 60000`
  - `invokeWithTimeout(cmd, args, timeoutMs?)` 支持可选 timeout 参数（默认 5s）
  - `exportStatistics` 显式传 60s
  - `getStatisticsTrends` / `statisticsCycleOutcomes` / `getDetectorCapabilities` / `saveHotkeysConfig` 显式传 10s

**AC**:
- [ ] `export_statistics` 30s+ 不再前端超时报错
- [ ] 现有 IPC 命令 timeout 行为不变（默认仍 5s）
- [ ] `npm test` 全绿

### Commit 6: chore(capabilities): tighten dead and unscoped permissions (F21+F22)

**Files**:
- `src-tauri/capabilities/main-window.json` — 删除 `shell:default`
- `src-tauri/capabilities/tray-panel.json` — `core:event:allow-emit` 改为 `{identifier: "core:event:allow-emit", allow: [{event: "navigate_tab"}]}`

**AC**:
- [ ] `npm run tauri dev` 启动正常
- [ ] 点击托盘 Settings 仍能 emit `navigate_tab`
- [ ] 任何 shell 操作（如有）正常或确认确实未用

### Commit 7: docs: sync drift in CLAUDE.md / index.json / memory (F25)

**Files**:
- `.claude/index.json` — 扫一遍版本号、services 数量、pages 列表对齐 v0.7.0 实际
- `CLAUDE.md` — "下一步" 段落更新（移除已完成的 v0.7.x candidates，保留剩余）
- `memory/*.md` — `MEMORY.md` 索引 + project state 中过期项扫一遍

**AC**:
- [ ] `.claude/index.json` 中 `files.root`、`services`、`pages` 与实际 repo 一致
- [ ] CLAUDE.md "下一步" 反映本 epic 完工状态
- [ ] memory project state 中 v0.7.x candidates 标记更新

### Commit 8: docs(platform): document known limitations for process path handling (F29)

**Files**:
- `.trellis/spec/backend/platform-storage.md` 新增 "Known limitations" 节：
  - Windows: process whitelist 不支持 `MAX_PATH` (260 字符) 以外的长路径
  - Linux: `/proc/{pid}/exe` 中非 UTF-8 basename 处理为 lossy 字符串
  - macOS: kCGWindowOwnerName fallback 同样按 lossy 处理

**AC**:
- [ ] spec 包含 "Known limitations" 节明确三平台 caveats
- [ ] CHANGELOG 提及（"docs: ..."）

### Commit 9: fix(platform): implement macOS fullscreen detection via CGWindowList (F03+F28)

**Files**:
- `src-tauri/src/platform/macos.rs:94-107` — 真实现：
  - `CGGetActiveDisplayList` → vec of display rects
  - `CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID)`
  - 遍历 layer==0 窗口，bounds 与任一 display rect 比较（±1px 容差）
  - 抽 `compare_window_bounds_to_displays(window: CGRect, displays: &[CGRect]) -> bool` 纯 helper（跨平台跑单测）
- `src-tauri/src/platform/macos.rs:74` — `supports_fullscreen_detection()` 改为 `true`
- 单元测试：在 `#[cfg(test)]` 下跨平台跑 `compare_window_bounds_to_displays` 的 hand-crafted rect 覆盖（exact match / sub-pixel offset / multi-monitor / off-bounds）
- 失败兜底：`CGGetActiveDisplayList` 返回 0 displays 或 error → `FullscreenStatus::DegradedFalse`

**AC**:
- [ ] CI macOS-latest runner `cargo test` 全绿（API 调用 Ok + 不 panic）
- [ ] 跨平台单元测试 `compare_window_bounds_to_displays` 全绿
- [ ] `is_fullscreen_app_active()` 在 CI mac runner 返回 `Ok(_)`（具体 false 因无实际全屏窗口）
- [ ] PR 描述显式标注："CI cannot exercise real fullscreen; integration test verifies API binds cleanly"

### Commit 10: fix(tray): auto-hide tray-panel on focus loss (NEW)

**Files**:
- `src-tauri/src/services/tray.rs` — tray-panel 创建/获取处加 `on_window_event` listener：
  - `WindowEvent::Focused(false) → panel.hide()`
- 通过 `WindowPort` trait 抽象（v0.7.0 PR #31/#32 已建立的 Port pattern）

**AC**:
- [ ] 点击托盘 → 面板出现 + 自动 focused
- [ ] 点击桌面/其他窗口 → 面板自动隐藏
- [ ] 再次点击托盘 → 面板再次出现（show/hide 循环正常）
- [ ] `cargo test` 含 tray 测试全绿（通过 WindowPort fake）

## 六、Acceptance Criteria（整体）

- [ ] 所有 10 个 commit 各自通过自己的 AC
- [ ] `npm run ci` 8 步本地全绿（fmt + clippy + cargo test + svelte-check + vitest + prettier + cargo fmt check + version sync）
- [ ] 覆盖率门禁未降低：前端 ≥90% lines / ≥85% functions；后端 ≥90% lines / ≥85% functions
- [ ] CHANGELOG 加 `## [0.7.1]` 段，按 emoji 分类列 commit
- [ ] PR CI 三平台全绿（Windows + Linux + macOS）
- [ ] macOS runner 的 fullscreen 测试通过（API 调用 Ok）

## 七、Definition of Done

- 全部 10 commit 跑通本地 + 远端 CI
- `/trellis:finish-work` 已跑（task archive + journal 记录）
- PR 已开 + auto-merge 配置好
- Auto-merge 后 main 状态更新到 `.claude/index.json` 与 CLAUDE.md
- 准备 v0.7.1 release（可选，最后另起 PR）

## 八、Out of Scope (复述)

- 覆盖率推至 95%
- tip-window mini/角落通知
- F15 stat fetch 全表扫描优化
- F06/F07 API 重命名 + Beta 移除（留 v1.0.0）
- 任何 feature 开发

## 九、Technical Notes

### 来源文档

- `docs/.local/v1.0.0-audit-report.md` — Findings F01-F29 完整描述
- `.trellis/tasks/archive/2026-05/05-23-v0-7-0-hardening-release-epic/prd.md` — v0.7.0 epic + v0.7.x follow-up
- `CLAUDE.md` 项目状态末尾 "v0.7.x candidates" 清单

### 实测发现

- 2026-05-24 本会话用户启动 `npm run tauri dev`，截图反馈 tray-panel "已暂停" 面板 always-on-top 驻留挡内容 → candidate 10

### Spec 引用

- `.trellis/spec/backend/service-pattern.md` — Commit 1 F17 拆分参考
- `.trellis/spec/frontend/component-guidelines.md` — Commit 2 F18 拆分参考
- `.trellis/spec/architecture/ipc-and-state.md` — Commit 3/4/5 IPC/事件改动准则
- `.trellis/spec/backend/platform-storage.md` — Commit 8 F29 跨平台路径准则
- `.trellis/spec/architecture/testing-quality.md` — 覆盖率门禁与测试质量
- 全局 `CLAUDE.md` "Writing code — minimum sufficient" + "Stop signals" — 反兜底/反过度抽象

## 十、Research References

- [`research/macos-fullscreen-apis.md`](research/macos-fullscreen-apis.md) — Option 1 CGWindowListCopyWindowInfo + CGDisplayBounds 推荐，其他 4 选项否决理由
