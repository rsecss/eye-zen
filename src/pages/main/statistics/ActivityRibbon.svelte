<script lang="ts">
  import type { CycleReason } from '$lib/bindings/CycleReason';
  import type { RibbonEntry } from '$lib/bindings/RibbonEntry';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';

  const outcomes = $derived(cycleOutcomesStore.current);
  const isRestDay = $derived(outcomes?.eye_care_index?.is_rest_day === true);

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

  function formatTime(occurredAt: string): string {
    const date = new Date(occurredAt);
    if (Number.isNaN(date.getTime())) return occurredAt;
    return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  }

  function offsetPercent(occurredAt: string): number {
    const now = Date.now();
    const ts = new Date(occurredAt).getTime();
    if (Number.isNaN(ts)) return 0;
    const ageMs = Math.max(0, now - ts);
    const dayMs = 24 * 60 * 60 * 1000;
    const pct = 100 - (ageMs / dayMs) * 100;
    return Math.max(0, Math.min(100, pct));
  }

  function tooltip(entry: RibbonEntry): string {
    const time = formatTime(entry.occurred_at);
    const outcome =
      entry.outcome === 'taken'
        ? i18nStore.t('ribbon.legend.taken')
        : entry.outcome === 'skipped'
          ? i18nStore.t('ribbon.legend.skipped')
          : i18nStore.t('ribbon.legend.suppressed');
    if (entry.reason) return `${time} · ${outcome} (${reasonLabel(entry.reason)})`;
    return `${time} · ${outcome}`;
  }
</script>

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
          style="left: {offsetPercent(entry.occurred_at)}%;"
          title={tooltip(entry)}
          aria-label={tooltip(entry)}
        ></span>
      {/each}
    </div>
  </section>
{/if}

<style>
  .ribbon-card {
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
    padding: 14px 16px;
  }

  .ribbon-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  .ribbon-header h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 16px;
    font-weight: 700;
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
</style>
