import {
  getConfig,
  updateBehaviorConfig,
  updateDisplayConfig,
  updateHotkeysConfig,
  updatePomodoroConfig,
  updateScheduleConfig,
  updateTimerConfig,
} from '$lib/commands';
import { onConfigChanged } from '$lib/events';
import type { BehaviorConfig } from '$lib/bindings/BehaviorConfig';
import type { Config } from '$lib/bindings/Config';
import type { ConfigSnapshot } from '$lib/bindings/ConfigSnapshot';
import type { DisplayConfig } from '$lib/bindings/DisplayConfig';
import type { HotkeysConfig } from '$lib/bindings/HotkeysConfig';
import type { PomodoroConfig } from '$lib/bindings/PomodoroConfig';
import type { ScheduleConfig } from '$lib/bindings/ScheduleConfig';
import type { TimerConfig } from '$lib/bindings/TimerConfig';

type ConfigWriteScope =
  | 'timer'
  | 'behavior'
  | 'autoStart'
  | 'whitelist'
  | 'display'
  | 'schedule'
  | 'hotkeys'
  | 'pomodoro';

type FieldIntent<Section extends object> = {
  [Field in keyof Section]: { field: Field; value: Section[Field] };
}[keyof Section];

const DEFAULT_CONFIG: Config = {
  timer: {
    work_minutes: 20,
    rest_seconds: 20,
    pre_alert_seconds: 15,
    alert_timeout_seconds: 60,
    mode: 'twenty_twenty_twenty',
  },
  behavior: {
    sound_enabled: true,
    fullscreen_skip: true,
    afk_skip_enabled: true,
    afk_threshold_minutes: 5,
    auto_start: false,
    process_whitelist_enabled: false,
    process_whitelist: [],
  },
  display: {
    language: 'zh-CN',
    theme: 'light',
  },
  schedule: {
    enabled: false,
    active_days: [true, true, true, true, true, false, false],
  },
  hotkeys: {
    start_rest: 'CommandOrControl+Alt+B',
    skip_rest: 'CommandOrControl+Alt+S',
    toggle_pause: 'CommandOrControl+Alt+P',
  },
  pomodoro: {
    focus_minutes: 25,
    short_break_minutes: 5,
    long_break_minutes: 15,
    cycles_per_long: 4,
  },
};

let config = $state<Config>({ ...DEFAULT_CONFIG });
let unlisten: (() => void) | null = null;
let revision = $state(0);
let loaded = $state(false);
let initGeneration = 0;
let writeTail: Promise<void> = Promise.resolve();
let sectionErrors = $state<Record<ConfigWriteScope, unknown | null>>({
  timer: null,
  behavior: null,
  autoStart: null,
  whitelist: null,
  display: null,
  schedule: null,
  hotkeys: null,
  pomodoro: null,
});
let sectionErrorGenerations: Record<ConfigWriteScope, number> = {
  timer: 0,
  behavior: 0,
  autoStart: 0,
  whitelist: 0,
  display: 0,
  schedule: 0,
  hotkeys: 0,
  pomodoro: 0,
};

function replaceSnapshot(snapshot: ConfigSnapshot): void {
  config = snapshot.config;
  revision = snapshot.revision;
  loaded = true;
}

function applyNewerSnapshot(snapshot: ConfigSnapshot): void {
  if (!loaded || snapshot.revision > revision) {
    replaceSnapshot(snapshot);
  }
}

function isConfigConflict(err: unknown): boolean {
  return typeof err === 'object' && err !== null && 'kind' in err && err.kind === 'ConfigConflict';
}

function recordSectionError(section: ConfigWriteScope, err: unknown): void {
  sectionErrorGenerations[section] += 1;
  sectionErrors[section] = err;
}

