/** Advisor category for diagnostic classification */
export enum AdvisorCategory {
  Budget = "budget",
  Growth = "growth",
  Services = "services",
  Traffic = "traffic",
  Safety = "safety",
  Environment = "environment",
}

/** Severity level for diagnostics */
export enum AdvisorSeverity {
  Info = "info",
  Warning = "warning",
  Critical = "critical",
}

/** A single diagnostic item from the advisor */
export interface DiagnosticItem {
  category: AdvisorCategory;
  severity: AdvisorSeverity;
  title: string;
  rootCause: string;
  levers: string[];           // top 3 corrective actions
  estimatedRecovery: number;  // ticks to recovery
  quickAction?: string;       // tool/panel to jump to
}

/** State of the advisor panel */
export interface AdvisorState {
  diagnostics: DiagnosticItem[];
  lastUpdateTick: number;
}

/** City metrics used as input for advisor analysis */
export interface CityMetrics {
  population: number;
  treasury: number;
  monthlyIncome: number;
  monthlyExpenses: number;
  unemploymentRate: number;
  crimeRate: number;
  pollutionIndex: number;
  trafficCongestion: number;
  servicesCoverage: number;
  happiness: number;
}

/** Severity ordering for sorting (higher = more severe) */
const SEVERITY_ORDER: Record<AdvisorSeverity, number> = {
  [AdvisorSeverity.Info]: 0,
  [AdvisorSeverity.Warning]: 1,
  [AdvisorSeverity.Critical]: 2,
};

/** Format a severity level as a human-readable label */
export function formatSeverityLabel(severity: AdvisorSeverity): string {
  switch (severity) {
    case AdvisorSeverity.Info: return 'Info';
    case AdvisorSeverity.Warning: return 'Warning';
    case AdvisorSeverity.Critical: return 'Critical';
  }
}

/**
 * AdvisorPanel -- analyzes city metrics and generates gameplay diagnostics.
 *
 * Feed it CityMetrics each tick and it produces prioritized DiagnosticItems
 * explaining what is wrong, why, and what to do about it.
 */
export class AdvisorPanel {
  private state: AdvisorState;

  constructor() {
    this.state = {
      diagnostics: [],
      lastUpdateTick: 0,
    };
  }

