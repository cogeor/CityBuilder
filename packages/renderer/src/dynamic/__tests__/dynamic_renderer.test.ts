import { describe, it, expect } from 'vitest';
import {
  AutomataType,
  type DynamicEntity,
  type SelectionHighlight,
  interpolatePosition,
  computeAnimFrame,
  computeSpriteOffset,
  DynamicRenderer,
} from '../dynamic_renderer.js';
import { INSTANCE_BYTE_SIZE, unpackInstance } from '../../types/index.js';
import type { CameraState } from '../../projection/index.js';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeCamera(overrides: Partial<CameraState> = {}): CameraState {
  return {
    x: 0,
    y: 0,
    zoom: 1,
    viewportWidth: 800,
    viewportHeight: 600,
    ...overrides,
  };
}

function makeEntity(overrides: Partial<DynamicEntity> = {}): DynamicEntity {
  return {
    id: 1,
    type: AutomataType.Car,
    tileX: 5,
    tileY: 5,
    prevTileX: 5,
    prevTileY: 5,
    direction: 0,
    spriteId: 100,
    atlasId: 0,
    frameCount: 4,
    selected: false,
    flags: 0,
    ...overrides,
  };
}

/** Unpack all instances from a buildInstances result. */
function unpackAll(result: { instances: Float32Array; count: number }) {
  const view = new DataView(
    result.instances.buffer,
    result.instances.byteOffset,
    result.instances.byteLength,
  );
  const out = [];
  for (let i = 0; i < result.count; i++) {
    out.push(unpackInstance(view, i * INSTANCE_BYTE_SIZE));
  }
  return out;
}

// ─── interpolatePosition ──────────────────────────────────────────────────────

describe('interpolatePosition', () => {
  it('returns prev position at alpha=0', () => {
    const [x, y] = interpolatePosition(2, 3, 6, 7, 0);
    expect(x).toBe(2);
    expect(y).toBe(3);
  });

  it('returns current position at alpha=1', () => {
    const [x, y] = interpolatePosition(2, 3, 6, 7, 1);
    expect(x).toBe(6);
    expect(y).toBe(7);
  });

  it('returns midpoint at alpha=0.5', () => {
    const [x, y] = interpolatePosition(0, 0, 10, 20, 0.5);
    expect(x).toBe(5);
    expect(y).toBe(10);
  });

  it('handles negative coordinates', () => {
    const [x, y] = interpolatePosition(-4, -8, 4, 8, 0.5);
    expect(x).toBe(0);
    expect(y).toBe(0);
  });
});

// ─── computeAnimFrame ────────────────────────────────────────────────────────

describe('computeAnimFrame', () => {
  it('cycles through frames', () => {
    expect(computeAnimFrame(0, 4)).toBe(0);
    expect(computeAnimFrame(1, 4)).toBe(1);
    expect(computeAnimFrame(2, 4)).toBe(2);
    expect(computeAnimFrame(3, 4)).toBe(3);
  });

  it('returns 0 for single frame', () => {
    expect(computeAnimFrame(5, 1)).toBe(0);
    expect(computeAnimFrame(100, 1)).toBe(0);
  });

  it('returns 0 for zero frameCount', () => {
    expect(computeAnimFrame(5, 0)).toBe(0);
  });

  it('wraps at frameCount boundary', () => {
    expect(computeAnimFrame(4, 4)).toBe(0);
    expect(computeAnimFrame(5, 4)).toBe(1);
    expect(computeAnimFrame(7, 4)).toBe(3);
    expect(computeAnimFrame(8, 4)).toBe(0);
  });
});

// ─── computeSpriteOffset ──────────────────────────────────────────────────────

describe('computeSpriteOffset', () => {
  it('combines base, direction, and frame', () => {
    // base=100, direction=2, animFrame=3 => 100 + 2*8 + 3 = 119
    expect(computeSpriteOffset(100, 2, 3)).toBe(119);
  });

  it('returns base sprite for direction 0 frame 0', () => {
    expect(computeSpriteOffset(100, 0, 0)).toBe(100);
  });

  it('offsets by 8 per direction', () => {
    expect(computeSpriteOffset(0, 1, 0)).toBe(8);
    expect(computeSpriteOffset(0, 3, 0)).toBe(24);
    expect(computeSpriteOffset(0, 7, 0)).toBe(56);
  });
});

// ─── DynamicRenderer ──────────────────────────────────────────────────────────

