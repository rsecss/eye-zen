# 平台抽象与数据存储

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## PlatformApi Trait

```rust
pub trait PlatformApi: Send + Sync {
    fn is_fullscreen_app_active(&self) -> bool;              // MVP
    fn get_cursor_position(&self) -> Option<(i32, i32)>;     // P2
    fn get_system_audio_peak(&self) -> Option<f32>;          // P2
    fn get_foreground_process_name(&self) -> Option<String>; // P2
}
```

- 新增平台能力 MUST 先在 trait 中定义签名
- 三个平台 MUST 全部实现（或显式降级）
- 不支持的能力 MUST 返回保守默认值，MUST NOT panic

## 平台能力矩阵

| 能力 | Windows | macOS | Linux X11 | Linux Wayland |
|------|---------|-------|-----------|---------------|
| 全屏检测 | `GetForegroundWindow` + `MonitorFromWindow` | `CGWindowListCopyWindowInfo` | `_NET_WM_STATE_FULLSCREEN` | 降级: `false` |
| 光标位置 | `GetCursorPos` | `CGEventSource` | `XQueryPointer` | 有限 |
| 系统音频 | `IAudioMeterInformation` COM | 降级: 无公共 API | PulseAudio peak | PulseAudio peak |
| 前台进程 | `GetWindowThreadProcessId` | `NSWorkspace` | `_NET_ACTIVE_WINDOW` + `/proc` | 降级: `None` |

## 降级原则

- 每个能力降级 MUST 只记录一次 `warn` 日志，MUST NOT 重复刷日志
- 保守降级：宁可多提醒，不漏提醒（如全屏检测失败 → 返回 `false`）
- 设置 UI SHOULD 展示降级信息

---

## 存储策略

| 数据 | 格式 | 位置 | 阶段 |
|------|------|------|------|
| 用户配置 | TOML | `config_dir/eyezen/config.toml` | MVP |
| 统计数据 | SQLite | `app_data_dir/eyezen/data.db` | P2 |
| 日志 | text | `app_data_dir/eyezen/logs/` | MVP |

- 配置和数据 MUST 分开存储（`config_dir` vs `app_data_dir`）
- 日志 MUST 使用每日轮转，SHOULD 保留最近 7 天

## 配置结构

```toml
[timer]
work_minutes = 20
rest_seconds = 20
pre_alert_seconds = 15
alert_timeout_seconds = 60

[behavior]
sound_enabled = true
fullscreen_skip = true
auto_start = false

[display]
language = "zh-CN"           # P2
theme = "light"              # MVP: light only
```

## ConfigService 设计

```rust
pub struct ConfigService {
    config: Arc<ArcSwap<Config>>,     // 无锁读取
    tx: watch::Sender<Arc<Config>>,   // 变更广播
    path: PathBuf,
}
```

| 操作 | 实现 | 规则 |
|------|------|------|
| 读取 | `ArcSwap` 无锁 | 任意线程可读 |
| 订阅 | `watch::Receiver` | 服务 init 时获取 |
| 更新 | section 级 command | MUST NOT 逐字段字符串匹配 |
| 写入 | 原子操作（tmp + rename） | MUST NOT 直接覆盖原文件 |
| 解析失败 | 保留 `.bak` 备份 | MUST NOT 覆盖为默认值 |

## 配置更新语义

| 配置项 | 生效时机 | 说明 |
|--------|---------|------|
| `sound_enabled`, `fullscreen_skip` | 即时 | 下次 tick 读取新值 |
| `work_minutes`, `rest_seconds`, `pre_alert_seconds` | 下周期 | 当前周期不中断 |
| `language`, `theme` | 即时 | 前端监听变更事件 |
| `shortcuts` | 重启 | 全局热键启动时注册 |

- 即时生效的配置 MUST 在变更后立即广播 `config_changed` 事件
- 下周期生效的配置 MUST NOT 中断当前计时周期

## SQLite Schema (P2)

```sql
CREATE TABLE activity_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    state TEXT NOT NULL,          -- 'working', 'resting', 'away', 'paused'
    started_at TEXT NOT NULL,     -- ISO 8601
    ended_at TEXT NOT NULL,
    date TEXT NOT NULL            -- YYYY-MM-DD
);
```

- Schema 变更 MUST 通过迁移脚本，MUST NOT 手动修改线上数据库
- 查询 MUST 使用参数化 SQL，MUST NOT 拼接字符串
