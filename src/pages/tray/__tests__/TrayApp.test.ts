import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  pauseTimer: vi.fn().mockResolvedValue(undefined),
  resumeTimer: vi.fn().mockResolvedValue(undefined),
  skipRest: vi.fn().mockResolvedValue(undefined),
  startRest: vi.fn().mockResolvedValue(undefined),
  getStateSnapshot: vi.fn(),
  getConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onStateChanged: vi.fn(),
  onConfigChanged: vi.fn(),
  emitNavigateTab: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/webviewWindow', () => ({
  WebviewWindow: {
    getByLabel: vi.fn().mockResolvedValue(null),
  },
}));

const { pauseTimer, resumeTimer, skipRest, startRest, getStateSnapshot, getConfig } =
  await import('$lib/commands');
const { onStateChanged, onConfigChanged, emitNavigateTab } = await import('$lib/events');
const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');

const { default: TrayApp } = await import('../TrayApp.svelte');

const DEFAULT_CONFIG = {
  timer: {
    work_minutes: 20,
    rest_seconds: 20,
    pre_alert_seconds: 15,
    alert_timeout_seconds: 60,
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
  // Re-prime default resolutions because mockRejectedValueOnce in prior tests
  // may have left the implementation queue in a partially-consumed state.
  vi.mocked(pauseTimer).mockResolvedValue(undefined);
  vi.mocked(resumeTimer).mockResolvedValue(undefined);
  vi.mocked(skipRest).mockResolvedValue(undefined);
  vi.mocked(startRest).mockResolvedValue(undefined);
  vi.mocked(emitNavigateTab).mockResolvedValue(undefined);
  vi.mocked(onStateChanged).mockResolvedValue(() => {});
  vi.mocked(onConfigChanged).mockResolvedValue(() => {});
  vi.mocked(getConfig).mockResolvedValue(DEFAULT_CONFIG);
});

afterEach(() => {
  cleanup();
});

describe('TrayApp smoke', () => {
  it('renders working state with pause button and time display', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    await waitFor(() => {
      expect(screen.getByText('10:00')).toBeInTheDocument();
    });

    expect(screen.getByText('工作中')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '暂停' })).toBeInTheDocument();
  });

  it('clicks pause and dispatches pauseTimer', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const pauseButton = await screen.findByRole('button', { name: '暂停' });
    await fireEvent.click(pauseButton);

    await waitFor(() => expect(pauseTimer).toHaveBeenCalledOnce());
  });

  it('clicks settings icon and emits navigate_tab', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const settingsButton = await screen.findByRole('button', { name: '设置' });
    await fireEvent.click(settingsButton);

    await waitFor(() => expect(emitNavigateTab).toHaveBeenCalledWith('Settings'));
  });

  it('renders resume button when state is paused', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'paused',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const resumeButton = await screen.findByRole('button', { name: '继续' });
    await fireEvent.click(resumeButton);

    await waitFor(() => expect(resumeTimer).toHaveBeenCalledOnce());
  });
});

