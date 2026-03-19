<script lang="ts">
  import { onMount } from 'svelte';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { timerStore } from '$lib/stores/timer.svelte';
  import { pauseTimer, resumeTimer, startRest, skipRest } from '$lib/commands';

  onMount(() => {
    timerStore.init();
    return () => timerStore.destroy();
  });

  // --- Derived state ---

  const currentState = $derived(timerStore.current.state);
  const remainingSecs = $derived(timerStore.current.remaining_secs);
  const workMinutes = $derived(timerStore.current.work_minutes);
  const restSeconds = $derived(timerStore.current.rest_seconds);

  const formattedTime = $derived(formatTime(remainingSecs));

  const progressPercent = $derived(
    computeProgress(currentState, remainingSecs, workMinutes, restSeconds),
  );

  const stateClass = $derived(
    currentState === 'working'
      ? 's-work'
      : currentState === 'paused'
        ? 's-pause'
        : currentState === 'resting'
          ? 's-rest'
          : 's-alert', // pre_alert + alerting
  );

  const dotColor = $derived(
    currentState === 'paused'
      ? '#9ca3af'
      : currentState === 'pre_alert' || currentState === 'alerting'
        ? '#f59e0b'
        : '#10b981',
  );

  const dotGlowColor = $derived(
    currentState === 'paused'
      ? 'transparent'
      : currentState === 'pre_alert' || currentState === 'alerting'
        ? 'rgba(245,158,11,0.3)'
        : 'rgba(16,185,129,0.3)',
  );

  const dotAnimated = $derived(currentState !== 'paused');

  const stateLabel = $derived(
    currentState === 'working'
      ? 'Working'
      : currentState === 'paused'
        ? 'Paused'
        : currentState === 'resting'
          ? 'Resting'
          : currentState === 'pre_alert'
            ? 'Break soon'
            : 'Time for a break',
  );

  const labelColor = $derived(
    currentState === 'paused'
      ? '#9ca3af'
      : currentState === 'pre_alert' || currentState === 'alerting'
        ? '#b45309'
        : '#059669',
  );

  const timeDimmed = $derived(currentState === 'paused');

  const barFillStyle = $derived(computeBarFill(currentState, progressPercent));

  // --- Helpers ---

  function formatTime(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${String(s).padStart(2, '0')}`;
  }

  function computeProgress(
    state: string,
    remaining: number,
    workMins: number,
    restSecs: number,
  ): number {
    if (state === 'working' || state === 'paused') {
      const total = workMins * 60;
      if (total <= 0) return 0;
      return Math.round(((total - remaining) / total) * 100);
    }
    if (state === 'resting') {
      if (restSecs <= 0) return 0;
      return Math.round(((restSecs - remaining) / restSecs) * 100);
    }
    // pre_alert / alerting: bar is full
    return 100;
  }

  function computeBarFill(state: string, percent: number): string {
    if (state === 'paused') {
      return `width: ${percent}%; background: #e5e7eb;`;
    }
    if (state === 'pre_alert' || state === 'alerting') {
      return `width: ${percent}%; background: linear-gradient(90deg, #fbbf24, #f59e0b);`;
    }
    return `width: ${percent}%; background: linear-gradient(90deg, #34d399, #6ee7b7);`;
  }

  // --- Button handlers ---

  async function handlePause(): Promise<void> {
    try {
      await pauseTimer();
    } catch (e) {
      console.error('Failed to pause timer:', e);
    }
  }

  async function handleResume(): Promise<void> {
    try {
      await resumeTimer();
    } catch (e) {
      console.error('Failed to resume timer:', e);
    }
  }

  async function handleStartRest(): Promise<void> {
    try {
      await startRest();
    } catch (e) {
      console.error('Failed to start rest:', e);
    }
  }

  async function handleSkip(): Promise<void> {
    try {
      await skipRest();
    } catch (e) {
      console.error('Failed to skip rest:', e);
    }
  }

  async function handleSettings(): Promise<void> {
    try {
      const win = await WebviewWindow.getByLabel('main-window');
      await win?.show();
    } catch (e) {
      console.error('Failed to open settings:', e);
    }
  }
</script>

