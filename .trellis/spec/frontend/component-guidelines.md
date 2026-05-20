# 组件规范

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## Svelte 5 Runes 语法

新代码 MUST 使用 Svelte 5 Runes：`$state` / `$derived` / `$effect` / `$props`。

- MUST NOT 在新代码中使用 Svelte 4 的 `$:` 响应式声明或顶层 `let` 自动响应。
- MUST NOT 使用 `createEventDispatcher()`（已弃用）；向上通信 MUST 使用 callback prop。
- 派生值 MUST 用 `$derived(...)`，副作用 MUST 用 `$effect(() => { ... })`。
- `$effect` 内 MUST NOT 直接驱动 IPC 写操作除非有明确的守卫；参考 `MainApp.svelte` 中根据 `configStore.loaded` 守卫的 theme 同步：

```ts
$effect(() => {
  if (configStore.loaded) {
    const theme = configStore.current.display.theme === 'dark' ? 'dark' : 'light';
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    getCurrentWindow().setTheme(theme).catch(() => {});
  }
});
```

## 组件大小与职责

- 单个组件 SHOULD NOT 超过 200 行；超过时 SHOULD 按职责拆分子组件。
- 组件 MUST 职责单一：要么是窗口编排（`MainApp.svelte`），要么是页面（`SettingsPage.svelte`），要么是可复用控件（`Stepper.svelte`）。
- 控件型组件 MUST NOT 内部直接调用 `commands.ts` 中的 IPC；保持纯展示 + 事件回调。

## Props 规范

- Props MUST 用 `$props()` + 显式解构，并提供精确 TypeScript 类型签名。
- MUST NOT 使用 `$$props` / `$$restProps`。
- 回调 prop 名 MUST 以 `on` 前缀开头（`onchange`、`onclick`），类型 MUST 精确（不允许 `any`）。
- 数据向下（props），操作向上（callback）。MUST NOT 在子组件内调用 `commands.ts`。
- MUST NOT 使用 `bind:` 让父组件双向绑定，那会打破单向数据流；唯一向上通道是 callback。

### 正确范式（来自 `Stepper.svelte`）

```svelte
<script lang="ts">
  let {
    value, min, max, step, unit,
    label = '',
    onchange,
  }: {
    value: number; min: number; max: number;
    step: number; unit: string;
    label?: string;
    onchange: (v: number) => void;
  } = $props();
</script>
```

### 反例（禁止）

```svelte
<!-- 错误：bind 打破单向数据流 -->
<script lang="ts">
  let { value = $bindable() } = $props();
</script>
```

```svelte
<!-- 错误：子组件直接调用 IPC -->
<Stepper bind:value={config.work_minutes}
         onsave={() => updateTimerConfig(config)} />
```

正确写法：

```svelte
<Stepper value={config.work_minutes} min={1} max={120} step={1}
         unit="min" onchange={handleWorkMinutesChange} />
```

## Snippet（内容投射）

- 内容投射 MUST 使用 Svelte 5 的 Snippet 取代旧版 slot，并显式声明 `Snippet` 类型。
- 参考 `SettingsCard.svelte`：

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title, children }: { title: string; children: Snippet } = $props();
</script>
<section class="card">
  <header class="card-header"><h2>{title}</h2></header>
  <div class="card-body">{@render children()}</div>
