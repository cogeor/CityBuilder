// @townbuilder/renderer — Overlay renderer for heatmaps and zoning grid

import { worldToScreen, visibleTileRange, type CameraState } from '../projection/index.js';
import {
  RenderPass,
  INSTANCE_BYTE_SIZE,
  packInstance,
  type RenderInstance,
  DEFAULT_INSTANCE,
} from '../types/index.js';

// ─── Enums ──────────────────────────────────────────────────────────────────

/** Overlay type identifiers */
export enum OverlayType {
  None = 0,
  Traffic = 1,
  Power = 2,
  Water = 3,
  Pollution = 4,
  Crime = 5,
  Noise = 6,
  LandValue = 7,
  Desirability = 8,
  Zoning = 9,
}

/** Zone type enum matching engine */
export enum ZoneDisplayType {
  None = 0,
  Residential = 1,
  Commercial = 2,
  Industrial = 3,
}

/** Status icon type */
export enum StatusIconType {
  NoPower = 0,
  NoWater = 1,
  OnFire = 2,
  Abandoned = 3,
}

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** Color with RGBA channels (0-255) */
export interface OverlayColor {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** Color gradient stop */
export interface GradientStop {
  value: number; // 0.0 to 1.0, normalized position in gradient
  color: OverlayColor;
}

/** Overlay configuration */
export interface OverlayConfig {
  type: OverlayType;
  name: string;
  gradient: GradientStop[];
  opacity: number; // 0.0 to 1.0
}

/** Overlay render stats */
export interface OverlayRenderStats {
  tileCount: number;
  instanceCount: number;
  activeOverlay: OverlayType;
}

// ─── Constants ──────────────────────────────────────────────────────────────

/** Sprite ID used for flat overlay tile sprites. */
export const OVERLAY_SPRITE_ID = 9500;

/** Sprite IDs for status icon types. */
export const STATUS_ICON_SPRITES: Record<StatusIconType, number> = {
  [StatusIconType.NoPower]: 9600,
  [StatusIconType.NoWater]: 9601,
  [StatusIconType.OnFire]: 9602,
  [StatusIconType.Abandoned]: 9603,
};

/** Vertical offset (in pixels) for status icons above buildings. */
const STATUS_ICON_OFFSET_Y = -48;

// ─── Gradient Presets ───────────────────────────────────────────────────────

export const GRADIENT_GREEN_RED: GradientStop[] = [
  { value: 0.0, color: { r: 0, g: 200, b: 0, a: 160 } },
  { value: 0.5, color: { r: 200, g: 200, b: 0, a: 160 } },
  { value: 1.0, color: { r: 200, g: 0, b: 0, a: 160 } },
];

export const GRADIENT_BLUE_BROWN: GradientStop[] = [
  { value: 0.0, color: { r: 0, g: 100, b: 200, a: 160 } },
  { value: 0.5, color: { r: 100, g: 180, b: 100, a: 160 } },
  { value: 1.0, color: { r: 160, g: 100, b: 40, a: 160 } },
];

export const GRADIENT_COOL_HOT: GradientStop[] = [
  { value: 0.0, color: { r: 50, g: 50, b: 200, a: 160 } },
  { value: 0.5, color: { r: 200, g: 200, b: 50, a: 160 } },
  { value: 1.0, color: { r: 200, g: 50, b: 50, a: 160 } },
];

export const ZONE_COLORS: Record<ZoneDisplayType, OverlayColor> = {
  [ZoneDisplayType.None]: { r: 0, g: 0, b: 0, a: 0 },
  [ZoneDisplayType.Residential]: { r: 50, g: 200, b: 50, a: 140 },
  [ZoneDisplayType.Commercial]: { r: 50, g: 50, b: 200, a: 140 },
  [ZoneDisplayType.Industrial]: { r: 200, g: 200, b: 50, a: 140 },
};

// ─── Overlay Defaults ───────────────────────────────────────────────────────

export const DEFAULT_OVERLAY_CONFIGS: Record<OverlayType, OverlayConfig> = {
  [OverlayType.None]: { type: OverlayType.None, name: 'None', gradient: [], opacity: 0 },
  [OverlayType.Traffic]: { type: OverlayType.Traffic, name: 'Traffic', gradient: GRADIENT_GREEN_RED, opacity: 0.6 },
  [OverlayType.Power]: { type: OverlayType.Power, name: 'Power', gradient: GRADIENT_COOL_HOT, opacity: 0.6 },
  [OverlayType.Water]: { type: OverlayType.Water, name: 'Water', gradient: GRADIENT_BLUE_BROWN, opacity: 0.6 },
  [OverlayType.Pollution]: { type: OverlayType.Pollution, name: 'Pollution', gradient: GRADIENT_GREEN_RED, opacity: 0.6 },
  [OverlayType.Crime]: { type: OverlayType.Crime, name: 'Crime', gradient: GRADIENT_GREEN_RED, opacity: 0.6 },
  [OverlayType.Noise]: { type: OverlayType.Noise, name: 'Noise', gradient: GRADIENT_COOL_HOT, opacity: 0.6 },
  [OverlayType.LandValue]: { type: OverlayType.LandValue, name: 'Land Value', gradient: GRADIENT_BLUE_BROWN, opacity: 0.6 },
  [OverlayType.Desirability]: { type: OverlayType.Desirability, name: 'Desirability', gradient: GRADIENT_GREEN_RED, opacity: 0.6 },
  [OverlayType.Zoning]: { type: OverlayType.Zoning, name: 'Zoning', gradient: [], opacity: 0.5 },
};

// ─── Utility Functions ──────────────────────────────────────────────────────

/**
 * Sample a color from a gradient at a normalized value (0.0-1.0).
 * Linearly interpolates between gradient stops.
 */
export function sampleGradient(gradient: GradientStop[], normalizedValue: number): OverlayColor {
  // Clamp to [0, 1]
  const t = Math.max(0, Math.min(1, normalizedValue));

  if (gradient.length === 0) return { r: 0, g: 0, b: 0, a: 0 };
  if (gradient.length === 1) return { ...gradient[0].color };

  // Find the two stops to interpolate between
  for (let i = 0; i < gradient.length - 1; i++) {
    if (t >= gradient[i].value && t <= gradient[i + 1].value) {
      const range = gradient[i + 1].value - gradient[i].value;
      const localT = range > 0 ? (t - gradient[i].value) / range : 0;
      const a = gradient[i].color;
      const b = gradient[i + 1].color;
      return {
        r: Math.round(a.r + (b.r - a.r) * localT),
        g: Math.round(a.g + (b.g - a.g) * localT),
        b: Math.round(a.b + (b.b - a.b) * localT),
        a: Math.round(a.a + (b.a - a.a) * localT),
      };
    }
  }

  return { ...gradient[gradient.length - 1].color };
}

/**
 * Normalize a u16 heatmap value (0-65535) to 0.0-1.0.
 */
export function normalizeHeatmapValue(value: number, maxValue: number = 65535): number {
  return Math.max(0, Math.min(1, value / maxValue));
}

// ─── OverlayRenderer Class ──────────────────────────────────────────────────

/**
 * Builds render instances for heatmap overlays, zoning grids, and status icons.
 *
 * Heatmap overlays map a u16 grid value to a color gradient.
 * Zoning overlays render color-coded tiles for zone types.
 * Status icons show issue indicators above buildings.
 */
export class OverlayRenderer {
  private activeOverlay: OverlayType;
  private configs: Record<OverlayType, OverlayConfig>;
  private camera: CameraState;

