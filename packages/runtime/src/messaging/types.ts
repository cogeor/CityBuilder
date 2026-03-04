// @townbuilder/runtime — Worker messaging protocol types
// Defines the wire format for main-thread <-> Web Worker communication.

// ---- Wire Version ----

/** Current wire protocol version. Bump on breaking changes. */
export const WIRE_VERSION = 1;

// ---- MessageType Enum ----

/** Discriminator for all worker messages. */
export enum MessageType {
  HANDSHAKE = "HANDSHAKE",
  HANDSHAKE_ACK = "HANDSHAKE_ACK",

  COMMAND = "COMMAND",
  COMMAND_RESULT = "COMMAND_RESULT",

  TICK_OUTPUT = "TICK_OUTPUT",

  ENTITY_DIFF = "ENTITY_DIFF",
  HEATMAP_DIFF = "HEATMAP_DIFF",
  BUDGET_DIFF = "BUDGET_DIFF",

  EVENT_NOTIFICATION = "EVENT_NOTIFICATION",

  CHUNK_DIRTY_LIST = "CHUNK_DIRTY_LIST",
  DYNAMIC_INSTANCE_BUFFER = "DYNAMIC_INSTANCE_BUFFER",
  OVERLAY_UPDATE = "OVERLAY_UPDATE",

  SAVE_REQUEST = "SAVE_REQUEST",
  SAVE_RESPONSE = "SAVE_RESPONSE",
  LOAD_REQUEST = "LOAD_REQUEST",
  LOAD_RESPONSE = "LOAD_RESPONSE",

  SET_SPEED = "SET_SPEED",
  PAUSE = "PAUSE",
  RESUME = "RESUME",

  PICK_REQUEST = "PICK_REQUEST",
  PICK_RESPONSE = "PICK_RESPONSE",

  CAMERA_UPDATE = "CAMERA_UPDATE",
}

// ---- Message Interfaces ----

/** Initial handshake sent from main thread to worker. */
export interface HandshakeMessage {
  readonly type: MessageType.HANDSHAKE;
  readonly wire_version: number;
  readonly timestamp: number;
}

/** Handshake acknowledgement sent from worker to main thread. */
export interface HandshakeAckMessage {
  readonly type: MessageType.HANDSHAKE_ACK;
  readonly wire_version: number;
  readonly timestamp: number;
}

/** Wraps a JSON-serialised command for the simulation. */
export interface CommandEnvelope {
  readonly type: MessageType.COMMAND;
  readonly command_json: string;
  readonly sequence_id: number;
}

/** Result of processing a command. */
export interface CommandResultMessage {
  readonly type: MessageType.COMMAND_RESULT;
  readonly success: boolean;
  readonly error?: string;
  readonly sequence_id: number;
}

/** Output produced by a single simulation tick. */
export interface TickOutputMessage {
  readonly type: MessageType.TICK_OUTPUT;
  readonly tick: number;
  readonly events_json: string;
  readonly population: number;
  readonly treasury: number;
}

/** A single field change on one entity. */
export interface EntityDiffEntry {
  readonly handle: { readonly index: number; readonly generation: number };
  readonly field: string;
  readonly old_value: number;
  readonly new_value: number;
}

/** Batch of entity field diffs. */
export interface EntityDiffMessage {
  readonly type: MessageType.ENTITY_DIFF;
  readonly diffs: ReadonlyArray<EntityDiffEntry>;
}

/** Updated heatmap data for a specific overlay. */
export interface HeatmapDiffMessage {
  readonly type: MessageType.HEATMAP_DIFF;
  readonly map_type: string;
  readonly data: Uint16Array;
  readonly width: number;
  readonly height: number;
}

/** Summary budget figures after a tick. */
export interface BudgetDiffMessage {
  readonly type: MessageType.BUDGET_DIFF;
  readonly income: number;
  readonly expenses: number;
  readonly net: number;
  readonly treasury: number;
}

