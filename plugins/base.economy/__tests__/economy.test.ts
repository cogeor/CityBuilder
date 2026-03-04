import { describe, it, expect } from 'vitest';
import {
  TAX_BRACKETS,
  WORKSPACE_DENSITY,
  GROWTH_MODIFIERS,
  DEPARTMENTS,
  getTaxBracket,
  getWorkspaceDensity,
  computeTaxRevenue,
  computeTaxGrowthModifier,
  getDepartment,
  validateEconomyConfig,
} from '../economy.js';

// ─── TAX_BRACKETS ───────────────────────────────────────────────────────────

describe('TAX_BRACKETS', () => {
  it('has 3 entries', () => {
    expect(TAX_BRACKETS).toHaveLength(3);
  });

  it('all brackets have valid rate ranges (min <= max)', () => {
    for (const bracket of TAX_BRACKETS) {
      expect(bracket.minRate).toBeLessThanOrEqual(bracket.maxRate);
    }
  });
});

// ─── getTaxBracket ──────────────────────────────────────────────────────────

describe('getTaxBracket', () => {
  it('finds bracket by category', () => {
    const res = getTaxBracket('residential');
    expect(res).toBeDefined();
    expect(res!.category).toBe('residential');
    expect(res!.defaultRate).toBe(0.09);
  });

  it('returns undefined for unknown category', () => {
    expect(getTaxBracket('agricultural')).toBeUndefined();
  });
});

// ─── getWorkspaceDensity ────────────────────────────────────────────────────

describe('getWorkspaceDensity', () => {
  it('returns correct value for known archetype tag', () => {
    expect(getWorkspaceDensity('commercial')).toBe(20);
    expect(getWorkspaceDensity('industrial')).toBe(50);
  });

  it('returns default value (25) for unknown tag', () => {
    expect(getWorkspaceDensity('unknown_tag')).toBe(25);
  });
});

// ─── computeTaxRevenue ──────────────────────────────────────────────────────

describe('computeTaxRevenue', () => {
  it('calculates revenue correctly', () => {
    // residential: 100 people * 5 cents * 0.09 rate = 45
    const revenue = computeTaxRevenue('residential', 100, 0.09);
    expect(revenue).toBe(45);
  });

  it('clamps rate to bracket range', () => {
    // rate of 0.50 should be clamped to maxRate (0.20)
    const clamped = computeTaxRevenue('residential', 100, 0.50);
    // 100 * 5 * 0.20 = 100
    expect(clamped).toBe(100);
  });

  it('returns 0 for unknown category', () => {
    expect(computeTaxRevenue('unknown', 100, 0.10)).toBe(0);
  });

  it('returns 0 when population is 0', () => {
    expect(computeTaxRevenue('residential', 0, 0.09)).toBe(0);
  });
});

// ─── computeTaxGrowthModifier ───────────────────────────────────────────────

describe('computeTaxGrowthModifier', () => {
  it('returns 1.5 at 0% tax', () => {
    expect(computeTaxGrowthModifier(0.0)).toBe(1.5);
  });

  it('returns 0.5 at 20% tax', () => {
    expect(computeTaxGrowthModifier(0.20)).toBeCloseTo(0.5, 5);
  });

  it('clamps to min 0.5 for very high tax', () => {
    expect(computeTaxGrowthModifier(1.0)).toBe(0.5);
  });

  it('clamps to max 1.5 for negative tax', () => {
    expect(computeTaxGrowthModifier(-0.5)).toBe(1.5);
  });
});

// ─── DEPARTMENTS ────────────────────────────────────────────────────────────

describe('DEPARTMENTS', () => {
  it('has 7 entries', () => {
    expect(DEPARTMENTS).toHaveLength(7);
  });
});

// ─── getDepartment ──────────────────────────────────────────────────────────

describe('getDepartment', () => {
  it('finds department by id', () => {
    const police = getDepartment('police');
    expect(police).toBeDefined();
    expect(police!.name).toBe('Police');
  });

  it('returns undefined for unknown id', () => {
    expect(getDepartment('unknown')).toBeUndefined();
  });
});

// ─── WORKSPACE_DENSITY ──────────────────────────────────────────────────────

describe('WORKSPACE_DENSITY', () => {
  it('has entries for all expected archetype tags', () => {
    const tags = WORKSPACE_DENSITY.map(w => w.archetypeTag);
    expect(tags).toContain('commercial');
    expect(tags).toContain('industrial');
    expect(tags).toContain('civic');
    expect(tags).toContain('education');
    expect(tags).toContain('health');
  });
});

// ─── GROWTH_MODIFIERS ───────────────────────────────────────────────────────

describe('GROWTH_MODIFIERS', () => {
  it('has expected growth factors', () => {
    const factors = GROWTH_MODIFIERS.map(g => g.factor);
    expect(factors).toContain('tax_rate');
    expect(factors).toContain('employment');
    expect(factors).toContain('services');
    expect(factors).toContain('pollution');
    expect(factors).toContain('crime');
  });
});

// ─── validateEconomyConfig ──────────────────────────────────────────────────

describe('validateEconomyConfig', () => {
  it('returns no errors for default configuration', () => {
    const errors = validateEconomyConfig();
    expect(errors).toHaveLength(0);
  });
});
