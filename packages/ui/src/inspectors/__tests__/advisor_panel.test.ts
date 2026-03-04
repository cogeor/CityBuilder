import { describe, it, expect, beforeEach } from 'vitest';
import {
  AdvisorPanel,
  AdvisorCategory,
  AdvisorSeverity,
  type CityMetrics,
  formatSeverityLabel,
} from '../advisor_panel.js';

/** Helper to create default healthy city metrics */
function makeMetrics(overrides?: Partial<CityMetrics>): CityMetrics {
  return {
    population: 1000,
    treasury: 50000,
    monthlyIncome: 10000,
    monthlyExpenses: 8000,
    unemploymentRate: 0.05,
    crimeRate: 0.1,
    pollutionIndex: 0.2,
    trafficCongestion: 0.3,
    servicesCoverage: 0.8,
    happiness: 0.7,
    ...overrides,
  };
}

describe('AdvisorPanel', () => {
  let panel: AdvisorPanel;

  beforeEach(() => {
    panel = new AdvisorPanel();
  });

  // --- Empty / healthy metrics ---

  it('healthy metrics produce no critical diagnostics', () => {
    panel.updateDiagnostics(makeMetrics());
    expect(panel.getCriticalCount()).toBe(0);
  });

  it('healthy metrics produce no diagnostics at all', () => {
    panel.updateDiagnostics(makeMetrics());
    expect(panel.getDiagnostics().length).toBe(0);
  });

  // --- Budget diagnostics ---

  it('negative treasury produces critical budget diagnostic', () => {
    panel.updateDiagnostics(makeMetrics({ treasury: -500 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Budget);
    expect(diags.length).toBeGreaterThanOrEqual(1);
    const critical = diags.find(d => d.severity === AdvisorSeverity.Critical);
    expect(critical).toBeDefined();
    expect(critical!.rootCause).toBe('negative treasury');
  });

  it('budget deficit produces warning diagnostic', () => {
    // expenses > income * 1.2 => 12001 > 10000 * 1.2 = 12000
    panel.updateDiagnostics(makeMetrics({ monthlyExpenses: 12001 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Budget);
    const warning = diags.find(d => d.severity === AdvisorSeverity.Warning);
    expect(warning).toBeDefined();
    expect(warning!.title).toContain('deficit');
  });

  it('quickAction is set for budget issues', () => {
    panel.updateDiagnostics(makeMetrics({ treasury: -100 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Budget);
    for (const d of diags) {
      expect(d.quickAction).toBe('budget_panel');
    }
  });

  // --- Safety diagnostics ---

  it('high crime rate produces warning safety diagnostic', () => {
    panel.updateDiagnostics(makeMetrics({ crimeRate: 0.5 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Safety);
    expect(diags.length).toBe(1);
    expect(diags[0].severity).toBe(AdvisorSeverity.Warning);
    expect(diags[0].title).toContain('Crime');
  });

  // --- Multiple issues ---

  it('multiple issues produce multiple diagnostics', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,
      crimeRate: 0.5,
      pollutionIndex: 0.8,
      trafficCongestion: 0.9,
    }));
    const all = panel.getDiagnostics();
    expect(all.length).toBeGreaterThanOrEqual(4);
  });

  // --- getTopDiagnostics sorting ---

  it('getTopDiagnostics returns sorted by severity (critical first)', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,       // critical budget
      crimeRate: 0.5,       // warning safety
      happiness: 0.2,       // critical growth
      pollutionIndex: 0.8,  // warning environment
    }));
    const top = panel.getTopDiagnostics(10);
    expect(top.length).toBeGreaterThanOrEqual(4);

    // First items should be critical
    const firstTwo = top.slice(0, 2);
    for (const d of firstTwo) {
      expect(d.severity).toBe(AdvisorSeverity.Critical);
    }

    // Verify overall ordering: no warning before a critical
    for (let i = 1; i < top.length; i++) {
      const prevOrder = top[i - 1].severity === AdvisorSeverity.Critical ? 2
        : top[i - 1].severity === AdvisorSeverity.Warning ? 1 : 0;
      const currOrder = top[i].severity === AdvisorSeverity.Critical ? 2
        : top[i].severity === AdvisorSeverity.Warning ? 1 : 0;
      expect(prevOrder).toBeGreaterThanOrEqual(currOrder);
    }
  });

  it('getTopDiagnostics limits to count', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,
      crimeRate: 0.5,
      pollutionIndex: 0.8,
      happiness: 0.2,
    }));
    const top2 = panel.getTopDiagnostics(2);
    expect(top2.length).toBe(2);
  });

  // --- Counts ---

  it('getCriticalCount returns correct count', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,   // critical
      happiness: 0.2,   // critical
      crimeRate: 0.5,   // warning
    }));
    expect(panel.getCriticalCount()).toBe(2);
  });

  it('getWarningCount returns correct count', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,       // critical
      crimeRate: 0.5,       // warning
      pollutionIndex: 0.8,  // warning
    }));
    expect(panel.getWarningCount()).toBe(2);
  });

  // --- Category filter ---

  it('getDiagnostics filters by category', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,
      crimeRate: 0.5,
    }));
    const budget = panel.getDiagnostics(AdvisorCategory.Budget);
    const safety = panel.getDiagnostics(AdvisorCategory.Safety);
    const traffic = panel.getDiagnostics(AdvisorCategory.Traffic);

    expect(budget.length).toBeGreaterThanOrEqual(1);
    for (const d of budget) expect(d.category).toBe(AdvisorCategory.Budget);

    expect(safety.length).toBe(1);
    expect(safety[0].category).toBe(AdvisorCategory.Safety);

    expect(traffic.length).toBe(0);
  });

  // --- Clear ---

  it('clear removes all diagnostics', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,
      crimeRate: 0.5,
    }));
    expect(panel.getDiagnostics().length).toBeGreaterThan(0);
    panel.clear();
    expect(panel.getDiagnostics().length).toBe(0);
    expect(panel.getCriticalCount()).toBe(0);
    expect(panel.getWarningCount()).toBe(0);
  });

  // --- Levers ---

  it('each diagnostic has exactly 3 levers', () => {
    panel.updateDiagnostics(makeMetrics({
      treasury: -100,
      crimeRate: 0.5,
      pollutionIndex: 0.8,
      trafficCongestion: 0.9,
      servicesCoverage: 0.3,
      happiness: 0.2,
      unemploymentRate: 0.25,
      population: 0,
    }));
    const all = panel.getDiagnostics();
    expect(all.length).toBeGreaterThan(0);
    for (const d of all) {
      expect(d.levers.length).toBe(3);
    }
  });

  // --- Growth diagnostics ---

  it('zero population produces info growth diagnostic', () => {
    panel.updateDiagnostics(makeMetrics({ population: 0 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Growth);
    const info = diags.find(d => d.severity === AdvisorSeverity.Info);
    expect(info).toBeDefined();
    expect(info!.title).toContain('No residents');
  });

  it('low happiness produces critical growth diagnostic', () => {
    panel.updateDiagnostics(makeMetrics({ happiness: 0.1 }));
    const diags = panel.getDiagnostics(AdvisorCategory.Growth);
    const critical = diags.find(d => d.severity === AdvisorSeverity.Critical);
    expect(critical).toBeDefined();
    expect(critical!.title).toContain('unhappy');
  });

  // --- formatSeverityLabel ---

  it('formatSeverityLabel returns correct labels', () => {
    expect(formatSeverityLabel(AdvisorSeverity.Info)).toBe('Info');
    expect(formatSeverityLabel(AdvisorSeverity.Warning)).toBe('Warning');
    expect(formatSeverityLabel(AdvisorSeverity.Critical)).toBe('Critical');
  });

  // --- Diagnostics are copies ---

  it('getDiagnostics returns copies (not references)', () => {
    panel.updateDiagnostics(makeMetrics({ treasury: -100 }));
    const diags = panel.getDiagnostics();
    diags[0].title = 'MODIFIED';
    diags[0].levers.push('extra');
    const fresh = panel.getDiagnostics();
    expect(fresh[0].title).not.toBe('MODIFIED');
    expect(fresh[0].levers.length).toBe(3);
  });
});
