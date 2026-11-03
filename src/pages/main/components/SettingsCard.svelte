<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    error = null,
    children,
  }: {
    title: string;
    error?: string | null;
    children: Snippet;
  } = $props();
</script>

<section class="card">
  <header class="card-header">
    <span class="gradient-bar"></span>
    <h2 class="card-title">{title}</h2>
  </header>
  <div class="card-body">
    {@render children()}
    {#if error}
      <div class="card-error" role="alert">{error}</div>
    {/if}
  </div>
</section>

<style>
  .card {
    background: var(--bg-card, #ffffff);
    border-radius: var(--radius-md, 12px);
    box-shadow: var(
      --shadow-card,
      0 1px 2px rgba(0, 0, 0, 0.02),
      0 2px 8px rgba(22, 100, 60, 0.03)
    );
    border: 1px solid var(--border-light, rgba(0, 0, 0, 0.03));
    transition:
      transform var(--transition, 150ms ease),
      box-shadow var(--transition, 150ms ease);
  }

  .card:hover {
    transform: translateY(-1px);
    box-shadow: var(
      --shadow-card-hover,
      0 2px 4px rgba(0, 0, 0, 0.03),
      0 4px 16px rgba(22, 100, 60, 0.05)
    );
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 18px 20px 12px;
    border-bottom: 1px solid var(--separator, #f0f3f1);
  }

  .gradient-bar {
    width: 3px;
    height: 14px;
    border-radius: 2px;
    background: linear-gradient(to bottom, var(--accent, #22c55e), #86efac);
    flex-shrink: 0;
  }

  .card-title {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary, #5f6b6a);
    line-height: 1;
  }

  .card-body {
    padding: 18px 20px;
  }

  .card-error {
    margin-top: 12px;
    padding: 8px 10px;
    border: 1px solid var(--state-alert);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--state-alert) 10%, transparent);
    color: var(--state-alert-label);
    font-size: 12px;
    line-height: 1.45;
  }
</style>
