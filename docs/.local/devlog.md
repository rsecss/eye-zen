# Eyezen 开发日志

> 记录项目关键决策、里程碑和会话摘要。
> 按时间倒序排列，最新的在前。

---

## 2026-03-20 — 前端原型实现 + Codex 审查修复 + E2E 测试

### 背景

后端 MVP 完成后，执行 Plan 009（前端原型），使用 Subagent-Driven Development 流程，每个 Task 派发独立 subagent 实现 + 双阶段审查。

### 完成事项

1. **ts-rs 类型桥接**
   - ts-rs ~10 作为 dev-dependency，`#[cfg_attr(test, derive(ts_rs::TS))]` 模式
   - 生成 5 个 TypeScript bindings：StatePayload / Config / TimerConfig / BehaviorConfig / DisplayConfig
   - 修复 Paused 状态 snapshot bug（service.rs + machine.rs 双路径）

2. **IPC 封装层**
   - `commands.ts`：9 个 invoke 函数 + 5s 超时 + clearTimeout 清理
   - `events.ts`：`onStateChanged` + `onConfigChanged` 类型化监听器

3. **Svelte 5 Runes Stores**
   - timer.svelte.ts + config.svelte.ts：$state + init/destroy 生命周期
   - 版本计数器防止快照覆盖事件数据（Codex 审查修复）
   - invoke 失败时自动回滚 listener

4. **视觉基础**
   - Plus Jakarta Sans 变体字体（本地 woff2）
   - biophilic CSS 变量体系：状态色 + 玻璃效果

5. **Tray Panel**
   - 5 状态完整映射（Working / Paused / PreAlert / Alerting / Resting）
   - 玻璃拟态卡片 + 呼吸动画 + 进度条
   - 跟随托盘图标定位（available_monitors 查找实际 monitor + 双轴 clamp）
   - Svelte 5 snippet 提取 settings icon

6. **Tip Window**
   - Alerting + Resting 双视图条件渲染
   - 深森林绿极光渐变背景 + 玻璃拟态内容卡
   - 80px 绿色渐变倒计时文字

7. **Tip Minimal**
   - 次显示器覆盖 + 呼吸文字动画

### 关键决策

- **ts-rs 放 dev-dependencies**：使用 `cfg_attr(test)` 保护 derive，生产构建不引入 ts-rs 依赖树
- **Store 竞态保护**：版本计数器模式——事件到达时递增版本，快照返回时仅在版本未变时才应用
- **Tray 面板定位**：放弃居中方案，采用跟随托盘图标（available_monitors 定位 + 边界 clamp）
- **pre_alert 按钮分离**：tray 面板中 pre_alert 显示 Pause（非 Start Rest），tip 窗口中 pre_alert 不显示

### Codex 审查结果

Codex 审查发现 1 个 Critical + 3 个 Important + 4 个 Minor，全部修复：
- Critical: store 初始化竞态（版本计数器 + listener 回滚）
- Important: 托盘多显示器定位（monitor 查找 + 双轴 clamp）
- Important: invokeWithTimeout 悬挂 timer（clearTimeout）
- Minor: TipApp pre_alert 分离、测试清理

### 质量门禁

- cargo test: 66 passed
- vitest: 4 passed
- svelte-check: 0 errors, 0 warnings
- vite build: 成功
- E2E 手动测试: 8 步全流程通过

---

## 2026-03-19 — 后端 MVP 实现 + 分支合并

### 背景

脚手架初始化后，在 `dev-codex` worktree 中完成了后端 MVP 全部服务实现，随后合并回 `dev` 主开发分支并清理所有 worktree。

### 完成事项

1. **6 个核心服务实现**
   - ConfigService: TOML 配置读写 + arc-swap 热更新 + watch channel
   - TimerService: 纯函数状态机 + tokio tick loop + effect executor
   - DetectorService: 全屏检测 (平台委托)
   - WindowService: 多显示器 tip-window 生命周期管理
   - SoundService: rodio 独立线程 + mpsc 通道
   - TrayService: 托盘菜单 + tooltip 动态更新

2. **基础设施层**
   - AppError: 统一错误类型 + IPC 序列化
   - Logging: tracing + tracing-subscriber + 日志轮转
   - ServiceContext: 服务间通信上下文 (AppHandle 封装)
   - Events: IPC 事件模块骨架

3. **IPC 层**
   - 9 个 Tauri Commands (get_config, update_*_config, timer 控制, state_snapshot)
   - 3 个 capability 文件 (main-window, tip-window, tray-panel) + 权限 TOML
   - default.json 拆分为最小权限基线

4. **跨平台层**
   - PlatformApi trait + Windows/macOS/Linux 三平台全屏检测实现
   - Windows: Win32 API (EnumWindows + GetForegroundWindow)
   - macOS: CoreGraphics CGWindowListCopyWindowInfo
   - Linux: x11rb (\_NET\_WM\_STATE\_FULLSCREEN)

