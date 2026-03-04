/** Map size preset */
export interface MapSizePreset {
  id: string;
  name: string;
  width: number;   // tiles
  height: number;  // tiles
  description: string;
}

/** Country preset — affects naming, culture, economy */
export interface CountryPreset {
  id: string;
  name: string;
  currency: string;
  currencySymbol: string;
  language: string;
  defaultTaxRate: number;  // 0.0-1.0
  populationGrowthModifier: number; // multiplier
  industrialModifier: number;
}

/** Time and lighting preset */
export interface TimePreset {
  startHour: number;       // game hour 0-23
  daylightStart: number;   // hour sunrise
  daylightEnd: number;     // hour sunset
  seasonalVariation: boolean;
}

/** World configuration defaults */
export interface WorldDefaults {
  startingTreasury: number; // cents
  startingPopulation: number;
  defaultSpeed: number;     // SimSpeed enum value
  autoSaveInterval: number; // seconds
  maxEntities: number;
}

export const MAP_SIZES: MapSizePreset[] = [
  { id: 'small', name: 'Small', width: 128, height: 128, description: '128\u00d7128 tiles \u2014 Ideal for quick games' },
  { id: 'medium', name: 'Medium', width: 192, height: 192, description: '192\u00d7192 tiles \u2014 Balanced gameplay' },
  { id: 'large', name: 'Large', width: 256, height: 256, description: '256\u00d7256 tiles \u2014 Full-scale city' },
];

export const COUNTRY_PRESETS: CountryPreset[] = [
  {
    id: 'generic',
    name: 'Generic',
    currency: 'Dollar',
    currencySymbol: '$',
    language: 'en',
    defaultTaxRate: 0.09,
    populationGrowthModifier: 1.0,
    industrialModifier: 1.0,
  },
  {
    id: 'us',
    name: 'United States',
    currency: 'Dollar',
    currencySymbol: '$',
    language: 'en',
    defaultTaxRate: 0.08,
    populationGrowthModifier: 1.1,
    industrialModifier: 0.9,
  },
  {
    id: 'france',
    name: 'France',
    currency: 'Euro',
    currencySymbol: '\u20ac',
    language: 'fr',
    defaultTaxRate: 0.12,
    populationGrowthModifier: 0.9,
    industrialModifier: 1.1,
  },
];

export const TIME_DEFAULTS: TimePreset = {
  startHour: 8,
  daylightStart: 6,
  daylightEnd: 20,
  seasonalVariation: false,
};

export const WORLD_DEFAULTS: WorldDefaults = {
  startingTreasury: 50_000_00, // $50,000 in cents
  startingPopulation: 0,
  defaultSpeed: 1,
  autoSaveInterval: 300,
  maxEntities: 16384,
};

/** Get map size by id */
export function getMapSize(id: string): MapSizePreset | undefined {
  return MAP_SIZES.find(m => m.id === id);
}

/** Get country preset by id */
export function getCountryPreset(id: string): CountryPreset | undefined {
  return COUNTRY_PRESETS.find(c => c.id === id);
}
