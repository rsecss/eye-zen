<script lang="ts">
  import { configStore } from '$lib/stores/config.svelte';
  import { updateTimerConfig, updateBehaviorConfig, updateDisplayConfig } from '$lib/commands';
  import type { TimerConfig } from '$lib/bindings/TimerConfig';
  import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
  import type { DisplayConfig } from '$lib/bindings/DisplayConfig';
  import Stepper from './components/Stepper.svelte';
  import Toggle from './components/Toggle.svelte';
  import Select from './components/Select.svelte';
  import SettingsCard from './components/SettingsCard.svelte';

  const cfg = $derived(configStore.current);

  function handleTimerChange(field: keyof TimerConfig, value: number) {
    const updated: TimerConfig = { ...cfg.timer, [field]: value };
    updateTimerConfig(updated).catch((err) => console.error('Failed to update timer config:', err));
  }

  function handleBehaviorChange(field: keyof BehaviorConfig, value: boolean) {
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

  const languageOptions = [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en-US', label: 'English' },
  ];

  const themeOptions = [
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];
</script>

<div class="settings-page">
  <SettingsCard title="Timer">
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">Work Duration</span>
        <span class="setting-desc">工作时长，到时提醒休息</span>
      </div>
      <Stepper
        value={cfg.timer.work_minutes}
        min={1}
        max={120}
        step={1}
        unit="min"
        label="Work Duration"
        onchange={(v) => handleTimerChange('work_minutes', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Rest Duration</span>
        <span class="setting-desc">每次休息时长</span>
      </div>
      <Stepper
        value={cfg.timer.rest_seconds}
        min={5}
        max={300}
        step={5}
        unit="sec"
        label="Rest Duration"
        onchange={(v) => handleTimerChange('rest_seconds', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Pre-alert</span>
        <span class="setting-desc">休息前预提醒时间</span>
      </div>
      <Stepper
        value={cfg.timer.pre_alert_seconds}
        min={5}
        max={60}
        step={5}
        unit="sec"
        label="Pre-alert"
        onchange={(v) => handleTimerChange('pre_alert_seconds', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Alert Timeout</span>
        <span class="setting-desc">提醒等待上限，超时自动休息</span>
      </div>
      <Stepper
        value={cfg.timer.alert_timeout_seconds}
        min={10}
        max={300}
        step={10}
        unit="sec"
        label="Alert Timeout"
        onchange={(v) => handleTimerChange('alert_timeout_seconds', v)}
      />
    </div>
  </SettingsCard>

  <SettingsCard title="Behavior">
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">Sound</span>
        <span class="setting-desc">休息提醒时播放音效</span>
      </div>
      <Toggle
        checked={cfg.behavior.sound_enabled}
        label="Sound"
        onchange={(v) => handleBehaviorChange('sound_enabled', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Fullscreen Skip</span>
        <span class="setting-desc">全屏应用时跳过提醒</span>
      </div>
      <Toggle
        checked={cfg.behavior.fullscreen_skip}
        label="Fullscreen Skip"
        onchange={(v) => handleBehaviorChange('fullscreen_skip', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Auto Start</span>
        <span class="setting-desc">系统启动时自动运行</span>
      </div>
      <Toggle
        checked={cfg.behavior.auto_start}
        label="Auto Start"
        onchange={(v) => handleBehaviorChange('auto_start', v)}
      />
    </div>
  </SettingsCard>

  <SettingsCard title="Display">
    <div class="setting-row">
      <div class="setting-info">
        <span class="setting-label">Language</span>
        <span class="setting-desc">界面显示语言</span>
      </div>
      <Select
        value={cfg.display.language}
        options={languageOptions}
        label="Language"
        onchange={(v) => handleDisplayChange('language', v)}
      />
    </div>

    <div class="setting-row separator">
      <div class="setting-info">
        <span class="setting-label">Theme</span>
        <span class="setting-desc">外观主题</span>
      </div>
      <Select
        value={cfg.display.theme}
        options={themeOptions}
        label="Theme"
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
