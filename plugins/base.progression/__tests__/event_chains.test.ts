import { describe, it, expect } from "vitest";
import {
  EventChainOutcome,
  SAMPLE_EVENT_CHAINS,
  getEventChainById,
  getNodeById,
  validateChain,
  type EventChainDefinition,
  type EventChainNode,
} from "../index.js";

// ---- getEventChainById ----

describe("getEventChainById", () => {
  it("finds the festival_proposal chain", () => {
    const chain = getEventChainById("festival_proposal");
    expect(chain).toBeDefined();
    expect(chain!.id).toBe("festival_proposal");
    expect(chain!.name).toBe("Festival Proposal");
  });

  it("returns undefined for unknown id", () => {
    const result = getEventChainById("nonexistent_chain");
    expect(result).toBeUndefined();
  });
});

// ---- getNodeById ----

describe("getNodeById", () => {
  it("finds a node by id within a chain", () => {
    const chain = getEventChainById("festival_proposal")!;
    const node = getNodeById(chain, "proposal");
    expect(node).toBeDefined();
    expect(node!.title).toBe("Festival Proposal");
  });

  it("finds a different node in the same chain", () => {
    const chain = getEventChainById("festival_proposal")!;
    const node = getNodeById(chain, "festival_result");
    expect(node).toBeDefined();
    expect(node!.title).toBe("Festival Success");
  });

  it("returns undefined for nonexistent node id", () => {
    const chain = getEventChainById("festival_proposal")!;
    const node = getNodeById(chain, "nonexistent_node");
    expect(node).toBeUndefined();
  });
});

// ---- validateChain ----

describe("validateChain", () => {
  it("validates sample chains as valid", () => {
    for (const chain of SAMPLE_EVENT_CHAINS) {
      const result = validateChain(chain);
      expect(result.valid).toBe(true);
      expect(result.errors).toEqual([]);
    }
  });

  it("fails when startNodeId references nonexistent node", () => {
    const chain: EventChainDefinition = {
      id: "bad_start",
      name: "Bad Start",
      triggerCondition: "population > 0",
      startNodeId: "nonexistent",
      nodes: [
        {
          id: "node1",
          title: "Node 1",
          description: "A node",
          choices: [
            {
              label: "OK",
              nextNodeId: null,
              outcome: EventChainOutcome.Neutral,
              effects: {},
            },
          ],
        },
      ],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("startNodeId"))).toBe(true);
  });

  it("fails when a choice references a nonexistent node", () => {
    const chain: EventChainDefinition = {
      id: "bad_ref",
      name: "Bad Reference",
      triggerCondition: "population > 0",
      startNodeId: "start",
      nodes: [
        {
          id: "start",
          title: "Start",
          description: "Start node",
          choices: [
            {
              label: "Go",
              nextNodeId: "missing_node",
              outcome: EventChainOutcome.Success,
              effects: {},
            },
          ],
        },
      ],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("missing_node"))).toBe(true);
  });

  it("fails when chain has no nodes", () => {
    const chain: EventChainDefinition = {
      id: "empty",
      name: "Empty",
      triggerCondition: "population > 0",
      startNodeId: "start",
      nodes: [],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("no nodes"))).toBe(true);
  });

  it("fails when a node has no choices", () => {
    const chain: EventChainDefinition = {
      id: "no_choices",
      name: "No Choices",
      triggerCondition: "population > 0",
      startNodeId: "start",
      nodes: [
        {
          id: "start",
          title: "Start",
          description: "Start node",
          choices: [],
        },
      ],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("no choices"))).toBe(true);
  });

  it("fails when duplicate node IDs exist", () => {
    const chain: EventChainDefinition = {
      id: "dup_nodes",
      name: "Duplicate Nodes",
      triggerCondition: "population > 0",
      startNodeId: "node1",
      nodes: [
        {
          id: "node1",
          title: "Node 1",
          description: "First",
          choices: [
            {
              label: "OK",
              nextNodeId: null,
              outcome: EventChainOutcome.Neutral,
              effects: {},
            },
          ],
        },
        {
          id: "node1",
          title: "Node 1 Duplicate",
          description: "Second",
          choices: [
            {
              label: "OK",
              nextNodeId: null,
              outcome: EventChainOutcome.Neutral,
              effects: {},
            },
          ],
        },
      ],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.includes("Duplicate"))).toBe(true);
  });

  it("passes a valid multi-node chain with null terminators", () => {
    const chain: EventChainDefinition = {
      id: "valid_chain",
      name: "Valid Chain",
      triggerCondition: "happiness > 50",
      startNodeId: "step1",
      nodes: [
        {
          id: "step1",
          title: "Step 1",
          description: "First step",
          choices: [
            {
              label: "Continue",
              nextNodeId: "step2",
              outcome: EventChainOutcome.Success,
              effects: { happiness: 5 },
            },
            {
              label: "Stop",
              nextNodeId: null,
              outcome: EventChainOutcome.Neutral,
              effects: {},
            },
          ],
        },
        {
          id: "step2",
          title: "Step 2",
          description: "Second step",
          choices: [
            {
              label: "Finish",
              nextNodeId: null,
              outcome: EventChainOutcome.Success,
              effects: { treasury: 1000 },
            },
          ],
        },
      ],
    };
    const result = validateChain(chain);
    expect(result.valid).toBe(true);
    expect(result.errors).toEqual([]);
  });
});

// ---- SAMPLE_EVENT_CHAINS structure ----

describe("SAMPLE_EVENT_CHAINS", () => {
  it("has at least one chain", () => {
    expect(SAMPLE_EVENT_CHAINS.length).toBeGreaterThanOrEqual(1);
  });

  it("all chains have unique ids", () => {
    const ids = SAMPLE_EVENT_CHAINS.map((c) => c.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
  });

  it("all chains have a non-empty triggerCondition", () => {
    for (const chain of SAMPLE_EVENT_CHAINS) {
      expect(chain.triggerCondition.length).toBeGreaterThan(0);
    }
  });

  it("festival_proposal has expected nodes", () => {
    const chain = getEventChainById("festival_proposal")!;
    expect(chain.nodes).toHaveLength(2);
    expect(chain.startNodeId).toBe("proposal");

    const proposalNode = getNodeById(chain, "proposal")!;
    expect(proposalNode.choices).toHaveLength(2);

    // First choice leads to festival_result
    expect(proposalNode.choices[0].nextNodeId).toBe("festival_result");
    // Second choice ends the chain
    expect(proposalNode.choices[1].nextNodeId).toBeNull();
  });

  it("festival_proposal choices have expected effects", () => {
    const chain = getEventChainById("festival_proposal")!;
    const proposalNode = getNodeById(chain, "proposal")!;

    // Approve choice
    const approve = proposalNode.choices[0];
    expect(approve.effects.treasury).toBe(-5000);
    expect(approve.effects.happiness).toBe(10);
    expect(approve.outcome).toBe(EventChainOutcome.Success);

    // Deny choice
    const deny = proposalNode.choices[1];
    expect(deny.effects.happiness).toBe(-5);
    expect(deny.outcome).toBe(EventChainOutcome.Neutral);
  });

  it("all chains pass validation", () => {
    for (const chain of SAMPLE_EVENT_CHAINS) {
      const result = validateChain(chain);
      expect(result.valid).toBe(true);
    }
  });
});

// ---- EventChainOutcome enum ----

describe("EventChainOutcome", () => {
  it("has expected values", () => {
    expect(EventChainOutcome.Success).toBe("success");
    expect(EventChainOutcome.Failure).toBe("failure");
    expect(EventChainOutcome.Neutral).toBe("neutral");
  });
});
