# IPC 接口与状态机

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## Commands (Frontend → Backend)

| Command | 参数 | 返回 | 窗口权限 | 说明 |
|---------|------|------|---------|------|
| `get_state_snapshot` | -- | `StatePayload` | all | 获取当前状态快照 |
| `start_rest` | -- | `()` | tip, tray | 开始休息 |
| `skip_rest` | -- | `()` | tip, tray | 跳过休息 |
| `pause_timer` | -- | `()` | tray, main | 暂停计时器 |
| `resume_timer` | -- | `()` | tray, main | 恢复计时器 |
| `get_config` | -- | `Config` | main | 获取完整配置 |
| `update_timer_config` | `TimerConfig` | `()` | main | 更新计时器配置 (下周期生效) |
| `update_behavior_config` | `BehaviorConfig` | `()` | main | 更新行为配置 (即时生效) |
| `update_display_config` | `DisplayConfig` | `()` | main | 更新显示配置 (即时生效) |
| `get_daily_stats` | `{ range }` | `Vec<DailyStat>` | main | P2: 查询每日统计 |

## Events (Backend → Frontend)

| Event | Payload | 说明 |
|-------|---------|------|
| `state_changed` | `StatePayload` | 状态变更通知 |
| `config_changed` | `Config` | 配置变更通知 |

## 错误类型

```rust
#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum AppError {
    ConfigInvalid { field: String, reason: String },
    ServiceNotReady { service: String },
    IoError { message: String },
}
```

## 通信模式

```
初始化: listen() 先注册 → invoke('get_state_snapshot') 拉取当前状态
运行时: listen('state_changed') 接收后端推送
操作:   invoke('start_rest') / invoke('skip_rest') / invoke('update_timer_config', {...})
```

## IPC 边界规则

### 输入校验

- Command 层 MUST 做参数存在性和基本类型校验
- Service 层 MUST 做业务规则校验（如 `work_minutes` 范围 1-120）
- 前端传入的数据 MUST 视为不可信

### 超时策略

- Command 执行 SHOULD 在 5 秒内完成
- 涉及磁盘 I/O 的操作（配置写入）MUST 使用 `spawn_blocking`
- 前端 `invoke()` SHOULD 设置超时，超时后显示错误提示

### 错误返回

- Command MUST 返回 `Result<T, AppError>`
- MUST NOT 在 Command 中 `unwrap()` 或 `panic!()`
- 错误信息 MUST 对用户友好，技术细节记入日志

---

## Timer 状态机

### 状态

```
Working     -- 工作计时中
PreAlert    -- 预提醒：托盘 tooltip 变化 + 可选预提醒音效
Alerting    -- 全屏提醒显示，等待用户操作（超时自动消失）
Resting     -- 用户选择休息，倒计时中
Paused(prev_state, remaining)  -- 用户手动暂停，携带暂停前状态和剩余时间
Away        -- 离席检测触发 (P2)
```

- `Paused` MUST 携带暂停前的状态（`prev_state`）和剩余时间（`remaining`）
- 恢复时 MUST 回到 `prev_state` 并从 `remaining` 继续倒计时，MUST NOT 总是跳回 Working

### 状态转换

```
start --> Working
Working --timeout--> PreAlert
PreAlert --timeout--> Alerting
Alerting --user:start_rest--> Resting
Alerting --user:skip--> Working
Alerting --timeout(alert_timeout_seconds)--> Working (自动消失，计为 skip)
Resting --timeout--> Working
Any(except Away) --user:pause--> Paused(current_state, remaining_time)
Paused --user:resume--> prev_state (从 remaining_time 继续)
Working --detector:away--> Away (P2)
Away --detector:back--> Working (P2)
```

- 状态转换 MUST 通过 `resolve_user_event()` 或 `step_time()` 纯函数触发
- MUST NOT 在业务代码中直接修改状态字段
- 任何非法转换 MUST 被忽略（返回 `None`），MUST NOT panic

### 核心三函数

```rust
// 纯函数：用户事件 → 状态转换
fn resolve_user_event(state: &TimerState, event: UserEvent) -> Option<Transition>;

// 纯函数：时间推进 → 状态转换
fn step_time(inner: &Inner, now: Instant, skip_flags: &SkipFlags) -> Option<Transition>;

// 收集副作用（锁内收集，锁外执行）
fn collect_effects(transition: &Transition, inner: &Inner) -> Vec<Effect>;
```

- 前两个 MUST 是纯函数（无 I/O、无 side effect）
- `collect_effects` MUST 在锁内调用，返回的 `Vec<Effect>` MUST 在锁外执行

### Effect 类型

```rust
pub enum Effect {
    EmitStateChanged(StatePayload),
    ShowTipWindows,
    HideTipWindows,
    PlaySound(SoundType),
    UpdateTray(TrayUpdate),
    RecordStat(StatEvent),       // P2
    ResetWorkTimer(Duration),
}
```

- Effect MUST NOT 持有锁引用
- Effect 执行失败 MUST 记录日志但 MUST NOT 阻塞状态机

### SkipFlags

```rust
pub struct SkipFlags {
    pub fullscreen_active: bool,      // MVP
    pub whitelisted_process: bool,    // P2
    pub user_away: bool,              // P2
    pub outside_workday: bool,        // P2
}
```

Working 超时时，如果任何 flag 为 true → 重置计时器，不进入 PreAlert。
