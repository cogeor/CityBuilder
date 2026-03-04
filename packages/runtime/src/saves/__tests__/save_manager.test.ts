// @townbuilder/runtime — Tests for SaveManager and InMemorySaveStorage
import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import {
  InMemorySaveStorage,
  SaveManager,
  type SaveSlot,
  type ISaveStorage,
  type SaveManagerConfig,
} from "../save_manager.js";

// ---- InMemorySaveStorage Tests ----

describe("InMemorySaveStorage", () => {
  let storage: InMemorySaveStorage;

  beforeEach(() => {
    storage = new InMemorySaveStorage();
  });

  // ---- Test 1: round-trip put/get ----
  it("round-trips data through put and get", async () => {
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const slot: SaveSlot = {
      id: "slot-1",
      name: "Test Save",
      timestamp: 1000,
      size: 5,
      city_name: "TestCity",
      tick: 42,
    };

    await storage.put("slot-1", data, slot);
    const result = await storage.get("slot-1");

    expect(result).not.toBeNull();
    expect(result).toEqual(data);
  });

  // ---- Test 2: list empty returns empty ----
  it("list returns empty array when no slots exist", async () => {
    const slots = await storage.list();
    expect(slots).toEqual([]);
  });

  // ---- Test 3: list returns stored metadata ----
  it("list returns metadata for stored slots", async () => {
    const slot: SaveSlot = {
      id: "slot-1",
      name: "My City",
      timestamp: 2000,
      size: 10,
      city_name: "Springfield",
      tick: 100,
    };

    await storage.put("slot-1", new Uint8Array(10), slot);
    const slots = await storage.list();

    expect(slots).toHaveLength(1);
    expect(slots[0].id).toBe("slot-1");
    expect(slots[0].city_name).toBe("Springfield");
    expect(slots[0].tick).toBe(100);
  });

  // ---- Test 4: get returns null for missing slot ----
  it("get returns null for non-existent slot", async () => {
    const result = await storage.get("no-such-slot");
    expect(result).toBeNull();
  });

  // ---- Test 5: delete removes slot ----
  it("delete removes both data and metadata", async () => {
    const slot: SaveSlot = {
      id: "slot-del",
      name: "Delete Me",
      timestamp: 3000,
      size: 3,
      city_name: "Gone",
      tick: 0,
    };

    await storage.put("slot-del", new Uint8Array(3), slot);
    await storage.delete("slot-del");

    const data = await storage.get("slot-del");
    const slots = await storage.list();

    expect(data).toBeNull();
    expect(slots).toHaveLength(0);
  });

  // ---- Test 6: clear removes all ----
  it("clear removes all slots", async () => {
    for (let i = 0; i < 5; i++) {
      const slot: SaveSlot = {
        id: `slot-${i}`,
        name: `Save ${i}`,
        timestamp: i * 1000,
        size: 1,
        city_name: "City",
        tick: i,
      };
      await storage.put(`slot-${i}`, new Uint8Array(1), slot);
    }

    await storage.clear();

    const slots = await storage.list();
    expect(slots).toHaveLength(0);
  });
});

// ---- SaveManager Tests ----

