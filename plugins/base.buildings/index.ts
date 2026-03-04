// @townbuilder/base.buildings — Base building archetype definitions and helpers
// Provides the 5 MVP building types: Small House, Coal Power Plant, Hospital,
// Corner Shop, and Elementary School.

// ---- Archetype Tag Type ----

/** Archetype tag type */
export type ArchetypeTag =
  | "residential"
  | "commercial"
  | "industrial"
  | "civic"
  | "utility"
  | "power"
  | "health"
  | "education";

// ---- Level Scaling ----

/** Level scaling configuration */
export interface LevelScaling {
  costMultiplier: number[];
  capacityMultiplier: number[];
  upkeepMultiplier: number[];
}

// ---- Archetype Definition ----

/** Archetype definition (matches JSON schema) */
export interface ArchetypeDefinition {
  id: number;
  name: string;
  tags: ArchetypeTag[];
  footprint: { w: number; h: number };
  coverageRatio: number;
  floors: number;
  usableRatio: number;
  baseCost: number;
  baseUpkeepPerTick: number;
  powerDemand: number;
  powerSupply: number;
  waterDemand: number;
  waterSupply: number;
  serviceRadius: number;
  desirabilityRadius: number;
  desirabilityMagnitude: number;
  pollution: number;
  noise: number;
  buildTimeTicks: number;
  maxLevel: number;
  prerequisites: string[];
  livingSpacePerPerson: number;
  workspacePerJob: number;
  levelScaling: LevelScaling;
}

// ---- Archetype Data ----
// Defined inline to avoid JSON import issues in test environments.
// These match the JSON files in archetypes/ exactly.

const smallHouse: ArchetypeDefinition = {
  id: 100,
  name: "Small House",
  tags: ["residential"],
  footprint: { w: 1, h: 1 },
  coverageRatio: 0.5,
  floors: 2,
  usableRatio: 0.8,
  baseCost: 15000,
  baseUpkeepPerTick: 1,
  powerDemand: 5,
  powerSupply: 0,
  waterDemand: 2,
  waterSupply: 0,
  serviceRadius: 0,
  desirabilityRadius: 2,
  desirabilityMagnitude: 1,
  pollution: 0,
  noise: 1,
  buildTimeTicks: 200,
  maxLevel: 3,
  prerequisites: [],
  livingSpacePerPerson: 40,
  workspacePerJob: 0,
  levelScaling: {
    costMultiplier: [1.0, 1.5, 2.0],
    capacityMultiplier: [1.0, 1.5, 2.5],
    upkeepMultiplier: [1.0, 1.3, 1.8],
  },
};

const powerPlant: ArchetypeDefinition = {
  id: 200,
  name: "Coal Power Plant",
  tags: ["utility", "power"],
  footprint: { w: 3, h: 3 },
  coverageRatio: 0.7,
  floors: 2,
  usableRatio: 0.9,
  baseCost: 250000,
  baseUpkeepPerTick: 15,
  powerDemand: 0,
  powerSupply: 500,
  waterDemand: 10,
  waterSupply: 0,
  serviceRadius: 0,
  desirabilityRadius: 8,
  desirabilityMagnitude: -5,
  pollution: 30,
  noise: 20,
  buildTimeTicks: 1000,
  maxLevel: 3,
  prerequisites: [],
  livingSpacePerPerson: 0,
  workspacePerJob: 25,
  levelScaling: {
    costMultiplier: [1.0, 2.0, 3.5],
    capacityMultiplier: [1.0, 1.8, 3.0],
    upkeepMultiplier: [1.0, 1.5, 2.5],
  },
};

const hospital: ArchetypeDefinition = {
  id: 300,
  name: "Hospital",
  tags: ["civic", "health"],
  footprint: { w: 5, h: 5 },
  coverageRatio: 0.6,
  floors: 4,
  usableRatio: 0.75,
  baseCost: 500000,
  baseUpkeepPerTick: 25,
  powerDemand: 50,
  powerSupply: 0,
  waterDemand: 20,
  waterSupply: 0,
  serviceRadius: 30,
  desirabilityRadius: 5,
  desirabilityMagnitude: 3,
  pollution: 0,
  noise: 5,
  buildTimeTicks: 1500,
  maxLevel: 3,
  prerequisites: [],
  livingSpacePerPerson: 0,
  workspacePerJob: 15,
  levelScaling: {
    costMultiplier: [1.0, 2.0, 4.0],
    capacityMultiplier: [1.0, 2.0, 3.5],
    upkeepMultiplier: [1.0, 1.8, 3.0],
  },
};

