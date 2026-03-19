<script lang="ts">
  let timerState = $state('Working');
  let countdown = $state('18:30');
</script>

<main class="p-3" style="background: var(--bg-card);">
  <div class="flex items-center justify-between mb-2">
    <span class="text-xs font-medium" style="color: var(--accent);">{timerState}</span>
    <span class="text-xs tabular-nums" style="color: var(--text-secondary);">{countdown}</span>
  </div>

  <div class="flex gap-2">
    <button
      class="flex-1 text-xs py-1.5 rounded-lg"
      style="background: var(--accent-soft); color: var(--accent);"
    >
      Pause
    </button>
    <button
      class="flex-1 text-xs py-1.5 rounded-lg"
      style="background: var(--bg-secondary); color: var(--text-secondary);"
    >
      Skip
    </button>
  </div>
</main>
