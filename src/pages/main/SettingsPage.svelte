<script lang="ts">
  import { onMount } from 'svelte';
  import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
  import type { DisplayConfig } from '$lib/bindings/DisplayConfig';
  import type { HotkeyAction } from '$lib/bindings/HotkeyAction';
  import type { HotkeyBindingStatus } from '$lib/bindings/HotkeyBindingStatus';
  import type { HotkeyStatus } from '$lib/bindings/HotkeyStatus';
  import type { HotkeysConfig } from '$lib/bindings/HotkeysConfig';
  import type { ScheduleConfig } from '$lib/bindings/ScheduleConfig';
  import type { TimerConfig } from '$lib/bindings/TimerConfig';
  import {
    getHotkeyStatus,
    updateBehaviorConfig,
    updateDisplayConfig,
    updateHotkeysConfig,
    updateScheduleConfig,
    updateTimerConfig,
  } from '$lib/commands';
  import { onHotkeyStatusChanged } from '$lib/events';
  import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import Select from './components/Select.svelte';
  import SettingsCard from './components/SettingsCard.svelte';
  import Stepper from './components/Stepper.svelte';
  import Toggle from './components/Toggle.svelte';

  const cfg = $derived(configStore.current);
  let hotkeyStatus = $state<HotkeyStatus | null>(null);
  let hotkeySaveError = $state<string | null>(null);

  function handleTimerChange(field: keyof TimerConfig, value: number) {
    const updated: TimerConfig = { ...cfg.timer, [field]: value };
    updateTimerConfig(updated).catch((err) => console.error('Failed to update timer config:', err));
  }

  async function handleBehaviorChange(field: keyof BehaviorConfig, value: boolean) {
    if (field === 'auto_start') {
      try {
        if (value) {
          await enable();
        } else {
          await disable();
        }
      } catch (err) {
        console.error('Failed to toggle autostart:', err);
        return;
      }
      try {
        const updated: BehaviorConfig = { ...cfg.behavior, auto_start: value };
        await updateBehaviorConfig(updated);
      } catch (err) {
        console.error('Failed to save autostart config, rolling back:', err);
        try {
          if (value) {
            await disable();
          } else {
            await enable();
          }
        } catch (_) {
          /* best-effort rollback */
        }
      }
      return;
    }
    const updated: BehaviorConfig = { ...cfg.behavior, [field]: value };
    updateBehaviorConfig(updated).catch((err) =>
      console.error('Failed to update behavior config:', err),
    );
  }

  function handleDisplayChange(field: keyof DisplayConfig, value: string) {
    const updated: DisplayConfig = { ...cfg.display, [field]: value };
    updateDisplayConfig(updated).catch((err) =>
      console.error('Failed to update display config:', err),
    );
  }

  function handleScheduleEnabledChange(value: boolean) {
    const updated: ScheduleConfig = { ...cfg.schedule, enabled: value };
    updateScheduleConfig(updated).catch((err) =>
      console.error('Failed to update schedule config:', err),
    );
  }

  function handleScheduleDayToggle(index: number, value: boolean) {
    const nextDays: [boolean, boolean, boolean, boolean, boolean, boolean, boolean] = [
      ...cfg.schedule.active_days,
    ];
    nextDays[index] = value;
    const updated: ScheduleConfig = { ...cfg.schedule, active_days: nextDays };
    updateScheduleConfig(updated).catch((err) =>
      console.error('Failed to update schedule config:', err),
    );
  }

  function handleHotkeyChange(field: keyof HotkeysConfig, value: string, input: HTMLInputElement) {
    const next = value.trim();
    if (next === cfg.hotkeys[field]) {
      input.value = cfg.hotkeys[field];
      return;
    }
    if (!next) {
      hotkeySaveError = i18nStore.t('settings.hotkeys.error.empty');
      input.value = cfg.hotkeys[field];
      return;
    }

    const updated: HotkeysConfig = { ...cfg.hotkeys, [field]: next };
    updateHotkeysConfig(updated)
      .then(() => {
        hotkeySaveError = null;
      })
      .catch((err) => {
        hotkeySaveError = errorMessage(err);
        input.value = cfg.hotkeys[field];
        console.error('Failed to update hotkeys config:', err);
      });
  }

  const weekdayKeys = [
    'settings.schedule.weekday.mon',
    'settings.schedule.weekday.tue',
    'settings.schedule.weekday.wed',
    'settings.schedule.weekday.thu',
    'settings.schedule.weekday.fri',
    'settings.schedule.weekday.sat',
    'settings.schedule.weekday.sun',
  ] as const;

  const languageOptions = [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en', label: 'English' },
  ];

  const displayLanguage = $derived(
    cfg.display.language === 'en' || cfg.display.language === 'en-US' ? 'en' : 'zh-CN',
  );

  const themeOptions = $derived([
    { value: 'light', label: i18nStore.t('settings.display.theme.light') },
    { value: 'dark', label: i18nStore.t('settings.display.theme.dark') },
  ]);

  const hotkeyPermissionMissing = $derived(
    hotkeyStatus?.macos_accessibility === 'missing' ||
      hotkeyStatus?.bindings.some((binding) => binding.state === 'permission_missing') === true,
  );

  const hotkeyLastError = $derived(hotkeySaveError ?? hotkeyStatus?.last_error ?? null);

  function hotkeyStatusFor(action: HotkeyAction): HotkeyBindingStatus | null {
    return hotkeyStatus?.bindings.find((binding) => binding.action === action) ?? null;
  }

  function hotkeyStatusLabel(status: HotkeyBindingStatus | null): string {
    if (!status) return i18nStore.t('settings.hotkeys.status.pending');
    if (status.state === 'registered') return i18nStore.t('settings.hotkeys.status.registered');
    if (status.state === 'permission_missing') {
      return i18nStore.t('settings.hotkeys.status.permissionMissing');
    }
    return i18nStore.t('settings.hotkeys.status.conflict');
  }

  function isHotkeyStatusError(action: HotkeyAction): boolean {
    const status = hotkeyStatusFor(action);
    return status !== null && status.state !== 'registered';
  }

  function errorMessage(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (isAppErrorLike(err)) {
      const detail = err.detail;
      if (typeof detail?.reason === 'string') return detail.reason;
      if (typeof detail?.message === 'string') return detail.message;
    }
    return i18nStore.t('settings.hotkeys.error.generic');
  }

  function isAppErrorLike(err: unknown): err is {
    detail?: { reason?: unknown; message?: unknown };
  } {
    return typeof err === 'object' && err !== null && 'detail' in err;
  }

  let autoStartSynced = false;

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

