<script lang="ts">
  import { i18nStore } from '$lib/i18n/index.svelte';
  import type { TranslationKey } from '$lib/i18n/zh-CN';

  type RangeOption = { value: string; labelKey: TranslationKey };

  let {
    value,
    options,
    label,
    onchange,
  }: {
    value: string;
    options: readonly RangeOption[];
    label: string;
    onchange: (value: string) => void;
  } = $props();
</script>

<div class="range-tabs" role="group" aria-label={label}>
  {#each options as option (option.value)}
    <button
      class:active={value === option.value}
      aria-pressed={value === option.value}
      onclick={() => onchange(option.value)}
    >
      {i18nStore.t(option.labelKey)}
    </button>
  {/each}
</div>

<style>
  .range-tabs {
    display: flex;
    gap: 6px;
    padding: 4px;
    background: var(--bg-tab);
    border-radius: var(--radius-md);
  }

  .range-tabs button {
    border: none;
    border-radius: var(--radius-sm);
    font-family: inherit;
    font-weight: 650;
    cursor: pointer;
    transition: var(--transition);
    padding: 7px 10px;
    background: transparent;
    color: var(--text-secondary);
  }

  .range-tabs button.active {
    background: var(--bg-card);
    color: var(--accent-deep);
    box-shadow: var(--shadow-subtle);
  }
</style>
