// @townbuilder/runtime — messaging barrel export
export {
  // Constants
  WIRE_VERSION,

  // Enum
  MessageType,

  // Message interfaces
  type HandshakeMessage,
  type HandshakeAckMessage,
  type CommandEnvelope,
  type CommandResultMessage,
  type TickOutputMessage,
  type EntityDiffEntry,
  type EntityDiffMessage,
  type HeatmapDiffMessage,
  type BudgetDiffMessage,
  type EventNotificationEntry,
  type EventNotificationMessage,
  type ChunkDirtyListMessage,
  type DynamicInstanceBufferMessage,
  type OverlayUpdateMessage,
  type SaveRequestMessage,
  type SaveResponseMessage,
  type LoadRequestMessage,
  type LoadResponseMessage,
  type SetSpeedMessage,
  type PauseMessage,
  type ResumeMessage,
  type PickRequestMessage,
  type PickResponseMessage,
  type CameraUpdateMessage,

  // Union type
  type WorkerMessage,

  // Helper functions
  createHandshake,
} from "./types.js";

// Worker manager
export {
  // Classes
  WorkerManager,
  MockWorker,

  // Interfaces
  type WorkerManagerState,
  type IWorkerManager,
} from "./worker_manager.js";

// Event hub — observer-pattern subscriptions
export { EventHub, type EventSubscription } from "./event_hub.js";
