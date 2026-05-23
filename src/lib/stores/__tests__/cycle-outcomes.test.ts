import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/commands', () => ({
  getStatisticsCycleOutcomes: vi.fn(),
}));

const { getStatisticsCycleOutcomes } = await import('$lib/commands');
const { cycleOutcomesStore } = await import('../cycle-outcomes.svelte');

const SAMPLE_PAYLOAD = {
  total_cycles: 3,
  completed_cycles: 2,
  skipped_cycles: 1,
  recent: [],
  generated_at: '2026-05-23T00:00:00Z',
  timezone: 'UTC',
  // Newer fields the binding may include; keep loose so the test doesn't
  // become brittle to non-load-bearing schema growth.
};

beforeEach(() => {
  vi.clearAllMocks();
  cycleOutcomesStore.reset();
});

afterEach(() => {
  cycleOutcomesStore.reset();
});

describe('cycleOutcomesStore', () => {
  it('exposes nullish initial state', () => {
    expect(cycleOutcomesStore.current).toBeNull();
    expect(cycleOutcomesStore.loading).toBe(false);
    expect(cycleOutcomesStore.error).toBeNull();
  });

  it('loads cycle outcomes through getStatisticsCycleOutcomes', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValueOnce(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      SAMPLE_PAYLOAD as any,
    );

    await cycleOutcomesStore.refresh();

    expect(getStatisticsCycleOutcomes).toHaveBeenCalledOnce();
    expect(cycleOutcomesStore.current).toEqual(SAMPLE_PAYLOAD);
    expect(cycleOutcomesStore.loading).toBe(false);
    expect(cycleOutcomesStore.error).toBeNull();
  });

  it('captures Error.message into store.error and logs the rejection', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(getStatisticsCycleOutcomes).mockRejectedValueOnce(new Error('boom'));

    await cycleOutcomesStore.refresh();

    expect(cycleOutcomesStore.error).toBe('boom');
    expect(cycleOutcomesStore.current).toBeNull();
    expect(cycleOutcomesStore.loading).toBe(false);
    expect(consoleError).toHaveBeenCalledWith('Failed to load cycle outcomes:', expect.any(Error));
    consoleError.mockRestore();
  });

  it('stringifies non-Error rejections into store.error', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(getStatisticsCycleOutcomes).mockRejectedValueOnce('plain string failure');

    await cycleOutcomesStore.refresh();

    expect(cycleOutcomesStore.error).toBe('plain string failure');
    consoleError.mockRestore();
  });

  it('reset() returns the store to a pristine state', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValueOnce(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      SAMPLE_PAYLOAD as any,
    );
    await cycleOutcomesStore.refresh();
    expect(cycleOutcomesStore.current).not.toBeNull();

    cycleOutcomesStore.reset();

    expect(cycleOutcomesStore.current).toBeNull();
    expect(cycleOutcomesStore.loading).toBe(false);
    expect(cycleOutcomesStore.error).toBeNull();
  });

  it('drops stale responses when a newer refresh has been issued', async () => {
    let resolveFirst: (value: unknown) => void = () => {};
    const firstPending = new Promise((resolve) => {
      resolveFirst = resolve;
    });

    vi.mocked(getStatisticsCycleOutcomes).mockImplementationOnce(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      () => firstPending as any,
    );
    vi.mocked(getStatisticsCycleOutcomes).mockImplementationOnce(() =>
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      Promise.resolve({ ...SAMPLE_PAYLOAD, total_cycles: 99 } as any),
    );

    const first = cycleOutcomesStore.refresh();
    // start a second refresh before resolving the first
    const second = cycleOutcomesStore.refresh();
    resolveFirst({ ...SAMPLE_PAYLOAD, total_cycles: 1 });

    await Promise.all([first, second]);

    // newer refresh wins regardless of order of resolution
    expect(cycleOutcomesStore.current).toEqual(expect.objectContaining({ total_cycles: 99 }));
  });
});
