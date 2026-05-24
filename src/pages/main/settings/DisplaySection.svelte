<script lang="ts">
  import type { DisplayConfig } from '$lib/bindings/DisplayConfig';
  import { updateDisplayConfig } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import Select from '../components/Select.svelte';
  import SettingsCard from '../components/SettingsCard.svelte';

  const cfg = $derived(configStore.current);

  const languageOptions = [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en', label: 'English' },
  ];

  const displayLanguage = $derived(cfg.display.language === 'en' ? 'en' : 'zh-CN');

  const themeOptions = $derived([
    { value: 'light', label: i18nStore.t('settings.display.theme.light') },
    { value: 'dark', label: i18nStore.t('settings.display.theme.dark') },
  ]);

  function handleChange(field: keyof DisplayConfig, value: string) {
    const updated: DisplayConfig = { ...cfg.display, [field]: value };
    updateDisplayConfig(updated).catch((err) =>
      console.error('Failed to update display config:', err),
    );
  }
</script>

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
      onchange={(v) => handleChange('language', v)}
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
      onchange={(v) => handleChange('theme', v)}
    />
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
</style>
