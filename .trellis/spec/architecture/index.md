# Architecture Spec

> Eyezen 跨层契约：分层依赖、IPC/状态机接口、变更管理、质量门禁。
> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选。

---

## 文件清单

| 文档 | 适用场景 | 主要内容 |
|------|---------|---------|
| [layering.md](./layering.md) | 新增模块 / 调整目录 | `commands → services → platform` 单向依赖、`models/` 拆分（config/timer/error/events/channels）、`pub`/`pub(crate)` 边界 |
| [ipc-and-state.md](./ipc-and-state.md) | 改 IPC / 改状态机 | Commands 表、Events 表、`AppError`、输入校验/超时/错误返回、Timer 状态机（状态/转换/三纯函数/Effect/SkipFlags） |
| [change-management.md](./change-management.md) | 任何跨层变更 / 发版 | 新增 Command/Service/Event/平台能力/前端页面的影响清单、配置向后兼容、破坏性变更协议、发版清单、依赖管理 |
| [testing-quality.md](./testing-quality.md) | 提 PR / 调 CI / 发版 | 测试工具表、变更类型测试要求、husky pre-commit/pre-push、CI 三平台矩阵、Instant 测试陷阱、性能预算 |

---

## 与 backend/、frontend/ 的边界

`architecture/` 只覆盖**跨层、跨实现的契约**。具体实现细节交给下游：

| 主题 | 这里（architecture/） | 下游 spec |
|------|---------------------|----------|
| 分层依赖图 / `models/` 拆分 / 可见性矩阵 | layering.md | -- |
| 服务依赖 DAG / 生命周期 / 关闭顺序 / 锁策略 | 仅 1 句话引用 | [`backend/service-pattern.md`](../backend/service-pattern.md) |
| 命名规范 / 错误传播 / 日志 / Rust 风格 | -- | [`backend/coding-standards.md`](../backend/coding-standards.md) |
| `#[tauri::command]` 写法 / `spawn_blocking` / 校验函数 | IPC 边界规则（接口层） | [`backend/coding-standards.md`](../backend/coding-standards.md) |
| Timer 状态机契约（状态/转换/纯函数签名） | ipc-and-state.md | 实现细节见 `src-tauri/src/services/timer/` |
| Svelte 组件 / store / IPC 封装 | -- | [`frontend/`](../frontend/index.md) |
| 平台抽象 / 降级策略 / TOML / 原子写 | -- | [`backend/platform-storage.md`](../backend/platform-storage.md) |

---

## 阅读路径

- **新增一个 Tauri Command**：change-management.md 新增 Command 清单 → ipc-and-state.md 校验/超时规则 → `backend/coding-standards.md` → `frontend/quality-guidelines.md`
- **新增一个 Service**：change-management.md 新增 Service 清单 → `backend/service-pattern.md`（DAG + 生命周期）→ layering.md（可见性）
- **改 Timer 状态机**：ipc-and-state.md（状态机契约 + Effect）→ testing-quality.md（fix 先写测试 / Instant 陷阱）→ change-management.md（破坏性变更协议）
- **发版**：change-management.md 发版清单 → testing-quality.md pre-push 全量 → [`docs/workflows/release.md`](../../../docs/workflows/release.md)
- **审 PR**：testing-quality.md 变更类型测试要求 + change-management.md 对应清单
