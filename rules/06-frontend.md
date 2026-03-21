# 前端架构

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## Vite 多入口

纯 Svelte 5 + Vite，每个窗口一个 HTML 入口：

```
(根目录)
├── index.html           → main-window
├── tip.html             → tip-window
├── tip-minimal.html     → tip-window-minimal
└── tray.html            → tray-panel

src/
├── entries/             各窗口 TS 入口
│   ├── main.ts
│   ├── tip.ts
│   ├── tip-minimal.ts
│   └── tray.ts
├── pages/               各窗口页面组件
│   ├── main/
│   │   ├── MainApp.svelte
│   │   ├── SettingsPage.svelte
│   │   ├── AboutPage.svelte
│   │   └── components/      窗口级可复用组件
│   │       ├── Stepper.svelte
│   │       ├── Toggle.svelte
│   │       ├── Select.svelte
│   │       └── SettingsCard.svelte
│   ├── tip/TipApp.svelte
│   ├── tip-minimal/TipMinimalApp.svelte
│   └── tray/TrayApp.svelte
├── lib/
│   ├── bindings/        ts-rs 生成类型（MUST NOT 手动编辑）
│   ├── components/      跨窗口共享组件
│   ├── stores/          Svelte stores
│   ├── commands.ts      IPC command 封装
│   └── events.ts        IPC event 封装
└── app.css              TailwindCSS v4 + CSS 变量
```

- HTML 入口 MUST 在项目根目录（Vite 多入口标准）
- `src/lib/bindings/` MUST NOT 手动编辑，只由 ts-rs 生成覆盖
- 新增窗口类型 MUST 同时添加：HTML 入口 → TS entry → Page 组件 → Vite rollupOptions

### 组件放置原则

| 位置 | 适用场景 | 示例 |
|------|---------|------|
| `pages/<window>/components/` | 仅该窗口使用的可复用组件 | Stepper, Toggle, SettingsCard |
| `lib/components/` | 跨窗口共享的组件 | （当前为空，按需提升） |

- 组件 MUST 先放在 `pages/<window>/components/`（就近原则）
- 当两个以上窗口需要同一组件时，才提升到 `lib/components/`
- MUST NOT 预设"可能被复用"而提前提升（YAGNI）

## 窗口定义

| 窗口 | 用途 | 生命周期 | 阶段 |
|------|------|---------|------|
| `main-window` | 设置/关于 | 用户按需打开，关闭时隐藏 | MVP |
| `tip-window-{n}` | 全屏休息提醒 | Alerting/Resting 时创建，结束后销毁 | MVP |
| `tip-window-minimal-{n}` | 副显示器遮罩 | 同上，仅副显示器 | MVP |
| `tray-panel` | 托盘快捷面板 | 预创建隐藏，托盘左键切换 | MVP |

- 动态窗口 MUST 使用 `{n}` 后缀（显示器编号）
- 动态窗口 MUST 在状态切换后及时销毁，MUST NOT 内存泄漏

## Capability 与 Permission（最小权限）

Tauri v2 使用 ACL 权限模型：先在 permission 文件中定义权限，再在 capability 文件中引用。

### Permission 定义

自定义 command MUST 在 `src-tauri/permissions/` 下定义 permission 文件：

```
src-tauri/permissions/
├── timer/
│   └── default.toml      # 定义 allow-start-rest, allow-skip-rest, allow-pause, allow-resume
├── config/
│   └── default.toml      # 定义 allow-get-config, allow-update-*
└── stat/
    └── default.toml      # 定义 allow-get-daily-stats (P2)
```

Permission 文件示例（`src-tauri/permissions/timer/default.toml`）：

```toml
[[permission]]
identifier = "allow-start-rest"
description = "Allow invoking start_rest command"
commands.allow = ["start_rest"]

[[permission]]
identifier = "allow-skip-rest"
description = "Allow invoking skip_rest command"
commands.allow = ["skip_rest"]
```

### Capability 引用

