<script lang="ts">
  import { onMount } from 'svelte';
  import { configStore } from '$lib/stores/config.svelte';
  import SettingsPage from './SettingsPage.svelte';
  import AboutPage from './AboutPage.svelte';

  const tabs = ['Settings', 'About'] as const;
  type Tab = (typeof tabs)[number];
  let activeTab = $state<Tab>('Settings');

  onMount(() => {
    configStore.init().catch((err) => console.error('Failed to init config store:', err));
    return () => configStore.destroy();
  });
</script>

<main class="h-screen flex flex-col" style="background: var(--bg-primary);">
  <nav class="flex gap-1 px-5 pt-3 pb-0" style="border-bottom: 1px solid rgba(34, 197, 94, 0.06);">
    {#each tabs as tab}
      <button class="tab-button" class:active={activeTab === tab} onclick={() => (activeTab = tab)}>
        {#if tab === 'Settings'}
          <svg
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="12" cy="12" r="3" />
            <path
              d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
            />
          </svg>
        {:else}
          <svg
            viewBox="0 0 24 24"
            width="16"
            height="16"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
        {/if}
        {tab}
      </button>
    {/each}
  </nav>

  <div class="flex-1 overflow-y-auto" style="padding: 18px 20px;">
    {#if activeTab === 'Settings'}
      <SettingsPage />
    {:else}
      <AboutPage />
    {/if}
  </div>
</main>

<style>
  .tab-button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
    position: relative;
    transition: color 0.15s ease;
  }

  .tab-button:hover {
    color: var(--text-secondary);
  }

  .tab-button.active {
    color: var(--accent-deep);
  }

  .tab-button.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 12px;
    right: 12px;
    height: 2px;
    background: linear-gradient(90deg, var(--accent), #4ade80);
    border-radius: 1px;
  }
</style>
