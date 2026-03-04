import { describe, it, expect } from "vitest";
import {
  SCENARIOS,
  ScenarioStatus,
  ObjectiveType,
  getScenarioById,
  computeScore,
  ScenarioManager,
  type Objective,
} from "../index.js";

// ---- getScenarioById ----

describe("getScenarioById", () => {
  it("finds existing scenario", () => {
    const scenario = getScenarioById("tutorial_growth");
    expect(scenario).toBeDefined();
    expect(scenario!.id).toBe("tutorial_growth");
    expect(scenario!.name).toBe("Growing Pains");
  });

  it("returns undefined for unknown id", () => {
    const result = getScenarioById("nonexistent_scenario");
    expect(result).toBeUndefined();
  });
});

// ---- ScenarioManager.startScenario ----

describe("ScenarioManager.startScenario", () => {
  it("creates progress for known scenario", () => {
    const manager = new ScenarioManager();
    const progress = manager.startScenario("tutorial_growth", 0);
    expect(progress).not.toBeNull();
    expect(progress!.scenarioId).toBe("tutorial_growth");
    expect(progress!.status).toBe(ScenarioStatus.InProgress);
    expect(progress!.startTick).toBe(0);
    expect(progress!.currentTick).toBe(0);
    expect(progress!.objectiveProgress.get("pop_10k")).toBe(0);
  });

  it("returns null for unknown scenario", () => {
    const manager = new ScenarioManager();
    const progress = manager.startScenario("does_not_exist", 0);
    expect(progress).toBeNull();
  });
});

// ---- ScenarioManager.evaluateObjective ----

describe("ScenarioManager.evaluateObjective", () => {
  const manager = new ScenarioManager();
  const objective: Objective = {
    id: "test_obj",
    type: ObjectiveType.Population,
    description: "Test",
    targetValue: 1000,
  };

  it("returns true when target met", () => {
    expect(manager.evaluateObjective(objective, 1000)).toBe(true);
  });

  it("returns true when target exceeded", () => {
    expect(manager.evaluateObjective(objective, 2000)).toBe(true);
  });

  it("returns false when below target", () => {
    expect(manager.evaluateObjective(objective, 999)).toBe(false);
  });
});

// ---- ScenarioManager.updateProgress ----

describe("ScenarioManager.updateProgress", () => {
  it("updates values correctly", () => {
    const manager = new ScenarioManager();
    manager.startScenario("tutorial_growth", 0);
    const updated = manager.updateProgress(
      "tutorial_growth",
      { pop_10k: 5000 },
      100,
    );
    expect(updated).not.toBeNull();
    expect(updated!.currentTick).toBe(100);
    expect(updated!.objectiveProgress.get("pop_10k")).toBe(5000);
  });

  it("returns null for scenario not started", () => {
    const manager = new ScenarioManager();
    const result = manager.updateProgress("tutorial_growth", {}, 0);
    expect(result).toBeNull();
  });
});

// ---- ScenarioManager.checkCompletion ----

describe("ScenarioManager.checkCompletion", () => {
  it("returns Completed when all objectives met", () => {
    const manager = new ScenarioManager();
    manager.startScenario("tutorial_growth", 0);
    manager.updateProgress("tutorial_growth", { pop_10k: 10000 }, 500);
    const status = manager.checkCompletion("tutorial_growth");
    expect(status).toBe(ScenarioStatus.Completed);
  });

  it("returns Failed when time limit exceeded and objective not met", () => {
    const manager = new ScenarioManager();
    manager.startScenario("budget_crisis", 0);
    // positive_budget has timeLimit 1000, target 0 (must reach >= 0)
    // Progress stays at default (0), but we set it below target to fail
    manager.updateProgress("budget_crisis", { positive_budget: -1 }, 1001);
    const status = manager.checkCompletion("budget_crisis");
    expect(status).toBe(ScenarioStatus.Failed);
  });

  it("returns InProgress when ongoing", () => {
    const manager = new ScenarioManager();
    manager.startScenario("tutorial_growth", 0);
    manager.updateProgress("tutorial_growth", { pop_10k: 500 }, 100);
    const status = manager.checkCompletion("tutorial_growth");
    expect(status).toBe(ScenarioStatus.InProgress);
  });
});

// ---- computeScore ----

describe("computeScore", () => {
  it("higher for faster completion", () => {
    const scenario = getScenarioById("tutorial_growth")!;

    // Fast completion at tick 100
    const fastProgress = {
      scenarioId: "tutorial_growth",
      status: ScenarioStatus.Completed,
      startTick: 0,
      currentTick: 100,
      objectiveProgress: new Map([["pop_10k", 10000]]),
    };

    // Slow completion at tick 1000
    const slowProgress = {
      scenarioId: "tutorial_growth",
      status: ScenarioStatus.Completed,
      startTick: 0,
      currentTick: 1000,
      objectiveProgress: new Map([["pop_10k", 10000]]),
    };

    const fastScore = computeScore(fastProgress, scenario);
    const slowScore = computeScore(slowProgress, scenario);
    expect(fastScore).toBeGreaterThan(slowScore);
  });

  it("returns 0 for no objectives met", () => {
    const scenario = getScenarioById("tutorial_growth")!;
    const progress = {
      scenarioId: "tutorial_growth",
      status: ScenarioStatus.InProgress,
      startTick: 0,
      currentTick: 100,
      objectiveProgress: new Map([["pop_10k", 0]]),
    };
    const score = computeScore(progress, scenario);
    expect(score).toBe(0);
  });
});

// ---- ScenarioManager.reset ----

describe("ScenarioManager.reset", () => {
  it("clears progress", () => {
    const manager = new ScenarioManager();
    manager.startScenario("tutorial_growth", 0);
    expect(manager.getProgress("tutorial_growth")).not.toBeNull();
    manager.reset("tutorial_growth");
    expect(manager.getProgress("tutorial_growth")).toBeNull();
  });
});

// ---- ScenarioManager.getAvailableScenarios ----

describe("ScenarioManager.getAvailableScenarios", () => {
  it("returns all scenarios", () => {
    const manager = new ScenarioManager();
    const scenarios = manager.getAvailableScenarios();
    expect(scenarios).toHaveLength(3);
    const ids = scenarios.map((s) => s.id);
    expect(ids).toContain("tutorial_growth");
    expect(ids).toContain("budget_crisis");
    expect(ids).toContain("green_city");
  });
});

// ---- Multiple objectives ----

describe("multiple objectives", () => {
  it("all must be met for completion", () => {
    const manager = new ScenarioManager();
    manager.startScenario("green_city", 0);

    // Only one objective met
    manager.updateProgress(
      "green_city",
      { pop_5k: 5000, happy_80: 50 },
      500,
    );
    expect(manager.checkCompletion("green_city")).toBe(
      ScenarioStatus.InProgress,
    );

    // Both objectives met
    manager.updateProgress(
      "green_city",
      { pop_5k: 6000, happy_80: 85 },
      800,
    );
    expect(manager.checkCompletion("green_city")).toBe(
      ScenarioStatus.Completed,
    );
  });
});

// ---- SCENARIOS array ----

describe("SCENARIOS", () => {
  it("has 3 entries", () => {
    expect(SCENARIOS).toHaveLength(3);
  });

  it("all scenarios have unique ids", () => {
    const ids = SCENARIOS.map((s) => s.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });

  it("all scenarios have valid difficulty 1-5", () => {
    for (const s of SCENARIOS) {
      expect(s.difficulty).toBeGreaterThanOrEqual(1);
      expect(s.difficulty).toBeLessThanOrEqual(5);
    }
  });
});
