import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/plugin-autostart', () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn().mockResolvedValue(false),
}));

vi.mock('$lib/commands', () => ({
  getDetectorCapabilities: vi.fn().mockResolvedValue({
    afk_detection_supported: true,
    foreground_process_detection_supported: true,
    fullscreen_detection_supported: true,
  }),
  getHotkeyStatus: vi.fn().mockResolvedValue({
    bindings: [],
    macos_accessibility: 'not_required',
    last_error: null,
  }),
  updateTimerConfig: vi.fn().mockResolvedValue(undefined),
  updateBehaviorConfig: vi.fn().mockResolvedValue(undefined),
  updateDisplayConfig: vi.fn().mockResolvedValue(undefined),
  updateScheduleConfig: vi.fn().mockResolvedValue(undefined),
  updateHotkeysConfig: vi.fn().mockResolvedValue(undefined),
  updatePomodoroConfig: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$lib/events', () => ({
  onHotkeyStatusChanged: vi.fn().mockResolvedValue(() => {}),
}));

const {
  updateTimerConfig,
  updateBehaviorConfig,
  updateDisplayConfig,
  updateScheduleConfig,
  updateHotkeysConfig,
  updatePomodoroConfig,
} = await import('$lib/commands');
const { enable, disable } = await import('@tauri-apps/plugin-autostart');
const { default: SettingsPage } = await import('../SettingsPage.svelte');

beforeEach(() => {
  vi.clearAllMocks();
});

describe('SettingsPage timer handlers', () => {
  it('updates work minutes via stepper increment', async () => {
    render(SettingsPage);

    const increase = await screen.findByLabelText('Increase 工作时间');
    await fireEvent.click(increase);

    await waitFor(() =>
      expect(updateTimerConfig).toHaveBeenCalledWith(expect.objectContaining({ work_minutes: 21 })),
    );
  });

  it('updates rest seconds via stepper', async () => {
    render(SettingsPage);

    const increase = await screen.findByLabelText('Increase 休息时间');
    await fireEvent.click(increase);

    await waitFor(() =>
      expect(updateTimerConfig).toHaveBeenCalledWith(expect.objectContaining({ rest_seconds: 25 })),
    );
  });

  it('updates pre-alert seconds via stepper', async () => {
    render(SettingsPage);

    const increase = await screen.findByLabelText('Increase 预提醒');
    await fireEvent.click(increase);

    await waitFor(() =>
      expect(updateTimerConfig).toHaveBeenCalledWith(
        expect.objectContaining({ pre_alert_seconds: 20 }),
      ),
    );
  });

  it('updates alert timeout via stepper', async () => {
    render(SettingsPage);

    const increase = await screen.findByLabelText('Increase 提醒超时');
    await fireEvent.click(increase);

    await waitFor(() =>
      expect(updateTimerConfig).toHaveBeenCalledWith(
        expect.objectContaining({ alert_timeout_seconds: 70 }),
      ),
    );
  });

  it('switches to pomodoro mode via select', async () => {
    render(SettingsPage);

    const select = await screen.findByLabelText('计时模式');
    await fireEvent.change(select, { target: { value: 'pomodoro' } });

    await waitFor(() =>
      expect(updateTimerConfig).toHaveBeenCalledWith(expect.objectContaining({ mode: 'pomodoro' })),
    );
  });
});

describe('SettingsPage pomodoro handlers', () => {
  it('updatePomodoroConfig is exported and callable', () => {
    expect(updatePomodoroConfig).toBeDefined();
    expect(typeof updatePomodoroConfig).toBe('function');
  });
});

