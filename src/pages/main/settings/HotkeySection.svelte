<script lang="ts">
  import { onMount } from 'svelte';
  import type { HotkeyAction } from '$lib/bindings/HotkeyAction';
  import type { HotkeyBindingStatus } from '$lib/bindings/HotkeyBindingStatus';
  import type { HotkeyStatus } from '$lib/bindings/HotkeyStatus';
  import type { HotkeysConfig } from '$lib/bindings/HotkeysConfig';
  import { getHotkeyStatus, updateHotkeysConfig } from '$lib/commands';
  import { onHotkeyStatusChanged } from '$lib/events';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';
  import { errorMessage } from './utils';

  const cfg = $derived(configStore.current);

  let hotkeyStatus = $state<HotkeyStatus | null>(null);
  let hotkeySaveError = $state<string | null>(null);

  const permissionMissing = $derived(
    hotkeyStatus?.macos_accessibility === 'missing' ||
      hotkeyStatus?.bindings.some((b) => b.state === 'permission_missing') === true,
  );

  const lastError = $derived(hotkeySaveError ?? hotkeyStatus?.last_error ?? null);

  // MUST replace in order: CommandOrControl before Command/Control
  function prettify(value: string): string {
    return value
      .replaceAll('CommandOrControl', 'Cmd/Ctrl')
      .replaceAll('Command', 'Cmd')
      .replaceAll('Control', 'Ctrl');
  }

  // MUST replace in order: Cmd/Ctrl before Cmd and Ctrl
  function rawify(value: string): string {
    return value
      .replaceAll('Cmd/Ctrl', 'CommandOrControl')
      .replaceAll('Cmd', 'Command')
      .replaceAll('Ctrl', 'Control');
  }

  function statusFor(action: HotkeyAction): HotkeyBindingStatus | null {
    return hotkeyStatus?.bindings.find((b) => b.action === action) ?? null;
  }

  function statusLabel(status: HotkeyBindingStatus | null): string {
    if (!status) return i18nStore.t('settings.hotkeys.status.pending');
    if (status.state === 'registered') return i18nStore.t('settings.hotkeys.status.registered');
    if (status.state === 'permission_missing') {
      return i18nStore.t('settings.hotkeys.status.permissionMissing');
    }
    return i18nStore.t('settings.hotkeys.status.conflict');
  }

  function isStatusError(action: HotkeyAction): boolean {
    const status = statusFor(action);
    return status !== null && status.state !== 'registered';
  }

  function handleChange(field: keyof HotkeysConfig, value: string, input: HTMLInputElement) {
    const next = rawify(value.trim());
    if (next === cfg.hotkeys[field]) {
      input.value = prettify(cfg.hotkeys[field]);
      return;
    }
    if (!next) {
      hotkeySaveError = i18nStore.t('settings.hotkeys.error.empty');
      input.value = prettify(cfg.hotkeys[field]);
      return;
    }
    const updated: HotkeysConfig = { ...cfg.hotkeys, [field]: next };
    updateHotkeysConfig(updated)
      .then(() => {
        hotkeySaveError = null;
      })
      .catch((err) => {
        hotkeySaveError = errorMessage(err);
        input.value = prettify(cfg.hotkeys[field]);
        console.error('Failed to update hotkeys config:', err);
      });
  }

  onMount(() => {
    let disposed = false;
    const unlistenHotkeys = onHotkeyStatusChanged((status) => {
      hotkeyStatus = status;
      if (!status.last_error) hotkeySaveError = null;
    });

    getHotkeyStatus()
      .then((status) => {
        if (!disposed) hotkeyStatus = status;
      })
      .catch((err) => {
        if (!disposed) hotkeySaveError = errorMessage(err);
        console.error('Failed to load hotkey status:', err);
      });

    return () => {
      disposed = true;
      unlistenHotkeys.then((fn) => fn()).catch(() => {});
    };
  });
</script>

<SettingsCard title={i18nStore.t('settings.hotkeys.title')}>
  <div class="hotkey-help">
    <span>{i18nStore.t('settings.hotkeys.desc')}</span>
  </div>

  {#if permissionMissing}
    <div class="hotkey-alert permission">
      <strong>{i18nStore.t('settings.hotkeys.permission.title')}</strong>
      <span>{i18nStore.t('settings.hotkeys.permission.desc')}</span>
    </div>
  {/if}

  {#if lastError}
    <div class="hotkey-alert conflict">
      <strong>{i18nStore.t('settings.hotkeys.conflict.title')}</strong>
      <span>{lastError}</span>
    </div>
  {/if}

  <div class="setting-row hotkey-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.hotkeys.startRest')}</span>
      <span class="setting-desc">{i18nStore.t('settings.hotkeys.startRest.desc')}</span>
    </div>
    <div class="hotkey-control">
      <input
        class="hotkey-input"
        value={prettify(cfg.hotkeys.start_rest)}
        aria-label={i18nStore.t('settings.hotkeys.startRest')}
        onchange={(e) => handleChange('start_rest', e.currentTarget.value, e.currentTarget)}
      />
      <span class="hotkey-status" class:error={isStatusError('start_rest')}>
        {statusLabel(statusFor('start_rest'))}
      </span>
    </div>
  </div>

  <div class="setting-row separator hotkey-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.hotkeys.skipRest')}</span>
      <span class="setting-desc">{i18nStore.t('settings.hotkeys.skipRest.desc')}</span>
    </div>
    <div class="hotkey-control">
      <input
        class="hotkey-input"
        value={prettify(cfg.hotkeys.skip_rest)}
        aria-label={i18nStore.t('settings.hotkeys.skipRest')}
        onchange={(e) => handleChange('skip_rest', e.currentTarget.value, e.currentTarget)}
      />
      <span class="hotkey-status" class:error={isStatusError('skip_rest')}>
        {statusLabel(statusFor('skip_rest'))}
      </span>
    </div>
  </div>

  <div class="setting-row separator hotkey-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.hotkeys.togglePause')}</span>
      <span class="setting-desc">{i18nStore.t('settings.hotkeys.togglePause.desc')}</span>
    </div>
    <div class="hotkey-control">
      <input
        class="hotkey-input"
        value={prettify(cfg.hotkeys.toggle_pause)}
        aria-label={i18nStore.t('settings.hotkeys.togglePause')}
        onchange={(e) => handleChange('toggle_pause', e.currentTarget.value, e.currentTarget)}
      />
      <span class="hotkey-status" class:error={isStatusError('toggle_pause')}>
        {statusLabel(statusFor('toggle_pause'))}
      </span>
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

  .hotkey-help {
    margin: -2px 0 14px;
    font-size: 12px;
    color: var(--text-hint);
    line-height: 1.5;
  }

  .hotkey-alert {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent-border);
    background: var(--accent-soft);
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }

  .hotkey-alert strong {
    color: var(--text-primary);
    font-size: 13px;
  }

  .hotkey-alert.conflict {
    border-color: var(--state-alert);
    background: color-mix(in srgb, var(--state-alert) 10%, transparent);
  }

  .hotkey-row {
    align-items: flex-start;
    gap: 16px;
  }

  .hotkey-control {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }

  .hotkey-input {
    width: 150px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-card);
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    text-align: center;
    outline: none;
    transition:
      border-color var(--transition),
      box-shadow var(--transition),
      background var(--transition);
  }

  .hotkey-input:hover {
    background: var(--accent-soft);
  }

  .hotkey-input:focus-visible {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }

  .hotkey-status {
    font-size: 11px;
    color: var(--state-active-label);
  }

  .hotkey-status.error {
    color: var(--state-alert-label);
  }
</style>
