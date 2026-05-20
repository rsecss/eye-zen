<script lang="ts">
  import { open } from '@tauri-apps/plugin-shell';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import logoPng from '../../assets/logo.png';

  const GITHUB_URL = 'https://github.com/rsecss/eye-zen';
  const RELEASES_URL = `${GITHUB_URL}/releases`;

  const platform = navigator.platform;

  function openUrl(url: string) {
    open(url).catch((err) => console.error('Failed to open URL:', err));
  }
</script>

<div class="about-page">
  <div class="logo">
    <img src={logoPng} alt="Eyezen Logo" width="80" height="80" />
  </div>

  <h1 class="app-name">Eyezen</h1>
  <span class="version">{i18nStore.t('about.version')} {__APP_VERSION__}</span>
  <p class="description">{i18nStore.t('about.description')}</p>
  <p class="sub-description">{i18nStore.t('about.rule')}</p>

  <button class="update-btn" onclick={() => openUrl(RELEASES_URL)}>
    {i18nStore.t('about.checkUpdate')}
  </button>

  <div class="info-card">
    <div class="info-row">
      <span class="info-label">{i18nStore.t('about.platform')}</span>
      <span class="info-value">{platform}</span>
    </div>
    <div class="info-row separator">
      <span class="info-label">{i18nStore.t('about.license')}</span>
      <span class="info-value">GPL-3.0-or-later</span>
    </div>
    <div class="info-row separator">
      <span class="info-label">{i18nStore.t('about.source')}</span>
      <button class="link-btn" onclick={() => openUrl(GITHUB_URL)}>GitHub</button>
    </div>
  </div>
</div>

<style>
  .about-page {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px 16px 16px;
    gap: 0;
  }

  .logo {
    width: 80px;
    height: 80px;
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    box-shadow:
      0 4px 16px rgba(34, 197, 94, 0.2),
      0 8px 32px rgba(34, 197, 94, 0.08);
    position: relative;
  }

  .logo::after {
    content: '';
    position: absolute;
    inset: -4px;
    border-radius: 24px;
    background: radial-gradient(circle, rgba(34, 197, 94, 0.1) 0%, transparent 70%);
    z-index: -1;
  }

  .app-name {
    margin: 16px 0 0;
    font-size: 24px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .version {
    margin-top: 4px;
    font-size: 13px;
    color: var(--text-hint);
  }

  .description {
    margin: 16px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
    text-align: center;
  }

  .sub-description {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--text-hint);
    font-style: italic;
    text-align: center;
  }

  .update-btn {
    margin-top: 20px;
    padding: 8px 24px;
    border: 1px solid var(--accent);
    border-radius: 999px;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition);
  }

  .update-btn:hover {
    background: var(--accent-soft);
  }

  .info-card {
    margin-top: 24px;
    width: 100%;
    max-width: 320px;
    background: var(--bg-card);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-card);
    padding: 12px 16px;
  }

  .info-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0;
  }

  .info-row.separator {
    border-top: 1px solid var(--separator);
  }

  .info-label {
    font-size: 13px;
    color: var(--text-hint);
  }

  .info-value {
    font-size: 13px;
    color: var(--text-primary);
  }

  .link-btn {
    font-size: 13px;
    color: var(--accent);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-weight: 500;
    transition: opacity var(--transition);
  }

  .link-btn:hover {
    opacity: 0.8;
  }
</style>
