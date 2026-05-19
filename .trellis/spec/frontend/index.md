# 前端 Spec

> Eyezen 前端（Svelte 5 Runes + Vite 6 多入口 + Tauri v2 IPC）开发约束。
> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选。

---

## 文件清单

| 文档 | 适用对象 | 主要内容 |
|------|---------|---------|
| [directory-structure.md](./directory-structure.md) | 新增窗口 / 调整目录 | Vite 多入口、HTML/entries/pages/lib 分层、组件放置原则 |
| [component-guidelines.md](./component-guidelines.md) | 写 `.svelte` 文件 | Svelte 5 Runes、Props/Snippet、200 行约束、即时保存模式、CSS 变量 |
| [store-and-ipc-patterns.md](./store-and-ipc-patterns.md) | 写 store、调 IPC | `$state` + version counter、`init/destroy` 生命周期、`invokeWithTimeout`、`listen` 封装 |
| [state-management.md](./state-management.md) | 处理后端推送数据 | 单一数据源、禁止乐观更新、禁止本地副本、即时保存数据流 |
| [type-safety.md](./type-safety.md) | 接触 IPC / DTO | ts-rs 桥接、`$lib/bindings/` 不可手编、跨边界 `u32`、callback 精确类型 |
| [quality-guidelines.md](./quality-guidelines.md) | 提 PR / 改 capability | Vitest 维度、svelte-check / format / build 必过、capability/permission 最小化、新增页面/组件清单 |

---

## 阅读路径

- **新加一个窗口**：directory-structure → quality-guidelines（新增清单）
- **改 Settings / 加配置项**：state-management（即时保存）→ component-guidelines → type-safety
- **调用新 command 或监听新 event**：store-and-ipc-patterns → type-safety
- **审查 PR**：quality-guidelines 优先

---

## 与 backend/ spec 的区别

| 边界 | 前端 spec 覆盖 | backend spec 覆盖 |
|------|---------------|------------------|
| Tauri command 定义 | 仅消费侧（`commands.ts` 封装） | 服务端实现 + permission 文件 |
| Event 监听 | 仅订阅侧（`events.ts` 封装、unlisten 清理） | Payload 类型、emit 时机、event 名常量 |
| 类型 | 使用 `$lib/bindings/` 生成产物 | 用 `#[derive(TS)]` 写 ts-rs 源 |
| 配置 | UI 控件 + 即时保存数据流 | TOML schema、迁移、原子写入 |

跨边界变更必须同时阅读 backend/ 对应章节，并满足 `quality-guidelines.md` 中的"新增 Command/Event 影响清单"。
