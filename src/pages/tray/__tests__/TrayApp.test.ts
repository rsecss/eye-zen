import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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

const { pauseTimer, resumeTimer, getStateSnapshot, getConfig } = await import('$lib/commands');
const { onStateChanged, onConfigChanged, emitNavigateTab } = await import('$lib/events');

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
  vi.mocked(onStateChanged).mockResolvedValue(() => {});
  vi.mocked(onConfigChanged).mockResolvedValue(() => {});
  vi.mocked(getConfig).mockResolvedValue(DEFAULT_CONFIG);
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
