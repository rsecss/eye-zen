# 修复系统审计 P1 问题

## Goal

按严重度一次性修复系统审计确认的全部 6 项 P1，使“配置 → 计时 → 提醒 → 用户动作 → 统计持久化”承重主路径在非法配置、超时、并发、队列饱和、导出失败和窗口创建失败时保持可预测、可验证且不损坏用户数据。

## Confirmed Facts

- 本任务按严重度分组，覆盖审计报告 P1-01 至 P1-06；不拆成一问题一任务。
- P1-01：Pomodoro `focus_minutes` 与 `timer.pre_alert_seconds` 分段合法但组合可非法，最终触发 timer `expect` panic；启动 TOML 加载也未执行完整配置语义校验。
- P1-02：当前状态机和设置文案采用“Alerting 超时自动进入 Resting”，但 `.trellis/spec/architecture/ipc-and-state.md` 定义超时回到 Working 并计为 Skipped；当前自动 Resting 最终会记录 Taken。
- P1-03：设置页以旧的完整 section 快照并发发送覆盖请求，后完成的请求可能覆盖先完成字段。
- P1-04：Stat writer 队列满时 `try_send(Shutdown)` 可丢失 sentinel，而 service 仍持有 sender，`handle.await` 可永久等待到外层超时。
- P1-05：统计导出覆盖已有文件时先删除旧文件再执行 `VACUUM INTO`，第二步失败会丢失旧备份。
- P1-06：提醒窗口创建是 fire-and-forget；零窗口成功时状态机仍推进，且主窗口使用 `focused=true`。
- 当前基线 `npm run ci` 通过，Rust 267 项测试通过；上述问题属于现有测试未覆盖的负路径。
- `.codex/config.toml` 是用户已有改动，不属于本任务，不得暂存、修改或提交。
- 已确认 Alerting 超时合同：自动关闭并回到 Working，记录 `CycleOutcome::Skipped`，不创建 RestSession；只有用户明确开始休息才能进入 Resting 并最终记录 Taken。
- 已确认统计导出保留“覆盖已有备份”能力：Save 对话框只提供目标路径，后端拥有安全替换合同；任何导出或替换失败都必须保留旧目标。
- 已确认提醒窗口默认不得抢焦点；Window Module 自己选择唯一交互主 Surface，调用者不再传递 `focused` 决策。
- 已确认 presentation failure 统计合同：若没有可交互提醒窗口成功显示，则恢复 Working、记录运行错误，但不写 Taken/Skipped/Suppressed，也不创建 RestSession。

## Requirements

- 统一完整配置的语义校验边界，所有 TOML 加载与相关 IPC 更新只能向 Timer 发布满足 active work duration 不变量的配置。
- 将 Alerting 超时统一为自动关闭、回到 Working 并记录 Skipped；同步状态机、统计 outcome、设置文案、规范和行为测试。
- 为同一配置 section 的快速连续修改建立不会丢字段的完整保存操作；冲突或失败必须用户可见，不得静默覆盖或自动吞错。
- 关闭 Stat writer 时必须可靠排空已接收写入并终止；队列满、重复 shutdown 和 writer 失败均有确定合同。
- 导出使用同目录临时数据库，完成 SQLite 校验与落盘后再执行平台可靠替换；Windows 使用 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`，Unix 使用同文件系统 rename，不得先删旧文件。
- Window presentation 必须返回 typed receipt；零窗口成功不得记为 Taken，部分成功只产生一个可操作/可宣告主 Surface，提醒默认不得抢占当前应用焦点。
- 每项修复先建立可失败回归测试，再实施最小修复；不得新增兼容双读、silent fallback、flag 驱动的平行业务流程或无真实变化点的 seam。
- 只修改完成 P1 修复所需的产品代码、测试和规范；不顺手处理 P2/P3。

## Acceptance Criteria

- [ ] P1-01：TOML load、`update_timer_config`、`update_pomodoro_config`、mode switch 对同一非法组合一致拒绝，任何边界接受的配置都不能使 timer task panic。
- [ ] P1-02：超时后的 state、cycle outcome、RestSession、UI 文案与规范完全一致；只有符合已确认产品语义的路径才能产生 Taken。
- [ ] P1-03：两个不同字段更新逆序完成时，最终持久化配置和 store 同时保留两项修改；错误在相关控件附近可见。
- [ ] P1-04：填满 Stat channel 后 shutdown 仍能排空、终止并在确定时限内返回；重复 shutdown 行为有测试。
- [ ] P1-05：向已有目标注入导出/替换失败时，旧文件内容保持不变；成功产物可打开并核对数据。
- [ ] P1-06：0/N、部分 N、全部 N 窗口创建均有测试；零窗口绝不产生 Taken，部分成功只提供一个交互主 Surface，焦点保持在原应用。
- [ ] 相关 architecture/backend/frontend 规范与最终合同同步，不留下已知实现—规范矛盾。
- [ ] `npm run ci`、针对新增负路径的回归测试和必要的 cargo-deny/diff 检查通过；未实测平台明确标注。
- [ ] 最终提交不包含 `.codex/config.toml`，不包含 P2/P3 修复或无关格式化。

## Out of Scope

- P2/P3 审计问题；P1 完成后另建 P2 修复任务。
- 新功能、依赖升级、数据库 schema 迁移、整库重写、发布和推送远端。
- 为旧开发期合同保留双路径兼容层。

## Open Questions

- 无阻塞问题。

## Notes

- 这是跨 architecture/backend/frontend 的复杂任务，必须补齐 `design.md` 与 `implement.md` 后再进入实现。
