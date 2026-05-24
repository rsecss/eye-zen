<script lang="ts">
  import type { ScheduleConfig } from '$lib/bindings/ScheduleConfig';
  import { updateScheduleConfig } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';
  import Toggle from '../components/Toggle.svelte';

  const cfg = $derived(configStore.current);

  const weekdayKeys = [
    'settings.schedule.weekday.mon',
    'settings.schedule.weekday.tue',
    'settings.schedule.weekday.wed',
    'settings.schedule.weekday.thu',
    'settings.schedule.weekday.fri',
    'settings.schedule.weekday.sat',
    'settings.schedule.weekday.sun',
  ] as const;

  function handleEnabledChange(value: boolean) {
    const updated: ScheduleConfig = { ...cfg.schedule, enabled: value };
    updateScheduleConfig(updated).catch((err) =>
      console.error('Failed to update schedule config:', err),
    );
  }

  function handleDayToggle(index: number, value: boolean) {
    const nextDays: [boolean, boolean, boolean, boolean, boolean, boolean, boolean] = [
      ...cfg.schedule.active_days,
    ];
    nextDays[index] = value;
    const updated: ScheduleConfig = { ...cfg.schedule, active_days: nextDays };
    updateScheduleConfig(updated).catch((err) =>
      console.error('Failed to update schedule config:', err),
    );
  }
</script>

<SettingsCard title={i18nStore.t('settings.schedule.title')}>
  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.schedule.enabled')}</span>
      <span class="setting-desc">{i18nStore.t('settings.schedule.enabled.desc')}</span>
    </div>
    <Toggle
      checked={cfg.schedule.enabled}
      label={i18nStore.t('settings.schedule.enabled')}
      onchange={(v) => handleEnabledChange(v)}
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
            onchange={(e) => handleDayToggle(index, e.currentTarget.checked)}
          />
          <span>{label}</span>
        </label>
      {/each}
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
