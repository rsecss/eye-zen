<script lang="ts">
  let {
    checked,
    onchange,
  }: {
    checked: boolean;
    onchange: (v: boolean) => void;
  } = $props();
</script>

<button
  role="switch"
  aria-checked={checked}
  aria-label="Toggle"
  class="toggle-track"
  class:on={checked}
  onclick={() => onchange(!checked)}
>
  <span class="toggle-knob" class:on={checked}></span>
</button>

<style>
  .toggle-track {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 40px;
    height: 22px;
    padding: 2px;
    border: none;
    border-radius: 11px;
    background: var(--toggle-off, #d1d8d4);
    cursor: pointer;
    transition: background 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    outline: none;
  }

  .toggle-track:focus-visible {
    box-shadow: 0 0 0 2px var(--accent-soft, rgba(34, 197, 94, 0.25));
  }

  .toggle-track.on {
    background: var(--toggle-on, #22c55e);
  }

  .toggle-knob {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    position: relative;
    left: 0;
    transition: left 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .toggle-knob.on {
    left: 18px;
  }
</style>
