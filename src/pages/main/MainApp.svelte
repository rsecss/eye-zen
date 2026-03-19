<script lang="ts">
  const tabs = ['Settings', 'About'] as const;
  type Tab = (typeof tabs)[number];

  let activeTab = $state<Tab>('Settings');
</script>

<main class="h-screen flex flex-col" style="background: var(--bg-primary);">
  <nav class="flex gap-1 p-3 pb-0">
    {#each tabs as tab}
      <button
        class="px-4 py-2 text-sm font-medium rounded-lg transition-colors"
        class:active={activeTab === tab}
        onclick={() => (activeTab = tab)}
        style="
          background: {activeTab === tab ? 'var(--accent-soft)' : 'transparent'};
          color: {activeTab === tab ? 'var(--accent)' : 'var(--text-secondary)'};
        "
      >
        {tab}
      </button>
    {/each}
  </nav>

  <div class="flex-1 p-4 overflow-y-auto">
    {#if activeTab === 'Settings'}
      <p style="color: var(--text-secondary);">Settings page placeholder</p>
    {:else if activeTab === 'About'}
      <p style="color: var(--text-secondary);">Eyezen v0.1.0</p>
    {/if}
  </div>
</main>
