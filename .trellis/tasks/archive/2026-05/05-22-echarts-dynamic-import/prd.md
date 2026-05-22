# ECharts dynamic import 拆包

## Goal

把 `StatisticsPage.svelte` 里对 `echarts/*` 的 runtime import 改成 `onMount` 内 dynamic `import()`，让 ECharts 代码不再进入 main entry chunk，仅在用户打开 Statistics 页时按需加载。消除 vite `chunks larger than 500 kB` warning。

## What I already know

### 当前基线（`npm run build` 2026-05-22）

```
dist/assets/main-AK6oK_0v.js        575.63 kB │ gzip: 195.10 kB   ← 主 bundle
dist/assets/config.svelte-...        47.93 kB │ gzip:  17.76 kB
其他入口（tray/tip/tip-minimal/window/...） 各 < 14 kB
```

vite 警告：`Some chunks are larger than 500 kB after minification`。

main bundle 里 ~520KB 是 `echarts/core + charts + components + renderers`（按 v6 tree-shaken 模块计）。

### 当前 echarts 引入方式（`src/pages/main/StatisticsPage.svelte:1-20`）

```ts
import { onMount } from 'svelte';
import * as echarts from 'echarts/core';
import { BarChart, LineChart } from 'echarts/charts';
import { GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';
import type { ECharts, EChartsOption } from 'echarts';  // type-only

echarts.use([BarChart, LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

onMount(() => {
  chart = echarts.init(chartEl);
  const resize = () => chart?.resize();
  window.addEventListener('resize', resize);
  loadStatistics();
  return () => { window.removeEventListener('resize', resize); chart?.dispose(); chart = null; };
});
```

只有 `echarts.use(...)` 和 `echarts.init(...)` 是 runtime 调用，其他都是 type-only。

### 测试 mock 结构（`__tests__/StatisticsPage.test.ts:6-37`）

`vi.mock('echarts/core', ...)` + `vi.mock('echarts/charts', ...)` + `vi.mock('echarts/components', ...)` + `vi.mock('echarts/renderers', ...)`，都是 hoisted，对 static 和 dynamic import 都生效（vitest 文档保证）。`echartsMock.init` 返回的 chart stub 有 `setOption / resize / dispose`。

### 不会冲击的事

- `import type { ECharts, EChartsOption } from 'echarts'` 是 type-only，TS 编译阶段擦除，**不产生 runtime 代码**，保留为 static import 即可
- `echarts` 包本身只用作 type 入口，runtime 不会被打包
- `vite.config.ts` 已有 4 个 multi-entry HTML 入口，不需要 manualChunks（Vite 6 默认会把 dynamic import 自动拆 chunk）

## Decision (ADR-lite)

**Context**: ECharts 是 Statistics 页面专用，但目前静态 import 让它进入 main entry chunk，所有打开主窗口的用户（即使从不点 Statistics）都为 195KB gzip 买单。

**Decision**:
- 改成 `onMount` 内 `await Promise.all([import('echarts/core'), ...])` 动态加载
- type-only `import type ... from 'echarts'` 保留 static
- 引入 `disposed` flag 防止 dynamic import resolve 时组件已 unmount 引发的 race
- 不动 vite.config.ts（不需要显式 manualChunks，Vite 自动拆）
- 不动测试（hoisted vi.mock 自动覆盖 dynamic import）

**Consequences**:
- 用户首次进入 Statistics 页面会多一次网络/磁盘读取（本地应用：磁盘 IO，~5-20ms）
- main chunk 预计降到 ~55KB（去掉 ~520KB ECharts）
- vite chunk size warning 消失
- onMount 内逻辑变成 async，要小心 cleanup 时序（unmount 时 dynamic import 可能还在进行）

## Requirements

### 修改文件

仅一个：`src/pages/main/StatisticsPage.svelte`。

