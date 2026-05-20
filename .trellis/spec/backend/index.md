# Backend Spec 索引

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

本目录收录 Eyezen 后端（Rust / Tauri v2）实现指南。所有规则均以 `src-tauri/src/` 下的真实代码为锚点，禁止与之冲突的写法合入。

## 与 `architecture/` 的区别

- `architecture/` 描述跨层契约（IPC、状态机、变更管理）和前后端共同遵守的规则
- `backend/` 仅约束 Rust 后端实现细节：模块组织、生命周期、错误、日志、平台抽象、配置存储
- 若同一议题两边都涉及，`backend/` 关注 "怎么在 Rust 里落地"，`architecture/` 关注 "跨进程契约"

## 文件清单

| 文件 | 主题 | 何时读 |
|------|------|--------|
| [`service-pattern.md`](./service-pattern.md) | 服务依赖 DAG、四阶段生命周期、关闭顺序、AppServices 注册、服务间 channel | 新增 / 修改任一 service、调整启动或关闭顺序、设计服务间通信 |
| [`coding-standards.md`](./coding-standards.md) | Rust 命名、错误传播、锁策略（reducer）、异步、ts-rs 跨界类型、clippy、可见性 | 写任何 Rust 代码前；review Rust PR 时 |
| [`platform-storage.md`](./platform-storage.md) | `PlatformApi` trait、平台能力矩阵、降级原则、TOML 配置、ConfigService 设计、SQLite P2 schema | 新增平台能力、修改配置结构、设计存储相关功能 |
| [`error-and-logging.md`](./error-and-logging.md) | `AppError` 类型、错误传播分层、tracing 日志级别、敏感信息脱敏 | 新增错误变体、加日志、排查 IPC 报错链路 |

## 阅读顺序建议

1. 先读 `coding-standards.md` 建立基线
2. 再读 `service-pattern.md` 理解整体生命周期
3. 涉及配置/平台时读 `platform-storage.md`
4. 排查错误/日志问题时读 `error-and-logging.md`

## 真实源码锚点（导航）

- 应用入口与服务编排：`src-tauri/src/lib.rs`
- 服务 trait 与 `AppServices`：`src-tauri/src/services/mod.rs`
- 服务上下文与 effect 路由：`src-tauri/src/services/context.rs`
- 配置服务范式：`src-tauri/src/services/config.rs`
- 状态机服务：`src-tauri/src/services/timer/`
- 平台抽象：`src-tauri/src/platform/mod.rs`
- 错误类型：`src-tauri/src/error.rs`
- 日志初始化：`src-tauri/src/logging.rs`
- 跨界类型定义：`src-tauri/src/models/config.rs`、`src-tauri/src/models/types.rs`
