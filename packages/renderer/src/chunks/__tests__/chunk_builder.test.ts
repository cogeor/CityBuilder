import { describe, it, expect } from 'vitest';
import {
  TerrainType,
  ZoneType,
  RoadType,
  type TileRenderData,
  type SpriteResolver,
  computeRoadMask,
  computeTerrainEdgeMask,
  ChunkBuilder,
} from '../chunk_builder.js';
import { CHUNK_SIZE } from '../chunk_cache.js';
import { INSTANCE_BYTE_SIZE, unpackInstance, RenderPass } from '../../types/index.js';
import { depthKey } from '../../projection/index.js';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function makeTile(overrides: Partial<TileRenderData> = {}): TileRenderData {
  return {
    terrain: TerrainType.Grass,
    elevation: 0,
    zone: ZoneType.None,
    road: RoadType.None,
    entityId: 0,
    archetypeId: 0,
    flags: 0,
    ...overrides,
  };
}

/** Unpack all instances from a buildChunk result. */
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

// ─── computeRoadMask ──────────────────────────────────────────────────────────

describe('computeRoadMask', () => {
  it('returns 0 when all false', () => {
    expect(computeRoadMask(false, false, false, false)).toBe(0);
  });

  it('returns 15 when all true', () => {
    expect(computeRoadMask(true, true, true, true)).toBe(15);
  });

  it('north sets bit 0', () => {
    expect(computeRoadMask(true, false, false, false)).toBe(1);
  });

  it('east sets bit 1', () => {
    expect(computeRoadMask(false, true, false, false)).toBe(2);
  });

  it('south sets bit 2', () => {
    expect(computeRoadMask(false, false, true, false)).toBe(4);
  });

  it('west sets bit 3', () => {
    expect(computeRoadMask(false, false, false, true)).toBe(8);
  });

  it('north+south gives 5', () => {
    expect(computeRoadMask(true, false, true, false)).toBe(5);
  });
});

// ─── computeTerrainEdgeMask ───────────────────────────────────────────────────

describe('computeTerrainEdgeMask', () => {
  it('returns 0 when no matching neighbors', () => {
    expect(computeTerrainEdgeMask(false, false, false, false)).toBe(0);
  });

  it('returns 15 when all neighbors match', () => {
    expect(computeTerrainEdgeMask(true, true, true, true)).toBe(15);
  });

  it('north+west matching gives 9', () => {
    expect(computeTerrainEdgeMask(true, false, false, true)).toBe(9);
  });

  it('east+south matching gives 6', () => {
    expect(computeTerrainEdgeMask(false, true, true, false)).toBe(6);
  });
});

// ─── ChunkBuilder constructor ─────────────────────────────────────────────────