```
src-tauri/capabilities/
├── default.json          # shared: core:default, shell:allow-open
├── main-window.json      # eyezen:allow-get-config, eyezen:allow-update-*, eyezen:allow-pause, eyezen:allow-resume
├── tip-window.json       # eyezen:allow-start-rest, eyezen:allow-skip-rest
└── tray-panel.json       # eyezen:allow-start-rest, eyezen:allow-skip-rest, eyezen:allow-pause, eyezen:allow-resume, eyezen:allow-get-config
```

### 规则

- 每个自定义 command MUST 有对应的 permission 定义
- 每个窗口 capability MUST 只引用所需的最小 permission 集合
- MUST NOT 使用 `core:default` 以外的通配符
- 新增 Command MUST 同时：定义 permission → 更新对应窗口 capability

## 安全基线

### CSP 与远程内容

- `tauri.conf.json` 的 CSP MUST 禁止加载远程脚本和样式
- 前端 MUST NOT 使用 `dangerouslySetInnerHTML` 或动态插入未转义的 HTML
- MUST NOT 从网络加载任何可执行内容（JS、WASM）

### Shell 与 Plugin

- `shell:allow-open` 仅允许打开 URL，MUST NOT 启用 `shell:allow-execute`
- 引入新 Tauri plugin MUST 评估其权限范围并在 capability 中最小化
- MUST NOT 使用 sidecar 除非有明确的隔离和签名方案

### 敏感数据

- 前端 MUST NOT 持有 API key、token、密码等敏感数据
- 日志和错误信息 MUST NOT 包含完整文件路径中的用户名

## 前端状态管理

### 数据来源分层

| 数据 | 存储位置 | 更新方式 |
|------|---------|---------|
| Timer 状态（state, countdown） | Svelte store | `listen('state_changed')` 事件驱动 |
| 配置数据 | Svelte store | `listen('config_changed')` + 初始 `invoke('get_config')` |
| 统计数据 (P2) | 组件本地 state | `invoke('get_daily_stats')` 按需拉取 |
| UI 临时状态（tab 选中、弹窗开关） | 组件本地 `$state` | 组件内管理 |

### 规则

- 后端推送的状态 MUST 通过 Svelte store 管理，MUST NOT 在组件间通过 props 层层传递
- Store MUST 是单一数据源（single source of truth）
- MUST NOT 在前端缓存后端状态的副本并自行修改
- 用户操作 MUST 通过 `invoke()` 发送到后端，等 `state_changed` 事件回推后更新 UI
- MUST NOT 做乐观更新（操作后立即改 UI），以后端为准

### IPC 封装

```typescript
// src/lib/commands.ts — 所有 invoke 调用集中在此
const INVOKE_TIMEOUT_MS = 5000;

async function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error(`Command "${cmd}" timed out`)), INVOKE_TIMEOUT_MS),
    ),
  ]);
}

export async function getStateSnapshot(): Promise<StatePayload> {
  return invokeWithTimeout('get_state_snapshot');
}

// src/lib/events.ts — 所有 listen 调用集中在此
export async function onStateChanged(
  callback: (payload: StatePayload) => void,
): Promise<UnlistenFn> {
  return listen('state_changed', (event) => callback(event.payload));
}
```

- 前端 MUST NOT 直接调用 `invoke()` / `listen()`，MUST 通过封装函数
- 封装函数 MUST 有正确的 TypeScript 类型签名
- `listen()` 返回 `Promise<UnlistenFn>`，组件卸载时 MUST 调用 unlisten

## 视觉设计体系

风格：Linear/Raycast 现代质感 + 清新淡绿 light（护眼主题统一绿色体系）

### 全局 CSS 变量（app.css）