  constructor(camera: CameraState) {
    this.activeOverlay = OverlayType.None;
    this.configs = { ...DEFAULT_OVERLAY_CONFIGS };
    this.camera = camera;
  }

  /** Set active overlay type. */
  setOverlay(type: OverlayType): void {
    this.activeOverlay = type;
  }

  /** Get current active overlay. */
  getOverlay(): OverlayType {
    return this.activeOverlay;
  }

  /** Toggle overlay (set to None if already active). */
  toggleOverlay(type: OverlayType): void {
    this.activeOverlay = this.activeOverlay === type ? OverlayType.None : type;
  }

  /** Update camera state. */
  updateCamera(camera: CameraState): void {
    this.camera = camera;
  }

  /**
   * Build overlay instances for the visible area from a heatmap.
   *
   * For each visible tile, calls getHeatmapValue(x, y) to obtain a u16 value,
   * normalizes it, samples the active overlay gradient, and creates a colored
   * RenderInstance at the tile's screen position.
   */
  buildHeatmapInstances(
    getHeatmapValue: (x: number, y: number) => number,
    mapWidth: number,
    mapHeight: number,
  ): { instances: Float32Array; count: number; stats: OverlayRenderStats } {
    const config = this.configs[this.activeOverlay];
    if (!config || this.activeOverlay === OverlayType.None || config.gradient.length === 0) {
      return {
        instances: new Float32Array(0),
        count: 0,
        stats: { tileCount: 0, instanceCount: 0, activeOverlay: this.activeOverlay },
      };
    }

    const visible = visibleTileRange(this.camera);
    const minX = Math.max(0, visible.minX);
    const maxX = Math.min(mapWidth - 1, visible.maxX);
    const minY = Math.max(0, visible.minY);
    const maxY = Math.min(mapHeight - 1, visible.maxY);

    const tileInstances: RenderInstance[] = [];
    let tileCount = 0;

    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        const rawValue = getHeatmapValue(x, y);
        const normalized = normalizeHeatmapValue(rawValue);
        const color = sampleGradient(config.gradient, normalized);

        // Apply overlay opacity to the color alpha
        const alpha = Math.round(color.a * config.opacity);

        const screen = worldToScreen(x, y, 0, this.camera);

        const instance: RenderInstance = {
          ...DEFAULT_INSTANCE,
          sprite_id: OVERLAY_SPRITE_ID,
          screen_x: screen.x,
          screen_y: screen.y,
          z_order: RenderPass.Overlays * 4294967296 + (x + y) * 65536,
          tint_r: color.r,
          tint_g: color.g,
          tint_b: color.b,
          tint_a: alpha,
        };

        tileInstances.push(instance);
        tileCount++;
      }
    }