describe('TrayApp state branches', () => {
  it('renders pre_alert state with pause button and amber label color', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'pre_alert',
      remaining_secs: 10,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    await waitFor(() => {
      expect(screen.getByText('即将休息')).toBeInTheDocument();
    });

    const pauseButton = await screen.findByRole('button', { name: '暂停' });
    await fireEvent.click(pauseButton);
    await waitFor(() => expect(pauseTimer).toHaveBeenCalledOnce());
  });

  it('renders alerting state with start-rest and skip buttons', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'alerting',
      remaining_secs: 30,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const startRestBtn = await screen.findByRole('button', { name: '开始休息' });
    await fireEvent.click(startRestBtn);
    await waitFor(() => expect(startRest).toHaveBeenCalledOnce());

    const skipBtn = await screen.findByRole('button', { name: '跳过' });
    await fireEvent.click(skipBtn);
    await waitFor(() => expect(skipRest).toHaveBeenCalledOnce());
  });

  it('renders resting state with skip button and settings icon', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'resting',
      remaining_secs: 10,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    await waitFor(() => {
      expect(screen.getByText('休息中')).toBeInTheDocument();
    });

    const skipBtn = await screen.findByRole('button', { name: '跳过' });
    await fireEvent.click(skipBtn);
    await waitFor(() => expect(skipRest).toHaveBeenCalledOnce());

    // Settings icon is also rendered in resting state
    const settingsBtn = await screen.findByRole('button', { name: '设置' });
    await fireEvent.click(settingsBtn);
    await waitFor(() => expect(emitNavigateTab).toHaveBeenCalledWith('Settings'));
  });

  it('renders pomodoro chip when mode is pomodoro', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 25,
      rest_seconds: 300,
      mode: 'pomodoro',
      pomodoro: {
        cycle_index: 2,
        cycles_per_long: 4,
        is_long_break: false,
      },
    });

    render(TrayApp);

    await waitFor(() => {
      expect(screen.getByText('Pomo 2/4')).toBeInTheDocument();
    });
  });
});

describe('TrayApp settings navigation', () => {
  it('shows + focuses the main window when WebviewWindow.getByLabel resolves', async () => {
    const show = vi.fn().mockResolvedValue(undefined);
    const unminimize = vi.fn().mockResolvedValue(undefined);
    const setFocus = vi.fn().mockResolvedValue(undefined);
    vi.mocked(WebviewWindow.getByLabel).mockResolvedValueOnce({
      show,
      unminimize,
      setFocus,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
    } as any);

    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const settingsBtn = await screen.findByRole('button', { name: '设置' });
    await fireEvent.click(settingsBtn);

    await waitFor(() => {
      expect(unminimize).toHaveBeenCalled();
      expect(show).toHaveBeenCalled();
      expect(setFocus).toHaveBeenCalled();
    });
  });

  it('logs an error when emitNavigateTab fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(emitNavigateTab).mockRejectedValueOnce(new Error('emit failed'));

    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const settingsBtn = await screen.findByRole('button', { name: '设置' });
    await fireEvent.click(settingsBtn);

    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to open settings:', expect.any(Error));
    });
    consoleError.mockRestore();
  });
});

describe('TrayApp command error paths', () => {
  it('logs an error when pauseTimer rejects', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(pauseTimer).mockRejectedValueOnce(new Error('pause failed'));
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const pauseBtn = await screen.findByRole('button', { name: '暂停' });
    await fireEvent.click(pauseBtn);
    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to pause timer:', expect.any(Error));
    });
    consoleError.mockRestore();
  });

  it('logs an error when resumeTimer rejects', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(resumeTimer).mockRejectedValueOnce(new Error('resume failed'));
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'paused',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const resumeBtn = await screen.findByRole('button', { name: '继续' });
    await fireEvent.click(resumeBtn);
    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to resume timer:', expect.any(Error));
    });
    consoleError.mockRestore();
  });

  it('logs an error when startRest rejects from alerting', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(startRest).mockRejectedValueOnce(new Error('start failed'));
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'alerting',
      remaining_secs: 30,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TrayApp);

    const startBtn = await screen.findByRole('button', { name: '开始休息' });
    await fireEvent.click(startBtn);
    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to start rest:', expect.any(Error));
    });
    consoleError.mockRestore();
  });

  it('logs an error when skipRest rejects from resting', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'resting',
      remaining_secs: 10,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(skipRest).mockImplementationOnce(() => Promise.reject(new Error('skip failed')));

    render(TrayApp);

    // Wait until the resting view is rendered.
    await screen.findByText('休息中');
    const skipBtn = document.querySelector('.btn.neutral') as HTMLButtonElement | null;
    expect(skipBtn).toBeTruthy();
    await fireEvent.click(skipBtn!);
    await waitFor(() => expect(skipRest).toHaveBeenCalled());
    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to skip rest:', expect.any(Error));
    });
    consoleError.mockRestore();
  });
});
