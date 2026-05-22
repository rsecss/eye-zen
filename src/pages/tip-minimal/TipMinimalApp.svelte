<script lang="ts">
  import { onMount } from 'svelte';
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

  const label = $derived.by(() => {
    const { state, mode, pomodoro } = timerStore.current;
    const isResting = state === 'resting';
    if (mode === 'pomodoro' && pomodoro && isResting) {
      const key = pomodoro.is_long_break
        ? 'tipMinimal.pomodoro.longBreak'
        : 'tipMinimal.pomodoro.shortBreak';
      return i18nStore
        .t(key)
        .replace('{current}', String(pomodoro.cycle_index))
        .replace('{total}', String(pomodoro.cycles_per_long));
    }
    return isResting ? i18nStore.t('tipMinimal.resting') : i18nStore.t('tipMinimal.alerting');
  });
</script>

<main
  class="h-screen w-screen flex items-center justify-center"
  style="background: linear-gradient(170deg, rgba(6,78,59,0.85) 0%, rgba(4,47,46,0.9) 50%, rgba(13,17,23,0.92) 100%);"
>
  <p
    class="breathing"
    style="color: rgba(255,255,255,0.32); font-size: 17px; letter-spacing: 0.8px; font-weight: 350;"
  >
    {label}
  </p>
</main>

<style>
  .breathing {
    animation: breathe 5s ease-in-out infinite;
  }

  @keyframes breathe {
    0%,
    100% {
      opacity: 0.32;
    }
    50% {
      opacity: 0.52;
    }
  }
</style>
