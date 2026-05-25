<script lang="ts">
  import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
  import { updateBehaviorConfig } from '$lib/commands';
  import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { errorMessage } from './utils';

  const cfg = $derived(configStore.current);

  let autoStartError = $state<string | null>(null);
  let autoStartSynced = false;

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
      const updated: BehaviorConfig = { ...cfg.behavior, auto_start: value };
      await updateBehaviorConfig(updated);
    } catch (err) {
      // OS-level autostart toggle succeeded but config save failed; attempt
      // to revert the OS state so it matches the saved config. If that
      // revert also fails, surface the drift to the user via the banner so
      // they know to retry instead of silently swallowing it.
      console.error('Failed to save autostart config, attempting OS-level revert:', err);
      try {
        if (value) await disable();
        else await enable();
      } catch (revertErr) {
        console.error('Failed to revert autostart after save failure:', revertErr);
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
            const updated: BehaviorConfig = { ...cfg.behavior, auto_start: enabled };
            updateBehaviorConfig(updated).catch((err) =>
              console.error('Failed to sync autostart config:', err),
            );
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

  {#if autoStartError}
    <div class="setting-error" role="alert">{autoStartError}</div>
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
