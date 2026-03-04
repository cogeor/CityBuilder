// @townbuilder/base.progression — Event chain definitions and validation
// Provides branching narrative event chains triggered by city conditions,
// with choices, outcomes, and metric effects.

// ---- Enums ----

/** Outcome classification for an event chain choice. */
export enum EventChainOutcome {
  Success = "success",
  Failure = "failure",
  Neutral = "neutral",
}

// ---- Interfaces ----

/** A single choice within an event chain node. */
export interface EventChainChoice {
  /** Display label for this choice. */
  label: string;
  /** ID of the next node to advance to, or null to end the chain. */
  nextNodeId: string | null;
  /** Outcome classification of this choice. */
  outcome: EventChainOutcome;
  /** Metric modifiers applied when this choice is selected. */
  effects: Record<string, number>;
}

/** A node in an event chain, presenting a situation and choices. */
export interface EventChainNode {
  /** Unique identifier within the chain. */
  id: string;
  /** Display title for this event node. */
  title: string;
  /** Narrative description of the situation. */
  description: string;
  /** Available choices the player can make. */
  choices: EventChainChoice[];
  /** If set, auto-advance to next node after this many ticks. */
  autoAdvanceTicks?: number;
}

/** Full definition of an event chain with trigger conditions and node graph. */
export interface EventChainDefinition {
  /** Unique identifier for this event chain. */
  id: string;
  /** Display name for the event chain. */
  name: string;
  /** Simple expression describing when this chain triggers, e.g. "population > 5000". */
  triggerCondition: string;
  /** ID of the first node in the chain. */
  startNodeId: string;
  /** All nodes in this event chain. */
  nodes: EventChainNode[];
}

// ---- Sample Data ----

/** Predefined sample event chains for the base progression pack. */
export const SAMPLE_EVENT_CHAINS: EventChainDefinition[] = [
  {
    id: "festival_proposal",
    name: "Festival Proposal",
    triggerCondition: "population > 1000",
    startNodeId: "proposal",
    nodes: [
      {
        id: "proposal",
        title: "Festival Proposal",
        description: "Citizens want to host a city festival",
        choices: [
          {
            label: "Approve ($5000)",
            nextNodeId: "festival_result",
            outcome: EventChainOutcome.Success,
            effects: { treasury: -5000, happiness: 10 },
          },
          {
            label: "Deny",
            nextNodeId: null,
            outcome: EventChainOutcome.Neutral,
            effects: { happiness: -5 },
          },
        ],
      },
      {
        id: "festival_result",
        title: "Festival Success",
        description: "The festival was a hit!",
        choices: [
          {
            label: "Great!",
            nextNodeId: null,
            outcome: EventChainOutcome.Success,
            effects: { happiness: 5, population: 100 },
          },
        ],
      },
    ],
  },
];

// ---- Lookup Functions ----

/** Find an event chain definition by its ID from the sample set. */
export function getEventChainById(
  id: string,
): EventChainDefinition | undefined {
  return SAMPLE_EVENT_CHAINS.find((chain) => chain.id === id);
}

/** Find a node within an event chain by its node ID. */
export function getNodeById(
  chain: EventChainDefinition,
  nodeId: string,
): EventChainNode | undefined {
  return chain.nodes.find((node) => node.id === nodeId);
}

// ---- Validation ----

/**
 * Validate an event chain definition for structural correctness.
 * Checks:
 * - startNodeId references an existing node
 * - All choice nextNodeId references point to existing nodes (or null)
 * - At least one node exists
 * - All nodes have at least one choice
 * - No duplicate node IDs
 */
export function validateChain(
  chain: EventChainDefinition,
): { valid: boolean; errors: string[] } {
  const errors: string[] = [];
  const nodeIds = new Set<string>();

  // Check for at least one node
  if (chain.nodes.length === 0) {
    errors.push("Chain has no nodes");
    return { valid: false, errors };
  }

  // Check for duplicate node IDs
  for (const node of chain.nodes) {
    if (nodeIds.has(node.id)) {
      errors.push(`Duplicate node ID: "${node.id}"`);
    }
    nodeIds.add(node.id);
  }

  // Check startNodeId references an existing node
  if (!nodeIds.has(chain.startNodeId)) {
    errors.push(
      `startNodeId "${chain.startNodeId}" does not reference an existing node`,
    );
  }

  // Check each node
  for (const node of chain.nodes) {
    // Each node must have at least one choice
    if (node.choices.length === 0) {
      errors.push(`Node "${node.id}" has no choices`);
    }

    // Check choice nextNodeId references
    for (const choice of node.choices) {
      if (choice.nextNodeId !== null && !nodeIds.has(choice.nextNodeId)) {
        errors.push(
          `Choice "${choice.label}" in node "${node.id}" references nonexistent node "${choice.nextNodeId}"`,
        );
      }
    }
  }

  return { valid: errors.length === 0, errors };
}
