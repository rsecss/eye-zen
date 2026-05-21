# 平台抽象与数据存储

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

本文档约束 `src-tauri/src/platform/` 平台层与配置 / 数据持久化设计。代码锚点：

- `src-tauri/src/platform/mod.rs`、`platform/windows.rs`、`platform/macos.rs`、`platform/linux.rs`
- `src-tauri/src/services/config.rs`
- `src-tauri/src/models/config.rs`

## `PlatformApi` Trait

当前 trait（见 `src-tauri/src/platform/mod.rs`）：

```rust
pub(crate) trait PlatformApi: Send + Sync {
    fn is_fullscreen_app_active(&self) -> bool; // MVP 已实现
    fn idle_duration(&self) -> Option<Duration>; // AFK 跳过
    fn supports_idle_detection(&self) -> bool;   // Settings 能力灰显
}
```

P2 / P3 计划扩展（尚未实现，新增时 MUST 同步本文档）：

```rust
fn get_system_audio_peak(&self) -> Option<f32>;           // P2 进程白名单
fn get_foreground_process_name(&self) -> Option<String>;  // P2 进程白名单
```

规则：

- 新增平台能力 MUST 先在 trait 中定义签名，再补三平台实现
- 三个平台 MUST 全部实现（或显式降级），MUST NOT 留 `unimplemented!()`
- 不支持的能力 MUST 返回保守默认值（`false` / `None`），MUST NOT panic
- `create_platform()` MUST 在编译期通过 `#[cfg(target_os = ...)]` 选实现，未知平台触发 `compile_error!`（已落地）

## 平台能力矩阵

| 能力 | Windows | macOS | Linux X11 | Linux Wayland |
|------|---------|-------|-----------|---------------|
| 全屏检测 | `GetForegroundWindow` + `MonitorFromWindow` | `CGWindowListCopyWindowInfo` | `_NET_WM_STATE_FULLSCREEN` | 降级: `false` |
| AFK idle 时长 | `GetLastInputInfo` + `GetTickCount` | `CGEventSourceSecondsSinceLastEventType` | XScreenSaver `query_info.ms_since_user_input` | 降级: `None` + Settings 灰显 |
| 系统音频峰值 (P2) | `IAudioMeterInformation` COM | 降级: 无公共 API | PulseAudio peak | PulseAudio peak |
| 前台进程 (P2) | `GetWindowThreadProcessId` | `NSWorkspace` | `_NET_ACTIVE_WINDOW` + `/proc` | 降级: `None` |

当前 `WindowsPlatform`、`MacosPlatform`、`LinuxPlatform` 实现位于同名文件中。每个实现 MUST 暴露 `pub(crate) const fn new() -> Self` 或同等构造，MUST 实现 `Default`。

## 降级原则

- 每个能力降级 MUST 只记录一次 `warn` 日志，MUST NOT 在每次调用都刷 warn
  - 实现范式：按能力拆分 `AtomicBool` 哨兵，见 `WindowsPlatform.fullscreen_warned` / `idle_warned`（`src-tauri/src/platform/windows.rs`）
- 保守降级：宁可多提醒，不漏提醒。例：全屏检测失败 → 返回 `false`（让用户照常收到提示）
- 设置 UI SHOULD 展示当前已降级的能力；AFK idle 不可用时 `get_detector_capabilities` MUST 返回 `afk_detection_supported = false`，Settings MUST 灰显 AFK 控件
- 平台错误 MUST 在 platform 层捕获，MUST NOT 跨层抛 `Result` 给 service 强制处理（service 拿到的是 `bool` / `Option`）

## 存储策略

| 数据 | 格式 | 位置 | 阶段 |
|------|------|------|------|
| 用户配置 | TOML | `app_config_dir/eyezen/config.toml` | MVP（已落地） |
| 统计数据 | SQLite | `app_data_dir/eyezen/data.db` | P2（已落地） |
| 日志 | text | `app_data_dir/eyezen/logs/eyezen.log` | MVP（已落地，每日轮转） |

代码锚点：

- 路径解析：`src-tauri/src/lib.rs` 中 `app.path().app_config_dir()` 与 `.app_data_dir()`
- 日志目录：`logs/` 子目录由 `logging::init_tracing` 负责创建（见 `src-tauri/src/logging.rs`）

规则：

- 配置与运行时数据 MUST 分别使用 `app_config_dir` 与 `app_data_dir`，便于备份与卸载策略
- 日志 MUST 使用每日轮转（`tracing_appender::rolling::daily`），SHOULD 保留最近 7 天（后续清理策略由 P2 完成）
- 任何持久化路径 MUST 通过 Tauri Path API 解析，MUST NOT 硬编码

## 配置结构

实际 TOML 形态（与 `src-tauri/src/models/config.rs` 完全对应）：

```toml
[timer]
work_minutes = 20
rest_seconds = 20
pre_alert_seconds = 15
alert_timeout_seconds = 60

[behavior]
sound_enabled = true
fullscreen_skip = true
afk_skip_enabled = true
afk_threshold_minutes = 5
auto_start = false

[display]
language = "zh-CN"
theme = "light"

[hotkeys]
start_rest = "CommandOrControl+Alt+B"
skip_rest = "CommandOrControl+Alt+S"
toggle_pause = "CommandOrControl+Alt+P"
```

规则：

- 所有字段 MUST 在 `models/config.rs` 中通过 `#[serde(default = "...")]` 提供默认值，缺字段不得报错
- `BehaviorConfig` 当前包含多个布尔开关，是持久化 TOML schema 的扁平配置；新增 bool 字段允许局部 `#[allow(clippy::struct_excessive_bools)]`，但 MUST 避免把运行时能力（如 `afk_detection_supported`）写入 TOML。
- 旧字段废弃 MUST 走 [`../architecture/change-management.md`](../architecture/change-management.md) 中的 deprecate → migrate → remove 流程，禁止直接删除导致老配置不可读
- 默认值 MUST 与 `Config::default()` 单元测试一致（参见 `src-tauri/src/models/config.rs` 的 `default_values` 测试）

