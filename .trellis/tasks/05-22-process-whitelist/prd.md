# 进程白名单（Process Whitelist）

## Goal

允许用户配置一组"工作中的关键进程"，当这些进程在前台运行时，到达 `Working → PreAlert` 跳转点不弹出休息提醒（与现有 fullscreen / schedule / afk skip 同语义、同管道），随 v0.4.0 发版。

## What I already know

### 架构已预留的扩展点

- `.trellis/spec/backend/platform-storage.md:23-28` 明确写明 P2 计划扩展 `PlatformApi`：
  ```rust
  fn get_foreground_process_name(&self) -> Option<String>; // P2 进程白名单
  ```
  规则：三平台 MUST 全部实现（或显式降级），不支持时返回 `None`。

- `SkipFlags`（`src-tauri/src/services/timer/state.rs:50-61`）已有三个 flag：`fullscreen_active` / `schedule_inactive` / `afk_active`，`any_active()` 是纯 OR 聚合。新增 `process_whitelisted` 一行入位即可。

- skip 聚合点（`src-tauri/src/services/context.rs:207-226`）在 `current_skip_flags()` 中读 config + 查 detector，统一组装 `SkipFlags`。

- 状态机消费点（`src-tauri/src/services/timer/machine.rs:35-77 step_time`）只在 `Working` timeout 时检查 `skip_flags.any_active()`，命中即 `Working → Working` 重置。

### 平台 API（research 已验证）

详见 [`research/platform-foreground-process-apis.md`](research/platform-foreground-process-apis.md)。核心：

| Platform | 链路 | 新依赖 |
|----------|------|--------|
| Windows | `GetForegroundWindow → GetWindowThreadProcessId → OpenProcess(QUERY_LIMITED_INFORMATION) → QueryFullProcessImageNameW → file_name` | 仅给 `windows` crate 加 `Win32_System_Threading` feature |
| macOS | `core-graphics::CGWindowListCopyWindowInfo` → `kCGWindowLayer==0` → `kCGWindowOwnerName` 取 CFString | 零新依赖 |
| Linux/X11 | `_NET_ACTIVE_WINDOW → _NET_WM_PID → readlink /proc/<pid>/exe → basename` | 零新依赖（X11Session 加一个 atom） |
| Linux/Wayland | 返回 `None`，沿用现有 fullscreen 降级模式 | n/a |

跨平台统一规范化：`trim().to_lowercase()`。macOS 返回的是 user-facing app name（如 `"Google Chrome"`、`"Code"`），可能含空格 —— 已知特性，UI 文案需告知用户照填 Activity Monitor 的名字。

### UX 决策

详见 [`research/ux-decisions.md`](research/ux-decisions.md)。核心：
- 列表上限 32 项
- 匹配策略：`trim().to_lowercase()` 后 exact equality（拒绝 substring）
- 自我保护：拒绝添加 `eyezen` / `eyezen.exe`
- Wayland 降级：UI 显示禁用 banner（复用 AFK pattern）
- 14 个 i18n key 已规划

### 现有样板（直接照搬模式）

- [`05-19-workday-scheduling/prd.md`](../archive/2026-05/05-19-workday-scheduling/prd.md) — SkipFlags + Config + SettingsCard + i18n + tests 切片，最相似样板
- [`05-20-afk-skip-next-working-interval/prd.md`](../archive/2026-05/05-20-afk-skip-next-working-interval/prd.md) — DetectorCapabilities + 平台降级 banner UX 样板

## Requirements

### Schema（`src-tauri/src/models/config.rs`）

`BehaviorConfig` 追加两个字段（带 `#[serde(default)]` 保证老 toml 兼容）：

```rust
#[serde(default)]
pub process_whitelist_enabled: bool,        // 默认 false
#[serde(default)]
pub process_whitelist: Vec<String>,         // 默认 vec![]
```

ts-rs 自动桥接到 `src/lib/bindings/BehaviorConfig.ts`，前端 `DEFAULT_CONFIG` 同步加默认。

### Platform 层（`src-tauri/src/platform/`）

```rust
// platform/mod.rs::PlatformApi
fn get_foreground_process_name(&self) -> Option<String>;
fn supports_foreground_process_detection(&self) -> bool;  // capability 用，Wayland=false
```

三平台实现按 research 文件落地。`Cargo.toml` 仅 Windows 端加 `Win32_System_Threading` feature。

公共规范化在 trait default method 或 mod 级辅助函数：

```rust
fn normalize_name(s: String) -> Option<String> {
    let trimmed = s.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_lowercase()) }
}
```

### Detector 层（`src-tauri/src/services/detector.rs`）

```rust
impl DetectorService {
    pub(crate) fn is_foreground_in_whitelist(&self, whitelist: &[String]) -> bool {
        let Some(name) = self.platform.get_foreground_process_name() else { return false };
        whitelist.iter().any(|w| w == &name)  // 列表已归一化为 lowercase
    }
}
```

