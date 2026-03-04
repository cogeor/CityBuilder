import { describe, it, expect, beforeEach } from 'vitest';
import {
  AudioCategory,
  AmbientLayer,
  SFXType,
  type SpatialParams,
  type VolumeSettings,
  DEFAULT_VOLUME,
  type IAudioEngine,
  computeSpatialAudio,
  NoOpAudioEngine,
  WebAudioEngine,
} from '../audio_engine.js';

// ─── NoOpAudioEngine ────────────────────────────────────────────────

describe('NoOpAudioEngine', () => {
  let engine: NoOpAudioEngine;

  beforeEach(() => {
    engine = new NoOpAudioEngine();
  });

  it('implements IAudioEngine interface', () => {
    const asInterface: IAudioEngine = engine;
    expect(asInterface).toBeDefined();
    expect(typeof asInterface.initialize).toBe('function');
    expect(typeof asInterface.shutdown).toBe('function');
    expect(typeof asInterface.setAmbientLevel).toBe('function');
    expect(typeof asInterface.playSFX).toBe('function');
    expect(typeof asInterface.setVolume).toBe('function');
    expect(typeof asInterface.getVolume).toBe('function');
    expect(typeof asInterface.setMuted).toBe('function');
    expect(typeof asInterface.isMuted).toBe('function');
    expect(typeof asInterface.isInitialized).toBe('function');
  });

  it('methods do not throw', async () => {
    await expect(engine.initialize()).resolves.toBeUndefined();
    expect(() => engine.shutdown()).not.toThrow();
    expect(() => engine.setAmbientLevel(AmbientLayer.CityHum, 0.5)).not.toThrow();
    expect(() => engine.playSFX(SFXType.Click)).not.toThrow();
    expect(() => engine.playSFX(SFXType.Alert, {
      tileX: 5, tileY: 5, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    })).not.toThrow();
    expect(() => engine.setVolume(AudioCategory.Master, 0.5)).not.toThrow();
    expect(engine.getVolume(AudioCategory.Master)).toBe(0);
    expect(() => engine.setMuted(true)).not.toThrow();
    expect(engine.isMuted()).toBe(true);
    expect(engine.isInitialized()).toBe(false);
  });
});

// ─── WebAudioEngine ─────────────────────────────────────────────────

