<script lang="ts">
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';

  const outcomes = $derived(cycleOutcomesStore.current);
  const eci = $derived(outcomes?.eye_care_index ?? null);
  const isRestDay = $derived(eci?.is_rest_day === true);
  const isWarmingUp = $derived(eci?.is_warming_up === true);

  const thresholdKey = $derived.by<
    'index.hero.threshold.good' | 'index.hero.threshold.okay' | 'index.hero.threshold.attention'
  >(() => {
    const score = eci?.score;
    if (typeof score !== 'number') return 'index.hero.threshold.okay';
    if (score >= 80) return 'index.hero.threshold.good';
    if (score >= 60) return 'index.hero.threshold.okay';
    return 'index.hero.threshold.attention';
  });

  const accentColor = $derived.by(() => {
    const score = eci?.score;
    if (typeof score !== 'number') return 'var(--accent)';
    if (score >= 80) return 'var(--state-active)';
    if (score >= 60) return 'var(--state-alert)';
    return 'var(--state-alert-label)';
  });
</script>

{#if !isRestDay && eci}
  <section
    class="index-card"
    style="--eci-accent: {accentColor};"
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
      {#if isWarmingUp || typeof eci.score !== 'number'}
        <div class="index-warming">
          <span class="index-warming-label">{i18nStore.t('index.hero.warming_up')}</span>
          <p>{i18nStore.t('index.hero.warming_up_hint')}</p>
        </div>
      {:else}
        <div class="index-score">
          <span class="index-score-number">{eci.score}</span>
          <span class="index-score-suffix">/100</span>
        </div>
        <span class="index-threshold">{i18nStore.t(thresholdKey)}</span>
      {/if}
    </div>
  </section>
{/if}

<style>
  .index-card {
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
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
    margin: 0;
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

  @media (max-width: 640px) {
    .index-header,
    .index-body {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
