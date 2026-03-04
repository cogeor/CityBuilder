// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  BootStage,
  DEFAULT_GAME_CONFIG,
  MAP_SIZE_PRESETS,
  resolveMapDimensions,
  updateLoadingProgress,
  hideLoadingScreen,
  showError,
  boot,
  type BootProgress,
  type GameConfig,
} from '../main.js';

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Set up minimal DOM elements used by the loading/error screens. */
function setupDOM(): void {
  document.body.innerHTML = `
    <div id="loading-screen">
      <div id="loading-progress" style="width: 0%"></div>
      <div id="loading-status">Initializing...</div>
    </div>
    <div id="error-screen">
      <div id="error-message"></div>
    </div>
  `;
}

// ─── DEFAULT_GAME_CONFIG ─────────────────────────────────────────────────────

describe('DEFAULT_GAME_CONFIG', () => {
  it('has medium map size', () => {
    expect(DEFAULT_GAME_CONFIG.mapSize).toBe('medium');
  });

  it('has a numeric seed', () => {
    expect(typeof DEFAULT_GAME_CONFIG.seed).toBe('number');
    expect(DEFAULT_GAME_CONFIG.seed).toBeGreaterThan(0);
  });

  it('has correct default city name', () => {
    expect(DEFAULT_GAME_CONFIG.cityName).toBe('New City');
  });

  it('has generic country preset', () => {
    expect(DEFAULT_GAME_CONFIG.countryPreset).toBe('generic');
  });
});

// ─── MAP_SIZE_TILES ──────────────────────────────────────────────────────────

describe('MAP_SIZE_PRESETS', () => {
  it('has 3 entries', () => {
    expect(Object.keys(MAP_SIZE_PRESETS)).toHaveLength(3);
  });

  it('small is 128x128', () => {
    expect(MAP_SIZE_PRESETS.small).toEqual({ width: 128, height: 128 });
  });

  it('medium is 192x192', () => {
    expect(MAP_SIZE_PRESETS.medium).toEqual({ width: 192, height: 192 });
  });

  it('large is 256x256', () => {
    expect(MAP_SIZE_PRESETS.large).toEqual({ width: 256, height: 256 });
  });

  it('resolveMapDimensions reads plugin-backed values', () => {
    expect(resolveMapDimensions('small')).toEqual({ width: 128, height: 128 });
    expect(resolveMapDimensions('medium')).toEqual({ width: 192, height: 192 });
    expect(resolveMapDimensions('large')).toEqual({ width: 256, height: 256 });
  });
});

// ─── BootStage ───────────────────────────────────────────────────────────────

describe('BootStage', () => {
  it('has 6 values', () => {
    const values = Object.values(BootStage);
    expect(values).toHaveLength(6);
  });

  it('contains all expected stages', () => {
    expect(BootStage.Init).toBe('init');
    expect(BootStage.LoadPlugins).toBe('load_plugins');
    expect(BootStage.CreateWorkers).toBe('create_workers');
    expect(BootStage.InitWorld).toBe('init_world');
    expect(BootStage.LoadAssets).toBe('load_assets');
    expect(BootStage.Ready).toBe('ready');
  });
});

// ─── updateLoadingProgress ───────────────────────────────────────────────────

describe('updateLoadingProgress', () => {
  beforeEach(() => {
    setupDOM();
  });

  it('updates bar width', () => {
    const progress: BootProgress = {
      stage: BootStage.LoadPlugins,
      progress: 0.5,
      message: 'Loading...',
    };
    updateLoadingProgress(progress);
    const bar = document.getElementById('loading-progress')!;
    expect(bar.style.width).toBe('50%');
  });

  it('updates status text', () => {
    const progress: BootProgress = {
      stage: BootStage.Init,
      progress: 0.1,
      message: 'Starting up...',
    };
    updateLoadingProgress(progress);
    const status = document.getElementById('loading-status')!;
    expect(status.textContent).toBe('Starting up...');
  });

  it('rounds progress to nearest integer percent', () => {
    const progress: BootProgress = {
      stage: BootStage.LoadAssets,
      progress: 0.333,
      message: 'Loading assets...',
    };
    updateLoadingProgress(progress);
    const bar = document.getElementById('loading-progress')!;
    expect(bar.style.width).toBe('33%');
  });

  it('handles missing DOM elements gracefully', () => {
    document.body.innerHTML = '';
    const progress: BootProgress = {
      stage: BootStage.Init,
      progress: 0.5,
      message: 'test',
    };
    // Should not throw
    expect(() => updateLoadingProgress(progress)).not.toThrow();
  });
});

// ─── hideLoadingScreen ───────────────────────────────────────────────────────

describe('hideLoadingScreen', () => {
  beforeEach(() => {
    setupDOM();
  });

  it('adds hidden class to loading screen', () => {
    hideLoadingScreen();
    const screen = document.getElementById('loading-screen')!;
    expect(screen.classList.contains('hidden')).toBe(true);
  });

  it('handles missing loading screen gracefully', () => {
    document.body.innerHTML = '';
    expect(() => hideLoadingScreen()).not.toThrow();
  });
});

// ─── showError ───────────────────────────────────────────────────────────────

describe('showError', () => {
  beforeEach(() => {
    setupDOM();
  });

  it('shows error screen', () => {
    showError('Test error');
    const screen = document.getElementById('error-screen')!;
    expect(screen.classList.contains('visible')).toBe(true);
  });

  it('sets error message', () => {
    showError('Something broke');
    const msg = document.getElementById('error-message')!;
    expect(msg.textContent).toBe('Something broke');
  });

  it('handles missing DOM elements gracefully', () => {
    document.body.innerHTML = '';
    expect(() => showError('test')).not.toThrow();
  });
});

// ─── boot ────────────────────────────────────────────────────────────────────

describe('boot', () => {
  beforeEach(() => {
    setupDOM();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('completes without error', async () => {
    const promise = boot();
    await vi.advanceTimersByTimeAsync(1000);
    await expect(promise).resolves.toBeUndefined();
  });

  it('updates loading progress to 100%', async () => {
    const promise = boot();
    await vi.advanceTimersByTimeAsync(1000);
    await promise;
    const bar = document.getElementById('loading-progress')!;
    expect(bar.style.width).toBe('100%');
  });

  it('hides loading screen after completion', async () => {
    const promise = boot();
    await vi.advanceTimersByTimeAsync(1000);
    await promise;
    const screen = document.getElementById('loading-screen')!;
    expect(screen.classList.contains('hidden')).toBe(true);
  });

  it('accepts custom config', async () => {
    const config: GameConfig = {
      mapSize: 'large',
      seed: 42,
      cityName: 'Test City',
      countryPreset: 'usa',
    };
    const promise = boot(config);
    await vi.advanceTimersByTimeAsync(1000);
    await expect(promise).resolves.toBeUndefined();
  });

  it('GameConfig interface accepts all map sizes', () => {
    const small: GameConfig = { mapSize: 'small', seed: 1, cityName: 'S', countryPreset: 'x' };
    const medium: GameConfig = { mapSize: 'medium', seed: 2, cityName: 'M', countryPreset: 'y' };
    const large: GameConfig = { mapSize: 'large', seed: 3, cityName: 'L', countryPreset: 'z' };
    expect(small.mapSize).toBe('small');
    expect(medium.mapSize).toBe('medium');
    expect(large.mapSize).toBe('large');
  });

  it('sets final status to Ready', async () => {
    const promise = boot();
    await vi.advanceTimersByTimeAsync(1000);
    await promise;
    const status = document.getElementById('loading-status')!;
    expect(status.textContent).toBe('Ready!');
  });
});
