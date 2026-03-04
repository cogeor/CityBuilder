// @townbuilder/runtime — Save/load orchestration for TownBuilder main thread.
// Manages save slots, auto-save, and storage abstraction.

// ---- SaveSlot Interface ----

/** Metadata for a single save slot. */
export interface SaveSlot {
  readonly id: string;
  readonly name: string;
  readonly timestamp: number;
  readonly size: number;
  readonly city_name: string;
  readonly tick: number;
}

// ---- SaveManagerConfig Interface ----

/** Configuration for SaveManager behaviour. */
export interface SaveManagerConfig {
  /** Interval between auto-saves in milliseconds. Default: 300000 (5 minutes). */
  readonly auto_save_interval_ms: number;
  /** Maximum number of save slots. Default: 10. */
  readonly max_slots: number;
}

/** Default configuration values. */
const DEFAULT_CONFIG: SaveManagerConfig = {
  auto_save_interval_ms: 300_000,
  max_slots: 10,
};

// ---- ISaveStorage Interface ----

/** Abstraction over persistent storage (e.g. IndexedDB). */
export interface ISaveStorage {
  /** List all save slots. */
  list(): Promise<SaveSlot[]>;
  /** Retrieve raw save data by slot ID, or null if not found. */
  get(id: string): Promise<Uint8Array | null>;
  /** Store raw save data with associated metadata. */
  put(id: string, data: Uint8Array, metadata: SaveSlot): Promise<void>;
  /** Delete a save slot by ID. */
  delete(id: string): Promise<void>;
  /** Remove all save slots. */
  clear(): Promise<void>;
}

// ---- InMemorySaveStorage ----

/** In-memory ISaveStorage implementation for testing. */
export class InMemorySaveStorage implements ISaveStorage {
  private readonly _data = new Map<string, Uint8Array>();
  private readonly _meta = new Map<string, SaveSlot>();

  async list(): Promise<SaveSlot[]> {
    return Array.from(this._meta.values());
  }

  async get(id: string): Promise<Uint8Array | null> {
    return this._data.get(id) ?? null;
  }

  async put(id: string, data: Uint8Array, metadata: SaveSlot): Promise<void> {
    this._data.set(id, data);
    this._meta.set(id, metadata);
  }

  async delete(id: string): Promise<void> {
    this._data.delete(id);
    this._meta.delete(id);
  }

  async clear(): Promise<void> {
    this._data.clear();
    this._meta.clear();
  }
}

// ---- SaveManager ----

/** Main-thread save/load orchestrator. */
export class SaveManager {
  readonly config: SaveManagerConfig;
  readonly storage: ISaveStorage;
  private _autoSaveTimer: ReturnType<typeof setInterval> | null = null;

  constructor(storage: ISaveStorage, config?: Partial<SaveManagerConfig>) {
    this.storage = storage;
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /** List all save slots from storage. */
  async listSlots(): Promise<SaveSlot[]> {
    return this.storage.list();
  }

  /**
   * Save game data to a slot.
   * If the max slot limit is reached, the oldest slot is evicted.
   * Returns the created SaveSlot metadata.
   */
  async saveGame(
    id: string,
    name: string,
    data: Uint8Array,
    cityName: string,
    tick: number,
  ): Promise<SaveSlot> {
    // Enforce max_slots — evict oldest if at limit
    const existing = await this.storage.list();
    if (existing.length >= this.config.max_slots) {
      // Find the slot with the lowest timestamp
      const oldest = existing.reduce((a, b) =>
        a.timestamp < b.timestamp ? a : b,
      );
      await this.storage.delete(oldest.id);
    }

    const slot: SaveSlot = {
      id,
      name,
      timestamp: Date.now(),
      size: data.byteLength,
      city_name: cityName,
      tick,
    };

    await this.storage.put(id, data, slot);
    return slot;
  }

  /** Load raw save data by slot ID. Returns null if not found. */
  async loadGame(id: string): Promise<Uint8Array | null> {
    return this.storage.get(id);
  }

  /** Delete a save slot by ID. */
  async deleteSlot(id: string): Promise<void> {
    return this.storage.delete(id);
  }

  /**
   * Start periodic auto-saving.
   * @param saveFn — callback that produces the current game state as bytes.
   */
  startAutoSave(saveFn: () => Promise<Uint8Array>): void {
    this.stopAutoSave();
    this._autoSaveTimer = setInterval(async () => {
      const data = await saveFn();
      const id = this.generateSlotId();
      await this.saveGame(id, "autosave", data, "auto", 0);
    }, this.config.auto_save_interval_ms);
  }

  /** Stop periodic auto-saving. */
  stopAutoSave(): void {
    if (this._autoSaveTimer !== null) {
      clearInterval(this._autoSaveTimer);
      this._autoSaveTimer = null;
    }
  }

  /** Check whether auto-save is currently active. */
  isAutoSaving(): boolean {
    return this._autoSaveTimer !== null;
  }

  /** Generate a timestamp-based unique slot ID. */
  generateSlotId(): string {
    return `save-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }
}
