import { render, screen, waitFor, cleanup } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  getStateSnapshot: vi.fn(),
  getConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onStateChanged: vi.fn(),
  onConfigChanged: vi.fn(),
}));

const { getStateSnapshot, getConfig } = await import('$lib/commands');
const { onStateChanged, onConfigChanged } = await import('$lib/events');

const { default: TipMinimalApp } = await import('../TipMinimalApp.svelte');

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(onStateChanged).mockResolvedValue(() => {});
  vi.mocked(onConfigChanged).mockResolvedValue(() => {});
  vi.mocked(getStateSnapshot).mockResolvedValue({
    state: 'resting',
    remaining_secs: 10,
    work_minutes: 20,
    rest_seconds: 20,
    mode: 'twenty_twenty_twenty',
    pomodoro: null,
  });
  vi.mocked(getConfig).mockResolvedValue({
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
      afk_skip_enabled: false,
      afk_threshold_minutes: 5,
      auto_start: false,
      process_whitelist_enabled: false,
      process_whitelist: [],
    },
    display: { language: 'zh-CN', theme: 'light' },
    schedule: { enabled: false, active_days: [true, true, true, true, true, false, false] },
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
  });
});

describe('TipMinimalApp smoke', () => {
  afterEach(() => cleanup());

  it('mounts without throwing and renders the resting label when state is resting', async () => {
    render(TipMinimalApp);

    await waitFor(() => {
      expect(screen.getByText('休息中... 请看向远处')).toBeInTheDocument();
    });
  });

  it('renders the alerting label when state is alerting', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValueOnce({
      state: 'alerting',
      remaining_secs: 30,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TipMinimalApp);

    await waitFor(() => {
      expect(screen.getByText('休息一下... 请看向远处')).toBeInTheDocument();
    });
  });

  it('renders pomodoro short-break label when resting in pomodoro mode with short break', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValueOnce({
      state: 'resting',
      remaining_secs: 60,
      work_minutes: 25,
      rest_seconds: 300,
      mode: 'pomodoro',
      pomodoro: {
        cycle_index: 2,
        cycles_per_long: 4,
        is_long_break: false,
      },
    });

    render(TipMinimalApp);

    await waitFor(() => {
      expect(screen.getByText(/短休 2\/4/)).toBeInTheDocument();
    });
  });

  it('renders pomodoro long-break label when is_long_break is true', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValueOnce({
      state: 'resting',
      remaining_secs: 600,
      work_minutes: 25,
      rest_seconds: 900,
      mode: 'pomodoro',
      pomodoro: {
        cycle_index: 4,
        cycles_per_long: 4,
        is_long_break: true,
      },
    });

    render(TipMinimalApp);

    await waitFor(() => {
      expect(screen.getByText(/长休时间/)).toBeInTheDocument();
    });
  });
});
