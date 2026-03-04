/** Era preset for start profiles */
export enum EraPreset {
  Modern = "modern",
  PostWar = "post_war",
  Victorian = "victorian",
}

/** World start profile — defines starting conditions for a new game */
export interface StartProfile {
  id: string;
  name: string;
  description: string;
  era: EraPreset;
  startingTreasury: number;        // in cents
  startingPopulation: number;
  unlockedArchetypes: number[];    // archetype IDs available at start
  disabledArchetypes: number[];    // locked until conditions met
  economyModifiers: Record<string, number>; // multipliers
  compatibleMapSizes: string[];    // "small", "medium", "large"
}

export const START_PROFILES: StartProfile[] = [
  {
    id: "modern_standard",
    name: "Modern City",
    description: "Start with full modern amenities",
    era: EraPreset.Modern,
    startingTreasury: 50_000_00,
    startingPopulation: 0,
    unlockedArchetypes: [100, 200, 300, 400, 500],
    disabledArchetypes: [],
    economyModifiers: { taxRate: 1.0, growthRate: 1.0 },
    compatibleMapSizes: ["small", "medium", "large"],
  },
  {
    id: "post_war_rebuild",
    name: "Post-War Rebuild",
    description: "Rebuild a damaged city with limited resources",
    era: EraPreset.PostWar,
    startingTreasury: 25_000_00,
    startingPopulation: 500,
    unlockedArchetypes: [100, 200],
    disabledArchetypes: [300, 400, 500],
    economyModifiers: { taxRate: 0.8, growthRate: 0.6 },
    compatibleMapSizes: ["small", "medium"],
  },
  {
    id: "victorian_era",
    name: "Victorian Era",
    description: "Build a city in the industrial age",
    era: EraPreset.Victorian,
    startingTreasury: 30_000_00,
    startingPopulation: 200,
    unlockedArchetypes: [100],
    disabledArchetypes: [200, 300, 400, 500],
    economyModifiers: { taxRate: 0.5, growthRate: 0.4 },
    compatibleMapSizes: ["small"],
  },
];

/** Get a start profile by its id */
export function getProfileById(id: string): StartProfile | undefined {
  return START_PROFILES.find(p => p.id === id);
}

/** Get all profiles compatible with a given map size */
export function getProfilesForMapSize(size: string): StartProfile[] {
  return START_PROFILES.filter(p => p.compatibleMapSizes.includes(size));
}

/** Get all profiles for a given era */
export function getProfilesForEra(era: EraPreset): StartProfile[] {
  return START_PROFILES.filter(p => p.era === era);
}

/** Validate that a profile is compatible with a given map size */
export function validateProfileCompatibility(profile: StartProfile, mapSize: string): boolean {
  return profile.compatibleMapSizes.includes(mapSize);
}
