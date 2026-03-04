// @townbuilder/base.progression — Barrel export
export {
  ObjectiveType,
  ScenarioStatus,
  type Objective,
  type ScenarioDefinition,
  type ScenarioProgress,
  SCENARIOS,
  getScenarioById,
  computeScore,
  ScenarioManager,
} from "./scenarios.js";

export {
  EventChainOutcome,
  type EventChainChoice,
  type EventChainNode,
  type EventChainDefinition,
  SAMPLE_EVENT_CHAINS,
  getEventChainById,
  getNodeById,
  validateChain,
} from "./event_chains.js";
