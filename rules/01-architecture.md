# 架构约束

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

## 分层依赖（单向，禁止反向）

```
commands/  →  services/  →  platform/
    ↓            ↓
  models/     models/
```

- `commands/` MUST 只依赖 `services/`、`models/`
- `services/` MUST 只依赖 `models/`、`platform/`、其他 `services/`（按 DAG）
- `platform/` MUST NOT 依赖 `services/`、`commands/`
- `models/` MUST NOT 依赖任何上层模块（纯数据定义）
- 前端 MUST NOT 直接访问 SQLite、文件系统或系统 API，只通过 IPC

### models/ 模块职责

`models/` 统一存放所有数据类型，按用途分文件：

```
models/
├── mod.rs
├── config.rs       # Config, TimerConfig, BehaviorConfig, DisplayConfig
├── timer.rs        # TimerState, Transition, Effect, SkipFlags, StatePayload
├── error.rs        # AppError, PlatformError
├── events.rs       # IPC event 名称常量 + payload 类型（前端可见，需 #[derive(TS)]）
└── channels.rs     # 服务间内部 channel 消息类型（pub(crate)，前端不可见）
```

- `models/events.rs` 中的类型 MUST 是 `pub` + `#[derive(TS)]`（前端需要）
- `models/channels.rs` 中的类型 MUST 是 `pub(crate)`（仅后端内部使用）
- 两者 MUST NOT 混放，以免内部消息类型意外暴露给前端

## 服务依赖 DAG

箭头表示"依赖于"，禁止循环：

```
ConfigService  ←  TimerService  ←  WindowService
                                ←  TrayService
                                ←  SoundService
                                ←  StatService (P2)
               ←  DetectorService → TimerService (via mpsc)
               ←  SoundService
               ←  TrayService
```

- **ConfigService** 是基础服务，MUST NOT 依赖其他服务
- **TimerService** 依赖 ConfigService（watch channel），MUST NOT 依赖 WindowService/TrayService/SoundService
- 副作用服务（Window/Tray/Sound/Stat）依赖 TimerService 的 broadcast，MUST NOT 反向调用 TimerService

## 服务注册

```rust
pub struct AppServices {
    pub config: ConfigService,
    pub timer: TimerService,
    pub detector: DetectorService,
    pub window: WindowService,
    pub stat: StatService,
    pub sound: SoundService,
    pub tray: TrayService,
    pub i18n: I18nService,
}
```

- 显式 struct，MUST NOT 使用 IoC/反射/trait object 注册
- `setup()` 中按依赖顺序初始化，所有 `init()` 在 `Arc` 包装前完成
- 包装后 `tauri::State<Arc<AppServices>>`（不可变共享引用）
- 每个服务管理自己的内部可变性（`Mutex<Inner>` 或 `ArcSwap`）

## 服务生命周期

每个服务 MUST 遵循四阶段生命周期：

| 阶段 | 方法 | 允许 | 禁止 |
|------|------|------|------|
| **构造** | `new()` | 初始化字段、创建 channel | I/O、网络、文件读写 |
| **初始化** | `init()` | 读配置、建连接、验证环境 | 启动后台任务 |
| **启动** | `start()` | 启动 tokio task、timer loop、polling | 阻塞主线程 |
| **关闭** | `shutdown()` | 发送停止信号、等待 task 结束、释放资源 | panic、忽略错误 |

- `new()` 和 `init()` MUST 在 `Arc` 包装前完成
- `start()` MUST 在 `Arc<AppServices>` 注册为 Tauri State 后调用
- `shutdown()` MUST 按依赖逆序执行（见下）

## 优雅关闭顺序

hook `RunEvent::ExitRequested`，按依赖安全逆序：

```
1. 停止事件源:     TrayService → TimerService → DetectorService
2. 停止效果执行器:  WindowService → SoundService
3. 停止基础设施:    StatService → ConfigService
```

- 每个 `shutdown()` MUST 有超时（建议 3 秒），超时后强制放弃
- MUST 记录关闭日志，不得静默失败

## 服务间通信

类型化 channel，MUST NOT 使用中心事件总线：

```
ConfigService  --watch channel-->  TimerService, TrayService, SoundService, ...
TimerService   --broadcast-->      WindowService, TrayService, StatService
DetectorService --mpsc-->          TimerService
```

- Channel 消息类型 MUST 在 `models/channels.rs` 中定义（`pub(crate)`）
- IPC event payload MUST 在 `models/events.rs` 中定义（`pub` + `#[derive(TS)]`）
- 发送方 MUST NOT 关心接收方的实现细节
- 接收方 MUST 处理 channel 关闭（`RecvError`）的情况

## Rust 可见性规则

| 对象 | 可见性 | 说明 |
|------|--------|------|
| Service struct | `pub(crate)` | 只在 crate 内可见 |
| Service 公开方法（供 command 调用） | `pub(crate)` | command 层可调用 |
| Service 内部方法 | `fn`（private） | 实现细节 |
| 纯函数（状态机核心） | `pub(crate)` | 便于单元测试 |
| models/ 中的 IPC 类型 (`events.rs`) | `pub` + `#[derive(TS)]` | 前端可见，ts-rs 导出 |
| models/ 中的内部类型 (`channels.rs`) | `pub(crate)` | 仅后端服务间使用 |
| models/ 中的共享类型 | `pub` | Config, AppError 等 |
| commands/ 中的函数 | `pub(crate)` | `generate_handler!` 宏注册即可 |

- `mod.rs` MUST 只重导出该模块的公开 API，MUST NOT 包含实现代码
- MUST NOT 使用 `pub use *` 通配重导出
