// @townbuilder/renderer — Dynamic renderer for automata and animated elements

import { worldToScreen, type CameraState } from '../projection/index.js';
import { INSTANCE_BYTE_SIZE, packInstance, type RenderInstance, DEFAULT_INSTANCE } from '../types/index.js';

// ─── Enums ──────────────────────────────────────────────────────────────────

/** Automata entity type */
export enum AutomataType {
  Car = 0,
  Pedestrian = 1,
  ServiceVehicle = 2,  // fire truck, police, ambulance
  Effect = 3,          // smoke, sparks, etc.
}

// ─── Interfaces ─────────────────────────────────────────────────────────────

/** A dynamic entity to be rendered */
export interface DynamicEntity {
  id: number;
  type: AutomataType;
  /** Current position in tile coordinates (fixed-point, Q16.16 stored as float) */
  tileX: number;
  tileY: number;
  /** Previous position for interpolation */
  prevTileX: number;
  prevTileY: number;
  /** Direction of travel (0-7, N/NE/E/SE/S/SW/W/NW) */
  direction: number;
  /** Sprite base ID */
  spriteId: number;
  /** Atlas ID */
  atlasId: number;
  /** Animation frame count for this entity type */
  frameCount: number;
  /** Is this entity selected/highlighted? */
  selected: boolean;
  /** Entity-specific flags */
  flags: number;
}

/** Selection highlight configuration */
export interface SelectionHighlight {
  tileX: number;
  tileY: number;
  width: number;   // in tiles
  height: number;  // in tiles
  color: { r: number; g: number; b: number; a: number };
}

/** Stats for dynamic rendering */
export interface DynamicRenderStats {
  entityCount: number;
  instanceCount: number;
  interpolatedCount: number;
}

// ─── Constants ──────────────────────────────────────────────────────────────

const DIRECTION_FRAME_OFFSET = 8;  // each direction adds 8 to sprite offset
const SELECTION_SPRITE_ID = 9000;  // sprite for selection highlight
const SELECTION_ATLAS_ID = 0;

// ─── Helper Functions ───────────────────────────────────────────────────────

/**
 * Compute interpolated position between prev and current.
 * alpha is 0.0 (at prev tick) to 1.0 (at current tick).
 */
export function interpolatePosition(
  prevX: number, prevY: number,
  currX: number, currY: number,
  alpha: number,
): [number, number] {
  return [
    prevX + (currX - prevX) * alpha,
    prevY + (currY - prevY) * alpha,
  ];
}

/**
 * Compute animation frame from tick counter and frame count.
 * Wraps around using modulo.
 */
export function computeAnimFrame(tickCounter: number, frameCount: number): number {
  if (frameCount <= 1) return 0;
  return tickCounter % frameCount;
}

/**
 * Compute sprite offset for direction and animation.
 * spriteId + direction * DIRECTION_FRAME_OFFSET + animFrame
 */
export function computeSpriteOffset(baseSprite: number, direction: number, animFrame: number): number {
  return baseSprite + direction * DIRECTION_FRAME_OFFSET + animFrame;
}

// ─── DynamicRenderer Class ──────────────────────────────────────────────────

/**
 * Builds instance buffers for render pass 5 (Automata) — cars, pedestrians,
 * service vehicles, and animated effects.
 */
export class DynamicRenderer {
  private camera: CameraState;
  private tickCounter: number;
  private interpAlpha: number;

  constructor(camera: CameraState) {
    this.camera = camera;
    this.tickCounter = 0;
    this.interpAlpha = 0;
  }

  /** Update timing state for interpolation */
  updateTiming(tickCounter: number, interpAlpha: number): void {
    this.tickCounter = tickCounter;
    this.interpAlpha = interpAlpha;
  }

  /** Update camera for visibility culling */
  updateCamera(camera: CameraState): void {
    this.camera = camera;
  }

  /**
   * Build render instances for all dynamic entities.
   * Returns packed Float32Array and instance count.
   */
  buildInstances(
    entities: DynamicEntity[],
    selections: SelectionHighlight[],
  ): { instances: Float32Array; count: number; stats: DynamicRenderStats } {
    const allInstances: RenderInstance[] = [];
    let interpolatedCount = 0;

    for (const entity of entities) {
      // Interpolate position
      const [interpX, interpY] = interpolatePosition(
        entity.prevTileX, entity.prevTileY,
        entity.tileX, entity.tileY,
        this.interpAlpha,
      );

      const needsInterp = entity.prevTileX !== entity.tileX || entity.prevTileY !== entity.tileY;
      if (needsInterp) interpolatedCount++;

      // Compute animation frame
      const animFrame = computeAnimFrame(this.tickCounter, entity.frameCount);
      const spriteOffset = computeSpriteOffset(entity.spriteId, entity.direction, animFrame);

      // Convert to screen coordinates
      const screen = worldToScreen(interpX, interpY, 0, this.camera);

      allInstances.push({
        ...DEFAULT_INSTANCE,
        screen_x: screen.x,
        screen_y: screen.y,
        sprite_id: spriteOffset,
        atlas_id: entity.atlasId,
        z_order: Math.floor(interpY * 256 + interpX),
        anim_frame: animFrame,
        mask_flags: entity.flags,
        tint_r: 255,
        tint_g: 255,
        tint_b: 255,
        tint_a: 255,
      });

      // Add selection highlight if selected
      if (entity.selected) {
        allInstances.push({
          ...DEFAULT_INSTANCE,
          screen_x: screen.x,
          screen_y: screen.y - 2,  // slight offset above
          sprite_id: SELECTION_SPRITE_ID,
          atlas_id: SELECTION_ATLAS_ID,
          z_order: Math.floor(interpY * 256 + interpX) + 1,
          tint_r: 255,
          tint_g: 255,
          tint_b: 0,
          tint_a: 200,
        });
      }
    }

    // Add selection highlights for tiles/buildings
    for (const sel of selections) {
      for (let dy = 0; dy < sel.height; dy++) {
        for (let dx = 0; dx < sel.width; dx++) {
          const tx = sel.tileX + dx;
          const ty = sel.tileY + dy;
          const screen = worldToScreen(tx, ty, 0, this.camera);
          allInstances.push({
            ...DEFAULT_INSTANCE,
            screen_x: screen.x,
            screen_y: screen.y,
            sprite_id: SELECTION_SPRITE_ID,
            atlas_id: SELECTION_ATLAS_ID,
            z_order: 0,
            tint_r: sel.color.r,
            tint_g: sel.color.g,
            tint_b: sel.color.b,
            tint_a: sel.color.a,
          });
        }
      }
    }

    // Pack instances into GPU-ready buffer
    const byteLength = allInstances.length * INSTANCE_BYTE_SIZE;
    const arrayBuffer = new ArrayBuffer(byteLength);
    const view = new DataView(arrayBuffer);

    for (let i = 0; i < allInstances.length; i++) {
      packInstance(allInstances[i], view, i * INSTANCE_BYTE_SIZE);
    }

    return {
      instances: new Float32Array(arrayBuffer),
      count: allInstances.length,
      stats: {
        entityCount: entities.length,
        instanceCount: allInstances.length,
        interpolatedCount,
      },
    };
  }
}
