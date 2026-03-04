import { describe, it, expect, beforeEach } from 'vitest';
import {
  StatsPanel,
  MetricType,
  METRIC_CONFIGS,
  type StatsViewConfig,
} from '../stats_panel.js';

describe('StatsPanel', () => {
  let panel: StatsPanel;

  beforeEach(() => {
    panel = new StatsPanel();
  });

  // --- Constructor ---

  it('constructor sets sensible defaults', () => {
    const cfg = panel.getConfig();
    expect(cfg.visibleMetrics.length).toBe(8); // all metrics visible
    expect(cfg.timeRangeStart).toBe(0);
    expect(cfg.timeRangeEnd).toBe(Infinity);
    expect(cfg.zoomLevel).toBe(1);
  });

  it('constructor accepts partial config overrides', () => {
    const custom = new StatsPanel({
      visibleMetrics: [MetricType.Population],
      zoomLevel: 3,
    });
    const cfg = custom.getConfig();
    expect(cfg.visibleMetrics).toEqual([MetricType.Population]);
    expect(cfg.zoomLevel).toBe(3);
    expect(cfg.timeRangeStart).toBe(0);
  });

  // --- addDataPoint ---

  it('addDataPoint creates a new series', () => {
    panel.addDataPoint(MetricType.Population, 100, 5000);
    const s = panel.getSeries(MetricType.Population);
    expect(s).toBeDefined();
    expect(s!.metric).toBe(MetricType.Population);
    expect(s!.points.length).toBe(1);
    expect(s!.points[0]).toEqual({ tick: 100, value: 5000 });
  });

  // --- getSeries ---

  it('getSeries returns undefined for missing metric', () => {
    expect(panel.getSeries(MetricType.CrimeRate)).toBeUndefined();
  });

  it('getSeries returns correct data', () => {
    panel.addDataPoint(MetricType.BudgetBalance, 10, -500);
    panel.addDataPoint(MetricType.BudgetBalance, 20, 1000);
    const s = panel.getSeries(MetricType.BudgetBalance)!;
    expect(s.label).toBe('Budget Balance');
    expect(s.unit).toBe('$');
    expect(s.points.length).toBe(2);
  });

  // --- getVisibleSeries ---

  it('getVisibleSeries filters by config', () => {
    const restricted = new StatsPanel({
      visibleMetrics: [MetricType.Population, MetricType.CrimeRate],
    });
    restricted.addDataPoint(MetricType.Population, 1, 100);
    restricted.addDataPoint(MetricType.CrimeRate, 1, 5);
    restricted.addDataPoint(MetricType.BudgetBalance, 1, 999);

    const visible = restricted.getVisibleSeries();
    expect(visible.length).toBe(2);
    const metrics = visible.map(s => s.metric);
    expect(metrics).toContain(MetricType.Population);
    expect(metrics).toContain(MetricType.CrimeRate);
    expect(metrics).not.toContain(MetricType.BudgetBalance);
  });

  it('getVisibleSeries excludes metrics with no data', () => {
    // All metrics are visible by default, but none have data
    const visible = panel.getVisibleSeries();
    expect(visible.length).toBe(0);
  });

  // --- setTimeRange ---

  it('setTimeRange updates config', () => {
    panel.setTimeRange(100, 500);
    const cfg = panel.getConfig();
    expect(cfg.timeRangeStart).toBe(100);
    expect(cfg.timeRangeEnd).toBe(500);
  });

  // --- setZoomLevel ---

  it('setZoomLevel updates config', () => {
    panel.setZoomLevel(5);
    const cfg = panel.getConfig();
    expect(cfg.zoomLevel).toBe(5);
  });

  // --- toggleMetric ---

  it('toggleMetric removes a visible metric', () => {
    panel.toggleMetric(MetricType.Population);
    const cfg = panel.getConfig();
    expect(cfg.visibleMetrics).not.toContain(MetricType.Population);
  });

  it('toggleMetric adds a removed metric back', () => {
    panel.toggleMetric(MetricType.Population); // remove
    panel.toggleMetric(MetricType.Population); // add back
    const cfg = panel.getConfig();
    expect(cfg.visibleMetrics).toContain(MetricType.Population);
  });

  // --- getMinMax ---

  it('getMinMax computes correctly', () => {
    panel.addDataPoint(MetricType.Unemployment, 1, 10);
    panel.addDataPoint(MetricType.Unemployment, 2, 50);
    panel.addDataPoint(MetricType.Unemployment, 3, 5);
    panel.addDataPoint(MetricType.Unemployment, 4, 30);
    const mm = panel.getMinMax(MetricType.Unemployment);
    expect(mm).toEqual({ min: 5, max: 50 });
  });

  it('getMinMax returns undefined for empty metric', () => {
    expect(panel.getMinMax(MetricType.AvgHappiness)).toBeUndefined();
  });

  // --- clear ---

  it('clear empties all series', () => {
    panel.addDataPoint(MetricType.Population, 1, 100);
    panel.addDataPoint(MetricType.CrimeRate, 1, 50);
    panel.clear();
    expect(panel.getSeries(MetricType.Population)).toBeUndefined();
    expect(panel.getSeries(MetricType.CrimeRate)).toBeUndefined();
  });

  // --- METRIC_CONFIGS ---

  it('METRIC_CONFIGS has all 8 metrics', () => {
    const allMetrics = [
      MetricType.Population,
      MetricType.BudgetBalance,
      MetricType.Unemployment,
      MetricType.AvgHappiness,
      MetricType.AvgCommuteTime,
      MetricType.CrimeRate,
      MetricType.PollutionIndex,
      MetricType.TrafficCongestion,
    ];
    for (const m of allMetrics) {
      const cfg = METRIC_CONFIGS[m];
      expect(cfg).toBeDefined();
      expect(cfg.label.length).toBeGreaterThan(0);
      expect(cfg.color).toMatch(/^#[0-9a-fA-F]{6}$/);
      expect(cfg.unit.length).toBeGreaterThan(0);
    }
  });

  // --- Multiple data points accumulate ---

  it('multiple data points accumulate in order', () => {
    for (let i = 0; i < 10; i++) {
      panel.addDataPoint(MetricType.TrafficCongestion, i * 100, i * 10);
    }
    const s = panel.getSeries(MetricType.TrafficCongestion)!;
    expect(s.points.length).toBe(10);
    expect(s.points[0].value).toBe(0);
    expect(s.points[9].value).toBe(90);
  });

  // --- Series uses METRIC_CONFIGS defaults ---

  it('addDataPoint uses correct color and unit from METRIC_CONFIGS', () => {
    panel.addDataPoint(MetricType.PollutionIndex, 1, 42);
    const s = panel.getSeries(MetricType.PollutionIndex)!;
    expect(s.color).toBe(METRIC_CONFIGS[MetricType.PollutionIndex].color);
    expect(s.unit).toBe(METRIC_CONFIGS[MetricType.PollutionIndex].unit);
  });
});
