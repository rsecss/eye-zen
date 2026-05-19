# 状态管理

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选

---

## 数据来源分层

Eyezen 前端的数据来源严格分层，每一类有明确的"住址"：

| 数据 | 存储位置 | 更新方式 | 当前实现 |
|------|---------|---------|---------|
| Timer 状态（state、remaining_secs、work/rest 配置） | Svelte store | `listen('state_changed')` 事件驱动 + 初始 `invoke('get_state_snapshot')` | `src/lib/stores/timer.svelte.ts` |
| 配置数据（timer/behavior/display） | Svelte store | `listen('config_changed')` 事件驱动 + 初始 `invoke('get_config')` | `src/lib/stores/config.svelte.ts` |
| i18n 语言 + 字典 | Svelte store | 通过 `i18nStore.setLocale(...)` 显式驱动；`MainApp.svelte` 中由 `$effect` 与 `configStore.current.display.language` 联动 | `src/lib/i18n/index.svelte.ts` |
| 统计数据 (P2) | 组件本地 state | `invoke('get_daily_stats')` 按需拉取 | 未实现 |
| UI 临时状态（tab 选中、弹窗开关） | 组件本地 `$state` | 组件内管理 | `MainApp.svelte` 的 `activeTab` |

## 单一数据源原则

- 后端推送的状态 MUST 通过 Svelte store 管理，MUST NOT 在组件间通过 props 层层透传。
- Store MUST 是单一数据源（single source of truth）。
- 多个组件需要同一份数据时 MUST 各自从 store 取值（store 是模块单例），MUST NOT 在组件 A 拷贝后 props 传给组件 B。
- MUST NOT 在前端缓存后端状态的副本并自行修改（典型反例：把 `configStore.current.timer` 拷贝到组件本地 `$state` 上编辑）。

## 禁止乐观更新

- 用户操作 MUST 通过 `commands.ts` 中的封装函数发送到后端，等待 `*_changed` 事件回推后由 store 更新 UI。
- MUST NOT 在调用 command 后立即修改本地状态（"乐观更新"）。
- 后端是真理来源，前端是镜像。控件值始终读 `configStore.current.*` / `timerStore.current.*`，事件没回来就显示旧值。

## 即时保存模式（Settings）

`SettingsPage.svelte` 是该模式的参考实现：

```
[控件值来源]   configStore.current（只读、单一数据源、$derived 为 cfg）
[用户操作]     onchange(v) → 组装完整子配置对象 → updateXxxConfig(updated)
[后端处理]     ConfigService 校验 + 原子写入 TOML + emit 'config_changed'
[前端更新]     configStore 收到 config_changed → 内部 $state 替换 → 控件自动响应
[错误恢复]     command reject → store 未变 → 控件自动回弹到旧值
```

实现要点（参考 `SettingsPage.svelte`）：

```ts
const cfg = $derived(configStore.current);

function handleTimerChange(field: keyof TimerConfig, value: number) {
  const updated: TimerConfig = { ...cfg.timer, [field]: value };
  updateTimerConfig(updated).catch((err) => console.error('Failed to update timer config:', err));
}
```

- 控件 `value` MUST 直接读 `cfg.xxx`，MUST NOT 引入中间缓冲变量。
- 每次 update 调用 MUST 传完整子配置对象（如 `TimerConfig` 整体），后端 command 是覆盖语义，前端 MUST NOT 自行做 patch。
- 复合操作（如 autostart 切换需要先调用 plugin API 再写配置）MUST 在写配置失败时回滚副作用，模式见 `handleBehaviorChange('auto_start', ...)`。

## 何时使用组件本地 `$state`

允许使用本地 `$state` 的场景：

- 纯 UI 临时态：tab 高亮、模态显隐、防抖中的 pending 标志
- 与后端无关的派生展示态

不允许的场景：

- 任何与 `configStore` / `timerStore` 内容重复的状态
- 任何"我先改了，等服务器确认"的乐观快照

## $derived 与 $effect 的选择

- 派生值 MUST 用 `$derived(...)`（如 `const cfg = $derived(configStore.current)`、`SettingsPage.svelte` 中的 `displayLanguage`、`themeOptions`）。
- `$effect` 仅用于副作用（DOM 操作、调用平台 API、向后端写入由用户动作触发的状态同步），MUST NOT 用 `$effect` 计算可派生的值。
- `$effect` 内做远程或破坏性写入时 MUST 加 `loaded`/`synced` 守卫，防止初始化期间误触发。参考 `SettingsPage.svelte` 的 `autoStartSynced` 一次性同步逻辑。

## i18n 与配置的联动

- 语言切换是"配置变更触发的副作用"，MUST 在根组件中通过 `$effect` 桥接：

```ts
$effect(() => {
  i18nStore.setLocale(configStore.current.display.language);
});
```

- i18nStore 自身 MUST NOT 监听 `config_changed`，避免循环依赖与多窗口重复订阅。配置仍是唯一数据源，i18nStore 是派生镜像。

## 常见错误

| 错误模式 | 后果 | 正确做法 |
|---------|------|---------|
| 在组件内 `let local = $state(cfg.work_minutes)` | 与 store 脱钩，事件回推不刷新 UI | 直接绑定 `cfg.work_minutes` |
| `bind:value` 父子双向绑定 | 打破单向流，子组件可篡改父级数据 | callback prop（`onchange`） |
| 调用 `updateTimerConfig` 后立刻 `state.x = newVal` | 与后端可能短暂不一致；后端拒绝时前端无回滚 | 不写本地副本，等 `config_changed` |
| 把 `configStore.current.timer` 传给子组件做编辑 | 子组件可能持有过期引用 | 子组件只接收标量 + callback |
