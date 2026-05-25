<script lang="ts">
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';

  const outcomes = $derived(cycleOutcomesStore.current);
</script>

{#if outcomes}
  <section class="rhythm-row" aria-label={i18nStore.t('rhythm.current.title')}>
    <article class="rhythm-card">
      <span class="rhythm-label">{i18nStore.t('rhythm.current.title')}</span>
      <strong class="rhythm-value">
        {i18nStore
          .t('rhythm.streak.days')
          .replace('{n}', String(outcomes.rhythm.current_streak_days))}
      </strong>
      <span class="rhythm-caption">
        {i18nStore.t('rhythm.threshold').replace('{n}', String(outcomes.rhythm.threshold))}
      </span>
    </article>
    <article class="rhythm-card">
      <span class="rhythm-label">{i18nStore.t('rhythm.best.title')}</span>
      <strong class="rhythm-value">
        {i18nStore.t('rhythm.streak.days').replace('{n}', String(outcomes.rhythm.best_streak_days))}
      </strong>
      <span class="rhythm-caption">
        {i18nStore.t('rhythm.threshold').replace('{n}', String(outcomes.rhythm.threshold))}
      </span>
    </article>
  </section>
{/if}

<style>
  .rhythm-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .rhythm-card {
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
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

  @media (max-width: 640px) {
    .rhythm-row {
      grid-template-columns: 1fr;
    }
  }
</style>
