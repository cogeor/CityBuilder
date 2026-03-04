// Audio system — spatial ambient + SFX

// ─── Enums ───────────────────────────────────────────────────────────

export enum AudioCategory {
  Master = "master",
  Ambient = "ambient",
  SFX = "sfx",
  Music = "music",
  UI = "ui",
}

export enum AmbientLayer {
  CityHum = "city_hum",
  Traffic = "traffic",
  Nature = "nature",
  Rain = "rain",
  Wind = "wind",
}

export enum SFXType {
  ConstructionStart = "construction_start",
  ConstructionComplete = "construction_complete",
  Bulldoze = "bulldoze",
  Alert = "alert",
  Fire = "fire",
  BudgetWarning = "budget_warning",
  Click = "click",
  Error = "error",
}

// ─── Types ───────────────────────────────────────────────────────────

export interface SpatialParams {
  tileX: number;
  tileY: number;
  cameraCenterX: number;
  cameraCenterY: number;
  maxDistance: number;
}

export interface VolumeSettings {
  master: number; // 0.0 - 1.0
  ambient: number;
  sfx: number;
  music: number;
  ui: number;
  muted: boolean;
}

export const DEFAULT_VOLUME: VolumeSettings = {
  master: 0.8,
  ambient: 0.6,
  sfx: 0.8,
  music: 0.5,
  ui: 0.7,
  muted: false,
};

// ─── Interface ───────────────────────────────────────────────────────

export interface IAudioEngine {
  initialize(): Promise<void>;
  shutdown(): void;

  // Ambient
  setAmbientLevel(layer: AmbientLayer, intensity: number): void;

  // SFX
  playSFX(sfx: SFXType, spatial?: SpatialParams): void;

  // Volume
  setVolume(category: AudioCategory, level: number): void;
  getVolume(category: AudioCategory): number;
  setMuted(muted: boolean): void;
  isMuted(): boolean;

  // State
  isInitialized(): boolean;
}

// ─── Spatial helpers ─────────────────────────────────────────────────

/**
 * Compute spatial pan and gain from tile position relative to camera.
 *
 * pan:  -1.0 (full left) to 1.0 (full right) based on tileX vs cameraCenterX.
 * gain:  1.0 at camera center, falling linearly to 0.0 at maxDistance.
 */
export function computeSpatialAudio(params: SpatialParams): { pan: number; gain: number } {
  const dx = params.tileX - params.cameraCenterX;
  const dy = params.tileY - params.cameraCenterY;
  const distance = Math.sqrt(dx * dx + dy * dy);

  // Pan: horizontal offset normalized to [-1, 1], clamped
  const rawPan = params.maxDistance > 0 ? dx / params.maxDistance : 0;
  const pan = Math.max(-1, Math.min(1, rawPan));

  // Gain: linear falloff from 1.0 at center to 0.0 at maxDistance
  const gain = params.maxDistance > 0
    ? Math.max(0, 1 - distance / params.maxDistance)
    : 0;

  return { pan, gain };
}

// ─── NoOpAudioEngine (headless / testing stub) ──────────────────────

export class NoOpAudioEngine implements IAudioEngine {
  async initialize(): Promise<void> {
    // no-op
  }

  shutdown(): void {
    // no-op
  }

  setAmbientLevel(_layer: AmbientLayer, _intensity: number): void {
    // no-op
  }

  playSFX(_sfx: SFXType, _spatial?: SpatialParams): void {
    // no-op
  }

  setVolume(_category: AudioCategory, _level: number): void {
    // no-op
  }

  getVolume(_category: AudioCategory): number {
    return 0;
  }

  setMuted(_muted: boolean): void {
    // no-op
  }

  isMuted(): boolean {
    return true;
  }

  isInitialized(): boolean {
    return false;
  }
}

// ─── SFX log entry ──────────────────────────────────────────────────

export interface SFXLogEntry {
  sfx: SFXType;
  time: number;
  spatial?: { pan: number; gain: number };
}

// ─── WebAudioEngine ─────────────────────────────────────────────────

/**
 * Default Web Audio API based implementation.
 *
 * In a browser environment this would create an AudioContext, GainNodes,
 * and PannerNodes. Here we track all state so the engine can be tested
 * without an actual audio backend.
 */
export class WebAudioEngine implements IAudioEngine {
  private volumes: VolumeSettings;
  private ambientLevels: Map<AmbientLayer, number> = new Map();
  private initialized = false;
  private sfxLog: SFXLogEntry[] = [];

  constructor(volumes?: Partial<VolumeSettings>) {
    this.volumes = { ...DEFAULT_VOLUME, ...volumes };

    // Initialize all ambient layers to 0
    for (const layer of Object.values(AmbientLayer)) {
      this.ambientLevels.set(layer, 0);
    }
  }

  // ── Lifecycle ──────────────────────────────────────────────────────

  async initialize(): Promise<void> {
    // In a real implementation this would create the AudioContext and
    // lazy-load audio packs. Here we just mark as initialized.
    this.initialized = true;
  }

  shutdown(): void {
    this.initialized = false;
    this.sfxLog = [];
    for (const layer of Object.values(AmbientLayer)) {
      this.ambientLevels.set(layer, 0);
    }
  }

  // ── Ambient ────────────────────────────────────────────────────────

  setAmbientLevel(layer: AmbientLayer, intensity: number): void {
    const clamped = Math.max(0, Math.min(1, intensity));
    this.ambientLevels.set(layer, clamped);
  }

  getAmbientLevel(layer: AmbientLayer): number {
    return this.ambientLevels.get(layer) ?? 0;
  }

  // ── SFX ────────────────────────────────────────────────────────────

  playSFX(sfx: SFXType, spatial?: SpatialParams): void {
    const entry: SFXLogEntry = {
      sfx,
      time: Date.now(),
    };

    if (spatial) {
      entry.spatial = computeSpatialAudio(spatial);
    }

    this.sfxLog.push(entry);
  }

  getSFXLog(): SFXLogEntry[] {
    return [...this.sfxLog];
  }

  // ── Volume ─────────────────────────────────────────────────────────

  setVolume(category: AudioCategory, level: number): void {
    const clamped = Math.max(0, Math.min(1, level));
    switch (category) {
      case AudioCategory.Master:
        this.volumes.master = clamped;
        break;
      case AudioCategory.Ambient:
        this.volumes.ambient = clamped;
        break;
      case AudioCategory.SFX:
        this.volumes.sfx = clamped;
        break;
      case AudioCategory.Music:
        this.volumes.music = clamped;
        break;
      case AudioCategory.UI:
        this.volumes.ui = clamped;
        break;
    }
  }

  getVolume(category: AudioCategory): number {
    switch (category) {
      case AudioCategory.Master:
        return this.volumes.master;
      case AudioCategory.Ambient:
        return this.volumes.ambient;
      case AudioCategory.SFX:
        return this.volumes.sfx;
      case AudioCategory.Music:
        return this.volumes.music;
      case AudioCategory.UI:
        return this.volumes.ui;
    }
  }

  setMuted(muted: boolean): void {
    this.volumes.muted = muted;
  }

  isMuted(): boolean {
    return this.volumes.muted;
  }

  // ── State ──────────────────────────────────────────────────────────

  isInitialized(): boolean {
    return this.initialized;
  }
}
