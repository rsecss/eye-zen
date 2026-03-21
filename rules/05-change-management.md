# 变更管理

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 变更影响清单

每种变更类型 MUST 完成对应清单的所有项目，缺一不可。

### 新增 Tauri Command

- [ ] 在 `commands/` 中定义 `#[tauri::command]` 函数
- [ ] 在 `models/` 中定义请求/响应类型（如有新类型）
- [ ] 类型添加 `#[derive(TS)]` 和 ts-rs 导出测试
- [ ] 在 `lib.rs` 的 `invoke_handler` 中注册
- [ ] 在 `src-tauri/permissions/` 中定义对应 permission
- [ ] 在对应窗口的 `capabilities/*.json` 中引用 permission
- [ ] 在 `src/lib/commands.ts` 中添加前端封装
- [ ] 运行 `cargo test` 刷新 ts-rs 绑定并确认前端类型一致
- [ ] 写至少一条边界测试

### 新增 Service

- [ ] 在 `services/` 中创建模块文件
- [ ] 实现四阶段生命周期（`new` → `init` → `start` → `shutdown`）
- [ ] 添加到 `AppServices` struct
- [ ] 在 `setup()` 中按依赖顺序初始化
- [ ] 更新 `rules/01-architecture.md` 的依赖 DAG
- [ ] 更新关闭顺序
- [ ] 定义服务间 channel 类型（如有通信需求）
- [ ] 写单元测试覆盖核心逻辑

### 新增 Event

- [ ] 在 `models/events.rs` 中定义 Event 名称常量和 Payload 类型
- [ ] Payload 类型添加 `#[derive(TS)]`
- [ ] 运行 `cargo test` 刷新 ts-rs 绑定
- [ ] 在 `src/lib/events.ts` 中添加前端监听封装

### 变更 tauri.conf.json / 窗口配置

- [ ] 更新 `tauri.conf.json` 中的窗口配置
- [ ] 如新增窗口：同步添加 HTML 入口、TS entry、Page 组件、Vite rollupOptions
- [ ] 更新对应 capability 文件
- [ ] 验证窗口标签（label）在所有引用处一致

### 变更 Config Schema

- [ ] 更新 `models/` 中的 Config struct
- [ ] 更新 TOML 默认值模板
- [ ] 确认向后兼容（见下方规则）
- [ ] 更新 `rules/07-platform-storage.md` 的配置结构
- [ ] 更新前端设置页（如有 UI 对应项）
- [ ] 写配置解析的边界测试

### 新增平台能力

- [ ] 在 `PlatformApi` trait 中添加方法
- [ ] 在三个平台文件中实现（或降级）
- [ ] 更新 `rules/07-platform-storage.md` 的能力矩阵
- [ ] 降级实现 MUST 遵循保守原则

### 新增前端页面/组件

- [ ] 组件放在对应 `pages/<window>/components/` 或 `lib/components/` 目录
- [ ] 类型从 `$lib/bindings/` 导入
- [ ] Props 使用 `$props()` + callback 模式，MUST NOT 使用 `bind:`
- [ ] 超过 200 行时按职责评估是否需要拆分
- [ ] 可复用组件 MUST 写组件测试（渲染 + 交互 + 边界值）
- [ ] 页面组件 SHOULD 写关键交互路径测试
- [ ] CSS 颜色 MUST 使用 `app.css` 中的 CSS 变量，MUST NOT 硬编码色值
- [ ] 确认 `npm run build` + `svelte-check` + `npm test` 全部通过

## 配置向后兼容

### 规则

- 新增字段 MUST 提供默认值，旧配置文件 MUST 能正常加载
- 重命名字段 MUST NOT 直接操作，先新增 → 迁移 → 移除旧字段（跨两个版本）
- 删除字段 MUST 在发布说明中标注
- 配置解析失败 MUST 保留 `.bak` 备份，MUST NOT 覆盖为默认值

### 迁移策略

```rust
// ConfigService 加载时检查
fn migrate_if_needed(config: &mut Config) {
    // 新增字段：缺失时填充默认值
    // 类型变更：尝试转换，失败则用默认值 + warn 日志
}
```

- MAY 在 Config struct 中添加 `config_version: u32` 用于未来迁移
- 当前阶段（MVP）SHOULD 简单处理：缺失字段用默认值填充

## 破坏性变更协议

### 什么构成破坏性变更

- 改变 IPC Command 的签名（参数类型/返回类型）
- 改变 Event Payload 的结构
- 移除/重命名 Config 字段
- 改变状态机的转换规则
- 改变服务间 channel 的消息类型

### 处理流程

1. 提交信息 MUST 使用 `!` 标记：`feat(api)!: redesign config update commands`
2. 提交 body MUST 包含 `BREAKING CHANGE:` 段落
3. 同时变更的前端代码 MUST 在同一 PR 中
4. MUST 更新对应的 rules 文档

## 发版清单

每次发版 MUST 按顺序完成以下清单，MUST NOT 跳步。

### 1. 本地验证（在 dev 分支上）

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npx svelte-check --tsconfig ./tsconfig.json
npm test -- --run
npm run format:check
npm run build
```

全部通过后才可进入下一步。

### 2. 推送 dev 并等待 CI 绿灯

```bash
git push origin dev
# 等待三平台 CI 全部通过
```

MUST NOT 在 CI 未通过时合并到 main。

### 3. 使用 PR 工作流合并

- MUST 通过 PR 合并到 main（`docs/workflows/pr.md`）
- MUST NOT 直接 `git merge dev` 到 main
- PR 描述 MUST 包含版本号和变更摘要
- 推荐 squash merge 保持 main 历史干净

### 4. 版本号同步

三个文件 MUST 同步更新（`docs/workflows/release.md`）：

| 文件 | 字段 |
|------|------|
| `package.json` | `"version"` |
| `src-tauri/Cargo.toml` | `version` under `[package]` |
| `src-tauri/tauri.conf.json` | `"version"` |

额外引用（手动更新）：README badge、AboutPage 版本号。

### 5. Tag 和 Release

```bash
git checkout main && git pull origin main
git tag v<VERSION>
git push origin v<VERSION>
```

Tag MUST 在 main 上创建，MUST NOT 在 dev 上创建。

### 6. 验证 Release 制品

- 等待 `release.yml` 四目标构建完成
- 检查 Draft Release 中所有制品命名正确
- 手动 Publish Release

### 第三方 Action 版本管理

- `tauri-apps/tauri-action` MUST 使用 `@v0`（v1 tag 已被上游删除）
- SHOULD 在 dependabot 或手动检查中定期验证 Action 版本可用性
- 关键 Action SHOULD 考虑 pin 到具体 commit SHA

## 依赖管理

### 引入新依赖前 MUST 评估

| 维度 | 要求 |
|------|------|
| 必要性 | 标准库或已有依赖能否解决？ |
| 维护状态 | 最近 6 个月有更新？star 数？ |
| 体积影响 | 对 bundle size / 编译时间的影响 |
| 许可证 | MIT / Apache-2.0 / BSD（不接受 GPL） |
| 替代方案 | 至少考虑一个替代方案 |

### 版本策略

- npm 和 Cargo 依赖 MUST 使用 tilde 锁定（`~x.y.z`）
- MUST NOT 使用 `*` 或 `>=` 版本范围
- 依赖更新 MUST 单独提交，不与功能代码混合
