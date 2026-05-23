import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock the Tauri invoke surface before importing the wrappers.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const { invoke } = await import('@tauri-apps/api/core');
const mockInvoke = vi.mocked(invoke);

const commands = await import('../commands');

beforeEach(() => {
  mockInvoke.mockReset();
});

describe('command wrappers forward to invoke with the correct name + args', () => {
  it('getStateSnapshot calls invoke("get_state_snapshot")', async () => {
    mockInvoke.mockResolvedValue({ state: 'working', remaining_secs: 0 });
    await commands.getStateSnapshot();
    expect(mockInvoke).toHaveBeenCalledWith('get_state_snapshot', undefined);
  });

  it('startRest calls invoke("start_rest")', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await commands.startRest();
    expect(mockInvoke).toHaveBeenCalledWith('start_rest', undefined);
  });

  it('skipRest calls invoke("skip_rest")', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await commands.skipRest();
    expect(mockInvoke).toHaveBeenCalledWith('skip_rest', undefined);
  });

  it('pauseTimer calls invoke("pause_timer")', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await commands.pauseTimer();
    expect(mockInvoke).toHaveBeenCalledWith('pause_timer', undefined);
  });

  it('resumeTimer calls invoke("resume_timer")', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await commands.resumeTimer();
    expect(mockInvoke).toHaveBeenCalledWith('resume_timer', undefined);
  });

  it('getConfig calls invoke("get_config")', async () => {
    mockInvoke.mockResolvedValue({});
    await commands.getConfig();
    expect(mockInvoke).toHaveBeenCalledWith('get_config', undefined);
  });

  it('getHotkeyStatus calls invoke("get_hotkey_status")', async () => {
    mockInvoke.mockResolvedValue({ bindings: [], macos_accessibility: 'not_required' });
    await commands.getHotkeyStatus();
    expect(mockInvoke).toHaveBeenCalledWith('get_hotkey_status', undefined);
  });

  it('getStatisticsTrends forwards timezone arg', async () => {
    mockInvoke.mockResolvedValue({});
    await commands.getStatisticsTrends('Asia/Shanghai');
    expect(mockInvoke).toHaveBeenCalledWith('get_statistics_trends', {
      timezone: 'Asia/Shanghai',
    });
  });

  it('getStatisticsCycleOutcomes forwards timezone arg', async () => {
    mockInvoke.mockResolvedValue({});
    await commands.getStatisticsCycleOutcomes('UTC');
    expect(mockInvoke).toHaveBeenCalledWith('statistics_cycle_outcomes', {
      timezone: 'UTC',
    });
  });

  it('exportStatistics forwards targetPath', async () => {
    mockInvoke.mockResolvedValue(undefined);
    await commands.exportStatistics('C:/backup/eyezen.db');
    expect(mockInvoke).toHaveBeenCalledWith('export_statistics', {
      targetPath: 'C:/backup/eyezen.db',
    });
  });

  it('getDetectorCapabilities calls invoke("get_detector_capabilities")', async () => {
    mockInvoke.mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: true,
      fullscreen_detection_supported: true,
    });
    await commands.getDetectorCapabilities();
    expect(mockInvoke).toHaveBeenCalledWith('get_detector_capabilities', undefined);
  });

  it('updateTimerConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = {
      work_minutes: 25,
      rest_seconds: 20,
      pre_alert_seconds: 15,
      alert_timeout_seconds: 60,
      mode: 'twenty_twenty_twenty' as const,
    };
    await commands.updateTimerConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_timer_config', { config: cfg });
  });

  it('updateBehaviorConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = {
      sound_enabled: true,
      fullscreen_skip: false,
      afk_skip_enabled: false,
      afk_threshold_minutes: 5,
      auto_start: false,
      process_whitelist_enabled: false,
      process_whitelist: [],
    };
    await commands.updateBehaviorConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_behavior_config', { config: cfg });
  });

  it('updateDisplayConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = { language: 'en' as const, theme: 'dark' as const };
    await commands.updateDisplayConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_display_config', { config: cfg });
  });

  it('updateScheduleConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = {
      enabled: true,
      active_days: [true, true, true, true, true, false, false] as [
        boolean,
        boolean,
        boolean,
        boolean,
        boolean,
        boolean,
        boolean,
      ],
    };
    await commands.updateScheduleConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_schedule_config', { config: cfg });
  });

  it('updateHotkeysConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = {
      start_rest: 'CommandOrControl+Alt+B',
      skip_rest: 'CommandOrControl+Alt+S',
      toggle_pause: 'CommandOrControl+Alt+P',
    };
    await commands.updateHotkeysConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_hotkeys_config', { config: cfg });
  });

  it('updatePomodoroConfig wraps the payload in { config }', async () => {
    mockInvoke.mockResolvedValue(undefined);
    const cfg = {
      focus_minutes: 25,
      short_break_minutes: 5,
      long_break_minutes: 15,
      cycles_per_long: 4,
    };
    await commands.updatePomodoroConfig(cfg);
    expect(mockInvoke).toHaveBeenCalledWith('update_pomodoro_config', { config: cfg });
  });
});

describe('invokeWithTimeout race semantics', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('resolves with invoke result when invoke wins the race', async () => {
    mockInvoke.mockResolvedValue('snapshot');
    await expect(commands.getStateSnapshot()).resolves.toBe('snapshot');
  });

  it('rejects with timeout error after 5000ms when invoke never settles', async () => {
    // Build a never-resolving invoke so the 5s timeout wins.
    mockInvoke.mockReturnValueOnce(new Promise(() => {}));

    const pending = commands.getStateSnapshot();
    // Surface the rejection silently to avoid an unhandled rejection warning.
    const settled = pending.catch((err) => err);

    await vi.advanceTimersByTimeAsync(5000);

    const err = await settled;
    expect(err).toBeInstanceOf(Error);
    expect((err as Error).message).toMatch(/timed out/i);
  });

  it('clears the timeout when invoke settles first (no late rejection)', async () => {
    // Spy on clearTimeout to assert the cleanup path runs.
    const clearSpy = vi.spyOn(globalThis, 'clearTimeout');
    mockInvoke.mockResolvedValue('ok');

    await expect(commands.getStateSnapshot()).resolves.toBe('ok');
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore();
  });
});
