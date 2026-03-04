/** Metric type identifiers (matches Rust MetricId) */
export enum MetricType {
  Population = 0,
  BudgetBalance = 1,
  Unemployment = 2,
  AvgHappiness = 3,
  AvgCommuteTime = 4,
  CrimeRate = 5,
  PollutionIndex = 6,
  TrafficCongestion = 7,
}

/** A single time-series data point */
export interface DataSeriesPoint {
  tick: number;
  value: number;
}

/** A complete data series for one metric */
export interface DataSeries {
  metric: MetricType;
  label: string;
  color: string;
  unit: string;
  points: DataSeriesPoint[];
}

/** Configuration for the stats panel view */
export interface StatsViewConfig {
  visibleMetrics: MetricType[];
  timeRangeStart: number;
  timeRangeEnd: number;
  zoomLevel: number;
}

/** Display configuration for each metric type */
export const METRIC_CONFIGS: Record<MetricType, { label: string; color: string; unit: string }> = {
  [MetricType.Population]:       { label: 'Population',        color: '#4a90d9', unit: 'people' },
  [MetricType.BudgetBalance]:    { label: 'Budget Balance',    color: '#5cb85c', unit: '$' },
  [MetricType.Unemployment]:     { label: 'Unemployment',      color: '#d9534f', unit: '%' },
  [MetricType.AvgHappiness]:     { label: 'Avg Happiness',     color: '#f0ad4e', unit: '%' },
  [MetricType.AvgCommuteTime]:   { label: 'Avg Commute Time',  color: '#5bc0de', unit: 'min' },
  [MetricType.CrimeRate]:        { label: 'Crime Rate',        color: '#c9302c', unit: '%' },
  [MetricType.PollutionIndex]:   { label: 'Pollution Index',   color: '#777777', unit: 'idx' },
  [MetricType.TrafficCongestion]: { label: 'Traffic Congestion', color: '#ec971f', unit: '%' },
};

/** All MetricType values for iteration */
const ALL_METRICS: MetricType[] = [
  MetricType.Population,
  MetricType.BudgetBalance,
  MetricType.Unemployment,
  MetricType.AvgHappiness,
  MetricType.AvgCommuteTime,
  MetricType.CrimeRate,
  MetricType.PollutionIndex,
  MetricType.TrafficCongestion,
];

/**
 * StatsPanel -- line graph data structures for city metrics.
 *
 * Manages data series for each metric and provides filtering / view
 * configuration for the stats overlay.
 */
export class StatsPanel {
  private series: Map<MetricType, DataSeries> = new Map();
  private config: StatsViewConfig;

  constructor(config?: Partial<StatsViewConfig>) {
    this.config = {
      visibleMetrics: config?.visibleMetrics ?? [...ALL_METRICS],
      timeRangeStart: config?.timeRangeStart ?? 0,
      timeRangeEnd: config?.timeRangeEnd ?? Infinity,
      zoomLevel: config?.zoomLevel ?? 1,
    };
  }

  /** Add a data point to a metric series (creates the series if needed). */
  addDataPoint(metric: MetricType, tick: number, value: number): void {
    let s = this.series.get(metric);
    if (!s) {
      const cfg = METRIC_CONFIGS[metric];
      s = {
        metric,
        label: cfg.label,
        color: cfg.color,
        unit: cfg.unit,
        points: [],
      };
      this.series.set(metric, s);
    }
    s.points.push({ tick, value });
  }

  /** Get the full data series for a metric. */
  getSeries(metric: MetricType): DataSeries | undefined {
    return this.series.get(metric);
  }

  /** Get all data series that are currently visible per config. */
  getVisibleSeries(): DataSeries[] {
    const result: DataSeries[] = [];
    for (const m of this.config.visibleMetrics) {
      const s = this.series.get(m);
      if (s) {
        result.push(s);
      }
    }
    return result;
  }

  /** Update the visible time range. */
  setTimeRange(start: number, end: number): void {
    this.config.timeRangeStart = start;
    this.config.timeRangeEnd = end;
  }

  /** Update the zoom level. */
  setZoomLevel(level: number): void {
    this.config.zoomLevel = level;
  }

  /** Toggle a metric's visibility. Adds it if absent, removes it if present. */
  toggleMetric(metric: MetricType): void {
    const idx = this.config.visibleMetrics.indexOf(metric);
    if (idx >= 0) {
      this.config.visibleMetrics.splice(idx, 1);
    } else {
      this.config.visibleMetrics.push(metric);
    }
  }

  /** Get the min and max values for a metric. Returns undefined if no data. */
  getMinMax(metric: MetricType): { min: number; max: number } | undefined {
    const s = this.series.get(metric);
    if (!s || s.points.length === 0) {
      return undefined;
    }
    let min = s.points[0].value;
    let max = s.points[0].value;
    for (let i = 1; i < s.points.length; i++) {
      const v = s.points[i].value;
      if (v < min) min = v;
      if (v > max) max = v;
    }
    return { min, max };
  }

  /** Get a snapshot of the current view configuration. */
  getConfig(): StatsViewConfig {
    return {
      ...this.config,
      visibleMetrics: [...this.config.visibleMetrics],
    };
  }

  /** Clear all stored data series. */
  clear(): void {
    this.series.clear();
  }
}
