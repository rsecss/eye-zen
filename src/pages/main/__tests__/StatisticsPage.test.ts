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
}));

const { getStatisticsTrends } = await import('$lib/commands');

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
});