<div class="settings-page">
  <SettingsCard title={i18nStore.t('settings.timer.title')}>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.timer.workMinutes')}</span>
        <span class="setting-desc">{i18nStore.t('settings.timer.workMinutes.desc')}</span>
      </div>
      <Stepper
        value={cfg.timer.work_minutes}
        min={1}
        max={120}
        step={1}
        unit={i18nStore.t('settings.timer.workMinutes.unit')}
        label={i18nStore.t('settings.timer.workMinutes')}
        onchange={(v) => handleTimerChange('work_minutes', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.timer.restSeconds')}</span>
        <span class="setting-desc">{i18nStore.t('settings.timer.restSeconds.desc')}</span>
      </div>
      <Stepper
        value={cfg.timer.rest_seconds}
        min={5}
        max={300}
        step={5}
        unit={i18nStore.t('settings.timer.restSeconds.unit')}
        label={i18nStore.t('settings.timer.restSeconds')}
        onchange={(v) => handleTimerChange('rest_seconds', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.timer.preAlertSeconds')}</span>
        <span class="setting-desc">{i18nStore.t('settings.timer.preAlertSeconds.desc')}</span>
      </div>
      <Stepper
        value={cfg.timer.pre_alert_seconds}
        min={5}
        max={60}
        step={5}
        unit={i18nStore.t('settings.timer.preAlertSeconds.unit')}
        label={i18nStore.t('settings.timer.preAlertSeconds')}
        onchange={(v) => handleTimerChange('pre_alert_seconds', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.timer.alertTimeout')}</span>
        <span class="setting-desc">{i18nStore.t('settings.timer.alertTimeout.desc')}</span>
      </div>
      <Stepper
        value={cfg.timer.alert_timeout_seconds}
        min={10}
        max={300}
        step={10}
        unit={i18nStore.t('settings.timer.alertTimeout.unit')}
        label={i18nStore.t('settings.timer.alertTimeout')}
        onchange={(v) => handleTimerChange('alert_timeout_seconds', v)}
      />
    </div>
  </SettingsCard>

  <SettingsCard title={i18nStore.t('settings.hotkeys.title')}>
    <div class="hotkey-help">
      <span>{i18nStore.t('settings.hotkeys.desc')}</span>
    </div>

    {#if hotkeyPermissionMissing}
      <div class="hotkey-alert permission">
        <strong>{i18nStore.t('settings.hotkeys.permission.title')}</strong>
        <span>{i18nStore.t('settings.hotkeys.permission.desc')}</span>
      </div>
    {/if}

    {#if hotkeyLastError}
      <div class="hotkey-alert conflict">
        <strong>{i18nStore.t('settings.hotkeys.conflict.title')}</strong>
        <span>{hotkeyLastError}</span>
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
          value={cfg.hotkeys.start_rest}
          aria-label={i18nStore.t('settings.hotkeys.startRest')}
          onchange={(e) => handleHotkeyChange('start_rest', e.currentTarget.value, e.currentTarget)}
        />
        <span class="hotkey-status" class:error={isHotkeyStatusError('start_rest')}>
          {hotkeyStatusLabel(hotkeyStatusFor('start_rest'))}
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
          value={cfg.hotkeys.skip_rest}
          aria-label={i18nStore.t('settings.hotkeys.skipRest')}
          onchange={(e) => handleHotkeyChange('skip_rest', e.currentTarget.value, e.currentTarget)}
        />
        <span class="hotkey-status" class:error={isHotkeyStatusError('skip_rest')}>
          {hotkeyStatusLabel(hotkeyStatusFor('skip_rest'))}
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
          value={cfg.hotkeys.toggle_pause}
          aria-label={i18nStore.t('settings.hotkeys.togglePause')}
          onchange={(e) =>
            handleHotkeyChange('toggle_pause', e.currentTarget.value, e.currentTarget)}
        />
        <span class="hotkey-status" class:error={isHotkeyStatusError('toggle_pause')}>
          {hotkeyStatusLabel(hotkeyStatusFor('toggle_pause'))}
        </span>
      </div>
    </div>
  </SettingsCard>

  <SettingsCard title={i18nStore.t('settings.behavior.title')}>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.behavior.sound')}</span>
        <span class="setting-desc">{i18nStore.t('settings.behavior.sound.desc')}</span>
      </div>
      <Toggle
        checked={cfg.behavior.sound_enabled}
        label={i18nStore.t('settings.behavior.sound')}
        onchange={(v) => handleBehaviorChange('sound_enabled', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.behavior.fullscreenSkip')}</span>
        <span class="setting-desc">{i18nStore.t('settings.behavior.fullscreenSkip.desc')}</span>
      </div>
      <Toggle
        checked={cfg.behavior.fullscreen_skip}
        label={i18nStore.t('settings.behavior.fullscreenSkip')}
        onchange={(v) => handleBehaviorChange('fullscreen_skip', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.behavior.autoStart')}</span>
        <span class="setting-desc">{i18nStore.t('settings.behavior.autoStart.desc')}</span>
      </div>
      <Toggle
        checked={cfg.behavior.auto_start}
        label={i18nStore.t('settings.behavior.autoStart')}
        onchange={(v) => handleBehaviorChange('auto_start', v)}
      />
    </div>
  </SettingsCard>

  <SettingsCard title={i18nStore.t('settings.schedule.title')}>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.schedule.enabled')}</span>
        <span class="setting-desc">{i18nStore.t('settings.schedule.enabled.desc')}</span>
      </div>
      <Toggle
        checked={cfg.schedule.enabled}
        label={i18nStore.t('settings.schedule.enabled')}
        onchange={(v) => handleScheduleEnabledChange(v)}
      />
    </div>

    <div class="setting-row separator schedule-days-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.schedule.days')}</span>
        <span class="setting-desc">{i18nStore.t('settings.schedule.days.desc')}</span>
      </div>
      <div class="weekday-grid" role="group" aria-label={i18nStore.t('settings.schedule.days')}>
        {#each weekdayKeys as key, index}
          {@const label = i18nStore.t(key)}
          {@const checked = cfg.schedule.active_days[index]}
          <label class="weekday-chip" class:active={checked} class:disabled={!cfg.schedule.enabled}>
            <input
              type="checkbox"
              class="weekday-input"
              {checked}
              disabled={!cfg.schedule.enabled}
              aria-label={label}
              onchange={(e) => handleScheduleDayToggle(index, e.currentTarget.checked)}
            />
            <span>{label}</span>
          </label>
        {/each}
      </div>
    </div>
  </SettingsCard>

  <SettingsCard title={i18nStore.t('settings.display.title')}>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.display.language')}</span>
        <span class="setting-desc">{i18nStore.t('settings.display.language.desc')}</span>
      </div>
      <Select
        value={displayLanguage}
        options={languageOptions}
        label={i18nStore.t('settings.display.language')}
        onchange={(v) => handleDisplayChange('language', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.display.theme')}</span>
        <span class="setting-desc">{i18nStore.t('settings.display.theme.desc')}</span>
      </div>
      <Select
        value={cfg.display.theme}
        options={themeOptions}
        label={i18nStore.t('settings.display.theme')}
        onchange={(v) => handleDisplayChange('theme', v)}
      />
    </div>
  </SettingsCard>
</div>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

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
    width: 172px;
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

  .schedule-days-row {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }

  .weekday-grid {
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    gap: 6px;
    width: 100%;
  }

  .weekday-chip {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 32px;
    padding: 0 8px;
    border-radius: 8px;
    border: 1px solid var(--separator, #f0f3f1);
    background: var(--bg-card, #fff);
    color: var(--text-secondary, #6b7280);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    user-select: none;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }

  .weekday-chip:hover:not(.disabled) {
    border-color: var(--accent, #4f8a6e);
  }

  .weekday-chip.active {
    background: var(--accent, #4f8a6e);
    border-color: var(--accent, #4f8a6e);
    color: var(--accent-foreground, #fff);
  }

  .weekday-chip.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .weekday-input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
    width: 0;
    height: 0;
  }
</style>
