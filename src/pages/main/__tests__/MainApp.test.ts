import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    setTheme: vi.fn().mockResolvedValue(undefined),
  })),
}));

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn().mockResolvedValue(false),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

vi.mock('$lib/commands', () => ({
  getConfig: vi.fn(),
  getDetectorCapabilities: vi.fn().mockResolvedValue({
    afk_detection_supported: true,
    foreground_process_detection_supported: true,
  }),
  getHotkeyStatus: vi.fn().mockResolvedValue({
    bindings: [],
    macos_accessibility: 'not_required',
    last_error: null,
  }),
  getStatisticsTrends: vi.fn().mockResolvedValue({
    daily: [],
    weekly: [],
    monthly: [],
    total_rest_secs: 0,
    ribbon: [],
    suppressed: [],
  }),
  getStatisticsCycleOutcomes: vi.fn(),
  exportStatistics: vi.fn(),
  updateTimerConfig: vi.fn(),
  updateBehaviorConfig: vi.fn(),
  updateDisplayConfig: vi.fn(),
  updateScheduleConfig: vi.fn(),
  updateHotkeysConfig: vi.fn(),
  updatePomodoroConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onConfigChanged: vi.fn().mockResolvedValue(() => {}),
  onNavigateTab: vi.fn().mockResolvedValue(() => {}),
  onHotkeyStatusChanged: vi.fn().mockResolvedValue(() => {}),
}));

const { getConfig } = await import('$lib/commands');

const { default: MainApp } = await import('../MainApp.svelte');

beforeEach(() => {
  vi.clearAllMocks();
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

describe('MainApp smoke', () => {
  it('mounts and shows the three top-level tabs', async () => {
    render(MainApp);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /设置/ })).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: /统计/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /关于/ })).toBeInTheDocument();
  });

  it('switches to About tab when clicked and shows GitHub link button', async () => {
    render(MainApp);

    const aboutTab = await screen.findByRole('button', { name: /关于/ });
    await fireEvent.click(aboutTab);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'GitHub' })).toBeInTheDocument();
    });
  });
});
