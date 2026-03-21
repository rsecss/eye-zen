import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock IPC modules before importing the store
vi.mock('$lib/commands', () => ({
  getStateSnapshot: vi.fn(),
}));

vi.mock('$lib/events', () => ({
  onStateChanged: vi.fn(),
}));

// Dynamic import after mocks are set up
const { getStateSnapshot } = await import('$lib/commands');
const { onStateChanged } = await import('$lib/events');

describe('timerStore', () => {
  let unlistenMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.clearAllMocks();
    unlistenMock = vi.fn();
  });

  it('init fetches snapshot and subscribes to events', async () => {
    const mockState = {
      state: 'working',
      remaining_secs: 1200,
      work_minutes: 20,
      rest_seconds: 20,
    };

    vi.mocked(onStateChanged).mockResolvedValue(unlistenMock);
    vi.mocked(getStateSnapshot).mockResolvedValue(mockState);

    const { timerStore } = await import('$lib/stores/timer.svelte');

    await timerStore.init();

    expect(onStateChanged).toHaveBeenCalledOnce();
    expect(getStateSnapshot).toHaveBeenCalledOnce();
  });

  it('event callback updates timerStore.current', async () => {
    let capturedCallback: ((payload: unknown) => void) | null = null;

    vi.mocked(onStateChanged).mockImplementation((cb) => {
      capturedCallback = cb as (payload: unknown) => void;
      return Promise.resolve(unlistenMock);
    });

    const updatedState = {
      state: 'resting',
      remaining_secs: 20,
      work_minutes: 20,
      rest_seconds: 20,
    };

    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 1200,
      work_minutes: 20,
      rest_seconds: 20,
    });

    const { timerStore } = await import('$lib/stores/timer.svelte');

    await timerStore.init();

    expect(capturedCallback).not.toBeNull();
    // Trigger the event callback
    capturedCallback!(updatedState);

    expect(timerStore.current.state).toBe('resting');
    expect(timerStore.current.remaining_secs).toBe(20);
  });

  it('destroy calls unlisten', async () => {
    vi.mocked(onStateChanged).mockResolvedValue(unlistenMock);
    vi.mocked(getStateSnapshot).mockResolvedValue({
      state: 'working',
      remaining_secs: 0,
      work_minutes: 20,
      rest_seconds: 20,
    });

    const { timerStore } = await import('$lib/stores/timer.svelte');

    await timerStore.init();
    timerStore.destroy();

    expect(unlistenMock).toHaveBeenCalledOnce();
  });
});
