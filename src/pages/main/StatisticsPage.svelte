<script lang="ts">
  import { onMount } from 'svelte';
  import type { ECharts, EChartsOption } from 'echarts';
  import type { StatBucket } from '$lib/bindings/StatBucket';
  import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
  import { getStatisticsTrends } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';

  type TrendRange = 'daily' | 'weekly' | 'monthly';

  let chartEl: HTMLDivElement;
  let chart: ECharts | null = null;
  let stats = $state<StatisticsTrendPayload | null>(null);
  let activeRange = $state<TrendRange>('daily');
  let loading = $state(true);
  let errorMessage = $state('');

  const rangeOptions = [
    { value: 'daily', labelKey: 'statistics.range.daily' },
    { value: 'weekly', labelKey: 'statistics.range.weekly' },
    { value: 'monthly', labelKey: 'statistics.range.monthly' },
  ] as const;

  const totalMinutes = $derived(stats ? Math.round(stats.total_rest_secs / 60) : 0);

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
      if (!disposed) {
        errorMessage = i18nStore.t('statistics.error');
        loading = false;
      }
    });

    return () => {
      disposed = true;
      if (resize) window.removeEventListener('resize', resize);
      chart?.dispose();
      chart = null;
    };
  });

  async function loadStatistics(): Promise<void> {
    loading = true;
    errorMessage = '';
    try {
      const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
      const payload = await getStatisticsTrends(timezone);
      stats = payload;
      renderChart(payload);
    } catch (err) {
      errorMessage = i18nStore.t('statistics.error');
      console.error('Failed to load statistics:', err);
    } finally {
      loading = false;
    }
  }

  function setRange(range: TrendRange): void {
    activeRange = range;
    if (stats) renderChart(stats);
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
        legend: {
          top: 0,
          textStyle: { color: cssVar('--text-secondary') },
        },
        grid: {
          top: 44,
          right: 12,
          bottom: 28,
          left: 36,
          containLabel: true,
        },
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
</script>

<section class="statistics-page">
  <div class="hero-card">
    <div>
      <p class="eyebrow">{i18nStore.t('statistics.eyebrow')}</p>
      <h1>{i18nStore.t('statistics.title')}</h1>
      <p class="subtitle">{i18nStore.t('statistics.subtitle')}</p>
    </div>
    <button class="refresh-button" onclick={loadStatistics} disabled={loading}>
      {loading ? i18nStore.t('statistics.loading') : i18nStore.t('statistics.refresh')}
    </button>
  </div>

  <div class="metric-grid">
    <article class="metric-card">
      <span>{i18nStore.t('statistics.totalSessions')}</span>
      <strong>{stats?.total_sessions ?? 0}</strong>
    </article>
    <article class="metric-card">
      <span>{i18nStore.t('statistics.totalMinutes')}</span>
      <strong>{totalMinutes}</strong>
    </article>
    <article class="metric-card">
      <span>{i18nStore.t('statistics.timezone')}</span>
      <strong class="timezone">{stats?.timezone ?? 'UTC'}</strong>
    </article>
  </div>

  <section class="chart-card" aria-label={i18nStore.t('statistics.chartLabel')}>
    <div class="chart-toolbar">
      <div>
        <h2>{i18nStore.t('statistics.trends')}</h2>
        <p>{stats ? bucketSummary(stats[activeRange]) : i18nStore.t('statistics.loading')}</p>
      </div>
      <div class="range-tabs" role="group" aria-label={i18nStore.t('statistics.rangeLabel')}>
        {#each rangeOptions as option}
          <button
            class:active={activeRange === option.value}
            aria-pressed={activeRange === option.value}
            onclick={() => setRange(option.value)}
          >
            {i18nStore.t(option.labelKey)}
          </button>
        {/each}
      </div>
    </div>

    {#if errorMessage}
      <div class="state-message" role="alert">{errorMessage}</div>
    {:else if stats && stats[activeRange].length === 0}
      <div class="state-message">{i18nStore.t('statistics.empty')}</div>
    {/if}

    <div class="chart-shell" bind:this={chartEl}></div>
  </section>
</section>

<style>
  .statistics-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .hero-card,
  .metric-card,
  .chart-card {
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
  }

  .hero-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px;
    background:
      radial-gradient(circle at top right, var(--accent-soft), transparent 34%), var(--bg-card);
  }

  .eyebrow {
    margin: 0 0 4px;
    color: var(--accent-deep);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1,
  h2,
  .subtitle {
    margin: 0;
  }

  h1 {
    color: var(--text-primary);
    font-size: 24px;
    font-weight: 750;
  }

  h2 {
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 700;
  }

  .subtitle,
  .chart-toolbar p,
  .metric-card span {
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .refresh-button,
  .range-tabs button {
    border: none;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: var(--transition);
  }

  .refresh-button {
    padding: 9px 14px;
    background: var(--accent);
    color: var(--accent-foreground);
  }

  .refresh-button:disabled {
    cursor: wait;
    opacity: 0.72;
  }

  .metric-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .metric-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px;
  }

  .metric-card strong {
    color: var(--text-primary);
    font-size: 24px;
    line-height: 1;
  }

  .metric-card strong.timezone {
    font-size: 15px;
    line-height: 1.2;
    word-break: break-word;
  }

  .chart-card {
    padding: 16px;
  }

  .chart-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }

  .chart-toolbar p {
    margin: 4px 0 0;
  }

  .range-tabs {
    display: flex;
    gap: 6px;
    padding: 4px;
    background: var(--bg-tab);
    border-radius: var(--radius-md);
  }

  .range-tabs button {
    padding: 7px 10px;
    background: transparent;
    color: var(--text-secondary);
  }

  .range-tabs button.active {
    background: var(--bg-card);
    color: var(--accent-deep);
    box-shadow: var(--shadow-subtle);
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
    .hero-card,
    .chart-toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .metric-grid {
      grid-template-columns: 1fr;
    }

    .chart-shell {
      height: 240px;
    }
  }
</style>
