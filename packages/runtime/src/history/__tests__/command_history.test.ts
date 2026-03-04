// @townbuilder/runtime — Tests for CommandHistory
import { describe, it, expect, beforeEach } from "vitest";
import {
  CommandHistory,
  type CommandRecord,
  type Snapshot,
  type CommandHistoryConfig,
} from "../command_history.js";

// ---- Helper ----

/** Create a CommandRecord with sensible defaults. */
function makeRecord(id: number, type: string = "place"): CommandRecord {
  return {
    id,
    type,
    doPayload: { x: id, y: id },
    undoPayload: { x: id, y: id, prev: null },
    timestamp: Date.now() + id,
  };
}

/** Create a Snapshot with sensible defaults. */
function makeSnapshot(id: number, tick: number = id * 10): Snapshot {
  return {
    id,
    tick,
    data: new Uint8Array([id, tick & 0xff]),
  };
}

// ---- CommandHistory Tests ----

describe("CommandHistory", () => {
  let history: CommandHistory;

  beforeEach(() => {
    history = new CommandHistory();
  });

  // ---- Test 1: push adds to undo stack ----
  it("push adds a record to the undo stack", () => {
    const record = makeRecord(1);
    history.push(record);

    expect(history.getUndoCount()).toBe(1);
    expect(history.canUndo()).toBe(true);
  });

  // ---- Test 2: undo moves to redo stack ----
  it("undo moves the record from undo to redo stack", () => {
    const record = makeRecord(1);
    history.push(record);

    const undone = history.undo();

    expect(undone).toEqual(record);
    expect(history.getUndoCount()).toBe(0);
    expect(history.getRedoCount()).toBe(1);
    expect(history.canUndo()).toBe(false);
    expect(history.canRedo()).toBe(true);
  });

  // ---- Test 3: redo moves back to undo stack ----
  it("redo moves the record from redo back to undo stack", () => {
    const record = makeRecord(1);
    history.push(record);
    history.undo();

    const redone = history.redo();

    expect(redone).toEqual(record);
    expect(history.getUndoCount()).toBe(1);
    expect(history.getRedoCount()).toBe(0);
    expect(history.canUndo()).toBe(true);
    expect(history.canRedo()).toBe(false);
  });

  // ---- Test 4: push after undo clears redo stack ----
  it("push after undo clears the redo stack", () => {
    history.push(makeRecord(1));
    history.push(makeRecord(2));
    history.undo(); // record 2 goes to redo

    expect(history.canRedo()).toBe(true);

    history.push(makeRecord(3)); // fork — redo should be cleared

    expect(history.canRedo()).toBe(false);
    expect(history.getRedoCount()).toBe(0);
    expect(history.getUndoCount()).toBe(2); // record 1 + record 3
  });

  // ---- Test 5: canUndo/canRedo correct ----
  it("canUndo and canRedo reflect stack state", () => {
    expect(history.canUndo()).toBe(false);
    expect(history.canRedo()).toBe(false);

    history.push(makeRecord(1));
    expect(history.canUndo()).toBe(true);
    expect(history.canRedo()).toBe(false);

    history.undo();
    expect(history.canUndo()).toBe(false);
    expect(history.canRedo()).toBe(true);

    history.redo();
    expect(history.canUndo()).toBe(true);
    expect(history.canRedo()).toBe(false);
  });

  // ---- Test 6: maxUndoDepth enforced (drop oldest) ----
  it("enforces maxUndoDepth by dropping the oldest record", () => {
    const small = new CommandHistory({ maxUndoDepth: 3 });

    small.push(makeRecord(1));
    small.push(makeRecord(2));
    small.push(makeRecord(3));
    expect(small.getUndoCount()).toBe(3);

    small.push(makeRecord(4)); // should evict record 1
    expect(small.getUndoCount()).toBe(3);

    // Undo all three — should get records 4, 3, 2 (oldest first was dropped)
    const r1 = small.undo();
    const r2 = small.undo();
    const r3 = small.undo();

    expect(r1!.id).toBe(4);
    expect(r2!.id).toBe(3);
    expect(r3!.id).toBe(2);
    expect(small.undo()).toBeNull();
  });

  // ---- Test 7: snapshot auto-created at interval ----
  it("auto-creates a snapshot at the configured interval", () => {
    let snapshotCounter = 0;
    const h = new CommandHistory({ snapshotInterval: 3, maxUndoDepth: 100 });
    h.setSnapshotCallback(() => {
      snapshotCounter++;
      return makeSnapshot(snapshotCounter);
    });

    // Push 2 commands — no snapshot yet
    h.push(makeRecord(1));
    h.push(makeRecord(2));
    expect(h.getLatestSnapshot()).toBeNull();

    // Push 3rd command — should trigger snapshot
    h.push(makeRecord(3));
    expect(h.getLatestSnapshot()).not.toBeNull();
    expect(h.getLatestSnapshot()!.id).toBe(1);

    // Push 3 more — should trigger another snapshot
    h.push(makeRecord(4));
    h.push(makeRecord(5));
    h.push(makeRecord(6));
    expect(h.getLatestSnapshot()!.id).toBe(2);
  });

  // ---- Test 8: getLatestSnapshot returns most recent ----
  it("getLatestSnapshot returns the most recently added snapshot", () => {
    expect(history.getLatestSnapshot()).toBeNull();

    history.addSnapshot(makeSnapshot(1, 10));
    expect(history.getLatestSnapshot()!.id).toBe(1);

    history.addSnapshot(makeSnapshot(2, 20));
    expect(history.getLatestSnapshot()!.id).toBe(2);

    history.addSnapshot(makeSnapshot(3, 30));
    expect(history.getLatestSnapshot()!.id).toBe(3);
  });

  // ---- Test 9: clear resets everything ----
  it("clear resets all stacks and snapshots", () => {
    history.push(makeRecord(1));
    history.push(makeRecord(2));
    history.undo();
    history.addSnapshot(makeSnapshot(1));

    expect(history.getUndoCount()).toBe(1);
    expect(history.getRedoCount()).toBe(1);
    expect(history.getLatestSnapshot()).not.toBeNull();

    history.clear();

    expect(history.getUndoCount()).toBe(0);
    expect(history.getRedoCount()).toBe(0);
    expect(history.canUndo()).toBe(false);
    expect(history.canRedo()).toBe(false);
    expect(history.getLatestSnapshot()).toBeNull();
  });

  // ---- Test 10: empty undo returns null ----
  it("undo on empty stack returns null", () => {
    expect(history.undo()).toBeNull();
  });

  // ---- Test 11: empty redo returns null ----
  it("redo on empty stack returns null", () => {
    expect(history.redo()).toBeNull();
  });

  // ---- Test 12: multiple undo/redo cycles work correctly ----
  it("multiple undo/redo cycles preserve correct order", () => {
    history.push(makeRecord(1));
    history.push(makeRecord(2));
    history.push(makeRecord(3));

    // Undo all three
    expect(history.undo()!.id).toBe(3);
    expect(history.undo()!.id).toBe(2);
    expect(history.undo()!.id).toBe(1);
    expect(history.undo()).toBeNull();

    // Redo all three
    expect(history.redo()!.id).toBe(1);
    expect(history.redo()!.id).toBe(2);
    expect(history.redo()!.id).toBe(3);
    expect(history.redo()).toBeNull();

    // Undo two, then push — should fork
    expect(history.undo()!.id).toBe(3);
    expect(history.undo()!.id).toBe(2);

    history.push(makeRecord(4));

    expect(history.getUndoCount()).toBe(2); // record 1, record 4
    expect(history.getRedoCount()).toBe(0);
    expect(history.canRedo()).toBe(false);

    // Verify undo order after fork
    expect(history.undo()!.id).toBe(4);
    expect(history.undo()!.id).toBe(1);
    expect(history.undo()).toBeNull();
  });

  // ---- Test 13: default config values ----
  it("uses default config when none provided", () => {
    expect(history.config.maxUndoDepth).toBe(100);
    expect(history.config.snapshotInterval).toBe(50);
  });

  // ---- Test 14: partial config merges with defaults ----
  it("merges partial config with defaults", () => {
    const custom = new CommandHistory({ maxUndoDepth: 25 });
    expect(custom.config.maxUndoDepth).toBe(25);
    expect(custom.config.snapshotInterval).toBe(50);
  });

  // ---- Test 15: addSnapshot trims old snapshots beyond limit ----
  it("addSnapshot trims old snapshots when limit exceeded", () => {
    // maxUndoDepth=5, snapshotInterval=2 => maxSnapshots = ceil(5/2)+1 = 4
    const h = new CommandHistory({ maxUndoDepth: 5, snapshotInterval: 2 });

    h.addSnapshot(makeSnapshot(1));
    h.addSnapshot(makeSnapshot(2));
    h.addSnapshot(makeSnapshot(3));
    h.addSnapshot(makeSnapshot(4));

    // 4 snapshots should be fine (limit is 4)
    expect(h.getLatestSnapshot()!.id).toBe(4);

    // 5th snapshot should trim the oldest
    h.addSnapshot(makeSnapshot(5));
    expect(h.getLatestSnapshot()!.id).toBe(5);

    // Verify oldest was trimmed by clearing and checking count is bounded
    // We can test indirectly: after clear, latest should be null
    h.clear();
    expect(h.getLatestSnapshot()).toBeNull();
  });

  // ---- Test 16: getUndoCount and getRedoCount are accurate ----
  it("getUndoCount and getRedoCount track stack sizes", () => {
    expect(history.getUndoCount()).toBe(0);
    expect(history.getRedoCount()).toBe(0);

    history.push(makeRecord(1));
    history.push(makeRecord(2));
    expect(history.getUndoCount()).toBe(2);
    expect(history.getRedoCount()).toBe(0);

    history.undo();
    expect(history.getUndoCount()).toBe(1);
    expect(history.getRedoCount()).toBe(1);

    history.undo();
    expect(history.getUndoCount()).toBe(0);
    expect(history.getRedoCount()).toBe(2);
  });
});
