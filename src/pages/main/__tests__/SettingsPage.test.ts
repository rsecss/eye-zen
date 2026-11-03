import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn().mockResolvedValue(false),
}));

vi.mock('$lib/commands', () => ({
  getConfig: vi.fn(),
  getDetectorCapabilities: vi.fn(),
  getHotkeyStatus: vi.fn().mockResolvedValue({
    bindings: [],
    macos_accessibility: 'not_required',
    last_error: null,
  }),
  updateTimerConfig: vi.fn(),
  updateBehaviorConfig: vi.fn(),
  updateDisplayConfig: vi.fn(),
  updateScheduleConfig: vi.fn(),
  updateHotkeysConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onConfigChanged: vi.fn().mockResolvedValue(() => {}),
  onHotkeyStatusChanged: vi.fn().mockResolvedValue(() => {}),
}));

const { getConfig, getDetectorCapabilities, updateBehaviorConfig } = await import('$lib/commands');
const { configStore } = await import('$lib/stores/config.svelte');
const { default: SettingsPage } = await import('../SettingsPage.svelte');

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
    afk_skip_enabled: true,
    afk_threshold_minutes: 5,
    auto_start: false,
    process_whitelist_enabled: false,
    process_whitelist: [],
  },
  display: { language: 'zh-CN', theme: 'light' },
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

beforeEach(async () => {
  vi.mocked(getConfig).mockResolvedValue({ revision: 0, config: DEFAULT_CONFIG });
  await configStore.init();
});

describe('SettingsPage AFK controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateBehaviorConfig).mockImplementation((behavior, expectedRevision) =>
      Promise.resolve({
        revision: expectedRevision + 1,
        config: { ...DEFAULT_CONFIG, behavior },
      }),
    );
  });

  it('disables AFK controls when detector capabilities are unavailable', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: false,
      foreground_process_detection_supported: false,
      fullscreen_detection_supported: false,
    });

    render(SettingsPage);

    await waitFor(() => expect(getDetectorCapabilities).toHaveBeenCalledOnce());

    expect(screen.getByLabelText('离开跳过')).toBeDisabled();
    expect(screen.getByLabelText('Increase 离开阈值')).toBeDisabled();
    expect(screen.getAllByText('当前会话不支持键鼠空闲检测，已禁用')).toHaveLength(2);
  });

  it('updates behavior config when AFK threshold changes', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: true,
      fullscreen_detection_supported: true,
    });

    render(SettingsPage);

    const increase = screen.getByLabelText('Increase 离开阈值');
    await waitFor(() => expect(increase).not.toBeDisabled());
    await fireEvent.click(increase);

    expect(updateBehaviorConfig).toHaveBeenCalledWith(
      expect.objectContaining({
        afk_skip_enabled: true,
        afk_threshold_minutes: 6,
      }),
      0,
    );
  });
});

describe('SettingsPage process whitelist', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateBehaviorConfig).mockImplementation((behavior, expectedRevision) =>
      Promise.resolve({
        revision: expectedRevision + 1,
        config: { ...DEFAULT_CONFIG, behavior },
      }),
    );
  });

  it('disables whitelist controls when foreground process detection is unsupported', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: false,
      fullscreen_detection_supported: true,
    });

    render(SettingsPage);

    await waitFor(() => expect(getDetectorCapabilities).toHaveBeenCalledOnce());

    expect(screen.getByLabelText('启用白名单')).toBeDisabled();
    expect(screen.getByText('当前会话不支持前台进程检测，已禁用')).toBeInTheDocument();
  });

  it('shows empty state when whitelist is empty', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: true,
      fullscreen_detection_supported: true,
    });

    render(SettingsPage);

    await waitFor(() => expect(getDetectorCapabilities).toHaveBeenCalledOnce());

    expect(screen.getByText('暂无白名单进程')).toBeInTheDocument();
  });
});

describe('SettingsPage fullscreen detection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateBehaviorConfig).mockImplementation((behavior, expectedRevision) =>
      Promise.resolve({
        revision: expectedRevision + 1,
        config: { ...DEFAULT_CONFIG, behavior },
      }),
    );
  });

  it('disables the fullscreen skip toggle when capability is unavailable', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: true,
      fullscreen_detection_supported: false,
    });

    render(SettingsPage);

    await waitFor(() => expect(getDetectorCapabilities).toHaveBeenCalledOnce());

    expect(screen.getByLabelText('全屏跳过')).toBeDisabled();
    expect(screen.getByText('当前平台暂不支持全屏检测，将于 v0.7.x 版本提供')).toBeInTheDocument();
  });

  it('keeps the fullscreen skip toggle enabled when capability is available', async () => {
    vi.mocked(getDetectorCapabilities).mockResolvedValue({
      afk_detection_supported: true,
      foreground_process_detection_supported: true,
      fullscreen_detection_supported: true,
    });

    render(SettingsPage);

    await waitFor(() => expect(getDetectorCapabilities).toHaveBeenCalledOnce());

    expect(screen.getByLabelText('全屏跳过')).not.toBeDisabled();
    expect(screen.getByText('全屏应用时跳过提醒')).toBeInTheDocument();
  });
});