describe('DynamicRenderer', () => {
  it('constructor sets camera', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    // No direct access to private camera, but buildInstances should work
    const result = renderer.buildInstances([], []);
    expect(result.count).toBe(0);
  });

  it('updateTiming updates tick counter and alpha', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    renderer.updateTiming(10, 0.5);
    // Verify by checking anim_frame in output
    const entity = makeEntity({ frameCount: 4 });
    const result = renderer.buildInstances([entity], []);
    const instances = unpackAll(result);
    // tick=10, frameCount=4 => animFrame = 10 % 4 = 2
    expect(instances[0].anim_frame).toBe(2);
  });

  it('buildInstances generates instances for entities', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const entities = [makeEntity(), makeEntity({ id: 2, tileX: 10, tileY: 10, prevTileX: 10, prevTileY: 10 })];
    const result = renderer.buildInstances(entities, []);
    expect(result.count).toBe(2);
    expect(result.stats.entityCount).toBe(2);
    expect(result.stats.instanceCount).toBe(2);
  });

  it('buildInstances interpolates moving entities', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    renderer.updateTiming(0, 0.5);

    const entity = makeEntity({
      tileX: 10,
      tileY: 10,
      prevTileX: 8,
      prevTileY: 8,
    });
    const result = renderer.buildInstances([entity], []);
    expect(result.stats.interpolatedCount).toBe(1);

    // At alpha=0.5, position should be (9, 9)
    const instances = unpackAll(result);
    expect(instances.length).toBe(1);
    // The z_order should be based on interpolated position: floor(9*256 + 9) = 2313
    expect(instances[0].z_order).toBe(Math.floor(9 * 256 + 9));
  });

  it('buildInstances adds selection highlight for selected entities', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const entity = makeEntity({ selected: true });
    const result = renderer.buildInstances([entity], []);

    // Should have 2 instances: entity + selection highlight
    expect(result.count).toBe(2);
    expect(result.stats.instanceCount).toBe(2);

    const instances = unpackAll(result);
    // Second instance is the selection highlight
    expect(instances[1].sprite_id).toBe(9000);
    expect(instances[1].tint_r).toBe(255);
    expect(instances[1].tint_g).toBe(255);
    expect(instances[1].tint_b).toBe(0);
    expect(instances[1].tint_a).toBe(200);
  });

  it('buildInstances generates tile selection highlights', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const sel: SelectionHighlight = {
      tileX: 3,
      tileY: 4,
      width: 2,
      height: 2,
      color: { r: 0, g: 255, b: 0, a: 128 },
    };
    const result = renderer.buildInstances([], [sel]);

    // 2x2 = 4 tile highlight instances
    expect(result.count).toBe(4);
    const instances = unpackAll(result);
    for (const inst of instances) {
      expect(inst.sprite_id).toBe(9000);
      expect(inst.tint_r).toBe(0);
      expect(inst.tint_g).toBe(255);
      expect(inst.tint_b).toBe(0);
      expect(inst.tint_a).toBe(128);
    }
  });

  it('buildInstances returns correct stats', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    renderer.updateTiming(0, 0.5);

    const entities = [
      makeEntity({ id: 1 }),
      makeEntity({ id: 2, selected: true }),
      makeEntity({ id: 3, tileX: 6, prevTileX: 4 }),
    ];
    const result = renderer.buildInstances(entities, []);

    expect(result.stats.entityCount).toBe(3);
    // 3 entities + 1 selection highlight = 4 instances
    expect(result.stats.instanceCount).toBe(4);
    // Only entity 3 has different prev/curr position
    expect(result.stats.interpolatedCount).toBe(1);
  });

  it('buildInstances handles empty input', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const result = renderer.buildInstances([], []);

    expect(result.count).toBe(0);
    expect(result.instances.length).toBe(0);
    expect(result.stats.entityCount).toBe(0);
    expect(result.stats.instanceCount).toBe(0);
    expect(result.stats.interpolatedCount).toBe(0);
  });

  it('updateCamera changes projection', () => {
    const camera1 = makeCamera({ x: 0, y: 0 });
    const camera2 = makeCamera({ x: 5, y: 0 });
    const renderer = new DynamicRenderer(camera1);

    const entity = makeEntity({ tileX: 5, tileY: 5, prevTileX: 5, prevTileY: 5 });

    const result1 = renderer.buildInstances([entity], []);
    const instances1 = unpackAll(result1);

    renderer.updateCamera(camera2);
    const result2 = renderer.buildInstances([entity], []);
    const instances2 = unpackAll(result2);

    // Different camera position should yield different screen coordinates
    expect(instances1[0].screen_x).not.toBe(instances2[0].screen_x);
    expect(instances1[0].screen_y).not.toBe(instances2[0].screen_y);
  });

  it('entity flags are stored in mask_flags', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const entity = makeEntity({ flags: 42 });
    const result = renderer.buildInstances([entity], []);
    const instances = unpackAll(result);
    expect(instances[0].mask_flags).toBe(42);
  });

  it('non-moving entity has interpolatedCount=0', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    renderer.updateTiming(0, 0.5);
    const entity = makeEntity({ tileX: 5, tileY: 5, prevTileX: 5, prevTileY: 5 });
    const result = renderer.buildInstances([entity], []);
    expect(result.stats.interpolatedCount).toBe(0);
  });

  it('packed buffer has correct byte size', () => {
    const camera = makeCamera();
    const renderer = new DynamicRenderer(camera);
    const entities = [makeEntity(), makeEntity({ id: 2 })];
    const result = renderer.buildInstances(entities, []);
    // 2 instances * 48 bytes / 4 bytes per float = 24 floats
    expect(result.instances.length).toBe(2 * (INSTANCE_BYTE_SIZE / 4));
  });
});
