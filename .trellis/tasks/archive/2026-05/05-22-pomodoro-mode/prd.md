# Pomodoro Mode

## Goal

为 Eyezen 引入番茄工作法模式，让一个应用同时承担"眼健康提醒"和"专注/任务节奏管理"两种角色，减少用户在多 app 间切换的成本。

经典 Pomodoro 节奏：Focus 25min → Short break 5min，重复 4 次后 Long break 15-30min。

## Requirements

- **模式互斥**：Settings 加 "Timer Mode" 单选 — `20-20-20` (default) / `Pomodoro`，同一时刻只跑一种节奏，切换时 Timer 重置
- **状态机复用**：保持现有 5 个 state（Working / PreAlert / Alerting / Resting / Paused），不新增 state；Pomodoro 的 cycle counter 进 `Inner` 运行态
- **长短休通过 duration 表达**：`Inner` 加 `cycle_index`；进 Working 时根据 `cycle_index % cycles_per_long` 预先计算下次 Resting duration
- **Config 新增** `PomodoroConfig { focus_minutes, short_break_minutes, long_break_minutes, cycles_per_long }`，默认 `25 / 5 / 15 / 4`
- **TimerConfig 新增** `mode: TimerMode`（`TwentyTwentyTwenty` 默认 / `Pomodoro`）
- **StatePayload 扩字段**：`mode` + 可选 `pomodoro: { cycle_index, cycles_per_long, is_long_break }`
- **UI 适配**：
  - Settings 加 "Timer Mode" 单选 + Pomodoro 子卡片（4 个 Stepper）
  - tip-window 文案根据 mode + cycle 显示（短休/长休文案不同）
  - tray tooltip 番茄模式显示进度（如 "Working - 18:42 (Pomo 2/4)"）
- **Skip 行为完全复用**：现有 SkipFlags（fullscreen / schedule / afk / process_whitelist）在两种 mode 下行为一致
- **模式切换时正在 Resting**：立即重置到新 mode 的 Working
- **Pause/Resume**：现有机制天然兼容，cycle counter 保留
- **重启重置 cycle counter**（符合番茄"session = 开机连续"的哲学）
- **20-20-20 模式行为不变**（回归保护，所有现有测试必须仍然通过）
- i18n zh-CN/en 双语补齐

## Acceptance Criteria

- [ ] Settings 有 "Timer Mode" 单选，默认 `20-20-20`，切到 `Pomodoro` 出现 Pomodoro Card（4 个 Stepper）
- [ ] Pomodoro 模式下，默认 25min focus → 5min short break，连续 4 个 cycle 后第 5 次 break 是 15min long break
- [ ] StatePayload 的 `mode` 字段正确随设置切换
- [ ] StatePayload 番茄模式下 `pomodoro` 非空，含 `cycle_index` / `cycles_per_long` / `is_long_break`
- [ ] tip-window 文案根据 mode 自适应（番茄模式短休/长休文案不同）
- [ ] tray tooltip 番茄模式显示 "Pomo N/M"
- [ ] 模式切换会立即重置 Timer 到新 mode 的 Working 起点
- [ ] Pause → Resume 后 cycle counter 不变
- [ ] App 重启后 cycle counter 重置为 0
- [ ] 20-20-20 模式全部现有测试和行为不变（回归保护）
- [ ] i18n zh-CN/en 全部就位
- [ ] `cargo fmt` / `cargo clippy --all-targets -D warnings` / `cargo test` 全绿
- [ ] `npx svelte-check` / `npm test` / `npm run build` 全绿

## Definition of Done

- Rust 单元测试：cycle 推进、第 4 个 cycle 后长休、mode 切换重置、Pause/Resume cycle 保留、20-20-20 模式回归
- Vitest：Mode 单选切换、Pomodoro Stepper 双向绑定、tip-window 文案根据 mode 渲染
- 手动验证：把参数调成短测时长（focus 30s / short 10s / long 20s / cycles 4），跑一整轮观察短休 ×4 + 长休 ×1
- `cargo fmt` / `cargo clippy --all-targets -D warnings` / `cargo test` 全绿
- `npx svelte-check` / `npm test` / `npm run build` 全绿
- `.trellis/spec/architecture/change-management.md` 变更清单核对完（IPC payload + Config schema + i18n + 文档）
- `.trellis/spec/architecture/ipc-and-state.md` 更新状态机契约描述
- CLAUDE.md 模块索引同步（如新增 PomodoroConfig 子模块需提及）

## Technical Approach

### 后端 — 数据模型

