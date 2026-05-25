<script lang="ts">
  import type { PomodoroConfig } from '$lib/bindings/PomodoroConfig';
  import type { TimerConfig } from '$lib/bindings/TimerConfig';
  import type { TimerMode } from '$lib/bindings/TimerMode';
  import { updatePomodoroConfig, updateTimerConfig } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import Select from '../components/Select.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';
  import Stepper from '../components/Stepper.svelte';

  const cfg = $derived(configStore.current);
  const isPomodoroMode = $derived(cfg.timer.mode === 'pomodoro');

  const timerModeOptions = $derived([
    { value: 'twenty_twenty_twenty', label: i18nStore.t('settings.timer.mode.20-20-20') },
    { value: 'pomodoro', label: i18nStore.t('settings.timer.mode.pomodoro') },
  ]);

  function handleTimerChange(field: keyof TimerConfig, value: number) {
    const updated: TimerConfig = { ...cfg.timer, [field]: value };
    updateTimerConfig(updated).catch((err) => console.error('Failed to update timer config:', err));
  }

  function handleModeChange(value: string) {
    if (value !== 'twenty_twenty_twenty' && value !== 'pomodoro') return;
    const updated: TimerConfig = { ...cfg.timer, mode: value as TimerMode };
    updateTimerConfig(updated).catch((err) => console.error('Failed to update timer mode:', err));
  }

  function handlePomodoroChange(field: keyof PomodoroConfig, value: number) {
    const updated: PomodoroConfig = { ...cfg.pomodoro, [field]: value };
    updatePomodoroConfig(updated).catch((err) =>
      console.error('Failed to update pomodoro config:', err),
    );
  }
</script>

<SettingsCard title={i18nStore.t('settings.timer.title')}>
  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.timer.mode')}</span>
      <span class="setting-desc">{i18nStore.t('settings.timer.mode.desc')}</span>
    </div>
    <Select
      value={cfg.timer.mode}
      options={timerModeOptions}
      label={i18nStore.t('settings.timer.mode')}
      onchange={handleModeChange}
    />
  </div>

  <div class="setting-row separator">
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

{#if isPomodoroMode}
  <SettingsCard title={i18nStore.t('settings.pomodoro.title')}>
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.pomodoro.focusMinutes')}</span>
        <span class="setting-desc">{i18nStore.t('settings.pomodoro.focusMinutes.desc')}</span>
      </div>
      <Stepper
        value={cfg.pomodoro.focus_minutes}
        min={1}
        max={180}
        step={1}
        unit={i18nStore.t('settings.pomodoro.focusMinutes.unit')}
        label={i18nStore.t('settings.pomodoro.focusMinutes')}
        onchange={(v) => handlePomodoroChange('focus_minutes', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.pomodoro.shortBreakMinutes')}</span>
        <span class="setting-desc">{i18nStore.t('settings.pomodoro.shortBreakMinutes.desc')}</span>
      </div>
      <Stepper
        value={cfg.pomodoro.short_break_minutes}
        min={1}
        max={60}
        step={1}
        unit={i18nStore.t('settings.pomodoro.shortBreakMinutes.unit')}
        label={i18nStore.t('settings.pomodoro.shortBreakMinutes')}
        onchange={(v) => handlePomodoroChange('short_break_minutes', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.pomodoro.longBreakMinutes')}</span>
        <span class="setting-desc">{i18nStore.t('settings.pomodoro.longBreakMinutes.desc')}</span>
      </div>
      <Stepper
        value={cfg.pomodoro.long_break_minutes}
        min={1}
        max={180}
        step={1}
        unit={i18nStore.t('settings.pomodoro.longBreakMinutes.unit')}
        label={i18nStore.t('settings.pomodoro.longBreakMinutes')}
        onchange={(v) => handlePomodoroChange('long_break_minutes', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">{i18nStore.t('settings.pomodoro.cyclesPerLong')}</span>
        <span class="setting-desc">{i18nStore.t('settings.pomodoro.cyclesPerLong.desc')}</span>
      </div>
      <Stepper
        value={cfg.pomodoro.cycles_per_long}
        min={1}
        max={12}
        step={1}
        unit={i18nStore.t('settings.pomodoro.cyclesPerLong.unit')}
        label={i18nStore.t('settings.pomodoro.cyclesPerLong')}
        onchange={(v) => handlePomodoroChange('cycles_per_long', v)}
      />
    </div>
  </SettingsCard>
{/if}

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
</style>
