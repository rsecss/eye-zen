# Store 与 IPC 模式

> 关键词约定：**MUST** = 强制，**SHOULD** = 推荐，**MAY** = 可选
>
> Svelte 没有 React Hook 概念。本文件覆盖：基于 `$state` 的 Svelte stores 设计、生命周期约定、Tauri IPC 封装层。

---

## Stores 设计原则

- Store 文件 MUST 命名为 `<name>.svelte.ts`（让 Svelte 编译器识别 Runes）。
- Store MUST 以模块单例形式导出（`export const xxxStore = { ... }`），MUST NOT 暴露内部 `$state` 变量。
- Store 暴露读取 MUST 用 getter（`get current()`），不暴露 setter；外部 MUST NOT 直接赋值。
- Store 内部 MUST 维护一个 `version` 计数器，用于防止 init/event 竞态（详见下节）。
- Store MUST 提供 `init()` 与 `destroy()` 生命周期方法。

参考 `src/lib/stores/config.svelte.ts` 与 `src/lib/stores/timer.svelte.ts`。

## init / event 竞态防护

后端推送事件可能在 `getConfig()`（或 `getStateSnapshot()`）的快照返回之前到达。直接覆盖会导致**新事件被旧快照覆写**。Store MUST 用 version counter 守卫：

```ts
async init(): Promise<void> {
  unlisten?.();
  unlisten = null;

  const initVersion = ++version;

  const newUnlisten = await onConfigChanged((payload) => {
    version++;        // 任何事件到达都自增
    config = payload;
    loaded = true;
  });

  try {
    const snapshot = await getConfig();
    // 只有在快照请求期间无事件抵达时，才用快照覆盖
    if (version === initVersion) {
      config = snapshot;
    }
    loaded = true;
  } catch (err) {
    newUnlisten();    // 快照失败则回滚监听器
    throw err;
  }

  unlisten = newUnlisten;
}
```

- MUST 先注册 `listen()`，再 `await` 快照；反过来会丢事件。
- 快照请求失败时 MUST 调用 `newUnlisten()` 回滚，避免悬挂监听器。
- `init()` MUST 是幂等的：进入时先 `unlisten?.()` 清理旧订阅。
- 完整实现见 `src/lib/stores/config.svelte.ts` 与 `src/lib/stores/timer.svelte.ts`。

## `loaded` 标志

- 配置类 store MUST 提供 `loaded: boolean` getter（如 `configStore.loaded`）。
- 「初始配置已抵达」MUST 用 `loaded === true` 判断，MUST NOT 用 `version > 0`（版本号在事件抵达时就自增，与快照是否到达无关）。
- 依赖配置的 `$effect`（如主题同步、autostart 同步）MUST 先检查 `configStore.loaded`，否则会读到默认值并误触发副作用。参考 `MainApp.svelte` 与 `SettingsPage.svelte` 的 `autoStartSynced` 守卫。

## Store 生命周期模式（在组件中使用）

页面组件 MUST 在 `onMount` 里 `init` 并在 cleanup 里 `destroy`：

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { configStore } from '$lib/stores/config.svelte';

  onMount(() => {
    configStore.init().catch((err) => console.error('Failed to init config store:', err));
    return () => configStore.destroy();
  });
</script>
```

- `init()` MUST 在 `onMount` 内调用（DOM 已就绪，监听器 lifecycle 与组件挂载对齐）。
- `destroy()` MUST 通过 `onMount` 的 cleanup 返回值调用。
- MUST NOT 在 `<script>` 模块顶层（onMount 外）调用 `init()`——会泄漏跨组件状态、绕过 unmount 清理。
- 同一 store 在 ≥2 个窗口共用时仍然是各自的模块实例（窗口间不共享内存），各自 init/destroy。

## IPC Command 封装（`src/lib/commands.ts`）

- 前端 MUST NOT 直接调用 `@tauri-apps/api/core` 的 `invoke()`；所有 command MUST 经过 `commands.ts` 中的封装函数。
- 封装函数 MUST 有精确的 TS 类型签名，类型来自 `$lib/bindings/`。
- 所有 invoke 调用 MUST 通过 `invokeWithTimeout`（5 秒超时），并在 `.finally` 中清理 timer，避免悬挂的 `setTimeout`：

```ts
const INVOKE_TIMEOUT_MS = 5000;

function invokeWithTimeout<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  let timeoutId: ReturnType<typeof setTimeout>;
  return Promise.race([
    invoke<T>(cmd, args),
    new Promise<never>((_, reject) => {
      timeoutId = setTimeout(
        () => reject(new Error(`Command "${cmd}" timed out`)),
        INVOKE_TIMEOUT_MS,
      );
    }),
  ]).finally(() => clearTimeout(timeoutId));
}

export function getConfig(): Promise<Config> {
  return invokeWithTimeout('get_config');
}
```

- 新增 command 时 MUST 在 `commands.ts` 添加封装函数（参考 `updateTimerConfig`、`startRest` 等）。
- MUST NOT 在业务组件里手写 `invoke('xxx')`；如果发现需要绕过封装，先回到 `commands.ts` 补一个函数。

## IPC Event 封装（`src/lib/events.ts`）

- `listen()` / `emit()` 同样 MUST 走 `events.ts` 封装：

```ts
export function onStateChanged(callback: (payload: StatePayload) => void): Promise<UnlistenFn> {
  return listen<StatePayload>('state_changed', (e) => callback(e.payload));
}

export function emitNavigateTab(tab: string): Promise<void> {
  return emit('navigate_tab', tab);
}
```

- `listen()` 返回 `Promise<UnlistenFn>`；调用方 MUST 在组件卸载或 store 销毁时 `await` 并执行 unlisten。Stores 内通过保存 `unlisten` 句柄实现；组件级监听看 `MainApp.svelte` 中 `onNavigateTab` 的 cleanup 模式：

```ts
const unlistenNav = onNavigateTab((tab) => { /* ... */ });
return () => { unlistenNav.then((fn) => fn()); };
```

- MUST NOT 在多个组件里重复监听同一事件做相同逻辑；应当聚合到 store。
- 新增 event 时 MUST 在 `events.ts` 添加 `onXxx(callback)` 函数，Payload 类型来自 `$lib/bindings/`。

## 错误处理

- 封装函数返回的 Promise reject 时 MUST 由调用方处理（通常 `.catch` 打 `console.error`）。
- 即时保存场景 MUST NOT 把错误吞掉，至少要 `console.error`；UI 状态会通过 `config_changed` 缺席而自动回弹。
- 自启动等"侧操作 + 配置写入"的复合流程 MUST 在配置写入失败时回滚副作用，参考 `SettingsPage.svelte` 中 `handleBehaviorChange('auto_start', ...)` 的回滚分支。

## 测试

- Stores MUST 写单元测试覆盖 init/destroy、event 优先 vs 快照优先两种竞态分支。参考 `src/lib/stores/__tests__/timer.test.ts`。
- 测试中 mock `commands.ts` / `events.ts` 而非直接 mock `@tauri-apps/api`，与封装层职责对齐。
