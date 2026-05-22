<script lang="ts">
  import { onMount } from 'svelte';
  import type { ECharts, EChartsOption } from 'echarts';
  import type { StatBucket } from '$lib/bindings/StatBucket';
  import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
  import type { CycleReason } from '$lib/bindings/CycleReason';
  import type { RibbonEntry } from '$lib/bindings/RibbonEntry';
  import { exportStatistics, getStatisticsTrends } from '$lib/commands';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { save } from '@tauri-apps/plugin-dialog';

  type TrendRange = 'daily' | 'weekly' | 'monthly';

  const REASON_KEYS: readonly CycleReason[] = [
    'fullscreen',
    'schedule',
    'afk',
    'process_whitelisted',
  ];

  let chartEl: HTMLDivElement;
  let chart: ECharts | null = null;
  let stats = $state<StatisticsTrendPayload | null>(null);
  let activeRange = $state<TrendRange>('daily');
  let loading = $state(true);
  let errorMessage = $state('');
  let exporting = $state(false);
  let exportSuccess = $state<string | null>(null);
  let exportError = $state<string | null>(null);
  let suppressedExpanded = $state(false);

  const rangeOptions = [
    { value: 'daily', labelKey: 'statistics.range.daily' },
    { value: 'weekly', labelKey: 'statistics.range.weekly' },
    { value: 'monthly', labelKey: 'statistics.range.monthly' },
  ] as const;

  const totalMinutes = $derived(stats ? Math.round(stats.total_rest_secs / 60) : 0);
  const outcomes = $derived(cycleOutcomesStore.current);
  const eci = $derived(outcomes?.eye_care_index ?? null);
  const isRestDay = $derived(eci?.is_rest_day === true);
  const isWarmingUp = $derived(eci?.is_warming_up === true);

  const eciThresholdKey = $derived.by<
    'index.hero.threshold.good' | 'index.hero.threshold.okay' | 'index.hero.threshold.attention'
  >(() => {
    const score = eci?.score;
    if (typeof score !== 'number') return 'index.hero.threshold.okay';
    if (score >= 80) return 'index.hero.threshold.good';
    if (score >= 60) return 'index.hero.threshold.okay';
    return 'index.hero.threshold.attention';
  });

  const eciAccentColor = $derived.by(() => {
    const score = eci?.score;
    if (typeof score !== 'number') return 'var(--accent)';
    if (score >= 80) return 'var(--state-active)';
    if (score >= 60) return 'var(--state-alert)';
    return 'var(--state-alert-label)';
  });

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
      cycleOutcomesStore.reset();
    };
  });

  async function loadStatistics(): Promise<void> {
    loading = true;
    errorMessage = '';
    try {
      const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
      const [payload] = await Promise.all([
        getStatisticsTrends(timezone),
        cycleOutcomesStore.refresh(),
      ]);
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

  function defaultExportFilename(): string {
    const now = new Date();
    const year = now.getFullYear();
    const month = String(now.getMonth() + 1).padStart(2, '0');
    const day = String(now.getDate()).padStart(2, '0');
    return `eyezen-stat-${year}-${month}-${day}.db`;
  }

  function humanizeError(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (
      typeof err === 'object' &&
      err !== null &&
      'detail' in err &&
      typeof (err as { detail?: unknown }).detail === 'object'
    ) {
      const detail = (err as { detail: Record<string, unknown> }).detail;
      if (typeof detail.reason === 'string') return detail.reason;
      if (typeof detail.message === 'string') return detail.message;
    }
    if (typeof err === 'string') return err;
    return String(err);
  }

  async function handleExportBackup(): Promise<void> {
    if (exporting) return;
    exportError = null;
    exportSuccess = null;

    let targetPath: string | null;
    try {
      targetPath = await save({
        title: i18nStore.t('statistics.exportBackup.dialogTitle'),
        defaultPath: defaultExportFilename(),
        filters: [{ name: 'SQLite Database', extensions: ['db'] }],
      });
    } catch (err) {
      console.error('Failed to open save dialog:', err);
      exportError = i18nStore
        .t('statistics.exportBackup.toastError')
        .replace('{reason}', humanizeError(err));
      return;
    }
    if (targetPath === null) return;

    exporting = true;
    try {
      await exportStatistics(targetPath);
      exportSuccess = i18nStore
        .t('statistics.exportBackup.toastSuccess')
        .replace('{path}', targetPath);
    } catch (err) {
      console.error('Failed to export statistics:', err);
      exportError = i18nStore
        .t('statistics.exportBackup.toastError')
        .replace('{reason}', humanizeError(err));
    } finally {
      exporting = false;
    }
  }

  function cssVar(name: string): string {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  function themeColors(): string[] {
    return [cssVar('--accent'), cssVar('--state-active-label')].filter(Boolean);
  }

  function reasonLabel(reason: CycleReason): string {
    switch (reason) {
      case 'fullscreen':
        return i18nStore.t('today.reason.fullscreen');
      case 'schedule':
        return i18nStore.t('today.reason.schedule');
      case 'afk':
        return i18nStore.t('today.reason.afk');
      case 'process_whitelisted':
        return i18nStore.t('today.reason.process_whitelisted');
    }
  }

  function formatRibbonTime(occurredAt: string): string {
    const date = new Date(occurredAt);
    if (Number.isNaN(date.getTime())) return occurredAt;
    return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }

  function ribbonOffsetPercent(occurredAt: string): number {
    const now = Date.now();
    const ts = new Date(occurredAt).getTime();
    if (Number.isNaN(ts)) return 0;
    const ageMs = Math.max(0, now - ts);
    const dayMs = 24 * 60 * 60 * 1000;
    const pct = 100 - (ageMs / dayMs) * 100;
    return Math.max(0, Math.min(100, pct));
  }

  function ribbonTooltip(entry: RibbonEntry): string {
    const time = formatRibbonTime(entry.occurred_at);
    const outcome =
      entry.outcome === 'taken'
        ? i18nStore.t('ribbon.legend.taken')
        : entry.outcome === 'skipped'
          ? i18nStore.t('ribbon.legend.skipped')
          : i18nStore.t('ribbon.legend.suppressed');
    if (entry.reason) {
      return `${time} · ${outcome} (${reasonLabel(entry.reason)})`;
    }
    return `${time} · ${outcome}`;
  }
</script>

<section class="statistics-page">
  <div class="hero-card">
    <div>
      <p class="eyebrow">{i18nStore.t('statistics.eyebrow')}</p>
      <h1>{i18nStore.t('statistics.title')}</h1>
      <p class="subtitle">{i18nStore.t('statistics.subtitle')}</p>
    </div>
    <div class="hero-actions">
      <button
        class="export-button"
        onclick={handleExportBackup}
        disabled={exporting}
        aria-busy={exporting}
      >
        {i18nStore.t('statistics.exportBackup.button')}
      </button>
      <button class="refresh-button" onclick={loadStatistics} disabled={loading}>
        {loading ? i18nStore.t('statistics.loading') : i18nStore.t('statistics.refresh')}
      </button>
    </div>
  </div>

  {#if exportSuccess}
    <div class="export-banner success" role="status">{exportSuccess}</div>
  {/if}
  {#if exportError}
    <div class="export-banner error" role="alert">{exportError}</div>
  {/if}

  {#if !isRestDay && eci}
    <section
      class="index-card"
      style="--eci-accent: {eciAccentColor};"
      aria-label={i18nStore.t('index.hero.title')}
    >
      <div class="index-header">
        <div class="index-titles">
          <h2 class="index-title">{i18nStore.t('index.hero.title')}</h2>
          <span class="index-beta">{i18nStore.t('index.hero.beta')}</span>
        </div>
        <div class="index-tooltip" role="note">
          <span class="index-tooltip-anchor" aria-describedby="eci-tooltip">i</span>
          <div class="index-tooltip-body" id="eci-tooltip" role="tooltip">
            <strong>{i18nStore.t('index.hero.tooltip.title')}</strong>
            <ul>
              <li>{i18nStore.t('index.hero.tooltip.adherence')}</li>
              <li>{i18nStore.t('index.hero.tooltip.longest')}</li>
              <li class="deferred">{i18nStore.t('index.hero.tooltip.deferred')}</li>
            </ul>
          </div>
        </div>
      </div>

      <div class="index-body">
        {#if isWarmingUp}
          <div class="index-warming">
            <span class="index-warming-label">{i18nStore.t('index.hero.warming_up')}</span>
            <p>{i18nStore.t('index.hero.warming_up_hint')}</p>
          </div>
        {:else if typeof eci.score === 'number'}
          <div class="index-score">
            <span class="index-score-number">{eci.score}</span>
            <span class="index-score-suffix">/100</span>
          </div>
          <span class="index-threshold">{i18nStore.t(eciThresholdKey)}</span>
        {:else}
          <div class="index-warming">
            <span class="index-warming-label">{i18nStore.t('index.hero.warming_up')}</span>
            <p>{i18nStore.t('index.hero.warming_up_hint')}</p>
          </div>
        {/if}
      </div>
    </section>
  {/if}

  {#if !isRestDay && outcomes}
    <section class="today-row" aria-label={i18nStore.t('today.title')}>
      <article class="today-tile">
        <span class="today-label">{i18nStore.t('today.taken')}</span>
        <strong class="today-value taken">{outcomes.today_taken}</strong>
      </article>
      <article class="today-tile">
        <span class="today-label">{i18nStore.t('today.skipped')}</span>
        <strong class="today-value skipped">{outcomes.today_skipped}</strong>
      </article>
      <article class="today-tile suppressed-tile">
        <span class="today-label">{i18nStore.t('today.suppressed')}</span>
        <strong class="today-value suppressed">{outcomes.today_suppressed}</strong>
        {#if outcomes.today_suppressed > 0}
          <button
            type="button"
            class="suppressed-toggle"
            aria-expanded={suppressedExpanded}
            onclick={() => (suppressedExpanded = !suppressedExpanded)}
          >
            {suppressedExpanded ? '−' : '+'}
          </button>
          {#if suppressedExpanded}
            <ul class="reason-list" role="list">
              {#each REASON_KEYS as reason (reason)}
                {@const count = outcomes.today_reason_breakdown[reason]}
                {#if count > 0}
                  <li>
                    <span class="reason-label">{reasonLabel(reason)}</span>
                    <span class="reason-count">{count}</span>
                  </li>
                {/if}
              {/each}
            </ul>
          {/if}
        {/if}
      </article>
    </section>
  {/if}

  {#if !isRestDay && outcomes && outcomes.last_24h_ribbon.length > 0}
    <section class="ribbon-card" aria-label={i18nStore.t('ribbon.title')}>
      <div class="ribbon-header">
        <h2>{i18nStore.t('ribbon.title')}</h2>
        <ul class="ribbon-legend" role="list">
          <li><span class="legend-dot taken"></span>{i18nStore.t('ribbon.legend.taken')}</li>
          <li><span class="legend-dot skipped"></span>{i18nStore.t('ribbon.legend.skipped')}</li>
          <li>
            <span class="legend-dot suppressed"></span>{i18nStore.t('ribbon.legend.suppressed')}
          </li>
        </ul>
      </div>
      <div class="ribbon-track" role="presentation">
        {#each outcomes.last_24h_ribbon as entry, i (i)}
          <span
            class="ribbon-marker {entry.outcome}"
            style="left: {ribbonOffsetPercent(entry.occurred_at)}%;"
            title={ribbonTooltip(entry)}
            aria-label={ribbonTooltip(entry)}
          ></span>
        {/each}
      </div>
    </section>
  {/if}

  {#if outcomes}
    <section class="rhythm-row" aria-label={i18nStore.t('rhythm.current.title')}>
      <article class="rhythm-card">
        <span class="rhythm-label">{i18nStore.t('rhythm.current.title')}</span>
        <strong class="rhythm-value"
          >{i18nStore
            .t('rhythm.streak.days')
            .replace('{n}', String(outcomes.rhythm.current_streak_days))}</strong
        >
        <span class="rhythm-caption"
          >{i18nStore.t('rhythm.threshold').replace('{n}', String(outcomes.rhythm.threshold))}</span
        >
      </article>
      <article class="rhythm-card">
        <span class="rhythm-label">{i18nStore.t('rhythm.best.title')}</span>
        <strong class="rhythm-value"
          >{i18nStore
            .t('rhythm.streak.days')
            .replace('{n}', String(outcomes.rhythm.best_streak_days))}</strong
        >
        <span class="rhythm-caption"
          >{i18nStore.t('rhythm.threshold').replace('{n}', String(outcomes.rhythm.threshold))}</span
        >
      </article>
    </section>
  {/if}

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

  {#if isRestDay}
    <div class="rest-day-note" role="status">
      <strong>{i18nStore.t('index.hero.rest_day')}</strong>
      <span>{i18nStore.t('index.hero.rest_day_hint')}</span>
    </div>
  {/if}

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
  .chart-card,
  .index-card,
  .today-tile,
  .ribbon-card,
  .rhythm-card {
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
    color: var(--accent-foreground, #fff);
  }

  .refresh-button:disabled {
    cursor: wait;
    opacity: 0.72;
  }

  .hero-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .export-button {
    padding: 9px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--accent-deep);
    font-family: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: var(--transition);
  }

  .export-button:hover:not(:disabled) {
    background: var(--accent-soft);
  }

  .export-button:disabled {
    cursor: wait;
    opacity: 0.55;
  }

  .export-banner {
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent-border);
    background: var(--accent-soft);
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
    word-break: break-word;
  }

  .export-banner.error {
    border-color: var(--state-alert);
    background: color-mix(in srgb, var(--state-alert) 10%, transparent);
    color: var(--text-primary);
  }

  /* ECI hero */
  .index-card {
    padding: 18px;
    background:
      radial-gradient(
        circle at top left,
        color-mix(in srgb, var(--eci-accent) 12%, transparent),
        transparent 38%
      ),
      var(--bg-card);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .index-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .index-titles {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .index-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .index-beta {
    color: var(--accent-deep);
    background: var(--accent-soft);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    padding: 2px 8px;
    font-size: 11px;
    font-weight: 600;
    align-self: flex-start;
  }

  .index-tooltip {
    position: relative;
  }

  .index-tooltip-anchor {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid var(--separator);
    color: var(--text-tertiary);
    font-size: 11px;
    font-weight: 700;
    background: var(--bg-card);
    cursor: help;
  }

  .index-tooltip-body {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    min-width: 240px;
    padding: 10px 12px;
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card-hover);
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.55;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--transition);
    z-index: 10;
  }

  .index-tooltip:hover .index-tooltip-body,
  .index-tooltip:focus-within .index-tooltip-body {
    opacity: 1;
    pointer-events: auto;
  }

  .index-tooltip-body strong {
    display: block;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .index-tooltip-body ul {
    margin: 0;
    padding-left: 18px;
  }

  .index-tooltip-body li.deferred {
    margin-top: 4px;
    color: var(--text-tertiary);
    font-style: italic;
  }

  .index-body {
    display: flex;
    align-items: center;
    gap: 18px;
  }

  .index-score {
    display: flex;
    align-items: baseline;
    gap: 4px;
    padding: 8px 16px;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--eci-accent) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--eci-accent) 30%, transparent);
  }

  .index-score-number {
    font-size: 44px;
    font-weight: 750;
    color: var(--eci-accent);
    line-height: 1;
  }

  .index-score-suffix {
    font-size: 14px;
    color: var(--text-tertiary);
    font-weight: 600;
  }

  .index-threshold {
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--eci-accent) 14%, transparent);
    color: var(--eci-accent);
    font-size: 12px;
    font-weight: 650;
  }

  .index-warming {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .index-warming-label {
    color: var(--text-primary);
    font-size: 18px;
    font-weight: 700;
  }

  .index-warming p {
    margin: 0;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  /* Today tiles */
  .today-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .today-tile {
    position: relative;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .today-label {
    color: var(--text-tertiary);
    font-size: 12px;
    font-weight: 600;
  }

  .today-value {
    color: var(--text-primary);
    font-size: 24px;
    line-height: 1;
  }

  .today-value.taken {
    color: var(--state-active-label);
  }

  .today-value.skipped {
    color: var(--state-alert-label);
  }

  .today-value.suppressed {
    color: var(--text-secondary);
  }

  .suppressed-toggle {
    position: absolute;
    top: 10px;
    right: 10px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid var(--separator);
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    font-weight: 700;
    transition: var(--transition);
  }

  .suppressed-toggle:hover {
    background: var(--accent-soft);
    color: var(--accent-deep);
  }

  .reason-list {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .reason-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .reason-label {
    color: var(--text-secondary);
  }

  .reason-count {
    color: var(--text-primary);
    font-weight: 650;
  }

  /* Ribbon */
  .ribbon-card {
    padding: 14px 16px;
  }

  .ribbon-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  .ribbon-legend {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    gap: 12px;
  }

  .ribbon-legend li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-tertiary);
    font-size: 11px;
  }

  .legend-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .legend-dot.taken,
  .ribbon-marker.taken {
    background: var(--state-active);
  }

  .legend-dot.skipped,
  .ribbon-marker.skipped {
    background: var(--state-alert);
  }

  .legend-dot.suppressed,
  .ribbon-marker.suppressed {
    background: var(--text-tertiary);
  }

  .ribbon-track {
    position: relative;
    width: 100%;
    height: 18px;
    border-radius: 999px;
    background: var(--bg-tab);
    border: 1px solid var(--separator);
    overflow: hidden;
  }

  .ribbon-marker {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--bg-card);
    cursor: help;
  }

  /* Rhythm */
  .rhythm-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .rhythm-card {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .rhythm-label {
    color: var(--text-tertiary);
    font-size: 12px;
    font-weight: 600;
  }

  .rhythm-value {
    color: var(--text-primary);
    font-size: 22px;
    font-weight: 700;
    line-height: 1;
  }

  .rhythm-caption {
    color: var(--text-tertiary);
    font-size: 11px;
  }

  .rest-day-note {
    padding: 12px 14px;
    border: 1px solid var(--separator);
    background: var(--bg-tab);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
  }

  .rest-day-note strong {
    color: var(--text-primary);
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
    .chart-toolbar,
    .index-header,
    .index-body {
      flex-direction: column;
      align-items: stretch;
    }

    .metric-grid,
    .today-row,
    .rhythm-row {
      grid-template-columns: 1fr;
    }

    .chart-shell {
      height: 240px;
    }
  }
</style>