`DetectorCapabilities` 扩展 `foreground_process_detection_supported: bool`。

### SkipFlags 聚合（`src-tauri/src/services/timer/state.rs` + `src-tauri/src/services/context.rs`）

```rust
// SkipFlags
pub(crate) process_whitelisted: bool,
// any_active(): OR 加一行

// current_skip_flags
let process_whitelisted = config.behavior.process_whitelist_enabled
    && !config.behavior.process_whitelist.is_empty()
    && services.detector.is_foreground_in_whitelist(&config.behavior.process_whitelist);
```

### Settings UI（`src/pages/main/SettingsPage.svelte`）

新增 `<SettingsCard title="进程白名单">`：
- 行 1：Toggle 总开关（绑定 `cfg.behavior.process_whitelist_enabled`）
- 行 2（separator）：输入框 + 添加按钮，下方 chip 列表（每个 chip 带删除按钮）
- 行 3（separator）：能力不支持时显示禁用 banner（复用 `settings.whitelist.unsupported`）

提交校验在前端先跑（empty / duplicate / self / limit），通过后调 `updateBehaviorConfig`。后端 command 不重复校验（trust UI boundary），但服务读取时静默 truncate >32（容错）。

### i18n（`src/lib/i18n/zh-CN.ts` + `en.ts`）

新增 14 个 key（含 5 个错误文案 + 1 个 unsupported banner），表见 `research/ux-decisions.md`。

### 测试

- `services/detector.rs` 单测：`is_foreground_in_whitelist` 走 MockPlatform，覆盖：命中 / 不命中 / Wayland None / 空列表
- `services/timer/machine.rs` 单测：`step_time` 在 `process_whitelisted=true` 时返回 `Working→Working`（参考现有 `working_timeout_with_fullscreen_skip_resets`）
- `services/context.rs` 单测：`current_skip_flags` 在 enabled=false 时不查 detector（避免无谓 syscall）
- 前端 `SettingsPage.test.ts`：add / remove / 拒绝 self / 拒绝 duplicate / 拒绝 empty / 拒绝 limit / capability disabled

## Acceptance Criteria

- [ ] `BehaviorConfig` 新增 `process_whitelist_enabled` 与 `process_whitelist`；老 toml 文件加载不报错且默认值正确
- [ ] `PlatformApi` 新增 `get_foreground_process_name` 与 `supports_foreground_process_detection`，三平台编译通过
- [ ] Windows 实现：聚焦 Notepad，调用返回 `Some("notepad.exe")`；切到 VS Code 返回 `Some("code.exe")`
- [ ] macOS 实现：聚焦 Activity Monitor 显示的应用，调用返回的 `kCGWindowOwnerName`（小写、可含空格）
- [ ] Linux X11 实现：聚焦 GNOME Terminal，调用返回 `Some("gnome-terminal-server")` 或类似
- [ ] Linux Wayland 实现：返回 `None`，`supports_foreground_process_detection() == false`，warn-once 已记录
- [ ] `DetectorService::is_foreground_in_whitelist` 在 platform 返回 `None` 时返回 `false`
- [ ] `SkipFlags::process_whitelisted` 加入 `any_active()`；`step_time` 在 Working 命中时返回 `Working→Working`
- [ ] `current_skip_flags` 仅在 `enabled && !list.is_empty()` 时查 detector
- [ ] Settings UI 显示 Whitelist Card，添加 / 删除 / 错误提示交互齐全
- [ ] 拒绝添加 `eyezen` / `eyezen.exe`、拒绝 duplicate、拒绝空、拒绝超过 32 项
- [ ] Wayland 平台 Settings Card 显示 unsupported banner，Toggle + add-input 禁用
- [ ] 全部 14 个 i18n key 在 zh-CN 与 en 翻译表中存在
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` 全部通过
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` 三平台无新增 warning
- [ ] `npx svelte-check` 无 error
- [ ] `npm test` 全部通过
- [ ] `npm run build` + `npm run tauri build` 成功
- [ ] 手动测试（开发者本机）：四种 skip flag 单独 / 两两组合 / 全开启时行为符合预期
- [ ] 三平台 CI matrix 全绿

## Definition of Done

- 全部 AC 勾选
- `npm run ci` 全绿
- CLAUDE.md 「Phase 2 进行中」章节追加进程白名单项
- `docs/devlog.md` 追加变更条目
- PR 合入 main（GitHub Flow，squash merge，分支用完即删）
- v0.4.0 tag 触发 release.yml，四目标制品产出，Draft → Published as latest
- `memory/MEMORY.md` "Project State" 同步更新到 v0.4.0

## Technical Approach

总览：跨层薄切片，复用 SkipFlags / DetectorCapabilities / SettingsCard 三个已有 pattern；零跨层新机制；零新 Rust crate；前端零新依赖；Windows 仅追加一个 `windows` crate feature。

