import { MAP_SIZES, getMapSize } from '../../plugins/base.world/presets.js';

/** Boot stages for loading screen */
export enum BootStage {
  Init = 'init',
  LoadPlugins = 'load_plugins',
  CreateWorkers = 'create_workers',
  InitWorld = 'init_world',
  LoadAssets = 'load_assets',
  Ready = 'ready',
}

/** Boot stage progress */
export interface BootProgress {
  stage: BootStage;
  progress: number;  // 0.0 to 1.0
  message: string;
}

/** Game configuration from new game dialog */
export interface GameConfig {
  mapSize: 'small' | 'medium' | 'large';
  seed: number;
  cityName: string;
  countryPreset: string;
}

/** Default game configuration */
export const DEFAULT_GAME_CONFIG: GameConfig = {
  mapSize: 'medium',
  seed: Date.now(),
  cityName: 'New City',
  countryPreset: 'generic',
};

/** Map-size dimensions derived from the base.world plugin. */
export const MAP_SIZE_PRESETS: Record<string, { width: number; height: number }> =
  Object.fromEntries(MAP_SIZES.map((preset) => [preset.id, { width: preset.width, height: preset.height }]));

/** Resolve map dimensions from plugin presets, with safe fallback for invalid IDs. */
export function resolveMapDimensions(mapSize: GameConfig['mapSize']): { width: number; height: number } {
  const preset = getMapSize(mapSize);
  if (preset) {
    return { width: preset.width, height: preset.height };
  }
  return MAP_SIZE_PRESETS.medium ?? { width: 192, height: 192 };
}

/**
 * Update loading screen progress.
 */
export function updateLoadingProgress(progress: BootProgress): void {
  const bar = document.getElementById('loading-progress');
  const status = document.getElementById('loading-status');
  if (bar) bar.style.width = `${Math.round(progress.progress * 100)}%`;
  if (status) status.textContent = progress.message;
}

/**
 * Hide loading screen with fade transition.
 */
export function hideLoadingScreen(): void {
  const screen = document.getElementById('loading-screen');
  if (screen) screen.classList.add('hidden');
}

/**
 * Show error screen.
 */
export function showError(message: string): void {
  const screen = document.getElementById('error-screen');
  const msg = document.getElementById('error-message');
  if (screen) screen.classList.add('visible');
  if (msg) msg.textContent = message;
}

/**
 * Boot sequence — initializes the game.
 *
 * Steps:
 * 1. Initialize subsystems
 * 2. Load plugin manifests
 * 3. Create sim and render workers
 * 4. Initialize world with seed and map size
 * 5. Load atlas textures and metadata
 * 6. Start game loop
 */
export async function boot(config: GameConfig = DEFAULT_GAME_CONFIG): Promise<void> {
  try {
    // Stage 1: Init
    updateLoadingProgress({
      stage: BootStage.Init,
      progress: 0.0,
      message: 'Initializing engine...',
    });

    // Stage 2: Load Plugins
    updateLoadingProgress({
      stage: BootStage.LoadPlugins,
      progress: 0.2,
      message: 'Loading plugins...',
    });

    // Stage 3: Create Workers
    updateLoadingProgress({
      stage: BootStage.CreateWorkers,
      progress: 0.4,
      message: 'Creating workers...',
    });

    // Stage 4: Init World
    updateLoadingProgress({
      stage: BootStage.InitWorld,
      progress: 0.6,
      message: `Creating ${config.cityName}...`,
    });

    const mapDims = resolveMapDimensions(config.mapSize);

    // Stage 5: Load Assets
    updateLoadingProgress({
      stage: BootStage.LoadAssets,
      progress: 0.8,
      message: 'Loading assets...',
    });

    // Stage 6: Ready
    updateLoadingProgress({
      stage: BootStage.Ready,
      progress: 1.0,
      message: 'Ready!',
    });

    // Delay before hiding loading screen for visual polish
    await new Promise(resolve => setTimeout(resolve, 500));
    hideLoadingScreen();

    console.log(`[TownBuilder] ${config.cityName} ready — ${mapDims.width}x${mapDims.height} map, seed ${config.seed}`);

  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error occurred';
    showError(message);
    console.error('[TownBuilder] Boot failed:', error);
  }
}

// Auto-boot when DOM is ready (only in browser environment)
if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => boot());
  } else {
    boot();
  }
}
