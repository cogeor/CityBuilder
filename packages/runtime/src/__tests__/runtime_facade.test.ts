// @townbuilder/runtime — Tests for RuntimeFacade
import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  RuntimeFacade,
  RuntimeState,
  type RuntimeConfig,
} from "../runtime_facade.js";
import type { EngineCommand } from "../engine/commands.js";
import { translateToolInteraction } from "../engine/interaction_bridge.js";

// ---- Helpers ----

/** Create a minimal RuntimeConfig for testing. */
function makeConfig(overrides?: Partial<RuntimeConfig>): RuntimeConfig {
  return {
    mapWidth: 64,
    mapHeight: 64,
    seed: 42,
    ...overrides,
  };
}

function makeCommand(kind: "place" | "demolish" | "zone"): EngineCommand {
  if (kind === "place") {
    return {
      PlaceEntity: {
        archetype_id: 100,
        x: 10,
        y: 20,
        rotation: 0,
      },
    };
  }
  if (kind === "demolish") {
    return {
      Bulldoze: {
        x: 1,
        y: 2,
        w: 1,
        h: 1,
      },
    };
  }
  return {
    SetZoning: {
      x: 0,
      y: 0,
      w: 2,
      h: 2,
      zone: "Residential",
    },
  };
}

// ---- RuntimeFacade Tests ----