{#snippet settingsIcon()}
  <svg viewBox="0 0 24 24">
    <path
      d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
    />
    <circle cx="12" cy="12" r="3" />
  </svg>
{/snippet}

<main class="tray-card {stateClass}">
  <!-- Status row -->
  <div class="status-row">
    <div class="status-left">
      <div
        class="dot"
        class:live={dotAnimated}
        style:background={dotColor}
        style:color={dotGlowColor}
      ></div>
      <span class="state-label" style:color={labelColor}>{stateLabel}</span>
    </div>
    <span class="countdown" class:dim={timeDimmed}>{formattedTime}</span>
  </div>

  <!-- Progress bar -->
  <div class="progress-bar">
    <div class="progress-fill" style={barFillStyle}></div>
  </div>

  <!-- Action buttons -->
  <div class="btn-row">
    {#if currentState === 'working'}
      <button class="btn primary" aria-label="Pause timer" onclick={handlePause}>Pause</button>
      <button
        class="btn icon-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={handleSettings}
      >
        {@render settingsIcon()}
      </button>
    {:else if currentState === 'paused'}
      <button class="btn primary" aria-label="Resume timer" onclick={handleResume}>Resume</button>
      <button
        class="btn icon-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={handleSettings}
      >
        {@render settingsIcon()}
      </button>
    {:else if currentState === 'pre_alert'}
      <!-- pre_alert: user can still pause, no rest action yet -->
      <button class="btn primary" aria-label="Pause timer" onclick={handlePause}>Pause</button>
      <button
        class="btn icon-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={handleSettings}
      >
        {@render settingsIcon()}
      </button>
    {:else if currentState === 'alerting'}
      <button class="btn solid" aria-label="Start rest" onclick={handleStartRest}>Start Rest</button
      >
      <button class="btn neutral" aria-label="Skip" onclick={handleSkip}>Skip</button>
    {:else if currentState === 'resting'}
      <button class="btn neutral" aria-label="Skip" onclick={handleSkip}>Skip</button>
      <button
        class="btn icon-btn"
        title="Settings"
        aria-label="Open settings"
        onclick={handleSettings}
      >
        {@render settingsIcon()}
      </button>
    {/if}
  </div>
</main>

<style>
  /* --- Glassmorphic tray card --- */
  .tray-card {
    width: 100%;
    min-height: 100vh;
    background: var(--glass-tray-bg);
    backdrop-filter: blur(24px) saturate(1.5);
    -webkit-backdrop-filter: blur(24px) saturate(1.5);
    border-radius: 16px;
    border: 1px solid rgba(255, 255, 255, 0.6);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.04),
      0 8px 28px rgba(0, 0, 0, 0.06);
    padding: 20px 22px;
    position: relative;
    overflow: hidden;
    box-sizing: border-box;
  }

  /* Top accent bar */
  .tray-card::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 3px;
    border-radius: 20px 20px 0 0;
  }

  .tray-card.s-work::before {
    background: linear-gradient(90deg, #34d399, #6ee7b7, #a7f3d0);
  }

  .tray-card.s-pause::before {
    background: linear-gradient(90deg, #d1d5db, #e5e7eb);
  }

  .tray-card.s-alert::before {
    background: linear-gradient(90deg, #fbbf24, #f59e0b, #fbbf24);
  }

  .tray-card.s-rest::before {
    background: linear-gradient(90deg, #6ee7b7, #34d399, #a7f3d0);
  }

  /* --- Status row --- */
  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  /* --- Breathing dot --- */
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .dot.live {
    animation: breatheDot 2.8s ease-in-out infinite;
  }

  @keyframes breatheDot {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
      box-shadow: 0 0 0 0 currentColor;
    }
    50% {
      opacity: 0.6;
      transform: scale(0.85);
      box-shadow: 0 0 8px 2px currentColor;
    }
  }

  /* --- State label --- */
  .state-label {
    font-size: 15px;
    font-weight: 650;
    letter-spacing: 0.1px;
  }

  /* --- Countdown --- */
  .countdown {
    font-size: 32px;
    font-weight: 300;
    color: #1a1d23;
    font-variant-numeric: tabular-nums;
    letter-spacing: 1px;
  }

  .countdown.dim {
    color: #c0c5ce;
  }

  /* --- Progress bar --- */
  .progress-bar {
    width: 100%;
    height: 4px;
    background: rgba(0, 0, 0, 0.04);
    border-radius: 3px;
    margin-bottom: 18px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.4s ease;
  }

  /* --- Action buttons --- */
  .btn-row {
    display: flex;
    gap: 8px;
  }

  .btn {
    flex: 1;
    font-size: 14px;
    padding: 11px 0;
    border-radius: 12px;
    border: none;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.18s ease;
    font-family: inherit;
    letter-spacing: 0.1px;
  }

  .btn:hover {
    transform: translateY(-1px);
  }

  .btn.primary {
    background: rgba(5, 150, 105, 0.1);
    color: var(--state-active-label);
  }

  .btn.primary:hover {
    background: rgba(5, 150, 105, 0.18);
  }

  .btn.solid {
    background: var(--state-active-label);
    color: #fff;
  }

  .btn.solid:hover {
    background: #047857;
  }

  .btn.neutral {
    background: rgba(0, 0, 0, 0.04);
    color: #6b7280;
  }

  .btn.neutral:hover {
    background: rgba(0, 0, 0, 0.07);
  }

  .btn.icon-btn {
    flex: none;
    width: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.03);
    color: #9ca3af;
    border-radius: 12px;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .btn.icon-btn:hover {
    background: rgba(0, 0, 0, 0.06);
    color: #6b7280;
  }

  .btn.icon-btn svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