describe('WebAudioEngine', () => {
  let engine: WebAudioEngine;

  beforeEach(() => {
    engine = new WebAudioEngine();
  });

  it('has correct default volumes', () => {
    expect(engine.getVolume(AudioCategory.Master)).toBe(0.8);
    expect(engine.getVolume(AudioCategory.Ambient)).toBe(0.6);
    expect(engine.getVolume(AudioCategory.SFX)).toBe(0.8);
    expect(engine.getVolume(AudioCategory.Music)).toBe(0.5);
    expect(engine.getVolume(AudioCategory.UI)).toBe(0.7);
    expect(engine.isMuted()).toBe(false);
  });

  it('set/get volume works for all categories', () => {
    engine.setVolume(AudioCategory.Master, 0.3);
    expect(engine.getVolume(AudioCategory.Master)).toBe(0.3);

    engine.setVolume(AudioCategory.Ambient, 0.1);
    expect(engine.getVolume(AudioCategory.Ambient)).toBe(0.1);

    engine.setVolume(AudioCategory.SFX, 0.9);
    expect(engine.getVolume(AudioCategory.SFX)).toBe(0.9);

    engine.setVolume(AudioCategory.Music, 0.4);
    expect(engine.getVolume(AudioCategory.Music)).toBe(0.4);

    engine.setVolume(AudioCategory.UI, 0.2);
    expect(engine.getVolume(AudioCategory.UI)).toBe(0.2);
  });

  it('volume is clamped to [0, 1]', () => {
    engine.setVolume(AudioCategory.Master, 1.5);
    expect(engine.getVolume(AudioCategory.Master)).toBe(1);

    engine.setVolume(AudioCategory.Master, -0.3);
    expect(engine.getVolume(AudioCategory.Master)).toBe(0);
  });

  it('mute and unmute', () => {
    expect(engine.isMuted()).toBe(false);
    engine.setMuted(true);
    expect(engine.isMuted()).toBe(true);
    engine.setMuted(false);
    expect(engine.isMuted()).toBe(false);
  });

  it('ambient level setting and retrieval', () => {
    engine.setAmbientLevel(AmbientLayer.CityHum, 0.7);
    expect(engine.getAmbientLevel(AmbientLayer.CityHum)).toBe(0.7);

    engine.setAmbientLevel(AmbientLayer.Traffic, 0.3);
    expect(engine.getAmbientLevel(AmbientLayer.Traffic)).toBe(0.3);

    // Clamped
    engine.setAmbientLevel(AmbientLayer.Rain, 1.5);
    expect(engine.getAmbientLevel(AmbientLayer.Rain)).toBe(1);

    engine.setAmbientLevel(AmbientLayer.Wind, -0.2);
    expect(engine.getAmbientLevel(AmbientLayer.Wind)).toBe(0);
  });

  it('SFX plays and logs', () => {
    engine.playSFX(SFXType.Click);
    const log = engine.getSFXLog();
    expect(log).toHaveLength(1);
    expect(log[0].sfx).toBe(SFXType.Click);
    expect(log[0].time).toBeGreaterThan(0);
    expect(log[0].spatial).toBeUndefined();
  });

  it('SFX with spatial params logs spatial data', () => {
    const spatial: SpatialParams = {
      tileX: 10, tileY: 5, cameraCenterX: 10, cameraCenterY: 5, maxDistance: 20,
    };
    engine.playSFX(SFXType.ConstructionStart, spatial);
    const log = engine.getSFXLog();
    expect(log).toHaveLength(1);
    expect(log[0].spatial).toBeDefined();
    expect(log[0].spatial!.pan).toBe(0);
    expect(log[0].spatial!.gain).toBe(1);
  });

  it('initialize and shutdown lifecycle', async () => {
    expect(engine.isInitialized()).toBe(false);
    await engine.initialize();
    expect(engine.isInitialized()).toBe(true);
    engine.shutdown();
    expect(engine.isInitialized()).toBe(false);
  });

  it('shutdown resets SFX log and ambient levels', async () => {
    await engine.initialize();
    engine.playSFX(SFXType.Alert);
    engine.setAmbientLevel(AmbientLayer.CityHum, 0.9);
    expect(engine.getSFXLog()).toHaveLength(1);
    expect(engine.getAmbientLevel(AmbientLayer.CityHum)).toBe(0.9);

    engine.shutdown();
    expect(engine.getSFXLog()).toHaveLength(0);
    expect(engine.getAmbientLevel(AmbientLayer.CityHum)).toBe(0);
  });

  it('accepts custom initial volumes', () => {
    const custom = new WebAudioEngine({ master: 1.0, ambient: 0.2 });
    expect(custom.getVolume(AudioCategory.Master)).toBe(1.0);
    expect(custom.getVolume(AudioCategory.Ambient)).toBe(0.2);
    // Others still default
    expect(custom.getVolume(AudioCategory.SFX)).toBe(0.8);
  });

  it('multiple SFX in sequence logged correctly', () => {
    engine.playSFX(SFXType.ConstructionStart);
    engine.playSFX(SFXType.ConstructionComplete);
    engine.playSFX(SFXType.Bulldoze);
    engine.playSFX(SFXType.Fire);
    engine.playSFX(SFXType.BudgetWarning);

    const log = engine.getSFXLog();
    expect(log).toHaveLength(5);
    expect(log[0].sfx).toBe(SFXType.ConstructionStart);
    expect(log[1].sfx).toBe(SFXType.ConstructionComplete);
    expect(log[2].sfx).toBe(SFXType.Bulldoze);
    expect(log[3].sfx).toBe(SFXType.Fire);
    expect(log[4].sfx).toBe(SFXType.BudgetWarning);

    // Times should be non-decreasing
    for (let i = 1; i < log.length; i++) {
      expect(log[i].time).toBeGreaterThanOrEqual(log[i - 1].time);
    }
  });
});