function enqueueConfigUpdate(
  section: ConfigWriteScope,
  submit: () => Promise<ConfigSnapshot>,
): Promise<void> {
  const observedErrorGeneration =
    sectionErrors[section] === null ? null : sectionErrorGenerations[section];
  const completion = writeTail.then(async () => {
    if (
      observedErrorGeneration !== null &&
      observedErrorGeneration === sectionErrorGenerations[section]
    ) {
      sectionErrors[section] = null;
    }
    try {
      const snapshot = await submit();
      applyNewerSnapshot(snapshot);
    } catch (err) {
      if (isConfigConflict(err)) {
        try {
          applyNewerSnapshot(await getConfig());
        } catch (refreshError) {
          recordSectionError(section, refreshError);
          throw refreshError;
        }
      }
      recordSectionError(section, err);
      throw err;
    }
  });

  // The caller receives `completion` and its rejection. The tail settles only
  // to keep later user intents schedulable after a visible failed write.
  writeTail = completion.then(
    () => undefined,
    () => undefined,
  );
  return completion;
}

export const configStore = {
  get current(): Config {
    return config;
  },

  get version(): number {
    return revision;
  },

  get loaded(): boolean {
    return loaded;
  },

  errorFor(section: ConfigWriteScope): unknown | null {
    return sectionErrors[section];
  },

  async init(): Promise<void> {
    // Clean up previous subscription if any
    unlisten?.();
    unlisten = null;
    loaded = false;
    const generation = ++initGeneration;
    let eventSeen = false;

    const newUnlisten = await onConfigChanged((payload) => {
      if (generation !== initGeneration) return;
      eventSeen = true;
      applyNewerSnapshot(payload);
    });

    try {
      const snapshot = await getConfig();
      if (generation !== initGeneration) {
        newUnlisten();
        return;
      }
      if (!eventSeen || snapshot.revision > revision) {
        replaceSnapshot(snapshot);
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
    initGeneration++;
    unlisten?.();
    unlisten = null;
    loaded = false;
  },

  updateTimer(intent: FieldIntent<TimerConfig>): Promise<void> {
    return enqueueConfigUpdate('timer', () => {
      const updated: TimerConfig = { ...config.timer, [intent.field]: intent.value };
      return updateTimerConfig(updated, revision);
    });
  },

  updateBehavior(intent: FieldIntent<BehaviorConfig>): Promise<void> {
    return enqueueConfigUpdate('behavior', () => {
      const updated: BehaviorConfig = { ...config.behavior, [intent.field]: intent.value };
      return updateBehaviorConfig(updated, revision);
    });
  },

  updateAutoStart(value: boolean): Promise<void> {
    return enqueueConfigUpdate('autoStart', () => {
      const updated: BehaviorConfig = { ...config.behavior, auto_start: value };
      return updateBehaviorConfig(updated, revision);
    });
  },

  updateWhitelist(
    intent: FieldIntent<Pick<BehaviorConfig, 'process_whitelist_enabled' | 'process_whitelist'>>,
  ): Promise<void> {
    return enqueueConfigUpdate('whitelist', () => {
      const updated: BehaviorConfig = { ...config.behavior, [intent.field]: intent.value };
      return updateBehaviorConfig(updated, revision);
    });
  },

  updateDisplay(intent: FieldIntent<DisplayConfig>): Promise<void> {
    return enqueueConfigUpdate('display', () => {
      const updated: DisplayConfig = { ...config.display, [intent.field]: intent.value };
      return updateDisplayConfig(updated, revision);
    });
  },

  updateSchedule(intent: FieldIntent<ScheduleConfig>): Promise<void> {
    return enqueueConfigUpdate('schedule', () => {
      const updated: ScheduleConfig = { ...config.schedule, [intent.field]: intent.value };
      return updateScheduleConfig(updated, revision);
    });
  },

  updateHotkeys(intent: FieldIntent<HotkeysConfig>): Promise<void> {
    return enqueueConfigUpdate('hotkeys', () => {
      const updated: HotkeysConfig = { ...config.hotkeys, [intent.field]: intent.value };
      return updateHotkeysConfig(updated, revision);
    });
  },

  updatePomodoro(intent: FieldIntent<PomodoroConfig>): Promise<void> {
    return enqueueConfigUpdate('pomodoro', () => {
      const updated: PomodoroConfig = { ...config.pomodoro, [intent.field]: intent.value };
      return updatePomodoroConfig(updated, revision);
    });
  },
};
