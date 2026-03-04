/** Tax bracket configuration */
export interface TaxBracket {
  category: 'residential' | 'commercial' | 'industrial';
  defaultRate: number;     // 0.0-1.0
  minRate: number;
  maxRate: number;
  revenuePerCapita: number; // cents per person/job per tick
}

/** Workspace density table entry */
export interface WorkspaceDensity {
  archetypeTag: string;
  sqmPerJob: number;
}

/** Economic growth modifier */
export interface GrowthModifier {
  factor: string;
  description: string;
  minValue: number;
  maxValue: number;
  defaultValue: number;
}

/** Budget department configuration */
export interface DepartmentConfig {
  id: string;
  name: string;
  defaultBudget: number;  // 0.0-1.0
  minBudget: number;
  maxBudget: number;
  effectDescription: string;
}

export const TAX_BRACKETS: TaxBracket[] = [
  { category: 'residential', defaultRate: 0.09, minRate: 0.0, maxRate: 0.20, revenuePerCapita: 5 },
  { category: 'commercial', defaultRate: 0.09, minRate: 0.0, maxRate: 0.20, revenuePerCapita: 8 },
  { category: 'industrial', defaultRate: 0.09, minRate: 0.0, maxRate: 0.20, revenuePerCapita: 6 },
];

export const WORKSPACE_DENSITY: WorkspaceDensity[] = [
  { archetypeTag: 'commercial', sqmPerJob: 20 },
  { archetypeTag: 'industrial', sqmPerJob: 50 },
  { archetypeTag: 'civic', sqmPerJob: 15 },
  { archetypeTag: 'education', sqmPerJob: 20 },
  { archetypeTag: 'health', sqmPerJob: 15 },
];

export const GROWTH_MODIFIERS: GrowthModifier[] = [
  { factor: 'tax_rate', description: 'Higher taxes reduce growth', minValue: 0.5, maxValue: 1.5, defaultValue: 1.0 },
  { factor: 'employment', description: 'High employment attracts migrants', minValue: 0.0, maxValue: 2.0, defaultValue: 1.0 },
  { factor: 'services', description: 'Service coverage affects desirability', minValue: 0.0, maxValue: 1.5, defaultValue: 1.0 },
  { factor: 'pollution', description: 'Pollution reduces desirability', minValue: 0.5, maxValue: 1.0, defaultValue: 1.0 },
  { factor: 'crime', description: 'Crime reduces desirability', minValue: 0.5, maxValue: 1.0, defaultValue: 1.0 },
];

export const DEPARTMENTS: DepartmentConfig[] = [
  { id: 'police', name: 'Police', defaultBudget: 1.0, minBudget: 0.0, maxBudget: 1.5, effectDescription: 'Affects crime rate' },
  { id: 'fire', name: 'Fire', defaultBudget: 1.0, minBudget: 0.0, maxBudget: 1.5, effectDescription: 'Affects fire spread' },
  { id: 'health', name: 'Health', defaultBudget: 1.0, minBudget: 0.0, maxBudget: 1.5, effectDescription: 'Affects health coverage' },
  { id: 'education', name: 'Education', defaultBudget: 1.0, minBudget: 0.0, maxBudget: 1.5, effectDescription: 'Affects education level' },
  { id: 'roads', name: 'Roads', defaultBudget: 1.0, minBudget: 0.5, maxBudget: 1.5, effectDescription: 'Affects road condition' },
  { id: 'parks', name: 'Parks', defaultBudget: 1.0, minBudget: 0.0, maxBudget: 1.5, effectDescription: 'Affects desirability' },
  { id: 'utilities', name: 'Utilities', defaultBudget: 1.0, minBudget: 0.5, maxBudget: 1.5, effectDescription: 'Affects power/water' },
];

/** Get tax bracket for a category */
export function getTaxBracket(category: string): TaxBracket | undefined {
  return TAX_BRACKETS.find(t => t.category === category);
}

/** Get workspace density for an archetype tag */
export function getWorkspaceDensity(tag: string): number {
  const entry = WORKSPACE_DENSITY.find(w => w.archetypeTag === tag);
  return entry?.sqmPerJob ?? 25; // default 25 sqm/job
}

/** Compute tax revenue per tick */
export function computeTaxRevenue(category: string, population: number, rate: number): number {
  const bracket = getTaxBracket(category);
  if (!bracket) return 0;
  const clampedRate = Math.max(bracket.minRate, Math.min(bracket.maxRate, rate));
  return Math.floor(population * bracket.revenuePerCapita * clampedRate);
}

/** Compute growth modifier from tax rate (inverse relationship) */
export function computeTaxGrowthModifier(rate: number): number {
  // At 0% tax -> 1.5x growth, at 20% -> 0.5x growth
  return Math.max(0.5, Math.min(1.5, 1.5 - rate * 5));
}

/** Get department config by id */
export function getDepartment(id: string): DepartmentConfig | undefined {
  return DEPARTMENTS.find(d => d.id === id);
}

/** Validate economy configuration */
export function validateEconomyConfig(): string[] {
  const errors: string[] = [];
  for (const t of TAX_BRACKETS) {
    if (t.minRate > t.maxRate) errors.push(`Invalid rate range for ${t.category}`);
    if (t.revenuePerCapita < 0) errors.push(`Negative revenue for ${t.category}`);
  }
  for (const d of DEPARTMENTS) {
    if (d.minBudget > d.maxBudget) errors.push(`Invalid budget range for ${d.name}`);
  }
  return errors;
}
