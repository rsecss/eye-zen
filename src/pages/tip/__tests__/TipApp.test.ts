import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

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