```rust
// models/config.rs
#[derive(Serialize, Deserialize, ...)]
pub enum TimerMode {
    #[serde(rename = "twenty_twenty_twenty")]
    TwentyTwentyTwenty,
    #[serde(rename = "pomodoro")]
    Pomodoro,
}

pub struct TimerConfig {
    // existing fields ...
    #[serde(default)]
    pub mode: TimerMode,  // default = TwentyTwentyTwenty
}

pub struct PomodoroConfig {
    #[serde(default = "default_focus_minutes")]      // 25
    pub focus_minutes: u32,
    #[serde(default = "default_short_break_minutes")]// 5
    pub short_break_minutes: u32,
    #[serde(default = "default_long_break_minutes")] // 15
    pub long_break_minutes: u32,
    #[serde(default = "default_cycles_per_long")]    // 4
    pub cycles_per_long: u32,
}

pub struct Config {
    // existing ...
    pub pomodoro: PomodoroConfig,
}
```

### 后端 — StatePayload 扩展

```rust
// models/types.rs
pub struct StatePayload {
    pub state: String,
    pub remaining_secs: u32,
    pub work_minutes: u32,   // 番茄模式下 = focus_minutes
    pub rest_seconds: u32,   // 番茄模式下 = 当前 short/long_break 时长
    pub mode: String,        // "twenty_twenty_twenty" | "pomodoro"
    pub pomodoro: Option<PomodoroStatePayload>,
}

pub struct PomodoroStatePayload {
    pub cycle_index: u32,     // 1-based，当前 cycle (1..=cycles_per_long)
    pub cycles_per_long: u32,
    pub is_long_break: bool,  // true 仅在长休 Resting 期间
}
```

### 后端 — Timer 状态机

- `Inner` 新增字段：`mode: TimerMode`, `cycle_index: u32`（pomodoro 模式下使用），`short_break_duration`, `long_break_duration`, `cycles_per_long`
- 进 Working 时调一个新函数 `next_rest_duration()`：根据 `mode` 和 `cycle_index % cycles_per_long` 选 short/long/standard
- `Resting → Working` transition：`cycle_index += 1`（pomodoro 模式）
- `step_time` / `resolve_user_event` 不变（状态机纯函数语义保持）
- `collect_effects` 透传 cycle 信息到 StatePayload + tray tooltip

### 前端 — UI

- `SettingsPage.svelte` 加 "Timer Mode" Card 顶部：`<Select>` 切 mode
- 当 mode = Pomodoro，下方展开 Pomodoro Card（4 个 Stepper：focus / short / long / cycles）
- `TipApp.svelte` / `TipMinimalApp.svelte` 文案根据 `mode` + `pomodoro?.is_long_break` 自适应
- `TrayApp.svelte` tooltip 番茄模式追加 `(Pomo N/M)` 后缀

### 前端 — IPC

- 新增 command `update_pomodoro_config(config: PomodoroConfig) -> Result<()>`
- ts-rs 重新生成 bindings（TimerMode / PomodoroConfig / PomodoroStatePayload）
- main-window.json 加 `allow-update-pomodoro-config`

### i18n keys（zh-CN / en）

- `settings.timerMode.label` — "Timer Mode" / "计时模式"
- `settings.timerMode.option20_20_20` — "20-20-20" / "20-20-20 护眼"
- `settings.timerMode.optionPomodoro` — "Pomodoro" / "番茄工作法"
- `settings.pomodoro.title` — "Pomodoro Settings" / "番茄设置"
- `settings.pomodoro.focusMinutes` / `shortBreakMinutes` / `longBreakMinutes` / `cyclesPerLong`
- `tip.pomodoro.shortBreak` — "Take a 5-minute break ({current} of {total})" / "短休 5 分钟（{current}/{total}）"
- `tip.pomodoro.longBreak` — "Long break! Stand up, drink water, stretch" / "长休时间！起身、喝水、伸展"
- `tip.twentyTwentyTwenty` — "Look 20 feet away for 20 seconds" / "看 20 英尺外 20 秒"
- `tray.pomodoro.progress` — "(Pomo {current}/{total})" / "（Pomo {current}/{total}）"

### 测试

- **Rust unit (machine.rs)**：
  - pomodoro 模式：cycle_index 从 1 推进；第 N=cycles_per_long 个 cycle 完成后下次是 long break；is_long_break 标记正确
  - 20-20-20 模式：所有现有测试不动（回归保护）
  - mode 切换：Inner 重置到新 mode 的 Working 起点；cycle_index 归零
  - Pause/Resume：cycle_index 不变
