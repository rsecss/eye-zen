<script lang="ts">
  import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
  import { updateBehaviorConfig } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';
  import Stepper from '../components/Stepper.svelte';
  import Toggle from '../components/Toggle.svelte';
  import AutoStartSection from './AutoStartSection.svelte';

  type BooleanField = 'sound_enabled' | 'fullscreen_skip' | 'afk_skip_enabled';

  let {
    capabilitiesLoaded,
    afkSupported,
    fullscreenSupported,
  }: {
    capabilitiesLoaded: boolean;
    afkSupported: boolean;
    fullscreenSupported: boolean;
  } = $props();

  const cfg = $derived(configStore.current);
  const afkDisabled = $derived(!capabilitiesLoaded || !afkSupported);
  const afkThresholdDisabled = $derived(afkDisabled || !cfg.behavior.afk_skip_enabled);
  const fullscreenSkipDisabled = $derived(!capabilitiesLoaded || !fullscreenSupported);

  function handleBooleanChange(field: BooleanField, value: boolean) {
    const updated: BehaviorConfig = { ...cfg.behavior, [field]: value };
    updateBehaviorConfig(updated).catch((err) =>
      console.error('Failed to update behavior config:', err),
    );
  }

  function handleAfkThreshold(value: number) {
    if (afkDisabled) return;
    const updated: BehaviorConfig = { ...cfg.behavior, afk_threshold_minutes: value };
    updateBehaviorConfig(updated).catch((err) =>
      console.error('Failed to update behavior config:', err),
    );
  }
</script>

<SettingsCard title={i18nStore.t('settings.behavior.title')}>
  <div class="setting-row">
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.behavior.sound')}</span>
      <span class="setting-desc">{i18nStore.t('settings.behavior.sound.desc')}</span>
    </div>
    <Toggle
      checked={cfg.behavior.sound_enabled}
      label={i18nStore.t('settings.behavior.sound')}
      onchange={(v) => handleBooleanChange('sound_enabled', v)}
    />
  </div>

  <div class="setting-row separator" class:disabled={fullscreenSkipDisabled}>
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.behavior.fullscreenSkip')}</span>
      <span class="setting-desc">
        {fullscreenSkipDisabled
          ? i18nStore.t('settings.behavior.fullscreenUnsupported')
          : i18nStore.t('settings.behavior.fullscreenSkip.desc')}
      </span>
    </div>
    <Toggle
      checked={cfg.behavior.fullscreen_skip}
      disabled={fullscreenSkipDisabled}
      label={i18nStore.t('settings.behavior.fullscreenSkip')}
      onchange={(v) => handleBooleanChange('fullscreen_skip', v)}
    />
  </div>

  <div class="setting-row separator" class:disabled={afkDisabled}>
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.behavior.afkSkip')}</span>
      <span class="setting-desc">
        {afkDisabled
          ? i18nStore.t('settings.behavior.afkUnsupported')
          : i18nStore.t('settings.behavior.afkSkip.desc')}
      </span>
    </div>
    <Toggle
      checked={cfg.behavior.afk_skip_enabled}
      disabled={afkDisabled}
      label={i18nStore.t('settings.behavior.afkSkip')}
      onchange={(v) => handleBooleanChange('afk_skip_enabled', v)}
    />
  </div>

  <div class="setting-row separator" class:disabled={afkThresholdDisabled}>
    <div class="setting-info">
      <span class="setting-label">{i18nStore.t('settings.behavior.afkThreshold')}</span>
      <span class="setting-desc">
        {afkDisabled
          ? i18nStore.t('settings.behavior.afkUnsupported')
          : i18nStore.t('settings.behavior.afkThreshold.desc')}
      </span>
    </div>
    <Stepper
      value={cfg.behavior.afk_threshold_minutes}
      min={1}
      max={120}
      step={1}
      unit={i18nStore.t('settings.behavior.afkThreshold.unit')}
      label={i18nStore.t('settings.behavior.afkThreshold')}
      disabled={afkThresholdDisabled}
      onchange={(v) => handleAfkThreshold(v)}
    />
  </div>

  <AutoStartSection />
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
</style>
