# 服务模式与生命周期

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

本文档约束后端服务（`src-tauri/src/services/`）的依赖结构、生命周期、关闭顺序、注册方式与服务间通信。所有规则以 `src-tauri/src/lib.rs` 与 `src-tauri/src/services/mod.rs` 的现有实现为基准。

## 分层依赖（单向）

```
commands/  →  services/  →  platform/
    ↓            ↓
  models/     models/
```

- `commands/` MUST 只依赖 `services/`、`models/`
- `services/` MUST 只依赖 `models/`、`platform/`、其他 `services/`（按 DAG）
- `platform/` MUST NOT 依赖 `services/`、`commands/`
- `models/` MUST NOT 依赖任何上层模块（纯数据定义）

## 服务依赖 DAG

箭头表示"依赖于"，禁止循环：

```
ConfigService  ←  I18nService
               ←  TimerService  ←  WindowService
                                ←  TrayService
                                ←  SoundService
                                ←  StatService (P2)
               ←  HotkeyService → TimerService (via app-level shortcut handler)
               ←  DetectorService → TimerService (via skip flags pull)
               ←  SoundService
               ←  TrayService
I18nService    ←  TrayService
```

- **ConfigService** 是基础服务，MUST NOT 依赖其他服务（见 `src-tauri/src/services/config.rs`）
- **TimerService** 依赖 ConfigService 的 `watch::Receiver<Arc<Config>>`（见 `TimerService::new` 入参），MUST NOT 调用 WindowService / TrayService / SoundService
- **HotkeyService** 依赖 ConfigService 的 `watch::Receiver<Arc<Config>>` 与 Tauri global-shortcut 插件；快捷键 handler 通过 `AppServices` 调用 TimerService，MUST NOT 在 platform 层直接控制 timer
- 副作用服务（Window / Tray / Sound / Stat）通过 `ServiceContext::execute_timer_effect` 被动接收 effect（见 `src-tauri/src/services/context.rs`），MUST NOT 反向调用 TimerService
- I18nService 提供托盘菜单翻译，被 TrayService 通过 `Arc<I18nService>` 共享（见 `src-tauri/src/lib.rs` 中 `Arc::clone(&i18n_service)`）

## `AppServices` 注册

`AppServices` 定义在 `src-tauri/src/services/mod.rs`：

```rust
pub(crate) struct AppServices {
    pub(crate) config: config::ConfigService,
    pub(crate) timer: timer::TimerService,
    pub(crate) detector: detector::DetectorService,
    pub(crate) window: window::WindowService,
    pub(crate) sound: sound::SoundService,
    pub(crate) tray: tray::TrayService,
    pub(crate) i18n: Arc<i18n::I18nService>,
    pub(crate) hotkeys: hotkeys::HotkeyService,
}

pub(crate) type SharedAppServices = Arc<AppServices>;
```

- MUST 使用显式 struct，MUST NOT 引入 IoC 容器、反射或 trait object 注册
- MUST 在所有 service 完成 `new()` 与 `init()` 之后再用 `Arc` 包装，并通过 `app.manage(Arc::clone(&services))` 注册为 Tauri State
- 每个 service 自行管理可变状态（如 `Mutex<Inner>`、`ArcSwap`），`AppServices` 字段以不可变共享引用形式暴露
- `i18n` 字段使用 `Arc<I18nService>`，因为 `TrayService` 需要持有同一实例的共享引用（见 `src-tauri/src/services/tray.rs`）

## 服务 trait

`Service` trait 定义在 `src-tauri/src/services/mod.rs`：

```rust
pub(crate) trait Service: Send + Sync {
    fn init(&self, app: &ServiceContext) -> impl Future<Output = Result<()>> + Send;
    fn start(&self, app: &ServiceContext) -> impl Future<Output = Result<()>> + Send;
    fn shutdown(&self) -> impl Future<Output = Result<()>> + Send;
}
```

- 所有 service MUST 实现 `Service`
- `init` / `start` / `shutdown` 返回 `crate::error::Result<()>`，MUST NOT 在签名里换其他错误类型
- 取消、终止信号 MUST 通过 `JoinHandle::abort` 或 channel 显式触发（参见 `TimerService::shutdown` 调用 `handle.abort()`）

## 四阶段生命周期

每个服务 MUST 遵循以下阶段，跨阶段的操作不可越界：

| 阶段 | 方法 | 允许 | 禁止 |
|------|------|------|------|
| 构造 | `new()` | 初始化字段、创建 channel、读取启动配置快照 | I/O 写入、启动 task |
| 初始化 | `init(&ServiceContext)` | 缓存 `ServiceContext`、订阅 watch、注册轻量回调 | 启动后台 task、阻塞 await |
| 启动 | `start(&ServiceContext)` | `tokio::spawn` 后台循环、Tauri 资源订阅（托盘、窗口事件） | 阻塞主线程、panic |
| 关闭 | `shutdown()` | 取消 task、发送 `Shutdown` 命令、释放资源 | panic、静默忽略错误 |

