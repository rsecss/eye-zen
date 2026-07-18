# P1 修复技术设计

## 1. 目标与边界

本任务沿唯一承重路径修复六项 P1：

`Config 输入 → ConfigService 发布 → Timer 状态机 → Window 呈现 → 用户动作 → Stat 写入/导出`

不引入平行配置系统、第二套状态机、兼容双读或新的治理层。每项修复必须把复杂度放回拥有该决定的 Module，并通过其 Interface 测试。

## 2. 所有权与依赖方向

| Module                | 拥有                                                          | 不拥有                        |
| --------------------- | ------------------------------------------------------------- | ----------------------------- |
| Config                | 完整配置语义校验、revision、持久化与发布顺序                  | Settings 控件、Timer 状态转换 |
| Frontend Config Store | 配置写入串行化、intent rebase、stale event 过滤、inline error | TOML、平台副作用实现          |
| Timer                 | 状态转换、cycle outcome、presentation failure 恢复            | 窗口创建细节                  |
| Window                | 多屏枚举、唯一交互主 Surface、创建 receipt、默认不抢焦点      | Timer 状态与统计              |
| Stat Writer           | bounded queue、排空和 shutdown 所有权                         | Timer 调度                    |
| Stat Export           | 临时数据库、校验、落盘和平台原子替换                          | Save 对话框                   |

依赖保持 `commands → services → platform/models`。Window 结果通过 typed receipt 返回 effect 执行层，再反馈给 Timer；Timer 不直接依赖 Window。

## 3. P1-01 + P1-03：完整 Config 与 revisioned update

### 3.1 完整配置校验

ConfigService 在候选配置持久化与发布前执行一次完整语义校验：

```text
active_work_secs =
  timer.work_minutes * 60      when mode == twenty_twenty_twenty
  pomodoro.focus_minutes * 60  when mode == pomodoro

active_work_secs > timer.pre_alert_seconds
```

- Command 层继续只校验单字段范围；跨 section 不变量只由 Config Module 拥有。
- TOML 解析、白名单清洗和语言 canonicalize 后执行完整校验；失败时保留 `.bak` 并使用默认配置，不向 Timer 发布非法值。
- `update_with` 在 write lock 内形成候选配置、校验、写盘、递增 revision、替换内存快照，再在锁外 emit。
- Timer 的 duration 派生只消费 ConfigService 已接受的配置；删除 `.max(1)` 等为非法配置服务的 fallback。

### 3.2 ConfigSnapshot Interface

新增跨 IPC DTO：

```rust
pub struct ConfigSnapshot {
    pub revision: u32,
    pub config: Config,
}
```

ConfigService 内部使用 `VersionedConfig { revision, config: Arc<Config> }` 保证 config 与 revision 同一快照；backend watch 仍发布 `Arc<Config>`，避免把 revision 扩散到无关服务。

- `get_config() -> ConfigSnapshot`
- `config_changed: ConfigSnapshot`
- `update_*_config(config, expected_revision) -> ConfigSnapshot`
- revision 从进程启动的 `0` 开始，仅成功持久化后递增，不写 TOML。
- stale revision 返回新的结构化 `AppError::ConfigConflict { expected, actual }`，不得自动覆盖。

Hotkey 与 autostart 保留现有副作用回滚：hotkey apply 后若 revision/save 失败则恢复旧绑定；autostart 配置失败则恢复 OS 状态。

### 3.3 Frontend Config Store

Config Store 成为唯一写入 owner：

- 保存 `config + revision`，只应用 revision 更高的 snapshot，旧 event/response 直接忽略。
- 暴露按 section 分开的完整操作，例如 `updateTimer(change)`、`updateBehavior(change)`；调用者提交字段 intent，Store 在队列执行时基于最新 snapshot 构造完整 section。
- 所有 section 共用一个串行队列，避免 timer/mode/pomodoro 跨 section 更新交错。
- conflict 不静默重试：刷新 snapshot、保存 section error，用户重新操作后再提交。
- 控件继续读取 `configStore.current`，不做乐观更新；各 Settings section 删除对 `commands.ts` 的直接配置写入。

## 4. P1-02：Alerting 超时唯一语义

- `step_time(Alerting)` 超时转换为 Working。
- `Alerting -> Working` 产生一个 `CycleOutcome::Skipped`，不产生 RestSession。
- 只有显式 StartRest 才进入 Resting；只有 Resting 完成/退出的现有合同才能产生 Taken。
- 设置文案改为“超时自动关闭”/“Auto-dismiss after timeout”。
- 状态机、统计、前端文案和 architecture spec 同一提交同步。

