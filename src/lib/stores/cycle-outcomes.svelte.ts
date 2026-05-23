import { getStatisticsCycleOutcomes } from '$lib/commands';
import type { CycleOutcomesPayload } from '$lib/bindings/CycleOutcomesPayload';

let outcomes = $state<CycleOutcomesPayload | null>(null);
let loading = $state(false);
let errorMessage = $state<string | null>(null);
let version = 0;

function resolveTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
}

export const cycleOutcomesStore = {
  get current(): CycleOutcomesPayload | null {
    return outcomes;
  },

  get loading(): boolean {
    return loading;
  },

  get error(): string | null {
    return errorMessage;
  },

  async refresh(): Promise<void> {
    const reqVersion = ++version;
    loading = true;
    errorMessage = null;
    try {
      const payload = await getStatisticsCycleOutcomes(resolveTimezone());
      // Drop stale responses if a newer refresh has been issued.
      if (reqVersion !== version) return;
      outcomes = payload;
    } catch (err) {
      if (reqVersion !== version) return;
      errorMessage = err instanceof Error ? err.message : String(err);
      console.error('Failed to load cycle outcomes:', err);
    } finally {
      if (reqVersion === version) {
        loading = false;
      }
    }
  },

  reset(): void {
    version++;
    outcomes = null;
    loading = false;
    errorMessage = null;
  },
};
