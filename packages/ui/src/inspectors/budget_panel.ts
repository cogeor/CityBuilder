import { TypedEventHub, type EventListener } from "../shared/typed_events.js";

/** Tax category */
export enum TaxCategory {
  Residential = 'residential',
  Commercial = 'commercial',
  Industrial = 'industrial',
}

/** Expense department */
export enum ExpenseDepartment {
  Police = 'police',
  Fire = 'fire',
  Health = 'health',
  Education = 'education',
  Roads = 'roads',
  Parks = 'parks',
  Utilities = 'utilities',
}

/** Income line item */
export interface IncomeItem {
  category: TaxCategory;
  rate: number;        // tax rate 0.0 - 1.0 (display as %)
  baseIncome: number;  // cents per month before rate
  actualIncome: number; // cents per month after rate
}

/** Expense line item */
export interface ExpenseItem {
  department: ExpenseDepartment;
  budget: number;       // budget slider 0.0 - 1.0 (display as %)
  baseCost: number;     // cents per month at 100% budget
  actualCost: number;   // cents per month at current budget
}

/** Monthly budget snapshot */
export interface BudgetSnapshot {
  month: number;
  year: number;
  totalIncome: number;    // cents
  totalExpenses: number;  // cents
  netBalance: number;     // cents (income - expenses)
  treasury: number;       // cents (running total)
}

/** Budget view mode */
export enum BudgetViewMode {
  Monthly = 'monthly',
  Yearly = 'yearly',
}

/** Budget change event */
export type BudgetEventType = 'taxRateChanged' | 'budgetChanged' | 'viewModeChanged';
export interface BudgetEventPayloads {
  taxRateChanged: { category: TaxCategory; rate: number };
  budgetChanged: { department: ExpenseDepartment; budget: number };
  viewModeChanged: { mode: BudgetViewMode };
}
export type BudgetEventHandler = EventListener<BudgetEventPayloads>;

/**
 * BudgetPanel — manages income/expense display and tax/budget controls.
 */
export class BudgetPanel {
  private incomeItems: Map<TaxCategory, IncomeItem>;
  private expenseItems: Map<ExpenseDepartment, ExpenseItem>;
  private history: BudgetSnapshot[];
  private maxHistory: number;
  private currentTreasury: number;
  private viewMode: BudgetViewMode;
  private readonly events: TypedEventHub<BudgetEventPayloads>;

  constructor() {
    this.incomeItems = new Map();
    this.expenseItems = new Map();
    this.history = [];
    this.maxHistory = 24; // 2 years of monthly data
    this.currentTreasury = 0;
    this.viewMode = BudgetViewMode.Monthly;
    this.events = new TypedEventHub<BudgetEventPayloads>();

    // Initialize with defaults
    this.initDefaults();
  }

  private initDefaults(): void {
    for (const cat of Object.values(TaxCategory)) {
      this.incomeItems.set(cat, {
        category: cat,
        rate: 0.09, // 9% default
        baseIncome: 0,
        actualIncome: 0,
      });
    }
    for (const dept of Object.values(ExpenseDepartment)) {
      this.expenseItems.set(dept, {
        department: dept,
        budget: 1.0, // 100% default
        baseCost: 0,
        actualCost: 0,
      });
    }
  }

  // --- View Mode ---
  getViewMode(): BudgetViewMode { return this.viewMode; }
  setViewMode(mode: BudgetViewMode): void {
    this.viewMode = mode;
    this.emit('viewModeChanged', { mode });
  }

  // --- Tax Rates ---
  getTaxRate(category: TaxCategory): number {
    return this.incomeItems.get(category)?.rate ?? 0;
  }

  setTaxRate(category: TaxCategory, rate: number): void {
    const clamped = Math.max(0, Math.min(1, rate));
    const item = this.incomeItems.get(category);
    if (item) {
      item.rate = clamped;
      item.actualIncome = Math.round(item.baseIncome * clamped);
      this.emit('taxRateChanged', { category, rate: clamped });
    }
  }

  // --- Budget Sliders ---
  getBudget(department: ExpenseDepartment): number {
    return this.expenseItems.get(department)?.budget ?? 0;
  }

  setBudget(department: ExpenseDepartment, budget: number): void {
    const clamped = Math.max(0, Math.min(1.5, budget)); // Allow up to 150%
    const item = this.expenseItems.get(department);
    if (item) {
      item.budget = clamped;
      item.actualCost = Math.round(item.baseCost * clamped);
      this.emit('budgetChanged', { department, budget: clamped });
    }
  }

  // --- Data Updates ---
  updateIncome(category: TaxCategory, baseIncome: number): void {
    const item = this.incomeItems.get(category);
    if (item) {
      item.baseIncome = baseIncome;
      item.actualIncome = Math.round(baseIncome * item.rate);
    }
  }

  updateExpense(department: ExpenseDepartment, baseCost: number): void {
    const item = this.expenseItems.get(department);
    if (item) {
      item.baseCost = baseCost;
      item.actualCost = Math.round(baseCost * item.budget);
    }
  }

  updateTreasury(treasury: number): void {
    this.currentTreasury = treasury;
  }

  // --- Computation ---
  getTotalIncome(): number {
    let total = 0;
    for (const item of this.incomeItems.values()) total += item.actualIncome;
    return total;
  }

  getTotalExpenses(): number {
    let total = 0;
    for (const item of this.expenseItems.values()) total += item.actualCost;
    return total;
  }

  getNetBalance(): number {
    return this.getTotalIncome() - this.getTotalExpenses();
  }

  getTreasury(): number { return this.currentTreasury; }

  getIncomeBreakdown(): IncomeItem[] {
    return Array.from(this.incomeItems.values()).map(i => ({ ...i }));
  }

  getExpenseBreakdown(): ExpenseItem[] {
    return Array.from(this.expenseItems.values()).map(e => ({ ...e }));
  }

  // --- History ---
  addSnapshot(month: number, year: number): void {
    this.history.push({
      month,
      year,
      totalIncome: this.getTotalIncome(),
      totalExpenses: this.getTotalExpenses(),
      netBalance: this.getNetBalance(),
      treasury: this.currentTreasury,
    });
    while (this.history.length > this.maxHistory) {
      this.history.shift();
    }
  }

  getHistory(): BudgetSnapshot[] {
    return this.history.map(s => ({ ...s }));
  }

  getTrend(): 'positive' | 'negative' | 'neutral' {
    if (this.history.length < 2) return 'neutral';
    const recent = this.history[this.history.length - 1];
    const prev = this.history[this.history.length - 2];
    if (recent.treasury > prev.treasury) return 'positive';
    if (recent.treasury < prev.treasury) return 'negative';
    return 'neutral';
  }

  // --- Display Helpers ---
  formatCents(cents: number): string {
    const dollars = cents / 100;
    if (Math.abs(dollars) >= 1_000_000) return `$${(dollars / 1_000_000).toFixed(1)}M`;
    if (Math.abs(dollars) >= 1_000) return `$${(dollars / 1_000).toFixed(1)}K`;
    return `$${dollars.toFixed(0)}`;
  }

  formatRate(rate: number): string {
    return `${(rate * 100).toFixed(1)}%`;
  }

  formatBudget(budget: number): string {
    return `${Math.round(budget * 100)}%`;
  }

  // --- Events ---
  addEventListener(handler: BudgetEventHandler): void {
    this.events.on(handler);
  }
  removeEventListener(handler: BudgetEventHandler): void {
    this.events.off(handler);
  }
  private emit<K extends BudgetEventType>(type: K, data: BudgetEventPayloads[K]): void {
    this.events.emit(type, data);
  }
}
