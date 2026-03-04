import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  CACHE_VERSION,
  STATIC_CACHE,
  WASM_CACHE,
  ASSET_CACHE,
  CACHE_FIRST_EXTENSIONS,
  NETWORK_FIRST_EXTENSIONS,
  CacheStrategy,
  getCacheStrategy,
  getCacheName,
  PRECACHE_URLS,
  registerServiceWorker,
  checkForUpdates,
  skipWaiting,
} from '../sw.js';

// ---- getCacheStrategy --------------------------------------------------------

describe('getCacheStrategy', () => {
  it('returns CacheFirst for .wasm files', () => {
    expect(getCacheStrategy('https://example.com/game.wasm')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns CacheFirst for .pack files', () => {
    expect(getCacheStrategy('https://example.com/data.pack')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns CacheFirst for .png files', () => {
    expect(getCacheStrategy('https://example.com/sprite.png')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns CacheFirst for .atlasbin files', () => {
    expect(getCacheStrategy('https://example.com/atlas.atlasbin')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns CacheFirst for .jpg files', () => {
    expect(getCacheStrategy('https://example.com/photo.jpg')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns CacheFirst for .webp files', () => {
    expect(getCacheStrategy('https://example.com/image.webp')).toBe(CacheStrategy.CacheFirst);
  });

  it('returns NetworkFirst for .json files', () => {
    expect(getCacheStrategy('https://example.com/manifest.json')).toBe(CacheStrategy.NetworkFirst);
  });

  it('returns NetworkFirst for .html files', () => {
    expect(getCacheStrategy('https://example.com/index.html')).toBe(CacheStrategy.NetworkFirst);
  });

  it('returns StaleWhileRevalidate for unknown extensions', () => {
    expect(getCacheStrategy('https://example.com/style.css')).toBe(CacheStrategy.StaleWhileRevalidate);
  });

  it('returns StaleWhileRevalidate for .js files', () => {
    expect(getCacheStrategy('https://example.com/app.js')).toBe(CacheStrategy.StaleWhileRevalidate);
  });
});

// ---- getCacheName -----------------------------------------------------------

describe('getCacheName', () => {
  it('returns WASM_CACHE for .wasm files', () => {
    expect(getCacheName('https://example.com/game.wasm')).toBe(WASM_CACHE);
  });

  it('returns ASSET_CACHE for .pack files', () => {
    expect(getCacheName('https://example.com/data.pack')).toBe(ASSET_CACHE);
  });

  it('returns ASSET_CACHE for .atlasbin files', () => {
    expect(getCacheName('https://example.com/atlas.atlasbin')).toBe(ASSET_CACHE);
  });

  it('returns STATIC_CACHE for other files', () => {
    expect(getCacheName('https://example.com/index.html')).toBe(STATIC_CACHE);
  });

  it('returns STATIC_CACHE for .js files', () => {
    expect(getCacheName('https://example.com/app.js')).toBe(STATIC_CACHE);
  });
});

// ---- Constants --------------------------------------------------------------

describe('constants', () => {
  it('CACHE_VERSION is defined', () => {
    expect(CACHE_VERSION).toBeDefined();
    expect(typeof CACHE_VERSION).toBe('string');
    expect(CACHE_VERSION.length).toBeGreaterThan(0);
  });

  it('PRECACHE_URLS includes index.html', () => {
    expect(PRECACHE_URLS).toContain('/index.html');
  });

  it('PRECACHE_URLS includes root', () => {
    expect(PRECACHE_URLS).toContain('/');
  });

  it('cache names contain CACHE_VERSION', () => {
    expect(STATIC_CACHE).toContain(CACHE_VERSION);
    expect(WASM_CACHE).toContain(CACHE_VERSION);
    expect(ASSET_CACHE).toContain(CACHE_VERSION);
  });

  it('CACHE_FIRST_EXTENSIONS includes .wasm', () => {
    expect(CACHE_FIRST_EXTENSIONS).toContain('.wasm');
  });

  it('NETWORK_FIRST_EXTENSIONS includes .json', () => {
    expect(NETWORK_FIRST_EXTENSIONS).toContain('.json');
  });
});

// ---- CacheStrategy enum -----------------------------------------------------

describe('CacheStrategy', () => {
  it('has CacheFirst value', () => {
    expect(CacheStrategy.CacheFirst).toBe('cache-first');
  });

  it('has NetworkFirst value', () => {
    expect(CacheStrategy.NetworkFirst).toBe('network-first');
  });

  it('has StaleWhileRevalidate value', () => {
    expect(CacheStrategy.StaleWhileRevalidate).toBe('stale-while-revalidate');
  });
});

// ---- registerServiceWorker --------------------------------------------------

describe('registerServiceWorker', () => {
  const originalNavigator = globalThis.navigator;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('returns false when navigator.serviceWorker is not available', async () => {
    // In the vitest/node environment, navigator.serviceWorker is not defined
    // so this should return false
    const hasServiceWorker = 'serviceWorker' in navigator;
    if (!hasServiceWorker) {
      const result = await registerServiceWorker('/sw.js');
      expect(result).toBe(false);
    } else {
      // If running in an environment that has serviceWorker, mock it away
      const nav = { ...navigator };
      Object.defineProperty(globalThis, 'navigator', {
        value: {},
        writable: true,
        configurable: true,
      });
      const result = await registerServiceWorker('/sw.js');
      expect(result).toBe(false);
      Object.defineProperty(globalThis, 'navigator', {
        value: nav,
        writable: true,
        configurable: true,
      });
    }
  });
});

// ---- checkForUpdates --------------------------------------------------------

describe('checkForUpdates', () => {
  it('returns false when navigator.serviceWorker is not available', async () => {
    const hasServiceWorker = 'serviceWorker' in navigator;
    if (!hasServiceWorker) {
      const result = await checkForUpdates();
      expect(result).toBe(false);
    } else {
      const nav = { ...navigator };
      Object.defineProperty(globalThis, 'navigator', {
        value: {},
        writable: true,
        configurable: true,
      });
      const result = await checkForUpdates();
      expect(result).toBe(false);
      Object.defineProperty(globalThis, 'navigator', {
        value: nav,
        writable: true,
        configurable: true,
      });
    }
  });
});