代码佐证：
- `ConfigService::init` 仅缓存 `ServiceContext`，文件 I/O 全部在 `new()` 内完成（`load_or_default`）
- `TimerService::start` 通过 `ServiceContext::spawn_timer_loop` 创建 tick loop，并把 `JoinHandle` 存进 `tick_handle: Mutex<Option<JoinHandle<()>>>`
- `SoundService::new` 用 `thread::Builder` 创建独立音频线程（rodio 需要专线程），shutdown 通过 `SoundCommand::Shutdown` 通知

## 启动顺序

`src-tauri/src/lib.rs` 中 `setup()` 的实际顺序 MUST 与依赖 DAG 一致：

1. `ConfigService::new` → 加载或生成 TOML
2. `SoundService::new` → 启动音频线程
3. `DetectorService::new(platform::create_platform())`
4. `TimerService::new(config_service.subscribe())`
5. `WindowService::new`
6. `I18nService::new(&initial_locale)` 并 `Arc::wrap`
7. `TrayService::new(config_rx, Arc::clone(&i18n_service))`
8. `HotkeyService::new(config_rx, app_handle)`（后端持有注册权限，前端不暴露 generic shortcut API）
9. 顺序 `init`：config → i18n → detector → sound → timer → window → tray → hotkeys
10. `Arc<AppServices>` 包装并 `app.manage`
11. 顺序 `start`：config → i18n → detector → sound → window → tray → timer → hotkeys

注意 `start` 阶段 hotkeys 必须在 timer 之后启动：快捷键是用户事件源，必须等 timer 与下游 effect 执行器就位后再接收全局输入。

## 优雅关闭顺序

监听 `RunEvent::ExitRequested` 后按依赖逆序关闭（见 `src-tauri/src/lib.rs` 末尾 `app.run`）：

```
1. 停止事件源:     HotkeyService → TrayService → TimerService → DetectorService
2. 停止效果执行器:  WindowService → SoundService
3. 停止基础设施:    I18nService → ConfigService
```

- 每个 `shutdown()` MUST 走 `shutdown_service()` 包装，超时 3 秒后强制放弃（`tokio::time::timeout(Duration::from_secs(3), ...)`）
- 失败或超时 MUST 记录 `warn!`，MUST NOT 静默
- StatService（P2）就位后插入到 `I18nService → ConfigService` 之间

## 服务间通信

服务间 MUST 使用类型化 channel，MUST NOT 引入中心事件总线：

```
ConfigService    --tokio::sync::watch-->  TimerService, TrayService, ...
TimerService     --Effect via ServiceContext-->  WindowService, TrayService, SoundService
DetectorService  --同步拉取 (current_skip_flags / capabilities)-->  TimerService loop / Settings command
SoundService     --tokio::sync::mpsc-->  内部音频线程
```

- watch channel 用于"广播最新值"语义，订阅者拿到 `Arc<Config>` 后 lock-free 读取
- 副作用通过 `Effect` 枚举集中分派，定义在 `src-tauri/src/services/timer/effect.rs`，执行在 `src-tauri/src/services/context.rs::execute_timer_effect`
- `DetectorService` MUST 保持薄包装：平台能力通过 `PlatformApi` 同步返回 `bool` / `Option<Duration>`，timer 只在 `current_skip_flags` 中拉取；Settings 只通过 `get_detector_capabilities` 读取能力，不直接依赖 platform 层。
- 发送方 MUST NOT 关心接收方实现细节
- 接收方 MUST 处理 channel 关闭（`RecvError`）的情况，关闭时不得 panic

## Rust 可见性

| 对象 | 可见性 | 说明 |
|------|--------|------|
| Service struct（如 `TimerService`） | `pub(crate)` | 仅 crate 内可见，对应 `services/mod.rs` 中字段 |
| Service 对外方法（供 command 调用） | `pub(crate)` | 例：`TimerService::handle_user_event` |
| Service 内部方法 | `fn`（private） | 实现细节，例：`TimerService::sync_runtime_config` 仅在 mod 内可见时 |
| 纯函数（状态机核心） | `pub(crate)` | 便于单元测试，例：`resolve_user_event`、`step_time` |
| `models/` 中前端可见类型 | `pub` + `#[cfg_attr(test, derive(ts_rs::TS))]` | 例：`Config`（`src-tauri/src/models/config.rs`） |
| `models/` 中仅后端共享类型 | `pub(crate)` | 不导出到 ts-rs |
| `commands/` 中函数 | `pub(crate)` | 通过 `tauri::generate_handler!` 注册 |

- `mod.rs` MUST 只重导出该模块的公开 API，MUST NOT 包含实现代码
- MUST NOT 使用 `pub use *` 通配重导出
