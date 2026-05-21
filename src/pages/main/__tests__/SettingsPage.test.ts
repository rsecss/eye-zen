import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn().mockResolvedValue(false),
}));

vi.mock('$lib/commands', () => ({
  getDetectorCapabilities: vi.fn(),
  updateTimerConfig: vi.fn(),
  updateBehaviorConfig: vi.fn().mockResolvedValue(undefined),
  updateDisplayConfig: vi.fn(),
  updateScheduleConfig: vi.fn(),
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
