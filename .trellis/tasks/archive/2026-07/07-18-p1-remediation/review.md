# P1 修复 Review 记录

## 2026-07-18 补丁审查

### 总体结论

尽管 `npm run ci` 和补丁格式检查均通过，慢 I/O、revision 冲突及残留窗口等负路径仍可能造成统计丢失、系统副作用与配置不一致或用户修改静默丢失。当前实现尚未完整满足本次 P1 修复合同。

本轮 review 共提出 6 条评论：4 条 P1、2 条 P2。它们均针对本任务实现引入或尚未闭合的合同缺口，当前统一标记为待修复；修复并重新通过完整质量门前不得提交。

### 完整 Review Comments

#### [P1] 避免通用超时取消统计排空

- 位置：`src-tauri/src/services/stat/mod.rs:421-428`
- 状态：已修复
- 原文：当写入排空超过 3 秒时（例如统计导出占用唯一 SQLite 连接），`lib.rs` 的通用 shutdown timeout 会取消这里的 `send`/`handle.await`；此时 sender 和 handle 已从服务中取出，writer 被留在后台，而应用随即退出，已接受的统计记录可能尚未落盘。统计关闭需要不受该通用超时取消，或在超时后仍明确接管排空任务。

#### [P1] 按权威配置恢复自启动状态

- 位置：`src/pages/main/settings/AutoStartSection.svelte:23-24`
- 状态：已修复
- 原文：当先前的慢写入在前端超时后继续提交，而用户重试导致 `ConfigConflict` 时，store 会先刷新到最新配置再让此 Promise 拒绝；下面的 catch 却始终执行请求值的反操作。例如最新配置已是 `auto_start=true` 时仍调用 `disable()`，最终造成 TOML 与系统自启动状态不一致。回滚目标应取刷新后的权威值，而不是简单取反。

#### [P1] 在应用快捷键副作用前校验 revision

- 位置：`src-tauri/src/commands/mod.rs:319-322`
- 状态：已修复
- 原文：当一个慢 hotkey 写入尚未提交、前端超时后又发起旧 revision 请求时，新快捷键会先被注册；前一个请求随后提交会使这里返回冲突，而回滚使用的是提交前捕获的旧绑定。根据 watcher 与命令任务的调度顺序，最终运行中的快捷键可能被恢复为旧值，但持久化配置已经是新值。revision 校验和副作用必须处于同一串行操作中。

#### [P1] 保留已排队冲突的可见错误

- 位置：`src/lib/stores/config.svelte.ts:109-110`
- 状态：已修复
- 原文：当同一 scope 已排队多个 intent 且其中一个发生冲突时，下一项会自动执行并立即清除此错误；若下一项成功，前一个未应用的字段修改将没有任何持续可见提示。这正是快速连续修改场景，错误甚至可能在同一轮微任务中被清掉。应只在用户于冲突出现后明确重试相应 intent 时清除它。

#### [P2] 不要让旧 refresh 响应覆盖新事件

- 位置：`src/lib/stores/config.svelte.ts:115-118`
- 状态：已修复
- 原文：当 `getConfig()` 读取 revision N 后，revision N+1 的 `config_changed` 事件先于命令响应到达时，这里的无条件 `replaceSnapshot` 会把 store 从 N+1 回退到 N，绕过本文件其余路径的 stale revision 过滤。冲突刷新也必须通过单调 revision 检查应用。

#### [P2] 记录现有交互窗口的实际显示器

- 位置：`src-tauri/src/services/window.rs:135-140`
- 状态：已修复
- 原文：当主屏创建失败后交互窗口曾被提升到副屏，并因关闭失败等原因在下一轮仍存在时，这里仅凭固定 label 就把它误认为位于 `primary_index`。随后 minimal 创建会跳过主屏并在实际副屏上再叠一层窗口，导致主屏无提醒、同一副屏重复覆盖。复用现有窗口时需要保留或重新确定其真实显示器。

### 质量门状态

