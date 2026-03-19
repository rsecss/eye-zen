import { getStateSnapshot } from '$lib/commands';
import { onStateChanged } from '$lib/events';
import type { StatePayload } from '$lib/bindings/StatePayload';

const DEFAULT_STATE: StatePayload = {
  state: 'working',
  remaining_secs: 0,
  work_minutes: 20,
  rest_seconds: 20,
};

let state = $state<StatePayload>({ ...DEFAULT_STATE });
let unlisten: (() => void) | null = null;

export const timerStore = {
  get current(): StatePayload {
    return state;
  },

  async init(): Promise<void> {
    unlisten = await onStateChanged((payload) => {
      state = payload;
    });
    state = await getStateSnapshot();
  },

  destroy(): void {
    unlisten?.();
    unlisten = null;
  },
};
