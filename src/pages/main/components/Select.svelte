<script lang="ts">
  let {
    value,
    options,
    onchange,
  }: {
    value: string;
    options: { value: string; label: string }[];
    onchange: (v: string) => void;
  } = $props();
</script>

<div class="select-wrapper">
  <select class="select-pill" {value} onchange={(e) => onchange(e.currentTarget.value)}>
    {#each options as opt (opt.value)}
      <option value={opt.value} selected={opt.value === value}>{opt.label}</option>
    {/each}
  </select>
  <span class="select-arrow" aria-hidden="true"></span>
</div>

<style>
  .select-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .select-pill {
    appearance: none;
    -webkit-appearance: none;
    padding: 5px 28px 5px 12px;
    border: 1px solid var(--border, #e5eae7);
    border-radius: var(--radius-sm, 8px);
    background: transparent;
    color: var(--text-primary, #1a1d23);
    font-size: 13px;
    line-height: 1.4;
    cursor: pointer;
    outline: none;
    transition:
      background var(--transition, 150ms ease),
      border-color var(--transition, 150ms ease);
  }

  .select-pill:hover {
    background: var(--accent-soft, rgba(34, 197, 94, 0.07));
    border-color: var(--accent-border, rgba(34, 197, 94, 0.12));
  }

  .select-pill:focus-visible {
    border-color: var(--accent-border, rgba(34, 197, 94, 0.12));
    box-shadow: 0 0 0 2px var(--accent-soft, rgba(34, 197, 94, 0.07));
  }

  .select-arrow {
    position: absolute;
    right: 10px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
    width: 0;
    height: 0;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--text-secondary, #5f6b6a);
  }
</style>