详见上方 Requirements 各小节 + research 文件。

## Decision (ADR-lite)

**Context**
- 进程白名单是 Phase 2 已规划项，架构层 `.trellis/spec/backend/platform-storage.md` 已为 `get_foreground_process_name` 预留位置
- SkipFlags 已三次复用（fullscreen / schedule / afk），新 flag 入位是自然延续
- 跨平台前台进程检测有多种 API 选项（libproc / sysinfo / objc2-app-kit / core-graphics / x11 / Wayland）

**Decision**
1. **匹配粒度**：跨平台统一 exe basename（macOS 实际为 CFBundleExecutable 等价的 `kCGWindowOwnerName`，可含空格），`trim().to_lowercase()` 后 exact equality
2. **平台 API 选型**：
   - Windows: `windows` crate（已有）+ 加一个 `Win32_System_Threading` feature，原生 Win32 调用链
   - macOS: `core-graphics::kCGWindowOwnerName`（零新依赖，user-facing name，已通过 layer==0 过滤得到稳定结果）
   - Linux X11: 复用现有 `x11rb` 连接，加一个 `_NET_WM_PID` atom
   - Linux Wayland: 显式返回 `None`，与现有 fullscreen 降级一致
   - **拒绝 libproc**（macOS 上 Electron 应用返回 helper basename 反而误导用户）
   - **拒绝 objc2-app-kit**（默认 307 features，构建成本不值得）
   - **拒绝 sysinfo**（依赖 Rust 1.95，超出当前 toolchain）
3. **UI**：纯文本 add/delete 列表，KISS，对齐 Schedule Card 视觉风格
4. **列表上限**：32，前端校验，后端读取时静默 truncate 容错
5. **macOS MUST 实现真实检测**（用户明确拒绝占位降级；core-graphics 路径已验证）
6. **fullscreen_skip 与白名单保持正交**（两个独立 SkipFlag，OR 聚合）

**Consequences**
- 优点：零跨层新机制，所有改动都走既有 pattern；后续要升 bundle id / 路径匹配可平滑扩展（PlatformApi 仅多一个 method）
- 风险：macOS 返回含空格的 user-facing name（如 `"Google Chrome"`），用户跨平台共享 config.toml 会有匹配差异 —— 已在 UI 文案 + ux-decisions.md 中标注
- 局限：Linux Wayland 完全失效，与现有 fullscreen / afk 局限一致；XWayland 下的 X11 客户端可工作

## Out of Scope (explicit)

- 进程路径 / 通配 / 正则匹配
- "列出当前运行进程"picker UI
- bundle identifier 匹配（如 `com.microsoft.VSCode`）
- macOS fullscreen 检测的修复（DegradedFalse 仍保留，独立任务）
- 进程白名单与全屏跳过的合并（保持正交）
- 反向白名单（"这些进程出现时强制弹"）
- 进程粒度统计（如"VSCode 占用了多少跳过的提醒"）
- 跨平台 config.toml 自动归一化（用户跨平台共享时差异自负）
- AppUserModelID (UWP) 解析、Windows ApplicationFrameHost.exe 穿透

## Technical Notes

### Inspected

- `src-tauri/src/services/detector.rs` — 当前 DetectorService 极薄，新增方法零阻力
- `src-tauri/src/services/context.rs:207-226` — skip 聚合中心
- `src-tauri/src/platform/windows.rs:62-103` — 已有 `GetForegroundWindow` 调用，复用基础结构
- `src-tauri/src/platform/macos.rs:75-89` — 已用 `copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID)`，模式复用
- `src-tauri/src/platform/linux.rs:103-156` — X11 connection 已建好，加 atom 几行代码
- `src-tauri/src/models/types.rs` — `DetectorCapabilities` 扩展模式清晰
- `src/pages/main/SettingsPage.svelte:495-528` — Schedule Card 是新 Card 的视觉模板
- `src/lib/i18n/{zh-CN,en}.ts` — 文案风格已读，新 keys 沿用

### Constraints

- ts-rs：`Vec<String>` 自动桥接成 `string[]`，零额外工作
- 配置兼容：`#[serde(default)]` 保证老 toml 不报错（参照 `.trellis/spec/architecture/change-management.md`「配置兼容」）
- 平台降级：参考 `.trellis/spec/backend/platform-storage.md` 的「不支持的能力 MUST 返回保守默认值（false / None），MUST NOT panic」
- 锁/异步：`get_foreground_process_name` 是同步快速调用（μs 级），可直接在 `current_skip_flags` 中同步调用（已是同步上下文）

### Research References

- [`research/platform-foreground-process-apis.md`](research/platform-foreground-process-apis.md) — 三平台 + Wayland 详细 API 与 Cargo.toml 改动
- [`research/ux-decisions.md`](research/ux-decisions.md) — 列表上限 / 匹配策略 / 自我保护 / 降级 / 14 个 i18n key 表
