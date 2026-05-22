import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
import type { CycleOutcomesPayload } from '$lib/bindings/CycleOutcomesPayload';
import StatisticsPage from '../StatisticsPage.svelte';

const echartsMock = vi.hoisted(() => {
  const chart = {
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  };
  return {
    chart,
    init: vi.fn(() => chart),
    use: vi.fn(),
  };
});

vi.mock('echarts/core', () => ({
  init: echartsMock.init,
  use: echartsMock.use,
}));

vi.mock('echarts/charts', () => ({
  BarChart: {},
  LineChart: {},
}));

vi.mock('echarts/components', () => ({
  GridComponent: {},
  LegendComponent: {},
  TooltipComponent: {},
}));

vi.mock('echarts/renderers', () => ({
  CanvasRenderer: {},
}));

vi.mock('$lib/commands', () => ({
  getStatisticsTrends: vi.fn(),
  getStatisticsCycleOutcomes: vi.fn(),
  exportStatistics: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

const { getStatisticsTrends, getStatisticsCycleOutcomes, exportStatistics } =
  await import('$lib/commands');
const { save } = await import('@tauri-apps/plugin-dialog');

const payload: StatisticsTrendPayload = {
  timezone: 'UTC',
  daily: [{ label: '2026-05-20', rest_sessions: 5, total_rest_secs: 100 }],
  weekly: [{ label: '2026-W21', rest_sessions: 5, total_rest_secs: 100 }],
  monthly: [{ label: '2026-05', rest_sessions: 5, total_rest_secs: 100 }],
  total_sessions: 5,
  total_rest_secs: 100,
};

function makeOutcomes(overrides: Partial<CycleOutcomesPayload> = {}): CycleOutcomesPayload {
  return {
    timezone: 'UTC',
    today_taken: 4,
    today_skipped: 1,
    today_suppressed: 2,
    today_adherence_rate: 0.8,
    today_reason_breakdown: {
      fullscreen: 1,
      schedule: 0,
      afk: 1,
      process_whitelisted: 0,
    },
    last_24h_ribbon: [{ occurred_at: new Date().toISOString(), outcome: 'taken', reason: null }],
    eye_care_index: {
      score: 82,
      is_warming_up: false,
      is_rest_day: false,
      components: { adherence: 80, longest_session: 86 },
    },
    rhythm: { current_streak_days: 3, best_streak_days: 7, threshold: 4 },
    is_beta: true,
    ...overrides,
  };
}

describe('StatisticsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStatisticsTrends).mockResolvedValue(payload);
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(makeOutcomes());
    vi.mocked(exportStatistics).mockResolvedValue();
    vi.mocked(save).mockResolvedValue(null);
  });

  it('loads statistics through the IPC wrapper and renders totals', async () => {
    render(StatisticsPage);

    await waitFor(() => expect(getStatisticsTrends).toHaveBeenCalledOnce());
    expect(await screen.findByText('休息趋势')).toBeInTheDocument();
    expect(screen.getByText('休息次数')).toBeInTheDocument();
    expect(screen.getByText('UTC')).toBeInTheDocument();
    expect(screen.getByText('最新数据：2026-05-20')).toBeInTheDocument();
    expect(echartsMock.chart.setOption).toHaveBeenCalled();
  });

  it('switches chart data between day week and month ranges', async () => {
    render(StatisticsPage);
    await waitFor(() => expect(echartsMock.chart.setOption).toHaveBeenCalled());

    await fireEvent.click(screen.getByRole('button', { name: '周' }));

    await waitFor(() => {
      expect(echartsMock.chart.setOption).toHaveBeenLastCalledWith(
        expect.objectContaining({
          xAxis: expect.objectContaining({ data: ['2026-W21'] }),
        }),
        true,
      );
    });
  });

  it('invokes export_statistics and shows success banner when user picks a path', async () => {
    vi.mocked(save).mockResolvedValueOnce('C:/tmp/backup.db');
    render(StatisticsPage);
    await waitFor(() => expect(getStatisticsTrends).toHaveBeenCalledOnce());

    await fireEvent.click(screen.getByRole('button', { name: '导出备份' }));

    await waitFor(() => {
      expect(save).toHaveBeenCalledOnce();
      expect(exportStatistics).toHaveBeenCalledWith('C:/tmp/backup.db');
    });
    const saveArgs = vi.mocked(save).mock.calls[0][0] as {
      defaultPath?: string;
      filters?: { extensions: string[] }[];
    };
    expect(saveArgs.defaultPath).toMatch(/^eyezen-stat-\d{4}-\d{2}-\d{2}\.db$/);
    expect(saveArgs.filters?.[0]?.extensions).toContain('db');
    expect(await screen.findByText('已备份到 C:/tmp/backup.db')).toBeInTheDocument();
  });

  it('does not invoke export when user cancels the dialog', async () => {
    vi.mocked(save).mockResolvedValueOnce(null);
    render(StatisticsPage);
    await waitFor(() => expect(getStatisticsTrends).toHaveBeenCalledOnce());

    await fireEvent.click(screen.getByRole('button', { name: '导出备份' }));

    await waitFor(() => expect(save).toHaveBeenCalledOnce());
    expect(exportStatistics).not.toHaveBeenCalled();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows error banner when export_statistics rejects', async () => {
    vi.mocked(save).mockResolvedValueOnce('C:/tmp/backup.db');
    vi.mocked(exportStatistics).mockRejectedValueOnce({
      detail: { reason: 'permission denied' },
    });
    render(StatisticsPage);
    await waitFor(() => expect(getStatisticsTrends).toHaveBeenCalledOnce());

    await fireEvent.click(screen.getByRole('button', { name: '导出备份' }));

    expect(await screen.findByText('导出失败：permission denied')).toBeInTheDocument();
  });
});

