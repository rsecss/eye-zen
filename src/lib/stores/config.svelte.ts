import { getConfig } from '$lib/commands';
import { onConfigChanged } from '$lib/events';
import type { Config } from '$lib/bindings/Config';

const DEFAULT_CONFIG: Config = {
  timer: {
    work_minutes: 20,
    rest_seconds: 20,
    pre_alert_seconds: 15,
    alert_timeout_seconds: 60,
  },
  behavior: {
    sound_enabled: true,
    fullscreen_skip: true,
    auto_start: false,
  },
  display: {
    language: 'zh-CN',
    theme: 'light',
  },
};

let config = $state<Config>({ ...DEFAULT_CONFIG });
let unlisten: (() => void) | null = null;
let version = $state(0);
let loaded = $state(false);

export const configStore = {
  get current(): Config {
    return config;
  },

  get version(): number {
    return version;
  },

  get loaded(): boolean {
    return loaded;
  },

  async init(): Promise<void> {
    // Clean up previous subscription if any
    unlisten?.();
    unlisten = null;

    const initVersion = ++version;

    const newUnlisten = await onConfigChanged((payload) => {
      version++;
      config = payload;
      loaded = true;
    });

    try {
      const snapshot = await getConfig();
      // Only apply snapshot if no event has arrived since we started
      if (version === initVersion) {
        config = snapshot;
      }
      loaded = true;
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
