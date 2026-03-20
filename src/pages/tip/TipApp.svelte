<script lang="ts">
  import { onMount } from 'svelte';
  import { skipRest, startRest } from '$lib/commands';
  import { i18nStore } from '$lib/i18n/index.svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import { timerStore } from '$lib/stores/timer.svelte';

  onMount(() => {
    timerStore.init().catch((err) => console.error('Failed to init timer store:', err));
    configStore.init().catch((err) => console.error('Failed to init config store:', err));

    return () => {
      timerStore.destroy();
      configStore.destroy();
    };
  });

  $effect(() => {
    i18nStore.setLocale(configStore.current.display.language);
  });

  const currentState = $derived(timerStore.current.state);
  const remainingSecs = $derived(timerStore.current.remaining_secs);
  const restSeconds = $derived(timerStore.current.rest_seconds);

  const formattedTime = $derived(formatTime(remainingSecs));

  const progressPercent = $derived(
    restSeconds > 0 ? Math.round(((restSeconds - remainingSecs) / restSeconds) * 100) : 0,
  );

  const isAlerting = $derived(currentState === 'alerting');
  const isResting = $derived(currentState === 'resting');

  function formatTime(secs: number): string {
    const minutes = Math.floor(secs / 60);
    const seconds = secs % 60;
    return `${minutes}:${String(seconds).padStart(2, '0')}`;
  }

  async function handleStartRest(): Promise<void> {
    try {
      await startRest();
    } catch (error) {
      console.error('Failed to start rest:', error);
    }
  }

  async function handleSkip(): Promise<void> {
    try {
      await skipRest();
    } catch (error) {
      console.error('Failed to skip rest:', error);
    }
  }
</script>

