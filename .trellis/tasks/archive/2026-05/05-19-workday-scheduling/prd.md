# workday scheduling

## Goal

让 Eyezen 在用户不想被打扰的时段自动停止弹窗（例如周末），无需手动 Pause。属于 Phase 2 增强切片，独立性强、无前端/后端外部新模块，作为 v0.1.0 后的热身切入点。

## Requirements

- 新增 `ScheduleConfig { enabled: bool, active_days: [bool; 7] }`，索引 0=Mon..6=Sun，与 `chrono::Weekday::num_days_from_monday` 对齐
- 默认值：`enabled = false`，`active_days = [true, true, true, true, true, false, false]`（Mon–Fri 默认勾选）
- 老 `config.toml` 缺 `schedule` 字段时使用默认（不报错；默认 enabled=false 即不影响现有行为）
- 状态机扩展 `SkipFlags.schedule_inactive: bool`，复用现有 `any_active()` 聚合路径，仅在 `Working→PreAlert` 跳转点生效；其他状态不打断当前周期
- 时间源：`chrono::Local::now().weekday()`，每次 tick 在 `TimerService::on_tick` 中现读 wall clock
- 配置热生效：复用 ConfigService 的 watch channel，无需新增机制
- Settings 新增独立 SettingsCard "schedule"：1 个总开关 Toggle + 7 个周几原生 checkbox
- i18n zh-CN + en：title、enabled label/desc、7 个周几缩写

## Acceptance Criteria

- [ ] `Config.schedule: ScheduleConfig` 落 TOML，ts-rs binding 同步导出
- [ ] 纯函数 `is_schedule_active(now: DateTime<Local>, schedule: &ScheduleConfig) -> bool` 覆盖单测：disabled→true、enabled+各周几 7 个测试用例
- [ ] 状态机新增 `working_timeout_with_schedule_inactive_resets` 测试，套用 `working_timeout_with_fullscreen_skip_resets` 模式
- [ ] Settings UI 渲染、勾选、保存、热生效（手动验证）
- [ ] 老 `config.toml`（无 `[schedule]` 段）启动不报错
- [ ] zh-CN + en 文案覆盖（设置区块 + tray 文案如需）
- [ ] cargo fmt + clippy --all-targets + cargo test + svelte-check + vitest + prettier + vite build 全过
- [ ] 手动 E2E：临时把 work_minutes 改为 1 + active_days 当前星期设为 false + enabled=true → 1 分钟后不弹窗

## Definition of Done

- AC 全部勾选
- CLAUDE.md "Phase 2 进行中" 章节追加 Schedule 项（注意是 Plan 014，沿用现有命名）
- docs/devlog.md 追加变更条目
- 不更新 `docs/plans/`（PRD 在 `.trellis/tasks/` 是单一来源）

## Technical Approach

**Schema**
- `Config` 新增 `pub schedule: ScheduleConfig`（与 timer/behavior/display 平级），`#[serde(default)]`
- `ScheduleConfig`：`enabled: bool`、`active_days: [bool; 7]`，全字段 `#[serde(default = ...)]`

**Dependency**
- 新增 `chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }`
- 先 `cargo tree | grep chrono` 验证是否已被间接引入（若已引入但缺 serde feature 则补 feature）

**状态机**
- `SkipFlags` 加 `schedule_inactive: bool` 字段，`any_active()` 已是 OR 聚合，零额外改动
- `TimerService::on_tick`（或类似入口）在调用 `step_time` 前算出 `schedule_inactive = !is_schedule_active(Local::now(), &config.schedule)`
- 纯函数 `is_schedule_active` 放在 `services/schedule.rs`（独立模块），无 IO，可直接单测

**UI**
- 新增 `components/SettingsScheduleCard.svelte` 或直接在 `SettingsPage.svelte` 中插入第三张 `SettingsCard`
- 复用 `Toggle.svelte`；7 个 weekday 用原生 `<input type="checkbox">` 即可（KISS，不抽 WeekdayPicker）
- 配置 store 字段读写按现有 `configStore` 模式

## Decision (ADR-lite)

**Context**
- 调度可以做得很复杂（每日时段、多时段、节假日 API），但 Phase 2 切片要"独立、热身"
- 状态机已有 `SkipFlags` 模式服务于全屏跳过，是天然扩展点

**Decision**
- 粒度=**仅按周几**，最小可表达「工作日 vs 周末」
- 语义=**仅在 Working→PreAlert 跳转点检查**，复用 `SkipFlags`；其他状态不打断
- UI=**独立 Schedule SettingsCard**，与 timer/behavior/display 平级
- PR=**单 PR 全部交付**（< ~300 LOC，原子上线）

**Consequences**
- ✅ 改动面小、风险低；纯函数 + 复用既有机制
- ✅ 老 config 自动兼容（serde default + enabled 默认 false）
- ✅ Alerting/Resting/Paused 状态切日不被中断 —— 这是「跳转点检查」的自然副作用，符合「不打断进行中流程」直觉
- ⚠️ 用户感知层面：托盘倒计时仍然在跑，但永远不弹窗——这是预期行为，需在 i18n 文案里点一句（或不点，靠状态自解释）
- ⚠️ 时区/跨日：用 `chrono::Local`，wall clock 改动会自然进/出非工作日，不存在持久化"上次切换时间"的复杂度
- ❌ 不支持每日时段——若用户表达"上班时间才工作"将不能满足，待 Phase 2 后续切片或 Phase 3 处理

## Out of Scope

- 节假日识别 / 中国法定节假日 API
- 每日时段（hh:mm 范围）
- 多时段 / 排除午休
- 离席检测联动
- Statistics 上的"调度命中率"
- 跨日凌晨复杂边界

## Technical Notes

**Inspected**
- `src-tauri/src/models/config.rs` — `Config { timer, behavior, display }`，无 schedule 字段
- `src-tauri/src/services/timer/machine.rs` — `SkipFlags` 当前只承载 `fullscreen_active`，`any_active()` 已是 OR 聚合；状态机仅在 `Working` timeout 时检查
- `src/pages/main/SettingsPage.svelte` — 3 张 SettingsCard，模式清晰；`SettingsCard.svelte` + `Toggle.svelte` 已就绪
- 状态机测试 `working_timeout_with_fullscreen_skip_resets` 是新增调度测试的样板

**Constraints**
- chrono 默认 features 含已弃用的 `oldtime`，必须 `default-features = false`
- TS bindings 由 ts-rs 测试用 `#[cfg_attr(test, derive(ts_rs::TS))]` 自动生成
- 配置兼容遵循 `.trellis/spec/architecture/change-management.md`「配置兼容」约定

## Research References

无（产品配置粒度决策，未触发 research-first）