5. **分支与 worktree 管理**
   - 合并 `dev-codex` → `dev` (解决 default.json 冲突)
   - 移除 `dev-codex` 和 `dev-claude` worktree
   - 删除对应分支，仅保留 `dev` 和 `main`

6. **新增依赖**
   - toml (~0.8), tracing (~0.1), tracing-subscriber (~0.3), tracing-appender (~0.2)
   - rodio (~0.20), arc-swap (~1.7)
   - windows (~0.61), core-foundation (~0.10), core-graphics (~0.24), x11rb (~0.13)
   - tempfile (~3.15, dev-dependency)

### 下一步

- 代码审查：检查后端 MVP 实现质量，对照 rules/ 规则
- 前端原型：tip-window / tray panel / settings 页面
- 集成测试：端到端 timer 流程验证
- ts-rs 类型桥接：Rust → TypeScript 自动生成

---

## 2026-03-18 — 重建设计定稿

### 背景

Phase 1 MVP 已完成，经三模型（Claude Opus + Codex + Gemini）圆桌审查后，决定进行完整重建：保留核心架构方向，修正工程完整性问题。

### 完成事项

1. **Phase 1 经验复盘** (`docs/.user/experience-review.md`)
   - 确认 6 项值得保留的设计（Timer 纯函数状态机、PlatformApi trait、SoundService 线程隔离等）
   - 识别 13 项问题（P0: tip-window 未闭环；P1: 配置热更新语义分裂；P2: 锁粒度、ServiceRegistry 过度工程等）

2. **竞品深度调研**
   - ProjectEye (`docs/.user/projecteye-research.md`) — C# + WPF，1.2k stars，停更但架构成熟
   - Blink Eye (`docs/.user/blinkeye-research.md`) — Tauri + React，最直接竞品，前端堆业务的反面教材

3. **重建设计规格** (`docs/superpowers/specs/2026-03-18-eyezen-rebuild-design.md`)
   - 状态：已通过审查（第 2 轮迭代）
   - 范围：后端精炼 + 前端重做 + 功能补全
   - 技术栈：Tauri v2 + Svelte 5 (Runes) + TailwindCSS v4 + ECharts + SQLite (sqlx) + TOML
   - 8 个服务：Config / Timer / Detector / Window / Stat / Sound / Tray / I18n
   - Timer 状态机 6 个状态：Working / PreAlert / Alerting / Resting / Paused / Away
   - 前端 Vite 多入口：main / tip / tip-minimal / tray

4. **UI 风格方向确定** (`docs/.user/style-comparison.html`)
   - 选定：Linear/Raycast 现代质感 + macOS 暖色干净 light 风格
   - CSS 变量体系已定义（accent: #6366f1, radius: 8/12/16px）

5. **开发工作流文档** (`docs/development-workflow.md`)
   - 10 个阶段全生命周期指南
   - CI/CD 配置（pre-commit / pre-push / GitHub Actions）
   - 多模型审查流程
   - Skill 链式调用策略

### 关键决策

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| 图表库 | ECharts vs Recharts vs Chart.js | ECharts (tree-shaken) | 功能全面，CJK 生态好 |
| i18n 方案 | JSON 资源文件 | JSON (前端 + Rust 嵌入) | 简单实用，前后端统一 |
| 页面布局 | Tab 切换 | Settings \| Statistics \| About | 符合桌面应用习惯 |
| 离开检测 | 双因子 | Win: cursor + audio; Mac/Linux: cursor only | 跨平台降级策略 |
| DB 位置 | app_data_dir | 与 config_dir 分离 | 数据和配置职责分离 |
| 快捷键生效 | 重启生效 | 全局热键启动时注册 | 简化实现复杂度 |
| ServiceRegistry | IoC vs 显式 struct | 显式 `AppServices` struct | 4-8 个服务不需要运行时注册表 |
| 锁策略 | 锁内执行 vs reducer | Reducer 返回 Effects，锁外执行 | 降低锁持有时间 |
| 配置存储 | JSON vs TOML | TOML + 原子写入 | Rust 生态标准，可读性好 |
| 前端类型 | 手写 vs ts-rs | ts-rs 生成，前端直接消费 | 消除类型漂移 |

### 下一步

- 进入阶段 3：技术栈确认与模块拆解
- 初始化项目脚手架（Tauri v2 + Svelte 5 + Vite + TailwindCSS v4）
- 按 MVP 范围拆分第一批实现切片

---

## 2026-03-17 — Phase 1 审查启动

### 完成事项

- Phase 1 MVP 代码完成，所有步骤已提交到 `dev` 分支
- 发起三模型圆桌审查（Claude Opus + Codex + Gemini）
- 初步识别需要重建的范围

---

## 模板

```
## YYYY-MM-DD — 标题

### 背景
简述当前阶段和上下文。

### 完成事项
1. ...
2. ...

### 关键决策
| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|

### 已知风险
- ...

### 下一步
- ...
```
