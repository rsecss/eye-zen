import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock the Tauri event surface before importing the wrappers.
vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn(),
  listen: vi.fn(),
}));

const { emit, listen } = await import('@tauri-apps/api/event');
const mockEmit = vi.mocked(emit);
const mockListen = vi.mocked(listen);

const events = await import('../events');

beforeEach(() => {
  mockEmit.mockReset();
  mockListen.mockReset();
});

describe('typed event listeners forward to listen with the documented event name', () => {
  it('onStateChanged subscribes to "state_changed" and unwraps payload', async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementation((_name, _handler) => Promise.resolve(unlisten));

    const cb = vi.fn();
    const returned = await events.onStateChanged(cb);

    expect(mockListen).toHaveBeenCalledOnce();
    expect(mockListen.mock.calls[0][0]).toBe('state_changed');
    expect(returned).toBe(unlisten);

    // Verify the registered handler unwraps the .payload field.
    const innerHandler = mockListen.mock.calls[0][1];
    innerHandler({ event: 'state_changed', id: 1, payload: { state: 'working' } });
    expect(cb).toHaveBeenCalledWith({ state: 'working' });
  });

  it('onConfigChanged subscribes to "config_changed" and unwraps payload', async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementation((_name, _handler) => Promise.resolve(unlisten));

    const cb = vi.fn();
    const returned = await events.onConfigChanged(cb);

    expect(mockListen.mock.calls[0][0]).toBe('config_changed');
    expect(returned).toBe(unlisten);

    const innerHandler = mockListen.mock.calls[0][1];
    innerHandler({ event: 'config_changed', id: 2, payload: { display: { theme: 'dark' } } });
    expect(cb).toHaveBeenCalledWith({ display: { theme: 'dark' } });
  });

  it('onHotkeyStatusChanged subscribes to "hotkey_status_changed" and unwraps payload', async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementation((_name, _handler) => Promise.resolve(unlisten));

    const cb = vi.fn();
    const returned = await events.onHotkeyStatusChanged(cb);

    expect(mockListen.mock.calls[0][0]).toBe('hotkey_status_changed');
    expect(returned).toBe(unlisten);

    const innerHandler = mockListen.mock.calls[0][1];
    innerHandler({
      event: 'hotkey_status_changed',
      id: 3,
      payload: { bindings: [], macos_accessibility: 'not_required', last_error: null },
    });
    expect(cb).toHaveBeenCalledWith({
      bindings: [],
      macos_accessibility: 'not_required',
      last_error: null,
    });
  });

  it('onNavigateTab subscribes to "navigate_tab" and forwards string payload', async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementation((_name, _handler) => Promise.resolve(unlisten));

    const cb = vi.fn();
    const returned = await events.onNavigateTab(cb);

    expect(mockListen.mock.calls[0][0]).toBe('navigate_tab');
    expect(returned).toBe(unlisten);

    const innerHandler = mockListen.mock.calls[0][1];
    innerHandler({ event: 'navigate_tab', id: 4, payload: 'Settings' });
    expect(cb).toHaveBeenCalledWith('Settings');
  });

  it('onStatPersistenceError subscribes to "stat_persistence_error" and unwraps payload', async () => {
    const unlisten = vi.fn();
    mockListen.mockImplementation((_name, _handler) => Promise.resolve(unlisten));

    const cb = vi.fn();
    const returned = await events.onStatPersistenceError(cb);

    expect(mockListen.mock.calls[0][0]).toBe('stat_persistence_error');
    expect(returned).toBe(unlisten);

    const innerHandler = mockListen.mock.calls[0][1];
    const payload = {
      kind: 'queue_overflow' as const,
      occurred_at: '2026-05-23T10:00:00Z',
      message: 'stat writer queue full (256 pending drafts)',
    };
    innerHandler({ event: 'stat_persistence_error', id: 5, payload });
    expect(cb).toHaveBeenCalledWith(payload);
  });
});

describe('emit helpers', () => {
  it('emitNavigateTab forwards the tab name via emit', async () => {
    mockEmit.mockResolvedValue(undefined);
    await events.emitNavigateTab('Statistics');
    expect(mockEmit).toHaveBeenCalledWith('navigate_tab', 'Statistics');
  });
});
