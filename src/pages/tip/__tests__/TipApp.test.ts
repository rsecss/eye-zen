import { render, screen, fireEvent, waitFor, cleanup } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  startRest: vi.fn().mockResolvedValue(undefined),
  skipRest: vi.fn().mockResolvedValue(undefined),
  getStateSnapshot: vi.fn(),
  getConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onStateChanged: vi.fn(),
  onConfigChanged: vi.fn(),
}));

const { startRest, skipRest, getStateSnapshot, getConfig } = await import('$lib/commands');
const { onStateChanged, onConfigChanged } = await import('$lib/events');

const { default: TipApp } = await import('../TipApp.svelte');

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
  vi.mocked(getStateSnapshot).mockResolvedValue({
    state: 'alerting',
    remaining_secs: 60,
    work_minutes: 20,
    rest_seconds: 20,
    mode: 'twenty_twenty_twenty',
    pomodoro: null,
  });
  vi.mocked(getConfig).mockResolvedValue(DEFAULT_CONFIG);
});

describe('TipApp smoke', () => {
  it('mounts without throwing and renders the alerting card when state is alerting', async () => {
    render(TipApp);

    await waitFor(() => {
      expect(screen.getByText('该让眼睛休息一下了')).toBeInTheDocument();
    });
  });

  it('clicks the start-rest button and dispatches startRest', async () => {
    render(TipApp);

    const startButton = await screen.findByRole('button', { name: '开始休息' });
    await fireEvent.click(startButton);

    await waitFor(() => expect(startRest).toHaveBeenCalledOnce());
  });

  it('clicks the skip button and dispatches skipRest', async () => {
    render(TipApp);

    const skipButton = await screen.findByRole('button', { name: '跳过' });
    await fireEvent.click(skipButton);

    await waitFor(() => expect(skipRest).toHaveBeenCalledOnce());
  });
});

describe('TipApp resting view', () => {
  afterEach(() => cleanup());

  it('renders the resting view with i18n title and skip button when state is resting', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValueOnce({
      state: 'resting',
      remaining_secs: 15,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TipApp);

    await screen.findByText('休息中');

    const skipButton = await screen.findByRole('button', { name: '跳过' });
    await fireEvent.click(skipButton);
    await waitFor(() => expect(skipRest).toHaveBeenCalled());
  });

  it('renders the fallback view when state is not alerting or resting', async () => {
    vi.mocked(getStateSnapshot).mockResolvedValueOnce({
      state: 'working',
      remaining_secs: 600,
      work_minutes: 20,
      rest_seconds: 20,
      mode: 'twenty_twenty_twenty',
      pomodoro: null,
    });

    render(TipApp);

    await screen.findByText('等待中...');
  });
});

describe('TipApp pomodoro short-break view', () => {
  afterEach(() => cleanup());

  it('renders pomodoro short-break title and subtitle when resting with short break', async () => {
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

    render(TipApp);

    // Short break i18n: title contains {current}/{total} substitution.
    await waitFor(() => {
      const card = document.querySelector('.tip-card');
      expect(card?.textContent ?? '').toMatch(/2.+4/);
    });
  });

  it('renders pomodoro long-break title when is_long_break is true', async () => {
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

    render(TipApp);

    await waitFor(() => {
      expect(screen.getByText('长休时间')).toBeInTheDocument();
    });
  });
});

describe('TipApp command error paths', () => {
  afterEach(() => cleanup());

  it('logs an error when startRest rejects', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(startRest).mockImplementationOnce(() => Promise.reject(new Error('start failed')));

    render(TipApp);

    const startBtn = await screen.findByRole('button', { name: '开始休息' });
    await fireEvent.click(startBtn);

    await waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith('Failed to start rest:', expect.any(Error)),
    );
    consoleError.mockRestore();
  });

  it('logs an error when skipRest rejects', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(skipRest).mockImplementationOnce(() => Promise.reject(new Error('skip failed')));

    render(TipApp);

    const skipBtn = await screen.findByRole('button', { name: '跳过' });
    await fireEvent.click(skipBtn);

    await waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith('Failed to skip rest:', expect.any(Error)),
    );
    consoleError.mockRestore();
  });
});