- `npm run ci`：复修后通过；Rust 296 项、前端 143 项、`svelte-check` 0 错误/0 警告、格式检查与构建通过。
- `cargo deny --manifest-path src-tauri/Cargo.toml check`：退出码 0；仅保留既有 duplicate/yanked/license allowance warnings。
- `git diff --check -- . ':(exclude).codex/config.toml'`：通过。
- `trellis-check`：通过；期间发现并修复 2 个 clippy 问题，最终 `clippy --all-targets --features plugin-shortcuts -D warnings` 通过。
- 当前结论：6 条 review 评论已闭合，进入 Phase 3.4 提交确认。
- 提交状态：未提交、未 push；`.codex/config.toml` 继续排除。

### 复修结果

| Review 项 | 根因修复 | 回归证据 |
|---|---|---|
| Stat shutdown timeout cancellation | `lib.rs` 对 Stat 使用 `shutdown_service_to_completion`：3 秒仅告警，随后继续 await 同一 pinned future | `critical_shutdown_continues_after_warning_timeout`；Stat writer 8 项聚焦测试 |
| AutoStart 权威恢复 | 写入失败后读取 `configStore.current.behavior.auto_start`，按权威 true/false 调用 plugin | `SettingsPage.handlers.test.ts` 两个 conflict refresh 用例 |
| Hotkey revision TOCTOU | `ConfigService::update_hotkeys` 在同一 write lock 内完成 revision check、apply、save、rollback | `stale_hotkey_update_cannot_apply_side_effect_while_first_update_is_slow` |
| 排队 intent 清除 conflict | scope error generation 在 enqueue 时捕获；只允许错误可见后的 retry 清除同一 generation | Config Store queued-before-conflict 与 explicit-retry 用例 |
| stale conflict refresh | conflict refresh 改走 `applyNewerSnapshot` 单调过滤 | revision N refresh 晚于 N+1 event 用例 |
| 残留交互窗口显示器 | `WindowPort::window_outer_position` 查询现有窗口物理位置并匹配 monitor | `existing_interactive_on_secondary_keeps_primary_minimal_without_secondary_overlap` |

### Bug Analysis: 绿灯后仍遗漏异步负路径

#### 1. Root Cause Category

- **Category B — Cross-Layer Contract**：revision、系统副作用、事件和错误可见性分别由不同层处理，但最初没有把它们建模为同一时间轴。
- **Category D — Test Coverage Gap**：既有测试覆盖单次成功/失败，没有覆盖 timeout 后提交、stale retry、queued intent、残留资源和 future cancellation。
- **Category E — Implicit Assumption**：实现默认“label 等于显示器”“refresh 一定最新”“timeout 等于未提交”“后续 intent 可以清错”，这些假设均未写入合同。

#### 2. Why the First Fix Was Incomplete

1. revisioned snapshot 修复了普通 event/response 逆序，但 conflict refresh 绕过了单调过滤。
2. command 层 rollback 修复了单次保存失败，却把 revision check 与 OS apply 留在两个串行域，仍有 TOCTOU。
3. writer sender 唯一所有权修复了 queue-full sentinel，却没有检查调用方 timeout 对 shutdown future 的取消。
4. presentation receipt 修复了创建结果反馈，却仍把固定 label 当作物理 placement。

#### 3. Prevention Mechanisms

| Priority | Mechanism | Specific Action | Status |
|---|---|---|---|
| P0 | Architecture | revision check、外部副作用、持久化与 rollback 由一个完整操作拥有 | DONE |
| P0 | Test Coverage | 为 timeout/conflict/queue ordering/resource residue 增加确定性并发测试 | DONE |
| P1 | Frontend Contract | 所有 snapshot source 统一走单调 gate；错误清理由 generation 控制 | DONE |
| P1 | Lifecycle Contract | 持有排空 ownership 的 future 超时后继续 await，不得直接 drop | DONE |
| P1 | Knowledge Capture | 更新 architecture/backend/frontend spec 与 cross-layer checklist | DONE |

#### 4. Systematic Expansion

- 类似风险面：任何“command timeout 后仍可能提交”的系统插件写入、任何 queue tail 自动清理 UI error 的 store、任何 stable label 但 movable placement 的窗口资源。
- 设计改进：调用者只触发完整操作，不编排“check → side effect → save → rollback”步骤；所有异步来源共享单调时间轴。
- 流程改进：质量门除成功路径外，必须至少覆盖 slow I/O、out-of-order、cancellation 和残留资源四类负路径。
