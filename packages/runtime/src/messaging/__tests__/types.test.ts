import { describe, it, expect } from "vitest";
import {
  WIRE_VERSION,
  MessageType,
  createHandshake,
  type CommandEnvelope,
  type TickOutputMessage,
  type WorkerMessage,
  type HandshakeMessage,
  type EntityDiffMessage,
  type PickResponseMessage,
  type SetSpeedMessage,
  type BudgetDiffMessage,
  type CameraUpdateMessage,
} from "../types.js";

describe("messaging/types", () => {
  it("WIRE_VERSION is 1", () => {
    expect(WIRE_VERSION).toBe(1);
  });

  it("createHandshake returns a HandshakeMessage with correct fields", () => {
    const before = Date.now();
    const msg = createHandshake();
    const after = Date.now();

    expect(msg.type).toBe(MessageType.HANDSHAKE);
    expect(msg.wire_version).toBe(WIRE_VERSION);
    expect(msg.timestamp).toBeGreaterThanOrEqual(before);
    expect(msg.timestamp).toBeLessThanOrEqual(after);
  });

  it("MessageType enum has all expected values", () => {
    const expected = [
      "HANDSHAKE",
      "HANDSHAKE_ACK",
      "COMMAND",
      "COMMAND_RESULT",
      "TICK_OUTPUT",
      "ENTITY_DIFF",
      "HEATMAP_DIFF",
      "BUDGET_DIFF",
      "EVENT_NOTIFICATION",
      "CHUNK_DIRTY_LIST",
      "DYNAMIC_INSTANCE_BUFFER",
      "OVERLAY_UPDATE",
      "SAVE_REQUEST",
      "SAVE_RESPONSE",
      "LOAD_REQUEST",
      "LOAD_RESPONSE",
      "SET_SPEED",
      "PAUSE",
      "RESUME",
      "PICK_REQUEST",
      "PICK_RESPONSE",
      "CAMERA_UPDATE",
    ];

    for (const key of expected) {
      expect(MessageType[key as keyof typeof MessageType]).toBe(key);
    }

    // Verify total count: string enums have one entry per member (no reverse mapping)
    const enumKeys = Object.keys(MessageType);
    expect(enumKeys.length).toBe(expected.length);
  });

  it("CommandEnvelope is constructable with correct shape", () => {
    const cmd: CommandEnvelope = {
      type: MessageType.COMMAND,
      command_json: '{"action":"zone","x":10,"y":20}',
      sequence_id: 42,
    };

    expect(cmd.type).toBe(MessageType.COMMAND);
    expect(cmd.command_json).toContain("zone");
    expect(cmd.sequence_id).toBe(42);
  });

  it("TickOutputMessage is constructable with correct shape", () => {
    const tick: TickOutputMessage = {
      type: MessageType.TICK_OUTPUT,
      tick: 100,
      events_json: "[]",
      population: 5000,
      treasury: 100000,
    };

    expect(tick.type).toBe(MessageType.TICK_OUTPUT);
    expect(tick.tick).toBe(100);
    expect(tick.events_json).toBe("[]");
    expect(tick.population).toBe(5000);
    expect(tick.treasury).toBe(100000);
  });

  it("WorkerMessage union accepts all message types", () => {
    // Verify the discriminated union works via type narrowing
    const handshake: WorkerMessage = createHandshake();
    expect(handshake.type).toBe(MessageType.HANDSHAKE);

    const cmd: WorkerMessage = {
      type: MessageType.COMMAND,
      command_json: "{}",
      sequence_id: 1,
    };
    expect(cmd.type).toBe(MessageType.COMMAND);

    const tick: WorkerMessage = {
      type: MessageType.TICK_OUTPUT,
      tick: 1,
      events_json: "[]",
      population: 0,
      treasury: 0,
    };
    expect(tick.type).toBe(MessageType.TICK_OUTPUT);

    // Type narrowing test
    if (handshake.type === MessageType.HANDSHAKE) {
      expect(handshake.wire_version).toBe(WIRE_VERSION);
    }
  });

  it("EntityDiffMessage holds diffs array correctly", () => {
    const msg: EntityDiffMessage = {
      type: MessageType.ENTITY_DIFF,
      diffs: [
        {
          handle: { index: 0, generation: 1 },
          field: "health",
          old_value: 100,
          new_value: 80,
        },
        {
          handle: { index: 5, generation: 2 },
          field: "population",
          old_value: 10,
          new_value: 15,
        },
      ],
    };

    expect(msg.type).toBe(MessageType.ENTITY_DIFF);
    expect(msg.diffs).toHaveLength(2);
    expect(msg.diffs[0].handle.index).toBe(0);
    expect(msg.diffs[0].field).toBe("health");
    expect(msg.diffs[1].new_value).toBe(15);
  });

  it("PickResponseMessage supports optional entity and tile", () => {
    const emptyPick: PickResponseMessage = {
      type: MessageType.PICK_RESPONSE,
    };
    expect(emptyPick.entity).toBeUndefined();
    expect(emptyPick.tile).toBeUndefined();

    const entityPick: PickResponseMessage = {
      type: MessageType.PICK_RESPONSE,
      entity: { index: 3, generation: 1 },
    };
    expect(entityPick.entity?.index).toBe(3);

    const tilePick: PickResponseMessage = {
      type: MessageType.PICK_RESPONSE,
      tile: { x: 10, y: 20 },
    };
    expect(tilePick.tile?.x).toBe(10);
    expect(tilePick.tile?.y).toBe(20);
  });

  it("SetSpeedMessage accepts valid speed values", () => {
    const speeds = [0, 1, 2, 4];
    for (const speed of speeds) {
      const msg: SetSpeedMessage = {
        type: MessageType.SET_SPEED,
        speed,
      };
      expect(msg.type).toBe(MessageType.SET_SPEED);
      expect(msg.speed).toBe(speed);
    }
  });

  it("BudgetDiffMessage has all financial fields", () => {
    const msg: BudgetDiffMessage = {
      type: MessageType.BUDGET_DIFF,
      income: 5000,
      expenses: 3000,
      net: 2000,
      treasury: 50000,
    };

    expect(msg.income).toBe(5000);
    expect(msg.expenses).toBe(3000);
    expect(msg.net).toBe(2000);
    expect(msg.treasury).toBe(50000);
  });

  it("CameraUpdateMessage has position and zoom", () => {
    const msg: CameraUpdateMessage = {
      type: MessageType.CAMERA_UPDATE,
      x: 100.5,
      y: 200.3,
      zoom: 1.5,
    };

    expect(msg.type).toBe(MessageType.CAMERA_UPDATE);
    expect(msg.x).toBeCloseTo(100.5);
    expect(msg.y).toBeCloseTo(200.3);
    expect(msg.zoom).toBeCloseTo(1.5);
  });
});