describe("SaveManager", () => {
  let storage: InMemorySaveStorage;
  let manager: SaveManager;

  beforeEach(() => {
    storage = new InMemorySaveStorage();
    manager = new SaveManager(storage);
    vi.useFakeTimers();
  });

  afterEach(() => {
    manager.stopAutoSave();
    vi.useRealTimers();
  });

  // ---- Test 7: config defaults ----
  it("uses default config when none provided", () => {
    expect(manager.config.auto_save_interval_ms).toBe(300_000);
    expect(manager.config.max_slots).toBe(10);
  });

  // ---- Test 8: config overrides ----
  it("merges partial config with defaults", () => {
    const custom = new SaveManager(storage, { max_slots: 5 });
    expect(custom.config.max_slots).toBe(5);
    expect(custom.config.auto_save_interval_ms).toBe(300_000);
  });

  // ---- Test 9: saveGame creates slot ----
  it("saveGame creates a slot and returns metadata", async () => {
    const data = new Uint8Array([10, 20, 30]);
    const slot = await manager.saveGame("s1", "First Save", data, "Metropolis", 50);

    expect(slot.id).toBe("s1");
    expect(slot.name).toBe("First Save");
    expect(slot.city_name).toBe("Metropolis");
    expect(slot.tick).toBe(50);
    expect(slot.size).toBe(3);
    expect(slot.timestamp).toBeGreaterThan(0);
  });

  // ---- Test 10: loadGame returns data ----
  it("loadGame returns saved data", async () => {
    const data = new Uint8Array([99, 88, 77]);
    await manager.saveGame("load-test", "Load Test", data, "Town", 10);

    const loaded = await manager.loadGame("load-test");
    expect(loaded).toEqual(data);
  });

  // ---- Test 11: loadGame returns null for missing slot ----
  it("loadGame returns null for non-existent slot", async () => {
    const result = await manager.loadGame("missing");
    expect(result).toBeNull();
  });

  // ---- Test 12: deleteSlot removes slot ----
  it("deleteSlot removes the slot", async () => {
    await manager.saveGame("del-1", "Delete", new Uint8Array(1), "City", 0);
    await manager.deleteSlot("del-1");

    const slots = await manager.listSlots();
    expect(slots).toHaveLength(0);
  });

  // ---- Test 13: max slots limit evicts oldest ----
  it("enforces max_slots by evicting the oldest slot", async () => {
    const small = new SaveManager(storage, { max_slots: 3 });

    // Save 3 slots with increasing timestamps
    vi.setSystemTime(1000);
    await small.saveGame("a", "A", new Uint8Array(1), "City", 1);

    vi.setSystemTime(2000);
    await small.saveGame("b", "B", new Uint8Array(1), "City", 2);

    vi.setSystemTime(3000);
    await small.saveGame("c", "C", new Uint8Array(1), "City", 3);

    // Now save a 4th — should evict the oldest (a, timestamp=1000)
    vi.setSystemTime(4000);
    await small.saveGame("d", "D", new Uint8Array(1), "City", 4);

    const slots = await small.listSlots();
    expect(slots).toHaveLength(3);

    const ids = slots.map((s) => s.id);
    expect(ids).not.toContain("a");
    expect(ids).toContain("b");
    expect(ids).toContain("c");
    expect(ids).toContain("d");
  });

  // ---- Test 14: generateSlotId produces unique IDs ----
  it("generateSlotId produces unique timestamp-based IDs", () => {
    const id1 = manager.generateSlotId();
    const id2 = manager.generateSlotId();

    expect(id1).toMatch(/^save-\d+-[a-z0-9]+$/);
    expect(id2).toMatch(/^save-\d+-[a-z0-9]+$/);
    expect(id1).not.toBe(id2);
  });

  // ---- Test 15: auto-save starts and stops ----
  it("startAutoSave and stopAutoSave toggle isAutoSaving", () => {
    expect(manager.isAutoSaving()).toBe(false);

    manager.startAutoSave(async () => new Uint8Array(0));
    expect(manager.isAutoSaving()).toBe(true);

    manager.stopAutoSave();
    expect(manager.isAutoSaving()).toBe(false);
  });

  // ---- Test 16: auto-save fires on interval ----
  it("auto-save calls saveFn on each interval tick", async () => {
    const saveFn = vi.fn(async () => new Uint8Array([1]));

    manager.startAutoSave(saveFn);

    // Advance time by one interval
    await vi.advanceTimersByTimeAsync(300_000);
    expect(saveFn).toHaveBeenCalledTimes(1);

    // Advance again
    await vi.advanceTimersByTimeAsync(300_000);
    expect(saveFn).toHaveBeenCalledTimes(2);

    manager.stopAutoSave();
  });

  // ---- Test 17: stopAutoSave is idempotent ----
  it("stopAutoSave is safe to call when not auto-saving", () => {
    expect(() => manager.stopAutoSave()).not.toThrow();
    expect(manager.isAutoSaving()).toBe(false);
  });

  // ---- Test 18: startAutoSave replaces previous timer ----
  it("startAutoSave stops previous auto-save before starting new one", () => {
    const fn1 = vi.fn(async () => new Uint8Array(0));
    const fn2 = vi.fn(async () => new Uint8Array(0));

    manager.startAutoSave(fn1);
    expect(manager.isAutoSaving()).toBe(true);

    manager.startAutoSave(fn2);
    expect(manager.isAutoSaving()).toBe(true);

    manager.stopAutoSave();
    expect(manager.isAutoSaving()).toBe(false);
  });

  // ---- Test 19: listSlots returns all saved slots ----
  it("listSlots returns all saved slots", async () => {
    await manager.saveGame("x1", "X1", new Uint8Array(1), "A", 1);
    await manager.saveGame("x2", "X2", new Uint8Array(2), "B", 2);
    await manager.saveGame("x3", "X3", new Uint8Array(3), "C", 3);

    const slots = await manager.listSlots();
    expect(slots).toHaveLength(3);
  });

  // ---- Test 20: saveGame records correct byte size ----
  it("saveGame records the correct byte size", async () => {
    const data = new Uint8Array(256);
    const slot = await manager.saveGame("sz", "Size Test", data, "City", 0);
    expect(slot.size).toBe(256);
  });
});