  /** Analyze metrics and regenerate all diagnostics */
  updateDiagnostics(metrics: CityMetrics): void {
    const diagnostics: DiagnosticItem[] = [];

    // --- Budget diagnostics ---

    // Critical: negative treasury
    if (metrics.treasury < 0) {
      diagnostics.push({
        category: AdvisorCategory.Budget,
        severity: AdvisorSeverity.Critical,
        title: 'Treasury is negative',
        rootCause: 'negative treasury',
        levers: ['Raise taxes', 'Cut spending', 'Take loan'],
        estimatedRecovery: 7200,
        quickAction: 'budget_panel',
      });
    }

    // Warning: monthly deficit (expenses > income * 1.2)
    if (metrics.monthlyExpenses > metrics.monthlyIncome * 1.2) {
      diagnostics.push({
        category: AdvisorCategory.Budget,
        severity: AdvisorSeverity.Warning,
        title: 'Budget deficit growing',
        rootCause: 'monthly expenses exceed income by more than 20%',
        levers: ['Reduce department budgets', 'Raise tax rates', 'Zone more commercial'],
        estimatedRecovery: 14400,
        quickAction: 'budget_panel',
      });
    }

    // --- Growth diagnostics ---

    // Info: no residents
    if (metrics.population === 0) {
      diagnostics.push({
        category: AdvisorCategory.Growth,
        severity: AdvisorSeverity.Info,
        title: 'No residents yet',
        rootCause: 'city has zero population',
        levers: ['Zone residential areas', 'Build roads for access', 'Ensure power and water'],
        estimatedRecovery: 3600,
      });
    }

    // Warning: high unemployment
    if (metrics.unemploymentRate > 0.15) {
      diagnostics.push({
        category: AdvisorCategory.Growth,
        severity: AdvisorSeverity.Warning,
        title: 'High unemployment',
        rootCause: 'unemployment rate exceeds 15%',
        levers: ['Zone more commercial', 'Zone more industrial', 'Reduce residential zoning'],
        estimatedRecovery: 7200,
      });
    }

    // Critical: happiness very low
    if (metrics.happiness < 0.3) {
      diagnostics.push({
        category: AdvisorCategory.Growth,
        severity: AdvisorSeverity.Critical,
        title: 'Citizens are unhappy',
        rootCause: 'overall happiness below 30%',
        levers: ['Improve services coverage', 'Reduce crime and pollution', 'Add parks and recreation'],
        estimatedRecovery: 14400,
      });
    }

    // --- Safety diagnostics ---

    // Warning: high crime
    if (metrics.crimeRate > 0.3) {
      diagnostics.push({
        category: AdvisorCategory.Safety,
        severity: AdvisorSeverity.Warning,
        title: 'Crime rate is high',
        rootCause: 'crime rate exceeds 30%',
        levers: ['Increase police budget', 'Add police station', 'Improve lighting and roads'],
        estimatedRecovery: 7200,
      });
    }

    // --- Environment diagnostics ---

    // Warning: high pollution
    if (metrics.pollutionIndex > 0.5) {
      diagnostics.push({
        category: AdvisorCategory.Environment,
        severity: AdvisorSeverity.Warning,
        title: 'Pollution levels high',
        rootCause: 'pollution index exceeds 50%',
        levers: ['Add parks and green spaces', 'Relocate heavy industry', 'Upgrade to clean power'],
        estimatedRecovery: 10800,
      });
    }

    // --- Traffic diagnostics ---

    // Warning: high congestion
    if (metrics.trafficCongestion > 0.7) {
      diagnostics.push({
        category: AdvisorCategory.Traffic,
        severity: AdvisorSeverity.Warning,
        title: 'Traffic congestion severe',
        rootCause: 'traffic congestion exceeds 70%',
        levers: ['Build additional roads', 'Add public transit', 'Spread out zoning'],
        estimatedRecovery: 10800,
      });
    }

    // --- Services diagnostics ---

    // Warning: low services coverage
    if (metrics.servicesCoverage < 0.5) {
      diagnostics.push({
        category: AdvisorCategory.Services,
        severity: AdvisorSeverity.Warning,
        title: 'Services coverage inadequate',
        rootCause: 'services coverage below 50%',
        levers: ['Build more service buildings', 'Increase service budgets', 'Redistribute facilities'],
        estimatedRecovery: 7200,
      });
    }

    this.state.diagnostics = diagnostics;
    this.state.lastUpdateTick++;
  }

  /** Get diagnostics, optionally filtered by category */
  getDiagnostics(category?: AdvisorCategory): DiagnosticItem[] {
    if (category === undefined) {
      return this.state.diagnostics.map(d => ({ ...d, levers: [...d.levers] }));
    }
    return this.state.diagnostics
      .filter(d => d.category === category)
      .map(d => ({ ...d, levers: [...d.levers] }));
  }

  /** Get top N diagnostics sorted by severity (critical first) */
  getTopDiagnostics(count: number): DiagnosticItem[] {
    return [...this.state.diagnostics]
      .sort((a, b) => SEVERITY_ORDER[b.severity] - SEVERITY_ORDER[a.severity])
      .slice(0, count)
      .map(d => ({ ...d, levers: [...d.levers] }));
  }

  /** Count of critical diagnostics */
  getCriticalCount(): number {
    return this.state.diagnostics.filter(d => d.severity === AdvisorSeverity.Critical).length;
  }

  /** Count of warning diagnostics */
  getWarningCount(): number {
    return this.state.diagnostics.filter(d => d.severity === AdvisorSeverity.Warning).length;
  }

  /** Clear all diagnostics */
  clear(): void {
    this.state.diagnostics = [];
    this.state.lastUpdateTick = 0;
  }
}
