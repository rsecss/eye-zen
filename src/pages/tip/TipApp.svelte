<script lang="ts">
  let remaining = $state(20);
</script>

<main
  class="h-screen w-screen flex items-center justify-center"
  style="background: rgba(0, 0, 0, 0.85);"
>
  <div class="text-center text-white">
    <p class="text-lg opacity-70 mb-2">Look at something 20 feet away</p>
    <p class="text-6xl font-light tabular-nums">{remaining}s</p>
  </div>
</main>
