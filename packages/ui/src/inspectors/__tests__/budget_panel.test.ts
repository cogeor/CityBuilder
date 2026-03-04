import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  BudgetPanel,
  BudgetViewMode,
  TaxCategory,
  ExpenseDepartment,
  type BudgetEventType,
} from '../budget_panel.js';

describe('BudgetPanel', () => {
  let panel: BudgetPanel;

  beforeEach(() => {
    panel = new BudgetPanel();
  });

  // --- Constructor ---

  it('constructor initializes with default tax rates (9%)', () => {
    for (const cat of Object.values(TaxCategory)) {
      expect(panel.getTaxRate(cat)).toBe(0.09);
    }
  });

  it('constructor initializes with default budgets (100%)', () => {
    for (const dept of Object.values(ExpenseDepartment)) {
      expect(panel.getBudget(dept)).toBe(1.0);
    }
  });

  // --- Tax Rates ---

  it('getTaxRate returns default 9%', () => {
    expect(panel.getTaxRate(TaxCategory.Residential)).toBe(0.09);
  });

  it('setTaxRate changes rate and updates actual income', () => {
    panel.updateIncome(TaxCategory.Residential, 10000);
    panel.setTaxRate(TaxCategory.Residential, 0.12);
    expect(panel.getTaxRate(TaxCategory.Residential)).toBe(0.12);
    const breakdown = panel.getIncomeBreakdown();
    const res = breakdown.find(i => i.category === TaxCategory.Residential)!;
    expect(res.actualIncome).toBe(Math.round(10000 * 0.12));
  });

  it('setTaxRate clamps to 0-1', () => {
    panel.setTaxRate(TaxCategory.Commercial, -0.5);
    expect(panel.getTaxRate(TaxCategory.Commercial)).toBe(0);

    panel.setTaxRate(TaxCategory.Commercial, 1.5);
    expect(panel.getTaxRate(TaxCategory.Commercial)).toBe(1);
  });

  // --- Budget Sliders ---

  it('getBudget returns default 100%', () => {
    expect(panel.getBudget(ExpenseDepartment.Police)).toBe(1.0);
  });

  it('setBudget changes budget and updates actual cost', () => {
    panel.updateExpense(ExpenseDepartment.Fire, 5000);
    panel.setBudget(ExpenseDepartment.Fire, 0.8);
    expect(panel.getBudget(ExpenseDepartment.Fire)).toBe(0.8);
    const breakdown = panel.getExpenseBreakdown();
    const fire = breakdown.find(e => e.department === ExpenseDepartment.Fire)!;
    expect(fire.actualCost).toBe(Math.round(5000 * 0.8));
  });

  it('setBudget clamps to 0-1.5', () => {
    panel.setBudget(ExpenseDepartment.Health, -0.3);
    expect(panel.getBudget(ExpenseDepartment.Health)).toBe(0);

    panel.setBudget(ExpenseDepartment.Health, 2.0);
    expect(panel.getBudget(ExpenseDepartment.Health)).toBe(1.5);
  });

  // --- Data Updates ---

  it('updateIncome updates base and computes actual', () => {
    panel.updateIncome(TaxCategory.Industrial, 20000);
    const breakdown = panel.getIncomeBreakdown();
    const ind = breakdown.find(i => i.category === TaxCategory.Industrial)!;
    expect(ind.baseIncome).toBe(20000);
    expect(ind.actualIncome).toBe(Math.round(20000 * 0.09));
  });

  it('updateExpense updates base and computes actual', () => {
    panel.updateExpense(ExpenseDepartment.Education, 8000);
    const breakdown = panel.getExpenseBreakdown();
    const edu = breakdown.find(e => e.department === ExpenseDepartment.Education)!;
    expect(edu.baseCost).toBe(8000);
    expect(edu.actualCost).toBe(Math.round(8000 * 1.0));
  });

  // --- Computation ---

  it('getTotalIncome sums all categories', () => {
    panel.updateIncome(TaxCategory.Residential, 10000);
    panel.updateIncome(TaxCategory.Commercial, 20000);
    panel.updateIncome(TaxCategory.Industrial, 30000);
    // All at 9% default
    const expected = Math.round(10000 * 0.09) + Math.round(20000 * 0.09) + Math.round(30000 * 0.09);
    expect(panel.getTotalIncome()).toBe(expected);
  });

  it('getTotalExpenses sums all departments', () => {
    panel.updateExpense(ExpenseDepartment.Police, 1000);
    panel.updateExpense(ExpenseDepartment.Fire, 2000);
    // All at 100% default, rest are 0
    expect(panel.getTotalExpenses()).toBe(3000);
  });

  it('getNetBalance computes income - expenses', () => {
    panel.updateIncome(TaxCategory.Residential, 100000);
    panel.updateExpense(ExpenseDepartment.Police, 5000);
    const expected = Math.round(100000 * 0.09) - 5000;
    expect(panel.getNetBalance()).toBe(expected);
  });

  // --- Breakdowns ---

  it('getIncomeBreakdown returns all items', () => {
    const breakdown = panel.getIncomeBreakdown();
    expect(breakdown.length).toBe(Object.values(TaxCategory).length);
    for (const cat of Object.values(TaxCategory)) {
      expect(breakdown.find(i => i.category === cat)).toBeDefined();
    }
  });

  it('getExpenseBreakdown returns all items', () => {
    const breakdown = panel.getExpenseBreakdown();
    expect(breakdown.length).toBe(Object.values(ExpenseDepartment).length);
    for (const dept of Object.values(ExpenseDepartment)) {
      expect(breakdown.find(e => e.department === dept)).toBeDefined();
    }
  });

  // --- History ---

  it('addSnapshot adds to history', () => {
    panel.updateIncome(TaxCategory.Residential, 10000);
    panel.updateTreasury(50000);
    panel.addSnapshot(1, 2026);
    const history = panel.getHistory();
    expect(history.length).toBe(1);
    expect(history[0].month).toBe(1);
    expect(history[0].year).toBe(2026);
    expect(history[0].treasury).toBe(50000);
  });

  it('addSnapshot trims old entries beyond maxHistory', () => {
    // maxHistory is 24
    for (let i = 0; i < 30; i++) {
      panel.addSnapshot(i % 12 + 1, 2026);
    }
    const history = panel.getHistory();
    expect(history.length).toBe(24);
  });

  it('getHistory returns copies', () => {
    panel.addSnapshot(1, 2026);
    const history = panel.getHistory();
    history[0].treasury = 999999;
    expect(panel.getHistory()[0].treasury).toBe(0); // original unchanged
  });

  // --- Trend ---

  it('getTrend returns neutral with fewer than 2 snapshots', () => {
    expect(panel.getTrend()).toBe('neutral');
    panel.addSnapshot(1, 2026);
    expect(panel.getTrend()).toBe('neutral');
  });

  it('getTrend returns positive when treasury increases', () => {
    panel.updateTreasury(1000);
    panel.addSnapshot(1, 2026);
    panel.updateTreasury(2000);
    panel.addSnapshot(2, 2026);
    expect(panel.getTrend()).toBe('positive');
  });

  it('getTrend returns negative when treasury decreases', () => {
    panel.updateTreasury(2000);
    panel.addSnapshot(1, 2026);
    panel.updateTreasury(1000);
    panel.addSnapshot(2, 2026);
    expect(panel.getTrend()).toBe('negative');
  });

  it('getTrend returns neutral when treasury unchanged', () => {
    panel.updateTreasury(1000);
    panel.addSnapshot(1, 2026);
    panel.addSnapshot(2, 2026);
    expect(panel.getTrend()).toBe('neutral');
  });

  // --- Display Helpers ---

  it('formatCents formats small amounts', () => {
    expect(panel.formatCents(500)).toBe('$5');
    expect(panel.formatCents(9999)).toBe('$100');
  });

  it('formatCents formats thousands', () => {
    expect(panel.formatCents(150000)).toBe('$1.5K');
  });

  it('formatCents formats millions', () => {
    expect(panel.formatCents(250000000)).toBe('$2.5M');
  });

  it('formatRate formats percentage', () => {
    expect(panel.formatRate(0.09)).toBe('9.0%');
    expect(panel.formatRate(0.125)).toBe('12.5%');
  });

  it('formatBudget formats percentage', () => {
    expect(panel.formatBudget(1.0)).toBe('100%');
    expect(panel.formatBudget(0.75)).toBe('75%');
    expect(panel.formatBudget(1.5)).toBe('150%');
  });

  // --- View Mode ---

  it('getViewMode returns default Monthly', () => {
    expect(panel.getViewMode()).toBe(BudgetViewMode.Monthly);
  });

  it('setViewMode changes mode and emits event', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.setViewMode(BudgetViewMode.Yearly);
    expect(panel.getViewMode()).toBe(BudgetViewMode.Yearly);
    expect(handler).toHaveBeenCalledWith('viewModeChanged', { mode: BudgetViewMode.Yearly });
  });

  // --- Events ---

  it('addEventListener receives events', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.setTaxRate(TaxCategory.Residential, 0.12);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('taxRateChanged', {
      category: TaxCategory.Residential,
      rate: 0.12,
    });
  });

  it('removeEventListener stops receiving events', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.removeEventListener(handler);
    panel.setTaxRate(TaxCategory.Residential, 0.12);
    expect(handler).not.toHaveBeenCalled();
  });

  it('setBudget emits budgetChanged event', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.setBudget(ExpenseDepartment.Roads, 0.5);
    expect(handler).toHaveBeenCalledWith('budgetChanged', {
      department: ExpenseDepartment.Roads,
      budget: 0.5,
    });
  });

  // --- updateTreasury ---

  it('updateTreasury updates value', () => {
    panel.updateTreasury(123456);
    expect(panel.getTreasury()).toBe(123456);
  });
});