## `ConfigService` 设计

实际类型签名（`src-tauri/src/services/config.rs`）：

```rust
pub(crate) struct ConfigService {
    config: Arc<ArcSwap<Config>>,     // 无锁读取
    tx: watch::Sender<Arc<Config>>,   // 变更广播
    path: PathBuf,
    write_lock: Mutex<()>,            // 串行化 read-modify-write
    app: Mutex<Option<ServiceContext>>,
}
```

| 操作 | 实现 | 规则 |
|------|------|------|
| 读取 | `config.load_full()` 通过 `ArcSwap` 无锁 | 任意线程可读，MUST NOT 持锁 |
| 订阅 | `subscribe() -> watch::Receiver<Arc<Config>>` | 服务 init 阶段获取 |
| 更新 | section 级方法：`update_timer` / `update_behavior` / `update_display` | MUST NOT 暴露逐字段字符串匹配的"通用 update" |
| 写入 | tmp + rename 原子操作（`write_config_file`） | MUST NOT 直接覆盖原文件 |
| 解析失败 | 写 `config.toml.bak` 备份并使用默认值 | MUST NOT 静默覆盖原始文件，原文件保留 |
| 串行化 | `write_lock: Mutex<()>` 包裹 read-modify-write | 防止并发 section update 互相覆盖 |

### `update_with` 范式

锁内完成 read-modify-write，锁外 emit 事件：

```rust
let updated = {
    let _write_guard = self.write_lock.lock()?;
    let mut config = (*self.current()).clone();
    update(&mut config);
    self.save(&config)?;
    let config = Arc::new(config);
    self.config.store(Arc::clone(&config));
    let _ = self.tx.send(Arc::clone(&config));
    config
};

self.emit_config_changed(updated.as_ref());
```

- write lock 用 `std::sync::Mutex<()>` 而非 `tokio::sync::Mutex`：临界区是同步 I/O（`std::fs::rename`），MUST NOT 跨 `.await` 持有
- 中毒处理：MUST 返回 `AppError::IoError`，MUST NOT `unwrap()`

## 配置更新语义

| 配置项 | 生效时机 | 说明 |
|--------|---------|------|
| `behavior.sound_enabled`、`behavior.fullscreen_skip`、`behavior.afk_skip_enabled`、`behavior.afk_threshold_minutes` | 即时 | 下次 tick 直接读 `services.config.current()`；AFK threshold 不进入 `TimerService::sync_runtime_config` |
| `timer.work_minutes`、`timer.rest_seconds`、`timer.pre_alert_seconds`、`timer.alert_timeout_seconds` | 下周期 | 当前周期不中断，通过 `TimerService::sync_runtime_config` 同步 |
| `display.language` | 即时 | I18nService 订阅 watch，托盘菜单与前端同步刷新 |
| `display.theme` | 即时 | 前端监听 `config_changed` 事件即时切换 |
| `behavior.auto_start` | 即时 | 调用 `tauri-plugin-autostart` 同步系统状态 |
| `hotkeys.*` | 即时 | `update_hotkeys_config` 先重绑 OS 快捷键，成功后写 TOML；失败回滚旧绑定并发 `hotkey_status_changed` |

规则：

- 即时生效配置 MUST 在变更后通过 `ServiceContext::emit_config_changed` 广播 `config_changed` 事件（见 `src-tauri/src/services/context.rs`）
- 下周期生效的字段 MUST NOT 中断当前计时周期；现行实现由 timer loop 在每秒 tick 时检查 `config_rx.has_changed()` 并调用 `sync_runtime_config` 更新内部 `Duration`
- 快捷键配置 MUST NOT 只依赖 `config_changed` 事后同步；Settings 写入路径 MUST 先完成 `HotkeyService` 事务式注册，避免 TOML 已保存但系统绑定失败。

## SQLite Schema（P2，已实现）

Schema v1 由 `StatService` 启动时自动创建；使用 `PRAGMA user_version = 1` 标记版本，旧版本无 DB 时 MUST 自动建库建表。

```sql
CREATE TABLE activity_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    state TEXT NOT NULL,          -- 'working', 'resting', 'away', 'paused'
    started_at TEXT NOT NULL,     -- ISO 8601
    ended_at TEXT NOT NULL,
    duration_secs INTEGER NOT NULL,
    date TEXT NOT NULL            -- YYYY-MM-DD
);
CREATE INDEX idx_activity_segments_date ON activity_segments(date);
CREATE INDEX idx_activity_segments_state_started_at
    ON activity_segments(state, started_at);
```

规则：

- Schema 变更 MUST 通过版本化迁移（当前用 `PRAGMA user_version` + 幂等 DDL），MUST NOT 手动修改用户数据库
- 查询 MUST 使用参数化 SQL（当前通过 `sqlx::query(...).bind(...)`），MUST NOT 字符串拼接
- 写入 MUST 在专属 service（`StatService`）内进行，其他 service MUST NOT 直接持有连接池
- StatService 在 `shutdown` 中 MUST flush 待写记录并关闭连接池
- 持久化时间戳 MUST 使用 UTC RFC3339；日/周/月 bucket MUST 在查询时按请求 IANA timezone 聚合，不能把单个 `date` 字段当作跨时区事实来源
- 当前记录语义：只在完成休息的 `Resting -> Working` transition 记录一条 `state = 'resting'` segment；跳过提醒不计入休息统计