```css
:root {
  /* 底色：极淡绿 */
  --bg-primary: #f8faf9;
  --bg-card: #ffffff;

  /* 文字：带绿调的灰阶 */
  --text-primary: #1a1d23;
  --text-secondary: #5f6b6a;
  --text-tertiary: #94a3a0;
  --text-hint: #adb8b5;

  /* 主色：清新翠绿 */
  --accent: #22c55e;
  --accent-deep: #16a34a;
  --accent-soft: rgba(34, 197, 94, 0.07);
  --accent-hover: rgba(34, 197, 94, 0.12);
  --accent-glow: rgba(34, 197, 94, 0.05);
  --accent-border: rgba(34, 197, 94, 0.12);

  /* 边框与阴影 */
  --border: #e5eae7;
  --border-light: rgba(0, 0, 0, 0.03);
  --separator: #f0f3f1;
  --shadow-card: 0 1px 2px rgba(0,0,0,0.02), 0 2px 8px rgba(22,100,60,0.03);
  --shadow-card-hover: 0 2px 4px rgba(0,0,0,0.03), 0 4px 16px rgba(22,100,60,0.05);

  /* 布局 */
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
  --transition: 150ms ease;

  /* Toggle */
  --toggle-on: #22c55e;
  --toggle-off: #d1d8d4;

  /* Biophilic 状态色（tray + tip 窗口） */
  --state-active: #10b981;
  --state-active-label: #059669;
  --state-alert: #f59e0b;
  --state-alert-label: #b45309;
  --state-paused: #9ca3af;

  /* Glass effects */
  --glass-tray-bg: rgba(255, 255, 255, 0.72);
  --glass-tip-bg: rgba(255, 255, 255, 0.06);
}
```

### 色彩分层原则

| 层 | 变量前缀 | 用途 |
|----|---------|------|
| 全局主色 | `--accent-*` | Tab 指示、按钮、链接、选中态 |
| 状态色 | `--state-*` | tray/tip 窗口的运行状态映射 |
| 玻璃效果 | `--glass-*` | tray/tip 的 backdrop-filter 背景 |

- 全局主色和状态色是独立体系，互不依赖
- 新增颜色 MUST 归入对应前缀层，MUST NOT 创建孤立变量

### 规则

- 颜色 MUST 使用 CSS 变量，MUST NOT 硬编码色值
- 圆角、阴影、过渡 SHOULD 使用预定义变量
- MVP 阶段只实现 light 主题；dark + system-auto 在 P2

## 可复用组件设计原则

### Props 接口设计

- Props MUST 是组件行为的最小完备集——缺一不可，多一冗余
- 回调命名 MUST 以 `on` 前缀：`onchange`, `onclick`（非 `handleChange`）
- 数据向下（props），操作向上（callback），MUST NOT 在子组件内直接调用 commands

```svelte
<!-- 正确：数据 + 回调分离 -->
<Stepper value={config.work_minutes} min={1} max={120} step={1}
         unit="min" onchange={handleWorkMinutesChange} />

<!-- 错误：子组件直接调用 IPC -->
<Stepper bind:value={config.work_minutes}
         onsave={() => updateTimerConfig(config)} />
```

### 组件封装边界

| 封装为组件 | 不封装 |
|-----------|-------|
| 有独立交互逻辑（Stepper 的 ±/clamp） | 纯布局排列（setting-row 用 CSS 类即可） |
| 被使用 ≥2 次 | 仅出现 1 次的特定结构 |
| 有自己的状态生命周期 | 只是 props 透传 |

- MUST NOT 为只用一次的结构创建组件（过度抽象）
- MUST NOT 把所有行内样式都抽为组件（CSS 类足够）

### 即时保存模式（Settings 页面）

```
控件值来源: configStore.current（只读，单一数据源）
用户操作:   onchange → 构造完整 *Config → update*Config()
后端处理:   validate → save TOML → emit config_changed
前端更新:   configStore 收到事件 → UI 自动响应
错误恢复:   command 失败 → store 未变 → 控件值自动回弹
```

- MUST NOT 在组件内维护配置的本地副本（buffer）
- MUST NOT 做乐观更新
- 每次 update 调用 MUST 传完整的子配置对象（不做 patch）