### 改动具体形态

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { ECharts, EChartsOption } from 'echarts';  // ← 保留 type-only static
  import type { StatBucket } from '$lib/bindings/StatBucket';
  import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
  import { getStatisticsTrends } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';

  // 移除：echarts.use([...]) 这行（移到 dynamic 加载后）

  type TrendRange = 'daily' | 'weekly' | 'monthly';

  let chartEl: HTMLDivElement;
  let chart: ECharts | null = null;
  // ... 原 state 不变

  onMount(() => {
    let disposed = false;
    let resize: (() => void) | null = null;

    (async () => {
      const [core, charts, components, renderers] = await Promise.all([
        import('echarts/core'),
        import('echarts/charts'),
        import('echarts/components'),
        import('echarts/renderers'),
      ]);
      if (disposed) return;

      core.use([
        charts.BarChart,
        charts.LineChart,
        components.GridComponent,
        components.LegendComponent,
        components.TooltipComponent,
        renderers.CanvasRenderer,
      ]);
      chart = core.init(chartEl);
      resize = () => chart?.resize();
      window.addEventListener('resize', resize);
      await loadStatistics();
    })().catch((err) => {
      console.error('Failed to initialize echarts:', err);
      if (!disposed) errorMessage = i18nStore.t('statistics.error');
    });

    return () => {
      disposed = true;
      if (resize) window.removeEventListener('resize', resize);
      chart?.dispose();
      chart = null;
    };
  });

  // renderChart / loadStatistics / setRange / ... 保持原样
</script>
```

### 关键约束

- `chart?.setOption` 调用前必须 await dynamic import 完成（loadStatistics 已经在 IIFE 内 await，自然满足）
- `disposed` 标志守住三种 race：
  1. dynamic import 进行中 unmount → 不调 `core.use / init`
  2. loadStatistics 进行中 unmount → 已有 `loading = false` 流程不变
  3. error 路径 → 不写 errorMessage 到已销毁组件
- 不引入新的 svelte runes（`$effect` 不需要，原 `onMount` cleanup 模式够用）

## Definition of Done

1. **构建产物**
   - `npm run build` 输出里 `main-*.js` 从 575.63 kB 降到 **< 100 kB**（gzip < 35 kB）
   - 出现一个新的 `echarts-*` 或类似名字的 async chunk（~520 kB）
   - 不再有 `chunks are larger than 500 kB` 警告

2. **测试**
   - `npm test` 全绿（StatisticsPage 两个测试用例）
   - `npx svelte-check --tsconfig ./tsconfig.json` 0 error
   - `npm run format:check` 0 error
   - `cargo` 相关不动，无需重跑

3. **手动验证**（dev mode）
   - `npm run tauri dev` → 打开主窗 → 切换到 Statistics tab → 图表正常显示
   - 切换 daily/weekly/monthly → 数据正常切换
   - 关闭主窗再开 → 图表正常重建（dispose race 不出问题）
   - 浏览器 DevTools Network（webview）→ 首次进 Statistics 页时看到 echarts chunk 单独请求

## Out of scope

- 不动 `vite.config.ts`（不加 manualChunks；Vite 自动拆）
- 不改 ECharts 版本（仍 `~6.1.0`）
- 不引入 loading skeleton（dynamic import < 20ms 本地）—— 现有 `loading` state 已经够用
- 不为 Statistics 页加 e2e（沿用 unit test mock 策略）
- 不动其他页面（StatisticsPage 是 echarts 唯一消费者，grep 已确认）

## Risks

| Risk | Mitigation |
|---|---|
| dynamic import 在测试环境跟 hoisted vi.mock 协作失败 | vitest 文档明确 hoisted mock 对 dynamic import 生效；如果异常，回退方案是用 `await import()` 顶层语句 + describe 内 await |
| `chart` 在 dynamic import 完成前被 renderChart 访问 | renderChart 只在 `loadStatistics` 内被调用，而 loadStatistics 是 dynamic import await 后才 call，时序安全 |
| unmount 时 dynamic import 还在 fly | `disposed` flag 拦截后续 mutation |
| Vite 拆出的 chunk 文件名变化可能影响 CSP（如果有） | Tauri webview 走 `tauri://localhost`，对 chunk 文件无 CSP 限制；已确认 |

## References

- 来源：[post-v030-cleanup-queue](C:/Users/rsecs/.claude/projects/E--Git-vibe-eye-zen/memory/post-v030-cleanup-queue.md) — 原本规划在下一个 feature PR 顺手做，因 Phase 2 已收尾、Phase 3 未启动，改单独 chore PR
- v6.x ECharts modular import：`echarts/core` + `echarts/charts` 等 是 v5 引入的官方 tree-shaken 模式
- Vite 6 manualChunks 文档：https://rollupjs.org/configuration-options/#output-manualchunks（本任务不使用，依赖默认 dynamic import splitting）
