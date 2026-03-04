/// Cache names
export const CACHE_VERSION = "v1";
export const STATIC_CACHE = `townbuilder-static-${CACHE_VERSION}`;
export const WASM_CACHE = `townbuilder-wasm-${CACHE_VERSION}`;
export const ASSET_CACHE = `townbuilder-assets-${CACHE_VERSION}`;

/// File extensions by cache strategy
export const CACHE_FIRST_EXTENSIONS = [".wasm", ".pack", ".atlasbin", ".png", ".jpg", ".webp"];
export const NETWORK_FIRST_EXTENSIONS = [".json", ".html"];

export enum CacheStrategy {
  CacheFirst = "cache-first",
  NetworkFirst = "network-first",
  StaleWhileRevalidate = "stale-while-revalidate",
}

/// Determine cache strategy for a URL.
export function getCacheStrategy(url: string): CacheStrategy {
  const ext = url.split(".").pop()?.toLowerCase() ?? "";
  if (CACHE_FIRST_EXTENSIONS.some(e => url.endsWith(e))) {
    return CacheStrategy.CacheFirst;
  }
  if (NETWORK_FIRST_EXTENSIONS.some(e => url.endsWith(e))) {
    return CacheStrategy.NetworkFirst;
  }
  return CacheStrategy.StaleWhileRevalidate;
}

/// Get appropriate cache name for a URL.
export function getCacheName(url: string): string {
  if (url.endsWith(".wasm")) return WASM_CACHE;
  if (url.endsWith(".pack") || url.endsWith(".atlasbin")) return ASSET_CACHE;
  return STATIC_CACHE;
}

/// List of URLs to precache (populated at build time).
export const PRECACHE_URLS: string[] = [
  "/",
  "/index.html",
];

/// Service worker registration helper (for main thread).
export async function registerServiceWorker(swUrl: string): Promise<boolean> {
  if (!("serviceWorker" in navigator)) {
    return false;
  }
  try {
    const registration = await navigator.serviceWorker.register(swUrl);
    console.log("SW registered:", registration.scope);
    return true;
  } catch (err) {
    console.warn("SW registration failed:", err);
    return false;
  }
}

/// Check for updates and prompt user.
export async function checkForUpdates(): Promise<boolean> {
  if (!("serviceWorker" in navigator)) return false;
  const registration = await navigator.serviceWorker.getRegistration();
  if (!registration) return false;
  await registration.update();
  return registration.waiting !== null;
}

/// Force activate waiting service worker.
export function skipWaiting(): void {
  if (navigator.serviceWorker.controller) {
    navigator.serviceWorker.controller.postMessage({ type: "SKIP_WAITING" });
  }
}
