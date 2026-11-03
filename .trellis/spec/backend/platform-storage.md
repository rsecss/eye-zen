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
    fn supports_fullscreen_detection(&self) -> bool; // Settings 全屏跳过灰显
    fn get_foreground_process_name(&self) -> Option<String>;  // P2 进程白名单
    fn supports_foreground_process_detection(&self) -> bool;  // Settings 白名单灰显
}
```

规则：

- 新增平台能力 MUST 先在 trait 中定义签名，再补三平台实现
- 三个平台 MUST 全部实现（或显式降级），MUST NOT 留 `unimplemented!()`
- 不支持的能力 MUST 返回保守默认值（`false` / `None`），MUST NOT panic
- `create_platform()` MUST 在编译期通过 `#[cfg(target_os = ...)]` 选实现，未知平台触发 `compile_error!`（已落地）

## 平台能力矩阵

| 能力 | Windows | macOS | Linux X11 | Linux Wayland |
|------|---------|-------|-----------|---------------|
| 全屏检测 | `GetForegroundWindow` + `MonitorFromWindow` | `CGGetActiveDisplayList` + `CGWindowListCopyWindowInfo`（layer==0 + bounds 与 display rect ±1px 比较） | `_NET_WM_STATE_FULLSCREEN` | 降级: `false`，Settings 灰显 |
| AFK idle 时长 | `GetLastInputInfo` + `GetTickCount` | `CGEventSourceSecondsSinceLastEventType` | XScreenSaver `query_info.ms_since_user_input` | 降级: `None` + Settings 灰显 |
| 系统音频峰值 (P2) | `IAudioMeterInformation` COM | 降级: 无公共 API | PulseAudio peak | PulseAudio peak |
| 前台进程 (P2) | `GetWindowThreadProcessId` | `NSWorkspace` | `_NET_ACTIVE_WINDOW` + `/proc` | 降级: `None` |

当前 `WindowsPlatform`、`MacosPlatform`、`LinuxPlatform` 实现位于同名文件中。每个实现 MUST 暴露 `pub(crate) const fn new() -> Self` 或同等构造，MUST 实现 `Default`。

## 降级原则

- 每个能力降级 MUST 只记录一次 `warn` 日志，MUST NOT 在每次调用都刷 warn
  - 实现范式：按能力拆分 `AtomicBool` 哨兵，见 `WindowsPlatform.fullscreen_warned` / `idle_warned`（`src-tauri/src/platform/windows.rs`）
- 保守降级：宁可多提醒，不漏提醒。例：全屏检测失败 → 返回 `false`（让用户照常收到提示）
- 设置 UI SHOULD 展示当前已降级的能力；AFK idle 不可用时 `get_detector_capabilities` MUST 返回 `afk_detection_supported = false`，Settings MUST 灰显 AFK 控件；全屏检测不可用时（macOS / Linux Wayland）MUST 返回 `fullscreen_detection_supported = false`，Settings MUST 灰显"全屏跳过"开关
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
    config: Arc<ArcSwap<VersionedConfig>>, // revision + config 同一无锁快照
    tx: watch::Sender<Arc<Config>>,   // 变更广播
    path: PathBuf,
    write_lock: Mutex<()>,            // 串行化 read-modify-write
    app: Mutex<Option<ServiceContext>>,
}
```

| 操作 | 实现 | 规则 |
|------|------|------|
| 读取 | `current()` / `snapshot()` 通过 `ArcSwap<VersionedConfig>` 无锁 | config 与 revision MUST 来自同一快照 |
| 订阅 | `subscribe() -> watch::Receiver<Arc<Config>>` | 服务 init 阶段获取 |
| 更新 | section 级方法接收 `expected_revision` 并返回 `ConfigSnapshot` | stale revision MUST 返回 `ConfigConflict` |
| 写入 | tmp + rename 原子操作（`write_config_file`） | MUST NOT 直接覆盖原文件 |
| 解析失败 | 写 `config.toml.bak` 备份并使用默认值 | MUST NOT 静默覆盖原始文件，原文件保留 |
| 串行化 | `write_lock: Mutex<()>` 包裹 read-modify-write | 防止并发 section update 互相覆盖 |

### `update_with` 范式

锁内完成 read-modify-write，锁外 emit 事件：

```rust
let snapshot = {
    let _write_guard = self.write_lock.lock().map_err(|_| AppError::IoError {
        message: "config write lock poisoned".to_string(),
    })?;
    let current = self.config.load_full();
    if current.revision != expected_revision {
        return Err(AppError::ConfigConflict {
            expected: expected_revision,
            actual: current.revision,
        });
    }
    let mut config = (*current.config).clone();
    update(&mut config);
    Self::validate_timer_contract(&config)?;
    let revision = current.revision.checked_add(1).ok_or_else(|| {
        AppError::InvalidOperation {
            operation: "config revision increment".to_string(),
            reason: "revision counter exhausted".to_string(),
        }
    })?;
    self.save(&config)?;
    let config = Arc::new(config);
    self.config.store(Arc::new(VersionedConfig {
        revision,
        config: Arc::clone(&config),
    }));
    let _ = self.tx.send(Arc::clone(&config));
    ConfigSnapshot { revision, config: (*config).clone() }
};

