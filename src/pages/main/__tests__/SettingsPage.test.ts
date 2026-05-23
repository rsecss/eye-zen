import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn().mockResolvedValue(false),
}));

vi.mock('$lib/commands', () => ({
  getDetectorCapabilities: vi.fn(),
  getHotkeyStatus: vi.fn().mockResolvedValue({
    bindings: [],
    macos_accessibility: 'not_required',
    last_error: null,
  }),
  updateTimerConfig: vi.fn(),
  updateBehaviorConfig: vi.fn().mockResolvedValue(undefined),
  updateDisplayConfig: vi.fn(),
  updateScheduleConfig: vi.fn(),
  updateHotkeysConfig: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onHotkeyStatusChanged: vi.fn().mockResolvedValue(() => {}),
}));

const { getDetectorCapabilities, updateBehaviorConfig } = await import('$lib/commands');
const { default: SettingsPage } = await import('../SettingsPage.svelte');

describe('SettingsPage AFK controls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateBehaviorConfig).mockResolvedValue(undefined);
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
    );
  });
});

describe('SettingsPage process whitelist', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(updateBehaviorConfig).mockResolvedValue(undefined);
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
    vi.mocked(updateBehaviorConfig).mockResolvedValue(undefined);
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
