import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  getConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onConfigChanged: vi.fn(),
}));

const { getConfig } = await import('$lib/commands');
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

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  configStore.destroy();
});

describe('configStore basics', () => {
  it('exposes version and loaded getters and reflects loaded=true after init', async () => {
    vi.mocked(onConfigChanged).mockResolvedValue(() => {});
    vi.mocked(getConfig).mockResolvedValue(SNAPSHOT_CONFIG);

    const before = configStore.version;
    expect(typeof before).toBe('number');

    await configStore.init();

    expect(configStore.loaded).toBe(true);
    expect(configStore.current.timer.work_minutes).toBe(30);
    // version should have incremented at least once during init
    expect(configStore.version).toBeGreaterThan(before);
  });

  it('event callback updates current and marks store as loaded', async () => {
    let capturedCallback: ((payload: unknown) => void) | null = null;
    vi.mocked(onConfigChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(() => {});
    });
    vi.mocked(getConfig).mockResolvedValue(SNAPSHOT_CONFIG);

    await configStore.init();

    const eventPayload = {
      ...SNAPSHOT_CONFIG,
      timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 99 },
    };

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
    vi.mocked(getConfig).mockResolvedValue(SNAPSHOT_CONFIG);

    await configStore.init();
    await configStore.init();

    expect(firstUnlisten).toHaveBeenCalled();
    expect(secondUnlisten).not.toHaveBeenCalled();
  });

  it('destroy unlistens the active subscription', async () => {
    const unlisten = vi.fn();
    vi.mocked(onConfigChanged).mockResolvedValue(unlisten);
    vi.mocked(getConfig).mockResolvedValue(SNAPSHOT_CONFIG);

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

    const eventFirst = {
      ...SNAPSHOT_CONFIG,
      timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 77 },
    };
    let resolveSnapshot: (config: typeof SNAPSHOT_CONFIG) => void = () => {};
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
    resolveSnapshot({
      ...SNAPSHOT_CONFIG,
      timer: { ...SNAPSHOT_CONFIG.timer, work_minutes: 1 },
    });

    await initPromise;

    expect(configStore.current.timer.work_minutes).toBe(77);
  });
});
