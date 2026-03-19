<script lang="ts">
  import { onMount } from 'svelte';
  import { timerStore } from '$lib/stores/timer.svelte';

  onMount(() => {
    timerStore.init().catch((err) => console.error('Failed to init timer store:', err));
    return () => timerStore.destroy();
  });

  const label = $derived(
    timerStore.current.state === 'resting'
      ? 'Resting... look away from the screen'
      : 'Take a break... look away from the screen',
  );
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
