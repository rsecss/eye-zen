# 规则索引

> 所有开发 **MUST** 遵循本目录下的规则文档。违反规则的代码不得合入。

## 按文件浏览

| 文件 | 覆盖范围 |
|------|---------|
| [01-architecture.md](01-architecture.md) | 分层依赖、服务 DAG、生命周期、可见性、通信、关闭顺序 |
| [02-ipc-and-state.md](02-ipc-and-state.md) | Commands/Events 定义、状态机、错误类型、边界验证 |
| [03-coding-standards.md](03-coding-standards.md) | 命名规范、Rust/Svelte 规范、错误传播、日志、注释 |
| [04-testing-quality.md](04-testing-quality.md) | 测试要求、质量门禁、CI 配置、性能预算 |
| [05-change-management.md](05-change-management.md) | 变更清单、发版清单、配置兼容、破坏性变更、依赖管理 |
| [06-frontend.md](06-frontend.md) | 前端架构、状态管理、窗口、权限、视觉设计 |
| [07-platform-storage.md](07-platform-storage.md) | 平台抽象、降级原则、配置读写、数据存储 |

## 按角色快速定位

### 我在写后端 Rust 代码

| 要做的事 | 先看 |
|---------|------|
| 新增/修改 Service | [01-architecture.md](01-architecture.md) — 依赖 DAG、生命周期、可见性 |
| 新增 Tauri Command | [02-ipc-and-state.md](02-ipc-and-state.md) — 接口定义、边界验证 |
| 修改状态机 | [02-ipc-and-state.md](02-ipc-and-state.md) — 状态转换、Effect、纯函数约束 |
| 错误处理 / 锁 / 异步 | [03-coding-standards.md](03-coding-standards.md) — Rust 规范、错误传播链 |
| 平台相关代码 | [07-platform-storage.md](07-platform-storage.md) — PlatformApi trait、降级 |
| 配置读写 | [07-platform-storage.md](07-platform-storage.md) — ConfigService、原子写入 |

### 我在写前端 Svelte 代码

| 要做的事 | 先看 |
|---------|------|
| 新增页面/组件 | [06-frontend.md](06-frontend.md) — 文件结构、组件约束 |
| 调用后端 API | [06-frontend.md](06-frontend.md) — IPC 封装、状态管理 |
| 管理状态 | [06-frontend.md](06-frontend.md) — store 规则、禁止乐观更新 |
| 样式/视觉 | [06-frontend.md](06-frontend.md) — CSS 变量、设计体系 |

### 我在做变更/提交

| 要做的事 | 先看 |
|---------|------|
| 提交代码 | [04-testing-quality.md](04-testing-quality.md) — 门禁、测试要求 |
| 新增功能（完整流程） | [05-change-management.md](05-change-management.md) — 变更影响清单 |
| 新增依赖 | [05-change-management.md](05-change-management.md) — 依赖评估标准 |
| 改配置 schema | [05-change-management.md](05-change-management.md) — 向后兼容规则 |
