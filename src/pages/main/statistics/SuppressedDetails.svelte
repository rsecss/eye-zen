<script lang="ts">
  import type { CycleReason } from '$lib/bindings/CycleReason';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { cycleOutcomesStore } from '$lib/stores/cycle-outcomes.svelte';

  const REASON_KEYS: readonly CycleReason[] = [
    'fullscreen',
    'schedule',
    'afk',
    'process_whitelisted',
  ];

  const outcomes = $derived(cycleOutcomesStore.current);
  const isRestDay = $derived(outcomes?.eye_care_index?.is_rest_day === true);

  let expanded = $state(false);

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
</script>

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
          aria-expanded={expanded}
          onclick={() => (expanded = !expanded)}
        >
          {expanded ? '−' : '+'}
        </button>
        {#if expanded}
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

<style>
  .today-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }

  .today-tile {
    position: relative;
    padding: 14px;
    background: var(--bg-card);
    border: 1px solid var(--separator);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
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

  @media (max-width: 640px) {
    .today-row {
      grid-template-columns: 1fr;
    }
  }
</style>
