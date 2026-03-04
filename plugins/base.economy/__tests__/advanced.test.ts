import { describe, it, expect } from "vitest";
import {
  SUPPLY_CHAIN_NODES,
  getNodeById,
  getNodeDependencies,
  validateSupplyChain,
} from "../advanced.js";
import type { SupplyChainNode } from "../advanced.js";

// ─── SUPPLY_CHAIN_NODES ─────────────────────────────────────────────────────

describe("SUPPLY_CHAIN_NODES", () => {
  it("has 3 entries", () => {
    expect(SUPPLY_CHAIN_NODES).toHaveLength(3);
  });

  it("all nodes have unique ids", () => {
    const ids = SUPPLY_CHAIN_NODES.map(n => n.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("all nodes have positive processingTicks and requiredWorkers", () => {
    for (const node of SUPPLY_CHAIN_NODES) {
      expect(node.processingTicks).toBeGreaterThan(0);
      expect(node.requiredWorkers).toBeGreaterThan(0);
    }
  });
});

// ─── getNodeById ────────────────────────────────────────────────────────────

describe("getNodeById", () => {
  it("finds existing node", () => {
    const node = getNodeById("raw_materials");
    expect(node).toBeDefined();
    expect(node!.name).toBe("Raw Materials");
    expect(node!.outputResource).toBe("raw");
  });

  it("returns undefined for unknown id", () => {
    expect(getNodeById("nonexistent")).toBeUndefined();
  });

  it("finds manufacturing node with correct inputs", () => {
    const node = getNodeById("manufacturing");
    expect(node).toBeDefined();
    expect(node!.inputResources).toEqual(["raw"]);
    expect(node!.outputResource).toBe("goods");
  });
});

// ─── getNodeDependencies ────────────────────────────────────────────────────

describe("getNodeDependencies", () => {
  it("returns empty array for node with no inputs", () => {
    const deps = getNodeDependencies("raw_materials");
    expect(deps).toEqual([]);
  });

  it("returns input resources for manufacturing", () => {
    const deps = getNodeDependencies("manufacturing");
    expect(deps).toEqual(["raw"]);
  });

  it("returns empty array for unknown node", () => {
    const deps = getNodeDependencies("nonexistent");
    expect(deps).toEqual([]);
  });
});

// ─── validateSupplyChain ────────────────────────────────────────────────────

describe("validateSupplyChain", () => {
  it("passes for valid default chain", () => {
    const result = validateSupplyChain(SUPPLY_CHAIN_NODES);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("detects missing input references", () => {
    const broken: SupplyChainNode[] = [
      { id: "retail", name: "Retail", inputResources: ["goods"], outputResource: "consumer_goods", processingTicks: 5, requiredWorkers: 3 },
    ];
    const result = validateSupplyChain(broken);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
    expect(result.errors[0]).toContain("goods");
  });

  it("detects invalid processingTicks", () => {
    const bad: SupplyChainNode[] = [
      { id: "bad", name: "Bad", inputResources: [], outputResource: "x", processingTicks: 0, requiredWorkers: 1 },
    ];
    const result = validateSupplyChain(bad);
    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("processingTicks");
  });

  it("detects invalid requiredWorkers", () => {
    const bad: SupplyChainNode[] = [
      { id: "bad", name: "Bad", inputResources: [], outputResource: "x", processingTicks: 5, requiredWorkers: -1 },
    ];
    const result = validateSupplyChain(bad);
    expect(result.valid).toBe(false);
    expect(result.errors[0]).toContain("requiredWorkers");
  });

  it("passes for empty chain", () => {
    const result = validateSupplyChain([]);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });
});
