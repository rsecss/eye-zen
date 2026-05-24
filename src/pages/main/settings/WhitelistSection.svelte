<script lang="ts">
  import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
  import { updateBehaviorConfig } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';
  import Toggle from '../components/Toggle.svelte';

  const WHITELIST_MAX = 32;
  const WHITELIST_RESERVED = new Set(['eyezen', 'eyezen.exe']);

  let {
    capabilitiesLoaded,
    processSupported,
  }: {
    capabilitiesLoaded: boolean;
    processSupported: boolean;
  } = $props();

  const cfg = $derived(configStore.current);
  const whitelistDisabled = $derived(!capabilitiesLoaded || !processSupported);
  const editingDisabled = $derived(whitelistDisabled || !cfg.behavior.process_whitelist_enabled);

  let whitelistInput = $state('');
  let whitelistError = $state<string | null>(null);

  function handleEnabledChange(value: boolean) {
    if (whitelistDisabled) return;
    whitelistError = null;
    const updated: BehaviorConfig = { ...cfg.behavior, process_whitelist_enabled: value };
    updateBehaviorConfig(updated).catch((err) =>
      console.error('Failed to update whitelist enabled:', err),
    );
  }

  function handleAdd() {
    if (editingDisabled) return;
    const raw = whitelistInput.trim().toLowerCase();
    if (!raw) {
      whitelistError = i18nStore.t('settings.whitelist.error.empty');
      return;
    }
    if (WHITELIST_RESERVED.has(raw)) {
      whitelistError = i18nStore.t('settings.whitelist.error.self');
      return;
    }
    if (cfg.behavior.process_whitelist.includes(raw)) {
      whitelistError = i18nStore.t('settings.whitelist.error.duplicate');
      return;
    }
    if (cfg.behavior.process_whitelist.length >= WHITELIST_MAX) {
      whitelistError = i18nStore
        .t('settings.whitelist.error.limit')
        .replace('{max}', String(WHITELIST_MAX));
      return;
    }
    const next = [...cfg.behavior.process_whitelist, raw];
    const updated: BehaviorConfig = { ...cfg.behavior, process_whitelist: next };
    updateBehaviorConfig(updated)
      .then(() => {
        whitelistInput = '';
        whitelistError = null;
      })
      .catch((err) => console.error('Failed to update whitelist:', err));
  }

  function handleRemove(index: number) {
    if (whitelistDisabled) return;
    const next = cfg.behavior.process_whitelist.filter((_, i) => i !== index);
    const updated: BehaviorConfig = { ...cfg.behavior, process_whitelist: next };
    updateBehaviorConfig(updated).catch((err) => console.error('Failed to update whitelist:', err));
  }

  function handleInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      handleAdd();
    }
  }
</script>

<SettingsCard title={i18nStore.t('settings.whitelist.title')}>
  <div class="setting-row" class:disabled={whitelistDisabled}>
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.whitelist.enabled')}</span>
      <span class="setting-desc">
        {whitelistDisabled
          ? i18nStore.t('settings.whitelist.unsupported')
          : i18nStore.t('settings.whitelist.enabled.desc')}
      </span>
    </div>
    <Toggle
      checked={cfg.behavior.process_whitelist_enabled}
      disabled={whitelistDisabled}
      label={i18nStore.t('settings.whitelist.enabled')}
      onchange={(v) => handleEnabledChange(v)}
    />
  </div>

  <div class="setting-row separator whitelist-list-row" class:disabled={editingDisabled}>
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.whitelist.list')}</span>
      <span class="setting-desc">{i18nStore.t('settings.whitelist.list.desc')}</span>
    </div>
    <div class="whitelist-control">
      <div class="whitelist-input-row">
        <input
          class="whitelist-input"
          type="text"
          bind:value={whitelistInput}
          disabled={editingDisabled}
          placeholder={i18nStore.t('settings.whitelist.add.placeholder')}
          aria-label={i18nStore.t('settings.whitelist.add')}
          onkeydown={handleInputKeydown}
          oninput={() => (whitelistError = null)}
        />
        <button
          type="button"
          class="whitelist-add-btn"
          disabled={editingDisabled}
          onclick={handleAdd}
        >
          {i18nStore.t('settings.whitelist.add')}
        </button>
      </div>
      {#if whitelistError}
        <span class="whitelist-error">{whitelistError}</span>
      {/if}
      {#if cfg.behavior.process_whitelist.length === 0}
        <span class="whitelist-empty">{i18nStore.t('settings.whitelist.empty')}</span>
      {:else}
        <ul class="whitelist-chips" aria-label={i18nStore.t('settings.whitelist.list')}>
          {#each cfg.behavior.process_whitelist as name, index (name)}
            <li class="whitelist-chip">
              <span class="whitelist-chip-name">{name}</span>
              <button
                type="button"
                class="whitelist-chip-remove"
                disabled={whitelistDisabled}
                aria-label={`${i18nStore.t('settings.whitelist.remove')}: ${name}`}
                onclick={() => handleRemove(index)}
              >
                ×
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</SettingsCard>

<style>
  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
  }
  .setting-row.separator {
    border-top: 1px solid var(--separator, #f0f3f1);
    padding-top: 12px;
    margin-top: 8px;
  }
  .setting-row.disabled .setting-info {
    opacity: 0.55;
  }
  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .setting-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary, #1a1d23);
  }
  .setting-desc {
    font-size: 12px;
    color: var(--text-hint, #9ca8a3);
  }
  .whitelist-list-row {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .whitelist-control {
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
  }
  .whitelist-input-row {
    display: flex;
    gap: 8px;
  }
  .whitelist-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    outline: none;
    transition:
      border-color var(--transition),
      box-shadow var(--transition);
  }
  .whitelist-input:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
  .whitelist-add-btn {
    padding: 6px 14px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: var(--accent-foreground, #fff);
    font-size: 13px;
    font-weight: 500;
    font-family: inherit;
    cursor: pointer;
    transition: opacity var(--transition);
  }
  .whitelist-add-btn:hover:not(:disabled) {
    opacity: 0.9;
  }
  .whitelist-input:disabled,
  .whitelist-add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .whitelist-error {
    font-size: 12px;
    color: var(--state-alert-label);
  }
  .whitelist-empty {
    font-size: 12px;
    color: var(--text-hint);
    font-style: italic;
  }
  .whitelist-chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .whitelist-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 4px 4px 10px;
    border-radius: 16px;
    background: var(--accent-soft);
    color: var(--text-primary);
    font-size: 12px;
  }
  .whitelist-chip-name {
    line-height: 1;
  }
  .whitelist-chip-remove {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-hint);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    transition:
      background var(--transition),
      color var(--transition);
  }
  .whitelist-chip-remove:hover:not(:disabled) {
    background: var(--state-alert);
    color: var(--accent-foreground, #fff);
  }
  .whitelist-chip-remove:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
