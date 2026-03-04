import { describe, it, expect } from 'vitest';
import {
  ROAD_TYPES,
  AUTO_TILE_RULES,
  IntersectionType,
  getRoadType,
  getAutoTileOffset,
  computeRoadSpriteId,
  detectIntersectionType,
  computeTravelTime,
  validateRoadConfig,
} from '../roads.js';

// ─── ROAD_TYPES ──────────────────────────────────────────────────────────────

describe('ROAD_TYPES', () => {
  it('has 4 entries', () => {
    expect(ROAD_TYPES).toHaveLength(4);
  });

  it('Local Street has correct speed limit (30 km/h)', () => {
    const local = ROAD_TYPES.find(r => r.name === 'Local Street');
    expect(local).toBeDefined();
    expect(local!.speedLimit).toBe(30);
  });

  it('Highway has the highest capacity', () => {
    const maxCapacity = Math.max(...ROAD_TYPES.map(r => r.capacity));
    const highway = ROAD_TYPES.find(r => r.name === 'Highway');
    expect(highway).toBeDefined();
    expect(highway!.capacity).toBe(maxCapacity);
    expect(highway!.capacity).toBe(2000);
  });
});

// ─── getRoadType ─────────────────────────────────────────────────────────────

describe('getRoadType', () => {
  it('finds road type by id', () => {
    const road = getRoadType(1);
    expect(road).toBeDefined();
    expect(road!.name).toBe('Local Street');
  });

  it('returns undefined for unknown id', () => {
    expect(getRoadType(999)).toBeUndefined();
  });
});

// ─── getAutoTileOffset ───────────────────────────────────────────────────────

describe('getAutoTileOffset', () => {
  it('returns correct sprite offset for each mask', () => {
    // isolated = 0
    expect(getAutoTileOffset(0b0000)).toBe(0);
    // straight NS = 5
    expect(getAutoTileOffset(0b0101)).toBe(5);
    // cross = 15
    expect(getAutoTileOffset(0b1111)).toBe(15);
    // curve NE = 3
    expect(getAutoTileOffset(0b0011)).toBe(3);
  });
});

// ─── computeRoadSpriteId ────────────────────────────────────────────────────

describe('computeRoadSpriteId', () => {
  it('combines sprite base and auto-tile offset', () => {
    // Local Street (spriteBase=100) with straight NS (offset=5)
    expect(computeRoadSpriteId(1, 0b0101)).toBe(105);
    // Highway (spriteBase=160) with cross (offset=15)
    expect(computeRoadSpriteId(4, 0b1111)).toBe(175);
  });

  it('returns 0 for unknown road type', () => {
    expect(computeRoadSpriteId(999, 0b0101)).toBe(0);
  });
});

// ─── detectIntersectionType ─────────────────────────────────────────────────

describe('detectIntersectionType', () => {
  it('returns Cross for 4 connections', () => {
    expect(detectIntersectionType(0b1111)).toBe(IntersectionType.Cross);
  });

  it('returns T for 3 connections', () => {
    expect(detectIntersectionType(0b0111)).toBe(IntersectionType.T);
    expect(detectIntersectionType(0b1011)).toBe(IntersectionType.T);
  });

  it('returns None for 2 or fewer connections', () => {
    expect(detectIntersectionType(0b0101)).toBe(IntersectionType.None);
    expect(detectIntersectionType(0b0001)).toBe(IntersectionType.None);
    expect(detectIntersectionType(0b0000)).toBe(IntersectionType.None);
  });
});

// ─── AUTO_TILE_RULES ────────────────────────────────────────────────────────

describe('AUTO_TILE_RULES', () => {
  it('has 16 entries for all NESW combinations', () => {
    expect(AUTO_TILE_RULES).toHaveLength(16);
  });

  it('covers all masks from 0 to 15', () => {
    const masks = AUTO_TILE_RULES.map(r => r.mask).sort((a, b) => a - b);
    expect(masks).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
  });
});

// ─── computeTravelTime ──────────────────────────────────────────────────────

describe('computeTravelTime', () => {
  it('returns a positive finite value for known road type', () => {
    const time = computeTravelTime(1, 10);
    expect(time).toBeGreaterThan(0);
    expect(time).toBeLessThan(Infinity);
  });

  it('returns Infinity for unknown road type', () => {
    expect(computeTravelTime(999, 10)).toBe(Infinity);
  });

  it('faster roads produce shorter travel times', () => {
    const localTime = computeTravelTime(1, 10);  // 30 km/h
    const highwayTime = computeTravelTime(4, 10); // 100 km/h
    expect(highwayTime).toBeLessThan(localTime);
  });
});

// ─── validateRoadConfig ─────────────────────────────────────────────────────

describe('validateRoadConfig', () => {
  it('returns no errors for default configuration', () => {
    const errors = validateRoadConfig();
    expect(errors).toHaveLength(0);
  });
});
