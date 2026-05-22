import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StatisticsTrendPayload } from '$lib/bindings/StatisticsTrendPayload';
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
  exportStatistics: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

const { getStatisticsTrends, exportStatistics } = await import('$lib/commands');
const { save } = await import('@tauri-apps/plugin-dialog');

const payload: StatisticsTrendPayload = {
  timezone: 'UTC',
  daily: [{ label: '2026-05-20', rest_sessions: 5, total_rest_secs: 100 }],
  weekly: [{ label: '2026-W21', rest_sessions: 5, total_rest_secs: 100 }],
  monthly: [{ label: '2026-05', rest_sessions: 5, total_rest_secs: 100 }],
  total_sessions: 5,
  total_rest_secs: 100,
};

describe('StatisticsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStatisticsTrends).mockResolvedValue(payload);
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
