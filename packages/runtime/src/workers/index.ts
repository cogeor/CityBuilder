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
