<script lang="ts">
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { errorMessage } from './utils';

  const cfg = $derived(configStore.current);
  let autoStartError = $state<string | null>(null);
  let autoStartSynced = false;
  const configError = $derived(configStore.errorFor('autoStart'));
  const visibleError = $derived(autoStartError ?? (configError ? errorMessage(configError) : null));

  async function handleToggle(value: boolean) {
    autoStartError = null;
    try {
      if (value) await enable();
      else await disable();
    } catch (err) {
      console.error('Failed to toggle autostart:', err);
      return;
    }
    try {
      await configStore.updateAutoStart(value);
    } catch (err) {
      // A conflict refreshes the store before rejecting, so reconcile the OS
      // state to that authoritative snapshot rather than inverting the request.
      console.error('Failed to save autostart config, reconciling OS state:', err);
      try {
        if (configStore.current.behavior.auto_start) await enable();
        else await disable();
      } catch (revertErr) {
        console.error('Failed to reconcile autostart after save failure:', revertErr);
        autoStartError = i18nStore
          .t('settings.behavior.autoStart.error.save')
          .replace('{reason}', errorMessage(err));
      }
    }
  }

  $effect(() => {
    if (configStore.loaded && !autoStartSynced) {
      isEnabled()
        .then((enabled) => {
          autoStartSynced = true;
          if (enabled !== cfg.behavior.auto_start) {
            configStore
              .updateAutoStart(enabled)
              .catch((err) => console.error('Failed to sync autostart config:', err));
          }
        })
        .catch((err) => console.error('Failed to check autostart status:', err));
    }
  });
</script>

<div class="auto-start-block">
  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.behavior.autoStart')}</span>
      <span class="setting-desc">{i18nStore.t('settings.behavior.autoStart.desc')}</span>
    </div>
    <Toggle
      checked={cfg.behavior.auto_start}
      label={i18nStore.t('settings.behavior.autoStart')}
      onchange={(v) => handleToggle(v)}
    />
  </div>

  {#if visibleError}
    <div class="setting-error" role="alert">{visibleError}</div>
  {/if}
</div>

<!--
  This section renders inside the Behavior card via {#snippet}/slot, so the
  outer wrapper uses `display: contents` to flatten the DOM. The separator
  rule lives on .setting-row directly because there is no preceding sibling
  inside a fresh slot.
-->

<style>
  .auto-start-block {
    display: contents;
  }

  .setting-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 0;
    border-top: 1px solid var(--separator, #f0f3f1);
    padding-top: 12px;
    margin-top: 8px;
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

  .setting-error {
    margin-top: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--state-alert);
    background: color-mix(in srgb, var(--state-alert) 10%, transparent);
    color: var(--state-alert-label);
    font-size: 12px;
    line-height: 1.45;
  }
</style>
