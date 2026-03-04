// @townbuilder/runtime — workers barrel export
export {
  // Enums
  SimSpeed,

  // Interfaces
  type TickResult,
  type SimWorkerState,

  // Constants
  TICK_INTERVAL_MS,

  // Classes
  SimWorker,
  MockGameHandle,

  // Utilities (exported for testing)
  _uint8ArrayToBase64,
  _base64ToUint8Array,
} from "./sim.worker.js";

export {
  // Enums
  RenderWorkerState,
  GPUBackendType,

  // Interfaces
  type RenderCamera,
  type RenderWorkerConfig,
  type PickRequest,
  type PickResult,
  type RenderFrameStats,

  // Constants
  DEFAULT_RENDER_CONFIG,

  // Classes
  RenderWorker,
} from "./render.worker.js";

export {
  // Enums
  WorkerLifecycleState,

  // Interfaces
  type StateTransition,

  // Types
  type TransitionGuard,

  // Constants
  VALID_TRANSITIONS,

  // Classes
  WorkerStateMachine,
} from "./state_machine.js";