self.emit_config_changed(&snapshot);
```

- write lock 用 `std::sync::Mutex<()>` 而非 `tokio::sync::Mutex`：临界区是同步 I/O（`std::fs::rename`），MUST NOT 跨 `.await` 持有
- 中毒处理：MUST 返回 `AppError::IoError`，MUST NOT `unwrap()`
- TOML load 与每个 section update MUST 校验 Timer/Pomodoro 字段范围及 active work duration；Timer 内部不得用 `.max(1)` 或 subtraction fallback 接受非法配置。
- revision 溢出检查 MUST 在写文件前完成，避免“文件已更新但内存/revision 返回错误”的分裂状态。

### Hotkey 配置完整操作

```rust
ConfigService::update_hotkeys(
    hotkeys: HotkeysConfig,
    expected_revision: u32,
    apply_hotkeys: impl FnMut(&HotkeysConfig) -> Result<()>,
) -> Result<ConfigSnapshot>
```

- `ConfigService` MUST hold `write_lock` across revision comparison, desired binding apply, TOML save, in-memory revision commit, and failure rollback.
- The apply closure is synchronous and the command invokes the complete operation through `spawn_blocking`; the transaction MUST NOT cross `.await`.
- A stale revision returns `ConfigConflict` before invoking the closure. Apply failure or save failure re-applies the previous `HotkeysConfig` best-effort and leaves TOML/revision unchanged.
- The config watch notification may re-apply the committed hotkeys idempotently, but it is not the transaction owner and MUST NOT repair a stale command after the fact.

## 配置更新语义

| 配置项 | 生效时机 | 说明 |
|--------|---------|------|
| `behavior.sound_enabled`、`behavior.fullscreen_skip`、`behavior.afk_skip_enabled`、`behavior.afk_threshold_minutes` | 即时 | 下次 tick 直接读 `services.config.current()`；AFK threshold 不进入 `TimerService::sync_runtime_config` |
| `timer.work_minutes`、`timer.rest_seconds`、`timer.pre_alert_seconds`、`timer.alert_timeout_seconds` | 下周期 | 当前周期不中断，通过 `TimerService::sync_runtime_config` 同步 |
| `display.language` | 即时 | I18nService 订阅 watch，托盘菜单与前端同步刷新 |
| `display.theme` | 即时 | 前端监听 `config_changed` 事件即时切换 |
| `behavior.auto_start` | 即时 | 调用 `tauri-plugin-autostart` 同步系统状态 |
| `hotkeys.*` | 即时 | ConfigService 在同一 revision-checked write transaction 内重绑 OS 快捷键并写 TOML；失败回滚旧绑定 |

规则：

- 即时生效配置 MUST 在变更后通过 `ServiceContext::emit_config_changed` 广播 `config_changed` 事件（见 `src-tauri/src/services/context.rs`）
- 下周期生效的字段 MUST NOT 中断当前计时周期；现行实现由 timer loop 在每秒 tick 时检查 `config_rx.has_changed()` 并调用 `sync_runtime_config` 更新内部 `Duration`
- 快捷键配置 MUST NOT 由 command 分步编排“apply → save”。完整操作由 ConfigService 串行拥有，避免 stale request 在 revision check 前改变系统绑定。

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
- 例外：SQLite `VACUUM INTO` 不支持 bind parameter（语法限制），MUST 用 `format!` 拼接并对路径做单引号 escape（`replace('\'', "''")`）；参数来源 MUST 为可信边界（如 OS save dialog 返回值），MUST NOT 直接拼接前端任意 string。实例见 `StatService::export_to`
- 写入 MUST 在专属 service（`StatService`）内进行，其他 service MUST NOT 直接持有连接池
- StatService 在 `shutdown` 中 MUST flush 待写记录并关闭连接池
- 持久化时间戳 MUST 使用 UTC RFC3339；日/周/月 bucket MUST 在查询时按请求 IANA timezone 聚合，不能把单个 `date` 字段当作跨时区事实来源
- 当前记录语义：只在完成休息的 `Resting -> Working` transition 记录一条 `state = 'resting'` segment；跳过提醒不计入休息统计

## Scenario: Safe Statistics Export Replacement

### 1. Scope / Trigger

- Trigger: `export_statistics` may overwrite an existing backup; any export, validation, sync, or replacement failure must preserve the old target.

### 2. Signatures

```rust
StatService::export_to(target: PathBuf) -> Result<()>
temporary_export_path(target: &Path) -> Result<PathBuf>
validate_exported_database(path: &Path) -> Result<()>
replace_file(temporary: PathBuf, target: PathBuf) -> Result<()>
```

### 3. Contracts

- Serialize exports with `StatService.export_lock` and create a unique temporary database in the target directory.
- Run `VACUUM INTO` only against the temporary path, then verify `PRAGMA user_version == SCHEMA_VERSION` and `PRAGMA quick_check == "ok"`.
- Call file `sync_all` before replacement. Never remove or truncate the target first.
- Unix uses same-filesystem `rename`; Windows uses `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`.
- Any failure cleans the temporary file best-effort and returns the original error; cleanup failure is logged without replacing it.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| invalid/relative/non-`.db`/source-self target | reject before export |
| temporary path collision | `IoError`; old target untouched |
| `VACUUM INTO`, schema, quick-check, or sync failure | remove temporary; old target untouched |
| platform replacement failure | remove temporary; old target bytes unchanged |
| success with existing target | atomically replace target with validated database |

### 5. Good / Base / Bad Cases

- Good: existing backup is atomically replaced and opens with expected rows.
- Base: absent target and empty source database produce a valid schema-versioned backup.
- Bad: injected replacement failure preserves the exact old bytes and leaves no export temp file.

### 6. Tests Required

- Success: empty DB, populated DB, absent target, existing target, concurrent exports.
- Failure: corrupt validation input, unwritable target directory, injected replacement failure and cleanup assertion.
- Platform: Windows local overwrite check; Unix/macOS parity through CI runners.

### 7. Wrong vs Correct

#### Wrong

```rust
tokio::fs::remove_file(&target).await?;
sqlx::query(&format!("VACUUM INTO '{target_sql}'")).execute(&pool).await?;
```

#### Correct

```rust
sqlx::query(&format!("VACUUM INTO '{temporary_sql}'"))
    .execute(&pool)
    .await?;