    // Pack into Float32Array
    const byteLength = tileInstances.length * INSTANCE_BYTE_SIZE;
    const buffer = new ArrayBuffer(byteLength);
    const view = new DataView(buffer);

    for (let i = 0; i < tileInstances.length; i++) {
      packInstance(tileInstances[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return {
      instances: new Float32Array(buffer),
      count: tileInstances.length,
      stats: {
        tileCount,
        instanceCount: tileInstances.length,
        activeOverlay: this.activeOverlay,
      },
    };
  }

  /**
   * Build zoning grid overlay instances.
   *
   * For each visible tile, calls getZoneType(x, y) to obtain the zone type,
   * and creates a colored RenderInstance using ZONE_COLORS. Tiles with
   * ZoneDisplayType.None are skipped.
   */
  buildZoningInstances(
    getZoneType: (x: number, y: number) => ZoneDisplayType,
    mapWidth: number,
    mapHeight: number,
  ): { instances: Float32Array; count: number; stats: OverlayRenderStats } {
    const config = this.configs[OverlayType.Zoning];
    const visible = visibleTileRange(this.camera);
    const minX = Math.max(0, visible.minX);
    const maxX = Math.min(mapWidth - 1, visible.maxX);
    const minY = Math.max(0, visible.minY);
    const maxY = Math.min(mapHeight - 1, visible.maxY);

    const tileInstances: RenderInstance[] = [];
    let tileCount = 0;

    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        const zoneType = getZoneType(x, y);
        if (zoneType === ZoneDisplayType.None) continue;

        const color = ZONE_COLORS[zoneType];
        if (!color || color.a === 0) continue;

        const alpha = Math.round(color.a * config.opacity);
        const screen = worldToScreen(x, y, 0, this.camera);

        const instance: RenderInstance = {
          ...DEFAULT_INSTANCE,
          sprite_id: OVERLAY_SPRITE_ID,
          screen_x: screen.x,
          screen_y: screen.y,
          z_order: RenderPass.Overlays * 4294967296 + (x + y) * 65536,
          tint_r: color.r,
          tint_g: color.g,
          tint_b: color.b,
          tint_a: alpha,
        };

        tileInstances.push(instance);
        tileCount++;
      }
    }

    // Pack into Float32Array
    const byteLength = tileInstances.length * INSTANCE_BYTE_SIZE;
    const buffer = new ArrayBuffer(byteLength);
    const view = new DataView(buffer);

    for (let i = 0; i < tileInstances.length; i++) {
      packInstance(tileInstances[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return {
      instances: new Float32Array(buffer),
      count: tileInstances.length,
      stats: {
        tileCount,
        instanceCount: tileInstances.length,
        activeOverlay: OverlayType.Zoning,
      },
    };
  }

  /**
   * Build status icon instances for buildings with issues.
   *
   * Each icon is rendered above the building's tile position using a
   * sprite ID from STATUS_ICON_SPRITES.
   */
  buildStatusIcons(
    icons: Array<{ tileX: number; tileY: number; icon: StatusIconType }>,
  ): { instances: Float32Array; count: number } {
    if (icons.length === 0) {
      return { instances: new Float32Array(0), count: 0 };
    }

    const iconInstances: RenderInstance[] = [];

    for (const entry of icons) {
      const screen = worldToScreen(entry.tileX, entry.tileY, 0, this.camera);
      const spriteId = STATUS_ICON_SPRITES[entry.icon];

      const instance: RenderInstance = {
        ...DEFAULT_INSTANCE,
        sprite_id: spriteId,
        screen_x: screen.x,
        screen_y: screen.y + STATUS_ICON_OFFSET_Y,
        z_order: RenderPass.Overlays * 4294967296 + (entry.tileX + entry.tileY) * 65536 + 256,
      };

      iconInstances.push(instance);
    }

    // Pack into Float32Array
    const byteLength = iconInstances.length * INSTANCE_BYTE_SIZE;
    const buffer = new ArrayBuffer(byteLength);
    const view = new DataView(buffer);

    for (let i = 0; i < iconInstances.length; i++) {
      packInstance(iconInstances[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return {
      instances: new Float32Array(buffer),
      count: iconInstances.length,
    };
  }
}
