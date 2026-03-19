<script lang="ts">
  let {
    value,
    min,
    max,
    step,
    unit,
    onchange,
  }: {
    value: number;
    min: number;
    max: number;
    step: number;
    unit: string;
    onchange: (v: number) => void;
  } = $props();

  function increment() {
    const next = Math.min(max, value + step);
    if (next !== value) onchange(next);
  }

  function decrement() {
    const next = Math.max(min, value - step);
    if (next !== value) onchange(next);
  }
</script>

<div class="stepper-group">
  <button class="stepper-btn" aria-label="Decrease" onclick={decrement} disabled={value <= min}
    >−</button
  >

  <div class="stepper-value">
    <span class="value-num">{value}</span>
    <span class="value-unit">{unit}</span>
  </div>

  <button class="stepper-btn" aria-label="Increase" onclick={increment} disabled={value >= max}
    >+</button
  >
</div>

<style>
  .stepper-group {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-sm);
    transition: background var(--transition);
  }

  .stepper-group:hover {
    background: var(--accent-glow);
  }

  .stepper-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary, #5f6b6a);
    font-size: 15px;
    cursor: pointer;
    transition:
      border-color 0.2s ease,
      background 0.2s ease,
      color 0.2s ease,
      transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1);
    user-select: none;
    line-height: 1;
  }

  .stepper-btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .stepper-btn:active:not(:disabled) {
    transform: scale(0.88);
  }

  .stepper-btn:disabled {
    opacity: 0.3;
    border-color: transparent;
    cursor: not-allowed;
  }

  .stepper-value {
    display: flex;
    align-items: baseline;
    gap: 3px;
    padding: 0 8px;
    width: 56px;
    justify-content: center;
  }

  .value-num {
    font-size: 18px;
    font-weight: 300;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .value-unit {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-hint);
    text-transform: lowercase;
  }
</style>