{#snippet eyeIcon()}
  <svg
    class="tip-icon"
    viewBox="0 0 24 24"
    fill="none"
    stroke="rgba(167,243,208,0.55)"
    stroke-width="1.3"
    stroke-linecap="round"
  >
    <path
      d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"
    />
    <circle cx="12" cy="12" r="3" />
  </svg>
{/snippet}

{#if isAlerting}
  <main class="tip-screen alerting">
    <div class="tip-inner">
      <div class="tip-card">
        {@render eyeIcon()}
        <p class="tip-h">{i18nStore.t('tip.alerting.title')}</p>
        <p class="tip-sub">{i18nStore.t('tip.alerting.subtitle')}</p>
        <p class="tip-time">{formattedTime}</p>
        <div class="tip-btns">
          <button
            class="tip-btn glass"
            aria-label={i18nStore.t('tip.startRest')}
            onclick={handleStartRest}
          >
            {i18nStore.t('tip.startRest')}
          </button>
          <button class="tip-btn ghost" aria-label={i18nStore.t('tip.skip')} onclick={handleSkip}>
            {i18nStore.t('tip.skip')}
          </button>
        </div>
      </div>
    </div>
  </main>
{:else if isResting}
  <main class="tip-screen resting">
    <div class="tip-inner">
      <div class="tip-card">
        {@render eyeIcon()}
        <p class="tip-h">{i18nStore.t('tip.resting.title')}</p>
        <p class="tip-sub">{i18nStore.t('tip.resting.subtitle')}</p>
        <p class="tip-time">{formattedTime}</p>
        <div class="tip-bar">
          <div class="tip-bar-fill" style:width="{progressPercent}%"></div>
        </div>
        <div class="tip-btns" style="margin-top: 28px;">
          <button class="tip-btn ghost" aria-label={i18nStore.t('tip.skip')} onclick={handleSkip}>
            {i18nStore.t('tip.skip')}
          </button>
        </div>
      </div>
    </div>
  </main>
{:else}
  <main class="tip-screen fallback">
    <div class="tip-inner">
      <p class="tip-fallback-text">{i18nStore.t('tip.waiting')}</p>
    </div>
  </main>
{/if}

<style>
  .tip-screen {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
  }

  .tip-screen.alerting {
    background: linear-gradient(155deg, #0c4a3e 0%, #0a2e28 30%, #0d1117 65%, #111318 100%);
  }

  .tip-screen.alerting::before {
    content: '';
    position: absolute;
    top: -20%;
    left: 20%;
    width: 60%;
    height: 70%;
    background: radial-gradient(
      ellipse,
      rgba(52, 211, 153, 0.12) 0%,
      rgba(16, 185, 129, 0.05) 40%,
      transparent 70%
    );
    filter: blur(20px);
  }

  .tip-screen.alerting::after {
    content: '';
    position: absolute;
    bottom: 0;
    right: 10%;
    width: 40%;
    height: 50%;
    background: radial-gradient(ellipse, rgba(99, 102, 241, 0.06) 0%, transparent 65%);
    filter: blur(30px);
  }

  .tip-screen.resting {
    background: linear-gradient(160deg, #064e3b 0%, #042f2e 35%, #0d1117 70%, #0f1115 100%);
  }

  .tip-screen.resting::before {
    content: '';
    position: absolute;
    top: 10%;
    left: 30%;
    width: 40%;
    height: 50%;
    background: radial-gradient(ellipse, rgba(110, 231, 183, 0.1) 0%, transparent 65%);
    filter: blur(25px);
  }

  .tip-screen.fallback {
    background: #0d1117;
  }

  .tip-inner {
    text-align: center;
    color: #fff;
    position: relative;
    z-index: 1;
    max-width: 420px;
  }

  .tip-card {
    background: var(--glass-tip-bg);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 24px;
    padding: 40px 48px;
  }

  .tip-icon {
    width: 36px;
    height: 36px;
    margin: 0 auto 22px;
    opacity: 0.35;
    display: block;
  }

  .tip-h {
    font-size: 22px;
    font-weight: 500;
    opacity: 0.75;
    margin: 0 0 6px;
    letter-spacing: 0.3px;
  }

  .tip-sub {
    font-size: 15px;
    font-weight: 400;
    opacity: 0.35;
    margin: 0 0 36px;
    line-height: 1.5;
    white-space: pre-line;
  }

  .tip-time {
    font-size: 80px;
    font-weight: 200;
    margin: 0;
    font-variant-numeric: tabular-nums;
    letter-spacing: 5px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.95) 20%,
      rgba(167, 243, 208, 0.6) 100%
    );
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    line-height: 1;
  }

  .tip-bar {
    width: 120px;
    height: 3px;
    margin: 12px auto 0;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.06);
    overflow: hidden;
  }

  .tip-bar-fill {
    height: 100%;
    border-radius: 2px;
    background: linear-gradient(90deg, rgba(52, 211, 153, 0.5), rgba(110, 231, 183, 0.4));
    transition: width 0.4s ease;
  }

  .tip-btns {
    display: flex;
    gap: 12px;
    justify-content: center;
    margin-top: 36px;
  }

  .tip-btn {
    padding: 13px 38px;
    border-radius: 14px;
    border: none;
    font-size: 15px;
    font-weight: 550;
    cursor: pointer;
    letter-spacing: 0.2px;
    transition: all 0.18s ease;
    font-family: inherit;
  }

  .tip-btn:hover {
    transform: translateY(-1px);
  }

  .tip-btn.glass {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.88);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .tip-btn.glass:hover {
    background: rgba(255, 255, 255, 0.16);
  }

  .tip-btn.ghost {
    background: transparent;
    color: rgba(255, 255, 255, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .tip-btn.ghost:hover {
    color: rgba(255, 255, 255, 0.5);
    border-color: rgba(255, 255, 255, 0.12);
  }

  .tip-fallback-text {
    font-size: 17px;
    color: rgba(255, 255, 255, 0.32);
    letter-spacing: 0.5px;
    font-weight: 350;
  }
</style>
