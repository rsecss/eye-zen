<script lang="ts">
  import { onMount } from 'svelte';
  import type { ECharts, EChartsOption } from 'echarts';
  import type { StatBucket } from '$lib/bindings/StatBucket';
  import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import RangeSwitcher from './RangeSwitcher.svelte';

  type TrendRange = 'daily' | 'weekly' | 'monthly';

  let {
    stats,
    loading,
    errorMessage,
  }: {
    stats: StatisticsTrendPayload | null;
    loading: boolean;
    errorMessage: string;
  } = $props();

  const rangeOptions = [
    { value: 'daily', labelKey: 'statistics.range.daily' },
    { value: 'weekly', labelKey: 'statistics.range.weekly' },
    { value: 'monthly', labelKey: 'statistics.range.monthly' },
  ] as const;

  let chartEl: HTMLDivElement;
  let chart = $state<ECharts | null>(null);
  let activeRange = $state<TrendRange>('daily');

  function bucketSummary(buckets: StatBucket[]): string {
    if (buckets.length === 0) return i18nStore.t('statistics.empty');
    const latest = buckets[buckets.length - 1];
    return i18nStore.t('statistics.latest').replace('{label}', latest.label);
  }

  function cssVar(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function themeColors(): string[] {
    return [cssVar('--accent'), cssVar('--state-active-label')].filter(Boolean);
  }

  function renderChart(payload: StatisticsTrendPayload): void {
    const buckets = payload[activeRange];
    const labels = buckets.map((bucket) => bucket.label);
    const sessionCounts = buckets.map((bucket) => bucket.rest_sessions);
    const restMinutes = buckets.map((bucket) => Math.round(bucket.total_rest_secs / 60));

    chart?.setOption(
      {
        color: themeColors(),
        tooltip: { trigger: 'axis' },
        legend: { top: 0, textStyle: { color: cssVar('--text-secondary') } },
        grid: { top: 44, right: 12, bottom: 28, left: 36, containLabel: true },
        xAxis: {
          type: 'category',
          data: labels,
          axisLabel: { color: cssVar('--text-tertiary') },
          axisLine: { lineStyle: { color: cssVar('--separator') } },
        },
        yAxis: [
          {
            type: 'value',
            minInterval: 1,
            axisLabel: { color: cssVar('--text-tertiary') },
            splitLine: { lineStyle: { color: cssVar('--separator') } },
          },
          {
            type: 'value',
            minInterval: 1,
            axisLabel: { color: cssVar('--text-tertiary') },
            splitLine: { show: false },
          },
        ],
        series: [
          {
            name: i18nStore.t('statistics.chart.sessions'),
            type: 'bar',
            data: sessionCounts,
            yAxisIndex: 0,
            barMaxWidth: 28,
            itemStyle: { borderRadius: [6, 6, 2, 2] },
          },
          {
            name: i18nStore.t('statistics.chart.minutes'),
            type: 'line',
            data: restMinutes,
            yAxisIndex: 1,
            smooth: true,
            symbolSize: 7,
          },
        ],
      } satisfies EChartsOption,
      true,
    );
  }

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
      // Eager first render: covers the case where `stats` resolved before
      // ECharts finished its async import (the effect alone would also fire,
      // but only after the next microtask, which is too late for tests that
      // synchronously assert on setOption right after the DOM text settles).
      if (stats) renderChart(stats);
    })().catch((err) => {
      console.error('Failed to initialize echarts:', err);
    });

    return () => {
      disposed = true;
      if (resize) window.removeEventListener('resize', resize);
      chart?.dispose();
      chart = null;
    };
  });

  $effect(() => {
    // Re-render whenever stats, activeRange, or chart changes. Reading
    // `stats[activeRange]` inside renderChart wires activeRange tracking.
    if (stats && chart) renderChart(stats);
  });

  function handleRangeChange(next: string): void {
    if (next === 'daily' || next === 'weekly' || next === 'monthly') {
      activeRange = next;
    }
  }
</script>

<section class="chart-card" aria-label={i18nStore.t('statistics.chartLabel')}>
  <div class="chart-toolbar">
    <div>
      <h2>{i18nStore.t('statistics.trends')}</h2>
      <p>{stats ? bucketSummary(stats[activeRange]) : i18nStore.t('statistics.loading')}</p>
    </div>
    <RangeSwitcher
      value={activeRange}
      options={rangeOptions}
      label={i18nStore.t('statistics.rangeLabel')}
      onchange={handleRangeChange}
    />
  </div>

  {#if errorMessage}
    <div class="state-message" role="alert">{errorMessage}</div>
  {:else if stats && stats[activeRange].length === 0}
    <div class="state-message">{i18nStore.t('statistics.empty')}</div>
  {:else if loading && !stats}
    <div class="state-message">{i18nStore.t('statistics.loading')}</div>
  {/if}

  <div class="chart-shell" bind:this={chartEl}></div>
</section>

<style>
  .chart-card {
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
    padding: 16px;
  }

  .chart-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .chart-toolbar h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 700;
  }

  .chart-toolbar p {
    margin: 4px 0 0;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .state-message {
    padding: 24px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .chart-shell {
    width: 100%;
    height: 280px;
  }

  @media (max-width: 640px) {
    .chart-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .chart-shell {
      height: 240px;
    }
  }
</style>