// ─── computeSpatialAudio ─────────────────────────────────────────────

describe('computeSpatialAudio', () => {
  it('center position returns pan=0, gain=1', () => {
    const result = computeSpatialAudio({
      tileX: 10, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(result.pan).toBe(0);
    expect(result.gain).toBe(1);
  });

  it('left position returns negative pan', () => {
    const result = computeSpatialAudio({
      tileX: 0, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(result.pan).toBe(-0.5);
    expect(result.gain).toBeGreaterThan(0);
  });

  it('right position returns positive pan', () => {
    const result = computeSpatialAudio({
      tileX: 20, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(result.pan).toBe(0.5);
    expect(result.gain).toBeGreaterThan(0);
  });

  it('far distance reduces gain', () => {
    const near = computeSpatialAudio({
      tileX: 12, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    const far = computeSpatialAudio({
      tileX: 18, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(far.gain).toBeLessThan(near.gain);
    expect(far.gain).toBeGreaterThan(0);
  });

  it('at max distance returns zero gain', () => {
    const result = computeSpatialAudio({
      tileX: 30, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(result.gain).toBe(0);
  });

  it('pan is clamped to [-1, 1]', () => {
    const result = computeSpatialAudio({
      tileX: 100, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(result.pan).toBe(1);

    const left = computeSpatialAudio({
      tileX: -100, tileY: 10, cameraCenterX: 10, cameraCenterY: 10, maxDistance: 20,
    });
    expect(left.pan).toBe(-1);
  });
});

// ─── Enums and defaults ─────────────────────────────────────────────

describe('VolumeSettings defaults', () => {
  it('all DEFAULT_VOLUME values are in [0, 1] range', () => {
    expect(DEFAULT_VOLUME.master).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_VOLUME.master).toBeLessThanOrEqual(1);
    expect(DEFAULT_VOLUME.ambient).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_VOLUME.ambient).toBeLessThanOrEqual(1);
    expect(DEFAULT_VOLUME.sfx).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_VOLUME.sfx).toBeLessThanOrEqual(1);
    expect(DEFAULT_VOLUME.music).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_VOLUME.music).toBeLessThanOrEqual(1);
    expect(DEFAULT_VOLUME.ui).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_VOLUME.ui).toBeLessThanOrEqual(1);
  });
});

describe('AudioCategory enum', () => {
  it('contains all expected values', () => {
    const values = Object.values(AudioCategory);
    expect(values).toContain('master');
    expect(values).toContain('ambient');
    expect(values).toContain('sfx');
    expect(values).toContain('music');
    expect(values).toContain('ui');
    expect(values).toHaveLength(5);
  });
});

describe('SFXType enum', () => {
  it('contains all expected values', () => {
    const values = Object.values(SFXType);
    expect(values).toContain('construction_start');
    expect(values).toContain('construction_complete');
    expect(values).toContain('bulldoze');
    expect(values).toContain('alert');
    expect(values).toContain('fire');
    expect(values).toContain('budget_warning');
    expect(values).toContain('click');
    expect(values).toContain('error');
    expect(values).toHaveLength(8);
  });
});

describe('AmbientLayer enum', () => {
  it('contains all expected values', () => {
    const values = Object.values(AmbientLayer);
    expect(values).toContain('city_hum');
    expect(values).toContain('traffic');
    expect(values).toContain('nature');
    expect(values).toContain('rain');
    expect(values).toContain('wind');
    expect(values).toHaveLength(5);
  });
});