describe('ChunkBuilder', () => {
  it('constructor accepts map dimensions', () => {
    const builder = new ChunkBuilder(128, 128);
    expect(builder).toBeDefined();
  });

  it('constructor accepts optional sprite resolver', () => {
    const resolver: SpriteResolver = (_t, _id, _v) => ({
      spriteId: 1,
      atlasId: 0,
      variant: 0,
    });
    const builder = new ChunkBuilder(64, 64, resolver);
    expect(builder).toBeDefined();
  });

  // ─── buildChunk: empty ────────────────────────────────────────────

  it('returns empty for chunk with no tiles', () => {
    const builder = new ChunkBuilder(64, 64);
    const result = builder.buildChunk(0, 0, () => null);
    expect(result.count).toBe(0);
    expect(result.instances.length).toBe(0);
  });

  // ─── buildChunk: terrain ──────────────────────────────────────────

  it('generates terrain instances for tiles', () => {
    const builder = new ChunkBuilder(64, 64);
    const tile = makeTile({ terrain: TerrainType.Water });
    const result = builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });
    expect(result.count).toBe(1);
    const unpacked = unpackAll(result);
    // Water terrain sprite = 2
    expect(unpacked[0].sprite_id).toBe(2);
  });

  // ─── buildChunk: roads ────────────────────────────────────────────

  it('generates road instances for tiles with roads', () => {
    const builder = new ChunkBuilder(64, 64);
    const tile = makeTile({ road: RoadType.Local });
    const result = builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });
    // 1 terrain + 1 road = 2 instances
    expect(result.count).toBe(2);
    const unpacked = unpackAll(result);
    // Road instance should have sprite_id = 10 (Local default)
    const road = unpacked.find((i) => i.sprite_id === 10);
    expect(road).toBeDefined();
  });

  // ─── buildChunk: buildings ────────────────────────────────────────

  it('generates building instances for tiles with entities', () => {
    const builder = new ChunkBuilder(64, 64);
    const tile = makeTile({ entityId: 5, archetypeId: 3 });
    const result = builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });
    // 1 terrain + 1 building + 1 shadow = 3
    expect(result.count).toBe(3);
    const unpacked = unpackAll(result);
    // Building sprite = archetypeId * 10 = 30
    const building = unpacked.find(
      (i) =>
        i.sprite_id === 30 &&
        i.z_order === depthKey(0, 0, 0, RenderPass.Buildings, 0),
    );
    expect(building).toBeDefined();
  });

  // ─── buildChunk: shadow ───────────────────────────────────────────

  it('generates shadow for buildings with semi-transparent tint', () => {
    const builder = new ChunkBuilder(64, 64);
    const tile = makeTile({ entityId: 1, archetypeId: 2 });
    const result = builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });
    const unpacked = unpackAll(result);
    // Shadow: tint_a=128, tint_r/g/b=0
    const shadow = unpacked.find(
      (i) => i.tint_a === 128 && i.tint_r === 0 && i.tint_g === 0 && i.tint_b === 0,
    );
    expect(shadow).toBeDefined();
    // Shadow depth should be on terrain layer (lower than building)
    expect(shadow!.z_order).toBe(depthKey(0, 0, 0, RenderPass.Terrain, 1));
  });

  // ─── buildChunk: map bounds clamping ──────────────────────────────

  it('clamps to map bounds for edge chunks', () => {
    // Map is 10x10, chunk 0 covers 0..31 but should clamp to 0..9
    const builder = new ChunkBuilder(10, 10);
    const result = builder.buildChunk(0, 0, (_x, _y) => makeTile());
    // 10x10 = 100 terrain instances (no roads, no buildings)
    expect(result.count).toBe(100);
  });

  // ─── buildChunk: depth sorting ────────────────────────────────────

  it('sorts instances by depth ascending', () => {
    const builder = new ChunkBuilder(64, 64);
    // Two tiles at different positions
    const result = builder.buildChunk(0, 0, (x, y) => {
      if ((x === 0 && y === 0) || (x === 1 && y === 1)) {
        return makeTile();
      }
      return null;
    });
    expect(result.count).toBe(2);
    const unpacked = unpackAll(result);
    // z_order should be ascending
    expect(unpacked[0].z_order).toBeLessThanOrEqual(unpacked[1].z_order);
  });

  // ─── buildChunk: custom sprite resolver ───────────────────────────

  it('calls custom sprite resolver correctly', () => {
    const calls: Array<{ type: string; id: number; variant: number }> = [];
    const resolver: SpriteResolver = (type, id, variant) => {
      calls.push({ type, id, variant });
      return { spriteId: 99, atlasId: 1, variant: 7 };
    };

    const builder = new ChunkBuilder(64, 64, resolver);
    const tile = makeTile({
      terrain: TerrainType.Forest,
      road: RoadType.Arterial,
      entityId: 1,
      archetypeId: 5,
    });

    builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });

    // Should have called resolver for terrain, road, building (x2 for shadow)
    expect(calls.length).toBe(4);
    expect(calls[0].type).toBe('terrain');
    expect(calls[0].id).toBe(TerrainType.Forest);
    expect(calls[1].type).toBe('road');
    expect(calls[1].id).toBe(RoadType.Arterial);
    expect(calls[2].type).toBe('building');
    expect(calls[2].id).toBe(5);
    expect(calls[3].type).toBe('building'); // shadow also resolves sprite

    // All instances should use resolver results
    const result = builder.buildChunk(0, 0, (x, y) => {
      return x === 0 && y === 0 ? tile : null;
    });
    const unpacked = unpackAll(result);
    for (const inst of unpacked) {
      expect(inst.sprite_id).toBe(99);
      expect(inst.atlas_id).toBe(1);
    }
  });

  // ─── buildChunk: multiple tiles ───────────────────────────────────

  it('generates multiple instances for multiple tiles', () => {
    const builder = new ChunkBuilder(64, 64);
    // 4 tiles in a 2x2 grid, each with terrain only
    const result = builder.buildChunk(0, 0, (x, y) => {
      if (x < 2 && y < 2) return makeTile();
      return null;
    });
    expect(result.count).toBe(4);
  });

  // ─── buildChunk: road auto-tiling uses neighbor data ──────────────

  it('road auto-tiling computes correct connection mask', () => {
    const builder = new ChunkBuilder(64, 64);
    // Place a road at (1,1) with road neighbors at (1,0)=north and (2,1)=east
    const result = builder.buildChunk(0, 0, (x, y) => {
      if (x === 1 && y === 0) return makeTile({ road: RoadType.Local });
      if (x === 1 && y === 1) return makeTile({ road: RoadType.Local });
      if (x === 2 && y === 1) return makeTile({ road: RoadType.Local });
      return null;
    });
    // 3 terrain + 3 roads = 6 instances
    expect(result.count).toBe(6);
  });

  // ─── buildChunk: terrain edge mask for water ──────────────────────

  it('terrain edge mask reflects neighbor terrain types', () => {
    const calls: Array<{ type: string; id: number; variant: number }> = [];
    const resolver: SpriteResolver = (type, id, variant) => {
      calls.push({ type, id, variant });
      return { spriteId: id, atlasId: 0, variant };
    };

    const builder = new ChunkBuilder(64, 64, resolver);
    // Water tile at (1,1) with water at (1,0)=north and (2,1)=east
    builder.buildChunk(0, 0, (x, y) => {
      if (x === 1 && y === 0) return makeTile({ terrain: TerrainType.Water });
      if (x === 1 && y === 1) return makeTile({ terrain: TerrainType.Water });
      if (x === 2 && y === 1) return makeTile({ terrain: TerrainType.Water });
      return null;
    });

    // Find the terrain call for tile (1,1) which should have north+east = 1|2 = 3
    const waterCalls = calls.filter(
      (c) => c.type === 'terrain' && c.id === TerrainType.Water,
    );
    // The tile at (1,1) has water north and east, so variant = 3
    const tileCall = waterCalls.find((c) => c.variant === 3);
    expect(tileCall).toBeDefined();
  });

  // ─── packed Float32Array size ─────────────────────────────────────

  it('packed Float32Array has correct byte size', () => {
    const builder = new ChunkBuilder(64, 64);
    const result = builder.buildChunk(0, 0, (x, y) => {
      if (x === 0 && y === 0) return makeTile();
      return null;
    });
    expect(result.count).toBe(1);
    // 1 instance * 48 bytes / 4 bytes per float = 12 floats
    expect(result.instances.length).toBe(INSTANCE_BYTE_SIZE / 4);
  });
});
