# P1 修复执行计划

## 0. 启动门槛

- [ ] 用户评审并批准 `prd.md`、`design.md`、`implement.md`。
- [ ] 运行 `task.py start` 后再修改产品代码。
- [ ] 记录 `git status`，确认 `.codex/config.toml` 为任务外用户改动且始终排除。
- [ ] 通过 `trellis-before-dev` 重新加载 architecture/backend/frontend 规范。

## 1. Config 不变量与 revisioned snapshot（P1-01/P1-03）

- [ ] 先写失败测试：四个配置入口接受非法 Pomodoro/pre-alert 组合；同 section 快速更新逆序完成丢字段；旧 config event 可覆盖新 snapshot。
- [ ] 新增 `ConfigSnapshot` ts-rs DTO 与 `AppError::ConfigConflict`，更新序列化测试。
- [ ] ConfigService 引入内部 `VersionedConfig`，在 write lock 内比较 revision、构造候选、完整校验、持久化、递增并发布。
- [ ] TOML 语义非法走 `.bak + defaults` 恢复；合法边界值测试覆盖 `==` 与 `+1s`。
- [ ] 更新 `get_config`、全部 `update_*_config` 和 `config_changed` event contract；重生成 bindings。
- [ ] Config Store 保存 revision、过滤 stale snapshot，并实现单一串行写队列与 section error。
- [ ] Settings sections 改为调用 Store 完整操作；保留 hotkey/autostart 失败回滚，删除直接配置 command 调用。
- [ ] 删除 Timer 内为非法 Config 服务的 `.max(1)`/无依据 fallback，并补 accepted-config 不 panic 的 property/regression tests。
- [ ] 聚焦验证：ConfigService tests、commands tests、config store tests、Settings handler tests、Timer tests、`npx svelte-check`。

## 2. Alerting timeout（P1-02）

- [ ] 先把 `alerting_timeout_auto_rest` 改为期望 Working/Skipped 并确认旧实现失败。
- [ ] 修改 `step_time`，复用现有 Alerting → Working effect/outcome 路径。
- [ ] 断言 timeout 后无 RestSession、产生一次 Skipped；显式 StartRest 才能产生 Taken。
- [ ] 同步中英文设置文案和 architecture state-machine contract。
- [ ] 聚焦验证：timer machine/service、stat writer/health、i18n key parity、Tip/Tray 相关行为测试。

## 3. Stat writer shutdown（P1-04）

- [ ] 先写 queue-full shutdown 失败测试和重复 shutdown 测试。
- [ ] 移除 StatService Clone，writer sender 改为可 take 的唯一所有权。
- [ ] shutdown 使用 `send(Shutdown).await`，drop sender 后 join，再关闭 pool；receiver closed 返回上下文错误。
- [ ] 验证已入队记录全部持久化，queue full 不丢 sentinel，重复 shutdown 幂等。
- [ ] 聚焦验证：StatService/writer 并发测试和服务关闭顺序测试。

## 4. 原子导出覆盖（P1-05）

- [ ] 先写 failure Adapter 回归测试：已有 target + 替换失败时旧 bytes 不变；临时文件被清理。
- [ ] 添加导出串行锁、同目录唯一临时路径和临时 SQLite 验证。
- [ ] 添加私有 `AtomicFileReplace` seam 与 Unix/Windows/Test Adapters。
- [ ] Windows dependency 仅增加 `Win32_Storage_FileSystem` feature，使用 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)`；不新增依赖版本。
- [ ] 保留成功覆盖测试，补空库、目标不存在、quick_check 失败与并发导出测试。
- [ ] 聚焦验证：stat export tests；Windows clippy/check；Linux/macOS 由 CI parity 验证并明确本机未实测项。

## 5. Window presentation receipt（P1-06）

- [ ] 先写 WindowService 0/N、partial、all、primary failure promotion、existing main/minimal 和 `focused=false` 测试。
- [ ] `show_tip_windows` 返回 receipt；Window Module 选择唯一交互主 Surface，删除 `TipWindowSpec.focused`。
- [ ] EffectSink/effect executor 返回 presentation receipt；Timer 新增独立 presentation failure 恢复操作。
- [ ] 测试 Alerting/直接 Resting 呈现失败均恢复 Working，且不产生 CycleEvent/RestSession。
- [ ] 测试 receipt 返回后用户动作已改变状态时不回滚新状态；部分成功只保留一个交互主 Surface。
- [ ] 聚焦验证：window、effect executor、timer machine/service/context tests；前端 Tip/Tray smoke tests。

## 6. 规范与全量质量门

- [x] 同步 `architecture/ipc-and-state.md` 的 ConfigSnapshot、conflict、timeout 与 receipt 合同。
- [x] 同步 `backend/platform-storage.md` 的 revision/导出替换与 `service-pattern.md` 的 writer shutdown。
- [x] 同步 `frontend/state-management.md`、`store-and-ipc-patterns.md` 的 Store 写入所有权与错误呈现。
- [x] 运行 `npm run ci`。
- [x] 运行 `cargo deny --manifest-path src-tauri/Cargo.toml check`；如需要联网更新 advisory DB，先请求授权。
- [x] 运行 `git diff --check -- . ':(exclude).codex/config.toml'`。
- [x] 复核生成物、capability/event allowlist、版本文件与 lockfile 没有无关变化。
- [x] 运行 `trellis-check` 全范围检查；发现问题回到对应切片修复并重跑。

## 7. 提交与回滚点

- [ ] 按 Config、timeout、writer、export、presentation、spec/quality 的逻辑切片拟定提交；不按文件拆提交。
- [ ] 每个切片保持可独立 revert；不得 amend、不得 push。
- [ ] `.codex/config.toml` 始终列为未识别/任务外路径，不暂存。

## 8. 补丁 Review 复修（2026-07-18）

完整审查原文见 [`review.md`](./review.md)。本轮 quality gate 未通过，回到 Phase 2.1；以下问题全部闭合并重跑第 6 节质量门前，不得进入提交阶段。

- [x] Stat shutdown 不受 `lib.rs` 通用 3 秒 timeout 取消，或超时后仍由明确 owner 接管排空；补慢 I/O 排空回归测试。
- [x] AutoStart 失败恢复以冲突刷新后的权威 `auto_start` 值为目标，不再对请求值简单取反；补权威值为 true/false 的冲突测试。
- [x] Hotkey revision 校验、系统绑定副作用、持久化及失败回滚处于同一 ConfigService 串行操作；补并发慢请求一致性测试。
- [x] Config Store 保留已排队 intent 产生的 conflict error，只有错误出现后的显式重试才能清除；补快速连续修改测试。
- [x] conflict refresh 通过单调 revision 规则应用，旧响应不得覆盖更新事件；补 revision N/N+1 逆序测试。
- [x] Window 复用现有交互窗口时识别其实际显示器；补副屏残留交互窗口场景，确保主屏得到 minimal 且副屏不重复覆盖。

## 停止条件

- 发现必须修改 SQLite schema、引入新 crate/version、改变公开产品统计含义或增加兼容双路径时，停止并重新评审。
- 修复扩散到大量无关文件、需要为每个例外新增 flag/mode，或出现绕过根因的 fallback 时，回退设计而不是继续堆补丁。
