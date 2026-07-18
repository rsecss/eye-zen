import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  getConfig: vi.fn(),
  updateTimerConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onConfigChanged: vi.fn(),
}));

const { getConfig, updateTimerConfig } = await import('$lib/commands');
const { onConfigChanged } = await import('$lib/events');
const { configStore } = await import('../config.svelte');

const SNAPSHOT_CONFIG = {
  timer: {
    work_minutes: 30,
    rest_seconds: 25,
    pre_alert_seconds: 10,
    alert_timeout_seconds: 45,
    mode: 'twenty_twenty_twenty' as const,
  },
  behavior: {
    sound_enabled: true,
    fullscreen_skip: true,
    afk_skip_enabled: false,
    afk_threshold_minutes: 5,
    auto_start: false,
    process_whitelist_enabled: false,
    process_whitelist: [],
  },
  display: { language: 'zh-CN' as const, theme: 'light' as const },
  schedule: {
    enabled: false,
    active_days: [true, true, true, true, true, false, false] as [
      boolean,
      boolean,
      boolean,
      boolean,
      boolean,
      boolean,
      boolean,
    ],
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

function snapshot(revision: number, config = SNAPSHOT_CONFIG) {
  return { revision, config };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  configStore.destroy();
});

describe('configStore basics', () => {
  it('exposes version and loaded getters and reflects loaded=true after init', async () => {
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig).mockResolvedValue(snapshot(0));

    expect(typeof configStore.version).toBe('number');

    await configStore.init();

    expect(configStore.loaded).toBe(true);
    expect(configStore.current.timer.work_minutes).toBe(30);
    expect(configStore.version).toBe(0);
  });

  it('event callback updates current and marks store as loaded', async () => {
    let capturedCallback: ((payload: unknown) => void) | null = null;
    vi.mocked(onConfigChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(() => {});
    });
    vi.mocked(getConfig).mockResolvedValue(snapshot(0));

    await configStore.init();

    const eventPayload = snapshot(1, {
      ...SNAPSHOT_CONFIG,
      timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 99 },
    });

    expect(capturedCallback).not.toBeNull();
    capturedCallback!(eventPayload);

    expect(configStore.current.timer.work_minutes).toBe(99);
    expect(configStore.loaded).toBe(true);
  });

  it('rolls back the event listener when getConfig throws', async () => {
    const unlisten = vi.fn();
    vi.mocked(onConfigChanged).mockResolvedValue(unlisten);
    vi.mocked(getConfig).mockRejectedValueOnce(new Error('snapshot failed'));

    await expect(configStore.init()).rejects.toThrow('snapshot failed');

    // unlisten MUST be called as part of the failure rollback
    expect(unlisten).toHaveBeenCalled();
  });

  it('cleans up the previous subscription when init is called twice', async () => {
    const firstUnlisten = vi.fn();
    const secondUnlisten = vi.fn();
    vi.mocked(onConfigChanged)
      .mockResolvedValueOnce(firstUnlisten)
      .mockResolvedValueOnce(secondUnlisten);
    vi.mocked(getConfig).mockResolvedValue(snapshot(0));

    await configStore.init();
    await configStore.init();

    expect(firstUnlisten).toHaveBeenCalled();
    expect(secondUnlisten).not.toHaveBeenCalled();
  });

  it('destroy unlistens the active subscription', async () => {
    const unlisten = vi.fn();
    vi.mocked(onConfigChanged).mockResolvedValue(unlisten);
    vi.mocked(getConfig).mockResolvedValue(snapshot(0));

    await configStore.init();
    configStore.destroy();

    expect(unlisten).toHaveBeenCalled();
  });

  it('skips applying snapshot when an event fires before the snapshot resolves', async () => {
    let capturedCallback: ((payload: unknown) => void) | null = null;
    vi.mocked(onConfigChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(() => {});
    });

    const eventFirst = snapshot(1, {
      ...SNAPSHOT_CONFIG,
      timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 77 },
    });
    let resolveSnapshot: (value: ReturnType<typeof snapshot>) => void = () => {};
    vi.mocked(getConfig).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveSnapshot = resolve;
        }),
    );

    const initPromise = configStore.init();
    // Yield a few microtasks so init() reaches `await getConfig()`.
    for (let i = 0; i < 5; i++) await Promise.resolve();

    expect(capturedCallback).not.toBeNull();
    capturedCallback!(eventFirst);

    // Now resolve snapshot with a different value: it MUST NOT overwrite the
    // event payload since version has advanced.
    resolveSnapshot(
      snapshot(0, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 1 },
      }),
    );

    await initPromise;

    expect(configStore.current.timer.work_minutes).toBe(77);
  });

  it('ignores config events older than the current backend revision', async () => {
    let capturedCallback: ((payload: unknown) => void) | null = null;
    vi.mocked(onConfigChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(() => {});
    });
    vi.mocked(getConfig).mockResolvedValue(snapshot(2));

    await configStore.init();
    capturedCallback!(
      snapshot(1, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 99 },
      }),
    );

    expect(configStore.version).toBe(2);
    expect(configStore.current.timer.work_minutes).toBe(30);
  });

  it('serializes field intents and rebases the second update on the first snapshot', async () => {
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig).mockResolvedValue(snapshot(0));
    await configStore.init();

    let resolveFirst: (value: ReturnType<typeof snapshot>) => void = () => {};
    let resolveSecond: (value: ReturnType<typeof snapshot>) => void = () => {};
    vi.mocked(updateTimerConfig)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveSecond = resolve;
          }),
      );

    const first = configStore.updateTimer({ field: 'work_minutes', value: 31 });
    const second = configStore.updateTimer({ field: 'rest_seconds', value: 30 });

    await vi.waitFor(() => expect(updateTimerConfig).toHaveBeenCalledTimes(1));
    expect(updateTimerConfig).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ work_minutes: 31, rest_seconds: 25 }),
      0,
    );

    resolveFirst(
      snapshot(1, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 31 },
      }),
    );
    await first;
    await vi.waitFor(() => expect(updateTimerConfig).toHaveBeenCalledTimes(2));
    expect(updateTimerConfig).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({ work_minutes: 31, rest_seconds: 30 }),
      1,
    );

    resolveSecond(
      snapshot(2, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 31, rest_seconds: 30 },
      }),
    );
    await second;

    expect(configStore.current.timer.work_minutes).toBe(31);
    expect(configStore.current.timer.rest_seconds).toBe(30);
    expect(configStore.version).toBe(2);
  });

  it('refreshes after a revision conflict and exposes the section error', async () => {
    const conflict = {
      kind: 'ConfigConflict',
      detail: { expected: 0, actual: 1 },
    };
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig)
      .mockResolvedValueOnce(snapshot(0))
      .mockResolvedValueOnce(
        snapshot(1, {
          ...SNAPSHOT_CONFIG,
          timer: { ...SNAPSHOT_CONFIG.timer, rest_seconds: 40 },
        }),
      );
    vi.mocked(updateTimerConfig).mockRejectedValueOnce(conflict);
    await configStore.init();

    await expect(configStore.updateTimer({ field: 'work_minutes', value: 31 })).rejects.toBe(
      conflict,
    );

    expect(updateTimerConfig).toHaveBeenCalledTimes(1);
    expect(getConfig).toHaveBeenCalledTimes(2);
    expect(configStore.current.timer.rest_seconds).toBe(40);
    expect(configStore.errorFor('timer')).toStrictEqual(conflict);
  });

  it('does not let a stale conflict refresh overwrite a newer config event', async () => {
    const conflict = {
      kind: 'ConfigConflict',
      detail: { expected: 0, actual: 1 },
    };
    let capturedCallback: ((payload: unknown) => void) | null = null;
    vi.mocked(onConfigChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(() => {});
    });
    let resolveRefresh: (value: ReturnType<typeof snapshot>) => void = () => {};
    vi.mocked(getConfig)
      .mockResolvedValueOnce(snapshot(0))
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveRefresh = resolve;
          }),
      );
    vi.mocked(updateTimerConfig).mockRejectedValueOnce(conflict);
    await configStore.init();

    const update = configStore.updateTimer({ field: 'work_minutes', value: 31 });
    const rejection = expect(update).rejects.toBe(conflict);
    await vi.waitFor(() => expect(getConfig).toHaveBeenCalledTimes(2));

    capturedCallback!(
      snapshot(2, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 99 },
      }),
    );
    resolveRefresh(
      snapshot(1, {
        ...SNAPSHOT_CONFIG,
        timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 31 },
      }),
    );

    await rejection;
    expect(configStore.version).toBe(2);
    expect(configStore.current.timer.work_minutes).toBe(99);
  });

  it('keeps a conflict visible when an intent was already queued before the conflict', async () => {
    const conflict = {
      kind: 'ConfigConflict',
      detail: { expected: 0, actual: 1 },
    };
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig).mockResolvedValueOnce(snapshot(0)).mockResolvedValueOnce(snapshot(1));
    vi.mocked(updateTimerConfig)
      .mockRejectedValueOnce(conflict)
      .mockResolvedValueOnce(
        snapshot(2, {
          ...SNAPSHOT_CONFIG,
          timer: { ...SNAPSHOT_CONFIG.timer, rest_seconds: 30 },
        }),
      );
    await configStore.init();

    const conflicted = configStore.updateTimer({ field: 'work_minutes', value: 31 });
    const queuedBeforeConflict = configStore.updateTimer({ field: 'rest_seconds', value: 30 });

    await expect(conflicted).rejects.toBe(conflict);
    await queuedBeforeConflict;

    expect(configStore.errorFor('timer')).toStrictEqual(conflict);
  });

  it('clears a visible conflict when the user explicitly retries that scope', async () => {
    const conflict = {
      kind: 'ConfigConflict',
      detail: { expected: 0, actual: 1 },
    };
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig).mockResolvedValueOnce(snapshot(0)).mockResolvedValueOnce(snapshot(1));
    vi.mocked(updateTimerConfig)
      .mockRejectedValueOnce(conflict)
      .mockResolvedValueOnce(
        snapshot(2, {
          ...SNAPSHOT_CONFIG,
          timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 31 },
        }),
      );
    await configStore.init();

    await expect(configStore.updateTimer({ field: 'work_minutes', value: 31 })).rejects.toBe(
      conflict,
    );
    expect(configStore.errorFor('timer')).toStrictEqual(conflict);

    await configStore.updateTimer({ field: 'work_minutes', value: 31 });

    expect(configStore.errorFor('timer')).toBeNull();
  });
});
