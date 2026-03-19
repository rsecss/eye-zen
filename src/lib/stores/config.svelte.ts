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

export const configStore = {
  get current(): Config {
    return config;
  },

  async init(): Promise<void> {
    unlisten = await onConfigChanged((payload) => {
      config = payload;
    });
    config = await getConfig();
  },

  destroy(): void {
    unlisten?.();
    unlisten = null;
  },
};