/** A single event notification entry. */
export interface EventNotificationEntry {
  readonly tick: number;
  readonly event_type: string;
  readonly data: Record<string, unknown>;
}

/** Batch of simulation events for the UI. */
export interface EventNotificationMessage {
  readonly type: MessageType.EVENT_NOTIFICATION;
  readonly events: ReadonlyArray<EventNotificationEntry>;
}

/** List of dirty chunk indices that need re-rendering. */
export interface ChunkDirtyListMessage {
  readonly type: MessageType.CHUNK_DIRTY_LIST;
  readonly chunk_indices: ReadonlyArray<number>;
}

/** Binary instance buffer for dynamic rendering. */
export interface DynamicInstanceBufferMessage {
  readonly type: MessageType.DYNAMIC_INSTANCE_BUFFER;
  readonly buffer: ArrayBuffer;
  readonly count: number;
}

/** Overlay update for a map layer. */
export interface OverlayUpdateMessage {
  readonly type: MessageType.OVERLAY_UPDATE;
  readonly overlay_name: string;
  readonly data: Uint8Array;
  readonly width: number;
  readonly height: number;
}

/** Request to save the game state. */
export interface SaveRequestMessage {
  readonly type: MessageType.SAVE_REQUEST;
  readonly slot: string;
}

/** Response with serialised save data. */
export interface SaveResponseMessage {
  readonly type: MessageType.SAVE_RESPONSE;
  readonly success: boolean;
  readonly data?: string;
  readonly error?: string;
}

/** Request to load a saved game. */
export interface LoadRequestMessage {
  readonly type: MessageType.LOAD_REQUEST;
  readonly slot: string;
  readonly data: string;
}

/** Response after loading a saved game. */
export interface LoadResponseMessage {
  readonly type: MessageType.LOAD_RESPONSE;
  readonly success: boolean;
  readonly error?: string;
}

/** Set simulation speed: 0=pause, 1=1x, 2=2x, 4=4x. */
export interface SetSpeedMessage {
  readonly type: MessageType.SET_SPEED;
  readonly speed: number;
}

/** Pause the simulation. */
export interface PauseMessage {
  readonly type: MessageType.PAUSE;
}

/** Resume the simulation. */
export interface ResumeMessage {
  readonly type: MessageType.RESUME;
}

/** Pick request with screen coordinates. */
export interface PickRequestMessage {
  readonly type: MessageType.PICK_REQUEST;
  readonly screen_x: number;
  readonly screen_y: number;
}

/** Pick response with optional entity/tile hit. */
export interface PickResponseMessage {
  readonly type: MessageType.PICK_RESPONSE;
  readonly entity?: { readonly index: number; readonly generation: number };
  readonly tile?: { readonly x: number; readonly y: number };
}

/** Camera position update from main thread. */
export interface CameraUpdateMessage {
  readonly type: MessageType.CAMERA_UPDATE;
  readonly x: number;
  readonly y: number;
  readonly zoom: number;
}

// ---- Union Type ----

/** Discriminated union of all worker messages. */
export type WorkerMessage =
  | HandshakeMessage
  | HandshakeAckMessage
  | CommandEnvelope
  | CommandResultMessage
  | TickOutputMessage
  | EntityDiffMessage
  | HeatmapDiffMessage
  | BudgetDiffMessage
  | EventNotificationMessage
  | ChunkDirtyListMessage
  | DynamicInstanceBufferMessage
  | OverlayUpdateMessage
  | SaveRequestMessage
  | SaveResponseMessage
  | LoadRequestMessage
  | LoadResponseMessage
  | SetSpeedMessage
  | PauseMessage
  | ResumeMessage
  | PickRequestMessage
  | PickResponseMessage
  | CameraUpdateMessage;

// ---- Helper Functions ----

/** Create a HandshakeMessage with the current wire version and timestamp. */
export function createHandshake(): HandshakeMessage {
  return {
    type: MessageType.HANDSHAKE,
    wire_version: WIRE_VERSION,
    timestamp: Date.now(),
  };
}
