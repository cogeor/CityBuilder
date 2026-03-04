// @townbuilder/base.progression — Scenarios and challenge objectives
// Provides scenario definitions, progress tracking, and objective evaluation.

// ---- Enums ----

export enum ObjectiveType {
  Population = "population",
  Treasury = "treasury",
  Happiness = "happiness",
  Services = "services",
  Traffic = "traffic",
}

export enum ScenarioStatus {
  NotStarted = "not_started",
  InProgress = "in_progress",
  Completed = "completed",
  Failed = "failed",
}

// ---- Interfaces ----

/** A single objective within a scenario */
export interface Objective {
  id: string;
  type: ObjectiveType;
  description: string;
  targetValue: number;
  timeLimit?: number; // ticks, undefined = no limit
}

/** Definition of a scenario (immutable template) */
export interface ScenarioDefinition {
  id: string;
  name: string;
  description: string;
  seed: number;
  mapSize: { width: number; height: number };
  startingTreasury: number;
  objectives: Objective[];
  difficulty: number; // 1-5
}

/** Mutable progress state for an active scenario */
export interface ScenarioProgress {
  scenarioId: string;
  status: ScenarioStatus;
  startTick: number;
  currentTick: number;
  objectiveProgress: Map<string, number>; // objective id -> current value
}

// ---- Predefined Scenarios ----

export const SCENARIOS: ScenarioDefinition[] = [
  {
    id: "tutorial_growth",
    name: "Growing Pains",
    description: "Grow your city to 10,000 population",
    seed: 42,
    mapSize: { width: 128, height: 128 },
    startingTreasury: 100_000_00, // $100,000 in cents
    objectives: [
      {
        id: "pop_10k",
        type: ObjectiveType.Population,
        description: "Reach 10,000 population",
        targetValue: 10000,
      },
    ],
    difficulty: 1,
  },
  {
    id: "budget_crisis",
    name: "Budget Crisis",
    description: "Maintain positive treasury for 1000 ticks starting in debt",
    seed: 123,
    mapSize: { width: 64, height: 64 },
    startingTreasury: -50_000_00,
    objectives: [
      {
        id: "positive_budget",
        type: ObjectiveType.Treasury,
        description: "Maintain positive treasury",
        targetValue: 0,
        timeLimit: 1000,
      },
    ],
    difficulty: 3,
  },
  {
    id: "green_city",
    name: "Green City",
    description: "Achieve 80% happiness with 5000 population",
    seed: 789,
    mapSize: { width: 128, height: 128 },
    startingTreasury: 200_000_00,
    objectives: [
      {
        id: "pop_5k",
        type: ObjectiveType.Population,
        description: "Reach 5,000 population",
        targetValue: 5000,
      },
      {
        id: "happy_80",
        type: ObjectiveType.Happiness,
        description: "Achieve 80% happiness",
        targetValue: 80,
      },
    ],
    difficulty: 4,
  },
];

// ---- Lookup Functions ----

/** Get a scenario definition by its ID */
export function getScenarioById(id: string): ScenarioDefinition | undefined {
  return SCENARIOS.find((s) => s.id === id);
}

// ---- Score Computation ----

/** Compute a score for a completed or in-progress scenario.
 *  Higher scores for faster completion and higher difficulty. */
export function computeScore(
  progress: ScenarioProgress,
  scenario: ScenarioDefinition,
): number {
  const elapsed = progress.currentTick - progress.startTick;
  // Base score from difficulty
  const difficultyBonus = scenario.difficulty * 1000;
  // Time bonus: more points for faster completion (minimum 100 ticks to avoid division issues)
  const effectiveElapsed = Math.max(elapsed, 1);
  const timeBonus = Math.floor(10000 / effectiveElapsed);
  // Objective completion ratio
  let completedCount = 0;
  for (const objective of scenario.objectives) {
    const current = progress.objectiveProgress.get(objective.id) ?? 0;
    if (current >= objective.targetValue) {
      completedCount++;
    }
  }
  const completionRatio =
    scenario.objectives.length > 0
      ? completedCount / scenario.objectives.length
      : 0;
  return Math.floor((difficultyBonus + timeBonus) * completionRatio);
}

// ---- Scenario Manager ----

/** Manages active scenario progress tracking and evaluation */
export class ScenarioManager {
  private progress: Map<string, ScenarioProgress> = new Map();

  /** Start a scenario, creating initial progress state.
   *  Returns null if the scenario ID is not found. */
  startScenario(scenarioId: string, tick: number): ScenarioProgress | null {
    const scenario = getScenarioById(scenarioId);
    if (!scenario) return null;

    const objectiveProgress = new Map<string, number>();
    for (const obj of scenario.objectives) {
      objectiveProgress.set(obj.id, 0);
    }

    const prog: ScenarioProgress = {
      scenarioId,
      status: ScenarioStatus.InProgress,
      startTick: tick,
      currentTick: tick,
      objectiveProgress,
    };

    this.progress.set(scenarioId, prog);
    return prog;
  }

  /** Update progress for a scenario with new metric values.
   *  Returns null if the scenario is not active. */
  updateProgress(
    scenarioId: string,
    metrics: Record<string, number>,
    tick: number,
  ): ScenarioProgress | null {
    const prog = this.progress.get(scenarioId);
    if (!prog) return null;

    prog.currentTick = tick;
    for (const [key, value] of Object.entries(metrics)) {
      if (prog.objectiveProgress.has(key)) {
        prog.objectiveProgress.set(key, value);
      }
    }

    // Re-evaluate status
    prog.status = this.checkCompletion(scenarioId);
    return prog;
  }

  /** Evaluate whether a single objective has been met */
  evaluateObjective(objective: Objective, currentValue: number): boolean {
    return currentValue >= objective.targetValue;
  }

  /** Check overall completion status of a scenario */
  checkCompletion(scenarioId: string): ScenarioStatus {
    const prog = this.progress.get(scenarioId);
    if (!prog) return ScenarioStatus.NotStarted;

    const scenario = getScenarioById(scenarioId);
    if (!scenario) return ScenarioStatus.NotStarted;

    const elapsed = prog.currentTick - prog.startTick;

    // Check if any time-limited objective has expired
    for (const obj of scenario.objectives) {
      if (obj.timeLimit !== undefined && elapsed > obj.timeLimit) {
        const current = prog.objectiveProgress.get(obj.id) ?? 0;
        if (!this.evaluateObjective(obj, current)) {
          return ScenarioStatus.Failed;
        }
      }
    }

    // Check if all objectives are met
    let allMet = true;
    for (const obj of scenario.objectives) {
      const current = prog.objectiveProgress.get(obj.id) ?? 0;
      if (!this.evaluateObjective(obj, current)) {
        allMet = false;
        break;
      }
    }

    if (allMet) return ScenarioStatus.Completed;
    return ScenarioStatus.InProgress;
  }

  /** Get current progress for a scenario, or null if not started */
  getProgress(scenarioId: string): ScenarioProgress | null {
    return this.progress.get(scenarioId) ?? null;
  }

  /** Get all available scenario definitions */
  getAvailableScenarios(): ScenarioDefinition[] {
    return [...SCENARIOS];
  }

  /** Reset progress for a scenario, removing all tracked state */
  reset(scenarioId: string): void {
    this.progress.delete(scenarioId);
  }
}