- **Vitest**：
  - TimerMode 切换更新 store 和 UI
  - Pomodoro Stepper 双向绑定 PomodoroConfig
  - tip-window 文案根据 StatePayload.mode 和 pomodoro.is_long_break 渲染

## Implementation Plan (small PRs)

- **PR1**：Config 扩展 + 状态机骨架。`PomodoroConfig` / `TimerMode` / `StatePayload` 扩字段 + ConfigService 接受 + Inner 加 cycle 字段 + machine.rs 改动 + 全部 Rust 测试（含回归）。**前端无入口，行为对外仍是 20-20-20 模式默认**。
- **PR2**：前端 UI。Settings "Timer Mode" 单选 + Pomodoro Card + tip-window/tray 文案适配 + i18n + vitest。

## Decision (ADR-lite)

**Context**: 番茄模式与 20-20-20 有不同的休息语义（5min 离座 vs 20s 看远处），需要决定两者关系。候选互斥 / 叠加 / 独立 Session。

**Decision**: **互斥模式**（用户在 Settings 选一种）+ **状态机不新增 state**（只在 Inner 加 cycle counter，长短休通过 Resting 的动态 duration 表达）。

**Consequences**:
- 优：用户认知最简单（一个 timer 一种节奏）；状态机改动最小（5 个 state 不变 + 现有测试可作回归保护）；实现复杂度低；保留 Eyezen 的全屏 tip-window 产品识别度
- 优：未来若要"两套并行 timer"或"番茄是 20-20-20 的子集"，PomodoroConfig 已经独立，演化路径开放
- 劣：模式切换时强制重置 Timer，用户在 20-20-20 期间想临时跑一个番茄需要先切 mode，体验略硬
- 劣：番茄的 15min 长休复用全屏 tip-window 体验偏重，但用户可随时按 Skip 离开；若反馈强烈未来再考虑"长休改弱提示"

## Out of Scope (explicit)

- **任务管理 / Todo 列表**：番茄关联任务超出 Eyezen 范畴
- **Pomo 统计页**：MVP 不做（用户在仅核心选项选定），未来按反馈再决定
- **Cycle counter 持久化**：MVP 不做，重启重置（符合番茄哲学）
- **白噪音 / 专注音乐**：MVP 不做
- **社交分享 / 排行**：永远不做（与 Eyezen 工具定位冲突）
- **自定义循环结构**（如 3-1 短-长比、自定义 step 序列）：MVP 仅固定 4-1 模式可配 cycles_per_long
- **长休改成弱提示（通知 / 托盘）**：MVP 复用全屏 tip-window
- **独立 Session 模式**（手动启动 Focus Session 覆盖 20-20-20）：已被互斥模式排除

## Technical Notes

### 仓库现状（已确认）

- 状态机文件：`src-tauri/src/services/timer/{state,machine,effect,effect_executor,service,mod}.rs`
- 现有 states：`Working / PreAlert / Alerting / Resting / Paused`
- 现有 UserEvents：`StartRest / Skip / Pause / Resume`
- 现有 SkipFlags：`fullscreen_active / schedule_inactive / afk_active / process_whitelisted`
- Effect 类型已支持 EmitStateChanged / PlaySound / ShowTipWindows / HideTipWindows / ResetWorkTimer / UpdateTray / RecordRestSession
- TimerConfig 全字段都有 `#[serde(default)]`，新增字段对现有 TOML 文件无破坏

### 跨层影响清单

- **后端**：`config.rs` + `types.rs` + `state.rs` + `machine.rs` + `service.rs`（重置逻辑）+ `commands/mod.rs`（新增 update_pomodoro_config）+ ts-rs 重新导出
- **前端**：`SettingsPage.svelte`（新增 Card）+ `TipApp.svelte` / `TipMinimalApp.svelte`（文案分支）+ `TrayApp.svelte` 或 tooltip 来源（追加 Pomo 进度）+ stores（mode store）+ i18n JSON
- **Capability**：`main-window.json` 加 `allow-update-pomodoro-config`
- **文档**：CLAUDE.md 模块索引 + `.trellis/spec/architecture/ipc-and-state.md`

### 参考文档

- `.trellis/spec/architecture/ipc-and-state.md`（状态机契约 + IPC payload 变更）
- `.trellis/spec/architecture/change-management.md`（变更清单）
- `.trellis/spec/backend/service-pattern.md`（TimerService 编排）
- `.trellis/spec/backend/coding-standards.md`（状态机纯函数 + 错误处理）
- `.trellis/spec/frontend/store-and-ipc-patterns.md`（mode 切换 store 设计）
- `.trellis/spec/frontend/component-guidelines.md`（Settings Card 新增模式）