validate_exported_database(&temporary).await?;
sync_export_file(&temporary).await?;
replace_file(temporary, target).await?;
```

## Known limitations（跨平台路径/字符串）

进程白名单与前台进程查询的 basename 提取在三个平台都有真实但低发生率的退化点。设计上 MUST 把这些当作"已知限制"，MUST NOT 在调用方加额外兜底逻辑去掩盖。

| 平台 | 退化场景 | 当前处理 | 用户可见行为 |
|------|---------|---------|-------------|
| Windows | 进程实际路径长度 > `MAX_PATH` (260 字符) | `QueryFullProcessImageNameW` 传入 `PROCESS_NAME_WIN32` 标志强制返回短路径；若仍超长，函数返回错误，`get_foreground_process_name` 返回 `Ok(None)`，回退到"非白名单" | 极少数超长安装路径的进程无法被白名单匹配；用户解决方案：移动到短路径目录。源：`src-tauri/src/platform/windows.rs:175-194` |
| Linux | `/proc/{pid}/exe` 目标路径包含非 UTF-8 字节 | `OsStr::to_str()` 对非 UTF-8 返回 `None`，整体 `and_then` 链短路返回 `None` | 非 UTF-8 文件名的进程被视为"非白名单"；用户解决方案：白名单条目须为 UTF-8（中文/Emoji 文件名均 OK，仅 latin-1/GBK 等非 UTF-8 编码会失败）。源：`src-tauri/src/platform/linux.rs:330-338` |
| macOS | `kCGWindowOwnerName` 返回的 `CFString` 含无效 UTF-16 代理对 | `CFString::to_string()` 把无效代理替换为 U+FFFD 替换字符（lossy） | 含 U+FFFD 的名字几乎不会与 ASCII/UTF-8 白名单条目相等，相当于"非白名单"；实际遇到 invalid surrogate 的概率接近零（系统应用名都是规范 UTF-16）。源：`src-tauri/src/platform/macos.rs:195-202` |

补充约束：

- 白名单条目在配置层 MUST 经过 `sanitize_process_whitelist`（`src-tauri/src/models/config.rs`）小写化与去重；用户输入 MUST NOT 包含路径，只 basename
- 上述退化均 MUST 表现为 `Ok(None)` 而非 panic / `Err`，因为前台进程查询失败 NEVER 应该阻断 timer 调度