</section>
```

## 可复用组件设计原则

### Props 接口设计

- Props MUST 是组件行为的最小完备集——缺一不可，多一冗余。
- 控件 MUST NOT 接收"业务对象"作为 prop（如 `BehaviorConfig`），只接收标量（`checked: boolean`）。业务装配由 Page 组件完成。

### 封装边界

| 应封装为组件 | 不应封装 |
|-------------|---------|
| 有独立交互逻辑（Stepper 的 ±/clamp、Toggle 的 aria-checked） | 纯布局排列（`setting-row` 用 CSS 类） |
| 被使用 ≥2 次 | 仅出现 1 次的特定结构 |
| 有自己的状态生命周期或可访问性约定 | 只是 props 透传 |

- MUST NOT 为只用一次的结构创建组件（过度抽象）。
- MUST NOT 把所有行内样式都抽为组件（CSS 类足够）。

### 可访问性

- 交互元素 MUST 提供 `aria-label`（参考 `Stepper.svelte` 的 ± 按钮）。
- 开关类组件 MUST 设置 `role="switch"` 和 `aria-checked`（见 `Toggle.svelte`）。
- 焦点态 MUST 通过 `:focus-visible` 提供视觉反馈。

## 即时保存模式（Settings 页面）

Settings 页面 MUST 遵循以下数据流（详见 `state-management.md`）：

```
控件值来源: configStore.current（只读，单一数据源）
用户操作:   onchange → 构造完整 *Config → update*Config()
后端处理:   validate → save TOML → emit config_changed
前端更新:   configStore 收到事件 → UI 自动响应
错误恢复:   command 失败 → store 未变 → 控件值自动回弹
```

- MUST NOT 在 Page 组件内维护配置的本地副本（buffer）。
- MUST NOT 做乐观更新；以后端推送为准。
- 每次 `update_*_config` 调用 MUST 传完整的子配置对象（不做 patch）：

```ts
function handleTimerChange(field: keyof TimerConfig, value: number) {
  const updated: TimerConfig = { ...cfg.timer, [field]: value };
  updateTimerConfig(updated).catch((err) => console.error(err));
}
```

（来自 `SettingsPage.svelte`）

## 样式规范（CSS 变量强制）

- 颜色 / 圆角 / 阴影 / 过渡 MUST 使用 `app.css` 中的 CSS 变量，MUST NOT 硬编码色值。
- TailwindCSS v4 工具类与 CSS 变量并存：`style="background: var(--bg-primary);"` 是允许的（见 `MainApp.svelte`）。
- 暗色主题 MUST 通过 `[data-theme='dark']` 选择器覆盖变量，MUST NOT 在组件内做 `theme === 'dark' ? a : b` 三元颜色判断（违反单一来源）。
- 原生控件（`<select>`、`<button>`）想完全跟随暗色主题，MUST 同时设置 `appearance: none` 和显式 `background-color: var(--bg-card)`（见 `Select.svelte` 与 `Stepper.svelte`）。

### app.css 全局变量索引（必须复用）

| 分层 | 变量前缀 | 用途 |
|------|---------|------|
| 底色与卡片 | `--bg-*` | `--bg-primary`、`--bg-card`、`--bg-tab` |
| 文字层级 | `--text-*` | `--text-primary` … `--text-hint`（四级灰阶） |
| 全局主色 | `--accent*` | `--accent`、`--accent-deep`、`--accent-soft`、`--accent-hover`、`--accent-glow`、`--accent-border` |
| 边框与阴影 | `--border*` / `--separator` / `--shadow-*` | 卡片描边 + 默认/hover 阴影 |
| 布局 | `--radius-*` / `--transition` | `--radius-sm/md/lg`、`--transition: 150ms ease` |
| Toggle | `--toggle-on` / `--toggle-off` | 开关轨道色 |
| Biophilic 状态色（tray / tip） | `--state-*` | `--state-active(-label)`、`--state-alert(-label)`、`--state-paused` |
| 玻璃效果 | `--glass-*` | `--glass-tray-bg`、`--glass-tip-bg` |

- 全局主色 `--accent-*` 与状态色 `--state-*` 是独立体系，MUST NOT 互相依赖或混用。
- 新增颜色 MUST 归入对应前缀层，MUST NOT 创建孤立变量。
- 完整定义见 `src/app.css`（含 `:root` 与 `[data-theme='dark']` 两份）。

## 注释规范

- 注释 MUST 只描述意图、约束、设计理由。
- MUST NOT 复述代码逻辑；MUST NOT 记录修改历史（属于版本控制）。
- 非显而易见的依赖关系或设计权衡 SHOULD 用一两行注释说明，保持极度简洁。
