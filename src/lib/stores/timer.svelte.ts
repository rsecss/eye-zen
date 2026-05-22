import { getStateSnapshot } from '$lib/commands';
import { onStateChanged } from '$lib/events';
import type { StatePayload } from '$lib/bindings/StatePayload';

const DEFAULT_STATE: StatePayload = {
  state: 'working',
  remaining_secs: 0,
  work_minutes: 20,
  rest_seconds: 20,
  mode: 'twenty_twenty_twenty',
  pomodoro: null,
};

let state = $state<StatePayload>({ ...DEFAULT_STATE });
let unlisten: (() => void) | null = null;
let version = 0;

export const timerStore = {
  get current(): StatePayload {
    return state;
  },

  async init(): Promise<void> {
    // Clean up previous subscription if any
    unlisten?.();
    unlisten = null;

    const initVersion = ++version;

    const newUnlisten = await onStateChanged((payload) => {
      version++;
      state = payload;
    });

    try {
      const snapshot = await getStateSnapshot();
      // Only apply snapshot if no event has arrived since we started
      if (version === initVersion) {
        state = snapshot;
      }
    } catch (err) {
      // Rollback listener on failure
      newUnlisten();
      throw err;
    }

    unlisten = newUnlisten;
  },

  destroy(): void {
    unlisten?.();
    unlisten = null;
  },
};