describe("RuntimeFacade", () => {
  let facade: RuntimeFacade;

  beforeEach(() => {
    facade = new RuntimeFacade(makeConfig());
  });

  // ---- Test 1: starts in Uninitialized state ----
  it("starts in Uninitialized state", () => {
    expect(facade.getState()).toBe(RuntimeState.Uninitialized);
  });

  // ---- Test 2: start transitions to Running ----
  it("start transitions to Running", async () => {
    await facade.start();
    expect(facade.getState()).toBe(RuntimeState.Running);
  });

  // ---- Test 3: shutdown transitions to Stopped ----
  it("shutdown transitions to Stopped", async () => {
    await facade.start();
    await facade.shutdown();
    expect(facade.getState()).toBe(RuntimeState.Stopped);
  });

  // ---- Test 4: sendCommand adds to history ----
  it("sendCommand adds to command history", async () => {
    await facade.start();

    facade.sendCommand(makeCommand("place"));

    const history = facade.getCommandHistory();
    expect(history.getUndoCount()).toBe(1);
    expect(history.canUndo()).toBe(true);
  });

  // ---- Test 5: undo delegates to CommandHistory ----
  it("undo delegates to CommandHistory", async () => {
    await facade.start();

    facade.sendCommand(makeCommand("place"));
    expect(facade.getCommandHistory().getUndoCount()).toBe(1);

    const undone = facade.undo();
    expect(undone).not.toBeNull();
    expect(undone!.type).toBe("PlaceEntity");
    expect(facade.getCommandHistory().getUndoCount()).toBe(0);
    expect(facade.getCommandHistory().getRedoCount()).toBe(1);
  });

  // ---- Test 6: redo delegates to CommandHistory ----
  it("redo delegates to CommandHistory", async () => {
    await facade.start();

    facade.sendCommand(makeCommand("place"));
    facade.undo();

    const redone = facade.redo();
    expect(redone).not.toBeNull();
    expect(redone!.type).toBe("PlaceEntity");
    expect(facade.getCommandHistory().getUndoCount()).toBe(1);
    expect(facade.getCommandHistory().getRedoCount()).toBe(0);
  });

  // ---- Test 7: setSpeed doesn't crash without workers ----
  it("setSpeed does not throw when no workers are set", async () => {
    await facade.start();
    expect(() => facade.setSpeed(2)).not.toThrow();
    expect(() => facade.setSpeed(0)).not.toThrow();
  });

  // ---- Test 8: getState returns current state ----
  it("getState returns the current lifecycle state at each point", async () => {
    expect(facade.getState()).toBe(RuntimeState.Uninitialized);

    await facade.start();
    expect(facade.getState()).toBe(RuntimeState.Running);

    await facade.shutdown();
    expect(facade.getState()).toBe(RuntimeState.Stopped);
  });

  // ---- Test 9: onStateChange fires on transition ----
  it("onStateChange fires on every state transition", async () => {
    const states: RuntimeState[] = [];
    facade.onStateChange((state) => states.push(state));

    await facade.start();
    expect(states).toEqual([RuntimeState.Starting, RuntimeState.Running]);

    await facade.shutdown();
    expect(states).toEqual([
      RuntimeState.Starting,
      RuntimeState.Running,
      RuntimeState.ShuttingDown,
      RuntimeState.Stopped,
    ]);
  });

  // ---- Test 10: can't start when already running ----
  it("throws when starting an already running runtime", async () => {
    await facade.start();

    await expect(facade.start()).rejects.toThrow(
      /Cannot start.*Running.*Uninitialized/,
    );
  });

  // ---- Test 11: can't send commands when not running ----
  it("throws when sending a command while not running", () => {
    expect(() => facade.sendCommand(makeCommand("place"))).toThrow(
      /Cannot send command.*Uninitialized.*Running/,
    );
  });

  // ---- Test 12: undo returns null when not running ----
  it("undo returns null when runtime is not running", () => {
    expect(facade.undo()).toBeNull();
  });

  // ---- Test 13: redo returns null when not running ----
  it("redo returns null when runtime is not running", () => {
    expect(facade.redo()).toBeNull();
  });

  // ---- Test 14: onStateChange unsubscribe works ----
  it("onStateChange unsubscribe stops further notifications", async () => {
    const states: RuntimeState[] = [];
    const unsubscribe = facade.onStateChange((state) => states.push(state));

    await facade.start();
    expect(states.length).toBe(2); // Starting, Running

    unsubscribe();

    await facade.shutdown();
    // Should still be 2 — no new notifications after unsubscribe
    expect(states.length).toBe(2);
  });

  // ---- Test 15: multiple commands tracked in history ----
  it("multiple commands are tracked in order", async () => {
    await facade.start();

    facade.sendCommand(makeCommand("place"));
    facade.sendCommand(makeCommand("demolish"));
    facade.sendCommand(makeCommand("zone"));

    const history = facade.getCommandHistory();
    expect(history.getUndoCount()).toBe(3);

    const r1 = facade.undo();
    expect(r1!.type).toBe("SetZoning");

    const r2 = facade.undo();
    expect(r2!.type).toBe("Bulldoze");

    const r3 = facade.undo();
    expect(r3!.type).toBe("PlaceEntity");

    expect(facade.undo()).toBeNull();
  });

  // ---- Test 16: config is accessible ----
  it("getConfig returns the provided configuration", () => {
    const config = facade.getConfig();
    expect(config.mapWidth).toBe(64);
    expect(config.mapHeight).toBe(64);
    expect(config.seed).toBe(42);
  });

  // ---- Test 17: maxUndoDepth is forwarded to CommandHistory ----
  it("maxUndoDepth is forwarded to CommandHistory config", () => {
    const custom = new RuntimeFacade(makeConfig({ maxUndoDepth: 25 }));
    expect(custom.getCommandHistory().config.maxUndoDepth).toBe(25);
  });

  // ---- Test 18: save and load work without workers ----
  it("save and load work without workers", async () => {
    await facade.start();

    await facade.save("slot1");

    const loaded = await facade.load("slot1");
    expect(loaded).toBe(true);
  });

  // ---- Test 19: load returns false for missing slot ----
  it("load returns false for a non-existent slot", async () => {
    await facade.start();

    const loaded = await facade.load("missing-slot");
    expect(loaded).toBe(false);
  });

  // ---- Test 20: can't shutdown when not running ----
  it("throws when shutting down from Uninitialized state", async () => {
    await expect(facade.shutdown()).rejects.toThrow(/Cannot shutdown/);
  });

  // ---- Test 21: getPluginRegistry returns registry instance ----
  it("getPluginRegistry returns a PluginRegistry instance", () => {
    const registry = facade.getPluginRegistry();
    expect(registry).toBeDefined();
    expect(registry.count()).toBe(0);
  });

  // ---- Test 22: getSaveManager returns save manager instance ----
  it("getSaveManager returns a SaveManager instance", () => {
    const sm = facade.getSaveManager();
    expect(sm).toBeDefined();
  });

  // ---- Test 23: getPluginHost exposes orchestration layer ----
  it("getPluginHost returns host bound to facade registry", () => {
    const host = facade.getPluginHost();
    expect(host).toBeDefined();
    expect(host.getRegistry()).toBe(facade.getPluginRegistry());
  });

  // ---- Test 24a: failed worker delivery removes command from history ----
  it("failed worker delivery removes command from history", async () => {
    let rejectFn: (err: Error) => void;
    const worker: any = {
      startSim: vi.fn(async () => {}),
      startRenderer: vi.fn(async () => {}),
      sendCommand: vi.fn(() => new Promise<void>((_resolve, reject) => {
        rejectFn = reject;
      })),
      setSpeed: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      saveGame: vi.fn(async () => new Uint8Array(0)),
      loadGame: vi.fn(async () => true),
      onTick: vi.fn(),
      onPick: vi.fn(),
      shutdown: vi.fn(),
    };

    facade.setWorkerManager(worker);
    await facade.start();
    facade.sendCommand(makeCommand("place"));

    // History now has 1 record
    expect(facade.getCommandHistory().getUndoCount()).toBe(1);

    // Simulate delivery failure
    rejectFn!(new Error("worker disconnected"));

    // Allow the microtask queue to flush the .catch() handler
    await Promise.resolve();
    await Promise.resolve();

    // History should now be empty — the failed record was rolled back
    expect(facade.getCommandHistory().getUndoCount()).toBe(0);
  });

  // ---- Test 24b: e2e — ToolManager.onMouseUp -> translateToolInteraction -> sendCommand ----
  it("e2e: tool drag -> translateToolInteraction -> sendCommand records in history", async () => {
    await facade.start();

    // Simulate a zone drag: tiles from (1,1) to (3,3)
    const toolCommand = {
      type: "zone" as const,
      tiles: [{ x: 1, y: 1 }, { x: 3, y: 3 }],
      zoneType: 1, // Residential
      estimatedCost: 0,
    };

    const engineCommands = translateToolInteraction(toolCommand);
    expect(engineCommands.length).toBe(1);
    expect(engineCommands[0]).toMatchObject({ SetZoning: { zone: "Residential" } });

    for (const cmd of engineCommands) {
      facade.sendCommand(cmd as EngineCommand);
    }

    const history = facade.getCommandHistory();
    expect(history.getUndoCount()).toBe(1);
    expect(history.canUndo()).toBe(true);

    const undone = facade.undo();
    expect(undone).not.toBeNull();
    expect(undone!.type).toBe("SetZoning");
    // Undo payload clears the zone
    expect((undone!.undoPayload as EngineCommand)).toMatchObject({ SetZoning: { zone: "None" } });
  });

  // ---- Test 24: undo/redo dispatch worker commands when payloads exist ----
  it("undo/redo dispatches to worker manager for reversible commands", async () => {
    const sent: string[] = [];
    const worker: any = {
      startSim: vi.fn(async () => {}),
      startRenderer: vi.fn(async () => {}),
      sendCommand: vi.fn(async (json: string) => {
        sent.push(json);
        return { type: "COMMAND_RESULT", success: true, sequence_id: 1 };
      }),
      setSpeed: vi.fn(),
      pause: vi.fn(),
      resume: vi.fn(),
      saveGame: vi.fn(async () => new Uint8Array(0)),
      loadGame: vi.fn(async () => true),
      onTick: vi.fn(),
      onPick: vi.fn(),
      shutdown: vi.fn(),
    };

    facade.setWorkerManager(worker);
    await facade.start();
    facade.sendCommand(makeCommand("zone"));
    facade.undo();
    facade.redo();

    expect(sent.length).toBeGreaterThanOrEqual(3);
    expect(sent[1]).toContain('"SetZoning"');
    expect(sent[1]).toContain('"zone":"None"');
    expect(sent[2]).toContain('"SetZoning"');
    expect(sent[2]).toContain('"zone":"Residential"');
  });
});