describe('StatisticsPage Eye-Care Index hero', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStatisticsTrends).mockResolvedValue(payload);
    vi.mocked(exportStatistics).mockResolvedValue();
    vi.mocked(save).mockResolvedValue(null);
  });

  it('renders numeric ECI score with Beta badge', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(makeOutcomes());
    render(StatisticsPage);

    await waitFor(() => expect(getStatisticsCycleOutcomes).toHaveBeenCalledOnce());
    expect(await screen.findByText('82')).toBeInTheDocument();
    expect(screen.getByText('Beta · v0.6')).toBeInTheDocument();
    expect(screen.getByText('良好')).toBeInTheDocument();
  });

  it('renders warming-up branch when is_warming_up is true', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(
      makeOutcomes({
        eye_care_index: {
          score: null,
          is_warming_up: true,
          is_rest_day: false,
          components: { adherence: 0, longest_session: 0 },
        },
      }),
    );
    render(StatisticsPage);

    expect(await screen.findByText('正在预热')).toBeInTheDocument();
    expect(screen.getByText('今天的第一次休息后会显示在这里')).toBeInTheDocument();
  });

  it('hides ECI and today tiles and shows rest-day note when is_rest_day is true', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(
      makeOutcomes({
        eye_care_index: {
          score: null,
          is_warming_up: false,
          is_rest_day: true,
          components: { adherence: 0, longest_session: 0 },
        },
      }),
    );
    render(StatisticsPage);

    await waitFor(() => expect(getStatisticsCycleOutcomes).toHaveBeenCalledOnce());
    // ECI title gone, rest-day note present
    expect(screen.queryByText('护眼指数')).not.toBeInTheDocument();
    expect(screen.queryByText('今日')).not.toBeInTheDocument();
    expect(await screen.findByText('休息日')).toBeInTheDocument();
    expect(screen.getByText('调度已暂停 — 请查看下方历史数据')).toBeInTheDocument();
  });
});

describe('StatisticsPage rhythm + reason breakdown', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStatisticsTrends).mockResolvedValue(payload);
    vi.mocked(exportStatistics).mockResolvedValue();
    vi.mocked(save).mockResolvedValue(null);
  });

  it('renders streak threshold caption with interpolated rest count', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(
      makeOutcomes({
        rhythm: { current_streak_days: 5, best_streak_days: 12, threshold: 6 },
      }),
    );
    render(StatisticsPage);

    expect(await screen.findByText('当前连续')).toBeInTheDocument();
    expect(screen.getByText('5 天')).toBeInTheDocument();
    expect(screen.getByText('12 天')).toBeInTheDocument();
    // Caption appears twice (current + best)
    expect(screen.getAllByText('基于近 30 日每天约 6 次的节奏')).toHaveLength(2);
  });

  it('reveals suppression reason breakdown when user expands the tile', async () => {
    vi.mocked(getStatisticsCycleOutcomes).mockResolvedValue(
      makeOutcomes({
        today_suppressed: 3,
        today_reason_breakdown: {
          fullscreen: 2,
          schedule: 0,
          afk: 1,
          process_whitelisted: 0,
        },
      }),
    );
    render(StatisticsPage);

    await waitFor(() => expect(getStatisticsCycleOutcomes).toHaveBeenCalledOnce());
    const toggle = await screen.findByRole('button', { name: '+' });
    await fireEvent.click(toggle);

    expect(screen.getByText('全屏应用')).toBeInTheDocument();
    expect(screen.getByText('离开（AFK）')).toBeInTheDocument();
    // Zero-count reasons are filtered out
    expect(screen.queryByText('非工作日')).not.toBeInTheDocument();
    expect(screen.queryByText('白名单应用')).not.toBeInTheDocument();
  });
});
