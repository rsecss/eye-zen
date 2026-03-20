<script lang="ts">
  import { open } from '@tauri-apps/plugin-shell';

  const GITHUB_URL = 'https://github.com/rsecss/eye-zen';
  const RELEASES_URL = `${GITHUB_URL}/releases`;

  const platform = navigator.platform;

  function openUrl(url: string) {
    open(url).catch((err) => console.error('Failed to open URL:', err));
  }
</script>

<div class="about-page">
  <!-- Logo -->
  <div class="logo">
    <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="white" stroke-width="1.5">
      <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" />
      <circle cx="12" cy="12" r="3" fill="white" />
    </svg>
  </div>

  <!-- App name -->
  <h1 class="app-name">Eyezen</h1>

  <!-- Version -->
  <span class="version">Version 0.1.0</span>

  <!-- Description -->
  <p class="description">基于 20-20-20 规则的跨平台桌面护眼工具</p>

  <!-- Sub-description -->
  <p class="sub-description">Every 20 min, look 20 feet away, for 20 seconds</p>

  <!-- Update button -->
  <button class="update-btn" onclick={() => openUrl(RELEASES_URL)}> Check for Updates </button>

  <!-- Info card -->
  <div class="info-card">
    <div class="info-row">
      <span class="info-label">Platform</span>
      <span class="info-value">{platform}</span>
    </div>
    <div class="info-row separator">
      <span class="info-label">License</span>
      <span class="info-value">MIT</span>
    </div>
    <div class="info-row separator">
      <span class="info-label">Source</span>
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
    background: linear-gradient(135deg, #22c55e, #4ade80, #86efac);
    border-radius: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
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
