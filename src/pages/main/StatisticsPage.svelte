<script lang="ts">
  import { onMount } from 'svelte';
  import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
  import { getStatisticsTrends } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';
  import ActivityRibbon from './statistics/ActivityRibbon.svelte';
  import EciDisplay from './statistics/EciDisplay.svelte';
  import ExportControls from './statistics/ExportControls.svelte';
  import RhythmCards from './statistics/RhythmCards.svelte';
  import SuppressedDetails from './statistics/SuppressedDetails.svelte';
  import TrendChart from './statistics/TrendChart.svelte';

  let stats = $state<StatisticsTrendPayload | null>(null);
  let loading = $state(true);
  let errorMessage = $state('');
  let exportSuccess = $state<string | null>(null);
  let exportError = $state<string | null>(null);

  const totalMinutes = $derived(stats ? Math.round(stats.total_rest_secs / 60) : 0);
  const outcomes = $derived(cycleOutcomesStore.current);
  const isRestDay = $derived(outcomes?.eye_care_index?.is_rest_day === true);

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
    } catch (err) {
      errorMessage = i18nStore.t('statistics.error');
      console.error('Failed to load statistics:', err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadStatistics();
    return () => cycleOutcomesStore.reset();
  });
</script>

<section class="statistics-page">
  <div class="hero-card">
    <div>
      <p class="eyebrow">{i18nStore.t('statistics.eyebrow')}</p>
      <h1>{i18nStore.t('statistics.title')}</h1>
      <p class="subtitle">{i18nStore.t('statistics.subtitle')}</p>
    </div>
    <div class="hero-actions">
      <ExportControls
        onSuccess={(m) => {
          exportSuccess = m;
          exportError = null;
        }}
        onError={(m) => {
          exportError = m;
          exportSuccess = null;
        }}
      />
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

  <EciDisplay />
  <SuppressedDetails />
  <ActivityRibbon />
  <RhythmCards />

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

  <TrendChart {stats} {loading} {errorMessage} />
</section>

<style>
  .statistics-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .hero-card,
  .metric-card {
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
  .subtitle {
    margin: 0;
  }

  h1 {
    color: var(--text-primary);
    font-size: 24px;
    font-weight: 750;
  }

  .subtitle,
  .metric-card span {
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .refresh-button {
    border: none;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: var(--transition);
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
    flex-wrap: wrap;
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

  @media (max-width: 640px) {
    .hero-card {
      flex-direction: column;
      align-items: stretch;
    }

    .metric-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