const shop: ArchetypeDefinition = {
  id: 400,
  name: "Corner Shop",
  tags: ["commercial"],
  footprint: { w: 1, h: 2 },
  coverageRatio: 0.8,
  floors: 1,
  usableRatio: 0.85,
  baseCost: 25000,
  baseUpkeepPerTick: 3,
  powerDemand: 8,
  powerSupply: 0,
  waterDemand: 3,
  waterSupply: 0,
  serviceRadius: 10,
  desirabilityRadius: 3,
  desirabilityMagnitude: 2,
  pollution: 1,
  noise: 3,
  buildTimeTicks: 300,
  maxLevel: 3,
  prerequisites: [],
  livingSpacePerPerson: 0,
  workspacePerJob: 20,
  levelScaling: {
    costMultiplier: [1.0, 1.5, 2.5],
    capacityMultiplier: [1.0, 2.0, 3.0],
    upkeepMultiplier: [1.0, 1.4, 2.0],
  },
};

const school: ArchetypeDefinition = {
  id: 500,
  name: "Elementary School",
  tags: ["civic", "education"],
  footprint: { w: 2, h: 3 },
  coverageRatio: 0.5,
  floors: 2,
  usableRatio: 0.7,
  baseCost: 200000,
  baseUpkeepPerTick: 12,
  powerDemand: 20,
  powerSupply: 0,
  waterDemand: 8,
  waterSupply: 0,
  serviceRadius: 20,
  desirabilityRadius: 4,
  desirabilityMagnitude: 4,
  pollution: 0,
  noise: 10,
  buildTimeTicks: 800,
  maxLevel: 3,
  prerequisites: [],
  livingSpacePerPerson: 0,
  workspacePerJob: 20,
  levelScaling: {
    costMultiplier: [1.0, 1.8, 3.0],
    capacityMultiplier: [1.0, 1.5, 2.5],
    upkeepMultiplier: [1.0, 1.5, 2.2],
  },
};

// ---- Exports ----

/** All base building archetypes */
export const BASE_BUILDINGS: ArchetypeDefinition[] = [
  smallHouse,
  powerPlant,
  hospital,
  shop,
  school,
];

// ---- Query Functions ----

/** Get archetype by ID */
export function getArchetypeById(
  id: number,
): ArchetypeDefinition | undefined {
  return BASE_BUILDINGS.find((a) => a.id === id);
}

/** Get archetypes by tag */
export function getArchetypesByTag(
  tag: ArchetypeTag,
): ArchetypeDefinition[] {
  return BASE_BUILDINGS.filter((a) => a.tags.includes(tag));
}

// ---- Computation Functions ----

/** Compute capacity at a given level */
export function computeCapacity(
  archetype: ArchetypeDefinition,
  level: number,
): number {
  const grossArea =
    archetype.footprint.w *
    archetype.footprint.h *
    256 *
    archetype.coverageRatio *
    archetype.floors;
  const netArea = grossArea * archetype.usableRatio;
  const baseCapacity =
    archetype.livingSpacePerPerson > 0
      ? Math.floor(netArea / archetype.livingSpacePerPerson)
      : archetype.workspacePerJob > 0
        ? Math.floor(netArea / archetype.workspacePerJob)
        : 0;
  const multiplier =
    archetype.levelScaling.capacityMultiplier[
      Math.min(
        level - 1,
        archetype.levelScaling.capacityMultiplier.length - 1,
      )
    ] ?? 1;
  return Math.floor(baseCapacity * multiplier);
}

/** Compute cost at a given level */
export function computeCost(
  archetype: ArchetypeDefinition,
  level: number,
): number {
  const multiplier =
    archetype.levelScaling.costMultiplier[
      Math.min(level - 1, archetype.levelScaling.costMultiplier.length - 1)
    ] ?? 1;
  return Math.floor(archetype.baseCost * multiplier);
}

/** Compute upkeep at a given level */
export function computeUpkeep(
  archetype: ArchetypeDefinition,
  level: number,
): number {
  const multiplier =
    archetype.levelScaling.upkeepMultiplier[
      Math.min(
        level - 1,
        archetype.levelScaling.upkeepMultiplier.length - 1,
      )
    ] ?? 1;
  return Math.floor(archetype.baseUpkeepPerTick * multiplier);
}

// ---- Validation ----

/** Validate an archetype definition */
export function validateArchetype(archetype: ArchetypeDefinition): string[] {
  const errors: string[] = [];
  if (!archetype.id || archetype.id <= 0) errors.push("Invalid id");
  if (!archetype.name) errors.push("Missing name");
  if (archetype.footprint.w <= 0 || archetype.footprint.h <= 0)
    errors.push("Invalid footprint");
  if (archetype.baseCost < 0) errors.push("Negative base cost");
  if (archetype.buildTimeTicks <= 0) errors.push("Invalid build time");
  if (archetype.maxLevel < 1) errors.push("Max level must be >= 1");
  if (archetype.coverageRatio <= 0 || archetype.coverageRatio > 1)
    errors.push("Coverage ratio must be (0, 1]");
  if (archetype.usableRatio <= 0 || archetype.usableRatio > 1)
    errors.push("Usable ratio must be (0, 1]");
  return errors;
}