describe('SettingsPage behavior handlers', () => {
  it('toggles sound and dispatches updateBehaviorConfig', async () => {
    render(SettingsPage);

    const toggle = await screen.findByLabelText('提示音');
    await fireEvent.click(toggle);

    await waitFor(() =>
      expect(updateBehaviorConfig).toHaveBeenCalledWith(
        expect.objectContaining({ sound_enabled: false }),
      ),
    );
  });

  it('toggles fullscreen skip and dispatches updateBehaviorConfig', async () => {
    render(SettingsPage);

    const toggle = await screen.findByLabelText('全屏跳过');
    await fireEvent.click(toggle);

    await waitFor(() =>
      expect(updateBehaviorConfig).toHaveBeenCalledWith(
        expect.objectContaining({ fullscreen_skip: false }),
      ),
    );
  });

  it('toggles auto-start ON: calls plugin enable then updateBehaviorConfig', async () => {
    vi.mocked(enable).mockResolvedValue(undefined);

    render(SettingsPage);

    const toggle = await screen.findByLabelText('开机启动');
    await fireEvent.click(toggle);

    await waitFor(() => expect(enable).toHaveBeenCalledOnce());
    await waitFor(() =>
      expect(updateBehaviorConfig).toHaveBeenCalledWith(
        expect.objectContaining({ auto_start: true }),
      ),
    );
  });

  it('toggles auto-start ON: rolls back via disable when updateBehaviorConfig fails', async () => {
    vi.mocked(enable).mockResolvedValue(undefined);
    vi.mocked(disable).mockResolvedValue(undefined);
    vi.mocked(updateBehaviorConfig).mockRejectedValueOnce(new Error('save failed'));

    render(SettingsPage);

    const toggle = await screen.findByLabelText('开机启动');
    await fireEvent.click(toggle);

    await waitFor(() => expect(enable).toHaveBeenCalledOnce());
    await waitFor(() => expect(disable).toHaveBeenCalledOnce());
  });

  it('toggles auto-start ON: aborts when plugin enable rejects', async () => {
    vi.mocked(enable).mockRejectedValueOnce(new Error('plugin error'));

    render(SettingsPage);

    const toggle = await screen.findByLabelText('开机启动');
    await fireEvent.click(toggle);

    await waitFor(() => expect(enable).toHaveBeenCalledOnce());
    // The config update MUST NOT fire when the plugin call fails.
    expect(updateBehaviorConfig).not.toHaveBeenCalledWith(
      expect.objectContaining({ auto_start: true }),
    );
  });

  it('toggles auto-start ON: surfaces error banner when both save and revert fail', async () => {
    vi.mocked(enable).mockResolvedValue(undefined);
    vi.mocked(disable).mockRejectedValueOnce(new Error('revert failed'));
    vi.mocked(updateBehaviorConfig).mockRejectedValueOnce(new Error('save failed'));

    render(SettingsPage);

    const toggle = await screen.findByLabelText('开机启动');
    await fireEvent.click(toggle);

    // OS toggle succeeded, save rejected, revert (disable) rejected → user-facing alert MUST appear.
    const banner = await screen.findByRole('alert');
    expect(banner.textContent).toContain('save failed');
  });
});

describe('SettingsPage display handlers', () => {
  it('changes language via select', async () => {
    render(SettingsPage);

    const select = await screen.findByLabelText('语言');
    await fireEvent.change(select, { target: { value: 'en' } });

    await waitFor(() =>
      expect(updateDisplayConfig).toHaveBeenCalledWith(expect.objectContaining({ language: 'en' })),
    );
  });

  it('changes theme via select', async () => {
    render(SettingsPage);

    const select = await screen.findByLabelText('主题');
    await fireEvent.change(select, { target: { value: 'dark' } });

    await waitFor(() =>
      expect(updateDisplayConfig).toHaveBeenCalledWith(expect.objectContaining({ theme: 'dark' })),
    );
  });
});

describe('SettingsPage schedule handlers', () => {
  it('toggles schedule enabled', async () => {
    render(SettingsPage);

    const toggle = await screen.findByLabelText('启用工作日调度');
    await fireEvent.click(toggle);

    await waitFor(() =>
      expect(updateScheduleConfig).toHaveBeenCalledWith(expect.objectContaining({ enabled: true })),
    );
  });
});

describe('SettingsPage hotkey handlers', () => {
  it('rejects empty hotkey value', async () => {
    render(SettingsPage);

    const input = await screen.findByLabelText('手动休息');
    await fireEvent.change(input, { target: { value: '   ' } });

    // Empty input must NOT call updateHotkeysConfig.
    expect(updateHotkeysConfig).not.toHaveBeenCalled();
  });

  it('updates start_rest hotkey with prettified value', async () => {
    render(SettingsPage);

    const input = await screen.findByLabelText('手动休息');
    await fireEvent.change(input, { target: { value: 'Cmd/Ctrl+Alt+R' } });

    await waitFor(() =>
      expect(updateHotkeysConfig).toHaveBeenCalledWith(
        expect.objectContaining({ start_rest: 'CommandOrControl+Alt+R' }),
      ),
    );
  });

  it('skips update when hotkey value is unchanged', async () => {
    render(SettingsPage);

    const input = await screen.findByLabelText('手动休息');
    // Existing default is CommandOrControl+Alt+B → displayed as Cmd/Ctrl+Alt+B.
    await fireEvent.change(input, { target: { value: 'Cmd/Ctrl+Alt+B' } });

    expect(updateHotkeysConfig).not.toHaveBeenCalled();
  });
});