## 5. P1-04：Stat writer shutdown 所有权

- 移除 `StatService: Clone`；AppServices 的 `Arc` 已提供共享，不再复制 sender 所有权。
- `writer_tx` 改为可 take 的 `Mutex<Option<Sender>>`。enqueue 在短同步锁内取得 sender 并 `try_send`；shutdown 先 take sender，再在锁外 `send(Shutdown).await`。
- 成功发送 sentinel 后 drop sender，再 take/join writer handle，最后关闭 pool。
- 队列满时 `send().await` 等待 writer 腾位，不丢 sentinel；receiver 已关闭时返回带上下文错误。
- 重复 shutdown 为幂等：sender、handle、pool 均已 take 时返回 Ok。

## 6. P1-05：安全覆盖导出

导出 Interface 保留“覆盖已有目标”，但目标在最终替换前不可触碰：

1. 在目标同目录生成唯一临时 `.db` 路径；StatService 串行化并发导出。
2. `VACUUM INTO` 临时路径。
3. 对临时数据库执行打开、schema/user_version/`PRAGMA quick_check` 验证。
4. `sync_all` 临时文件。
5. 通过私有 `AtomicFileReplace` seam 替换目标。

Adapters：

- Unix：同目录 `rename`。
- Windows：现有 `windows` dependency 增加 `Win32_Storage_FileSystem` feature，调用 `MoveFileExW`，flags 为 `MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH`；Microsoft 文档明确前者替换已有文件、后者在返回前落盘。
- Test：失败 Adapter 用于证明替换失败时旧 bytes 不变。

任何步骤失败都清理临时文件并返回错误；清理失败记录 warn 并保留原始错误上下文。不得先删除 target。

## 7. P1-06：presentation receipt 与恢复

### 7.1 Window Interface

`WindowService::show_tip_windows() -> PresentationReceipt`：

```rust
pub struct PresentationReceipt {
    pub windows_shown: usize,
    pub interactive_shown: bool,
}
```

- Window Module 按 primary-first 顺序选择交互主 Surface；主窗口创建失败时尝试下一显示器，成功后其余显示器只创建 minimal Surface。
- 若已有交互 tip window，计入 receipt；只有 minimal 存活时仍需创建交互主 Surface。
- `TipWindowSpec` 删除 `focused` 字段；production Adapter 始终 `.focused(false)`。
- 只有 `interactive_shown=true` 才算呈现成功；避免“有装饰窗口但无操作入口”。

### 7.2 Timer feedback

- `EffectSink::show_tip_windows` 返回 receipt，effect executor 把 receipt 返回调用者。
- 若 receipt 失败，调用独立的 `recover_from_presentation_failure` 完整操作：只允许 Alerting/Resting → Working，隐藏残留窗口、重置工作计时、更新 state/tray，但不创建 cycle event 或 RestSession。
- feedback 在 effect 执行后重新取得 Timer lock；若用户动作已改变状态，则不覆盖新状态。
- 失败写入 tracing error；本任务不扩展健康统计 schema，也不把基础设施故障伪装成 Skipped/Suppressed。

## 8. 关键取舍

- 采用 revisioned snapshot 而不是 debounce：debounce 不能处理逆序完成、跨 section 更新或其他写入者。
- revision 不写 TOML：它只解决单进程 IPC 时间轴，不是用户数据。
- Window receipt 反馈复用现有 Effect seam，不让 Timer 直接依赖 Window。
- 导出添加内部文件替换 seam，因为 Unix/Windows/Test 三个 Adapter 真实变化；不扩展通用 PlatformApi。
- 六项 P1 在同一 Trellis 任务内分切片提交，便于独立回滚，但不拆子任务。

## 9. 风险与回滚

- ConfigSnapshot 是开发期 breaking IPC contract；同一切片同步 Rust DTO、bindings、commands/events、stores 和测试，不保留旧 payload。
- Windows 文件替换需要扩展现有 dependency feature，不新增 crate/version；若 Windows 编译失败，回滚 P1-05 切片，不降级为先删目标。
- presentation feedback 涉及时间轴；通过 0/N、partial、all、用户动作抢先四类测试闭合竞态。
- SQLite schema 不变，无 migration；任何发现需要 schema 变更时停止并重新评审。
