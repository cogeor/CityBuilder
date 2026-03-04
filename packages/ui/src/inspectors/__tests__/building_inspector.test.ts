import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  InspectorManager,
  InspectorType,
  type BuildingInfo,
  type TileInfo,
  type DistrictInfo,
  type InspectorEventType,
} from '../building_inspector.js';

/** Helper to create a default BuildingInfo */
function makeBuildingInfo(overrides?: Partial<BuildingInfo>): BuildingInfo {
  return {
    entityId: 1,
    name: 'House',
    archetypeId: 10,
    level: 1,
    maxLevel: 3,
    occupants: 4,
    capacity: 8,
    hasPower: true,
    hasWater: true,
    isStaffed: true,
    underConstruction: false,
    constructionPercent: 0,
    upkeepCents: 50,
    onFire: false,
    powerDemandKw: 5,
    powerSupplyKw: 0,
    waterDemand: 2,
    waterSupply: 0,
    ...overrides,
  };
}

/** Helper to create a default TileInfo */
function makeTileInfo(overrides?: Partial<TileInfo>): TileInfo {
  return {
    x: 10,
    y: 20,
    terrainType: 'grass',
    elevation: 5,
    zoneType: 'residential',
    hasRoad: false,
    roadType: '',
    pollution: 10,
    crime: 5,
    landValue: 60,
    noise: 15,
    desirability: 70,
    ...overrides,
  };
}

/** Helper to create a default DistrictInfo */
function makeDistrictInfo(overrides?: Partial<DistrictInfo>): DistrictInfo {
  return {
    name: 'Downtown',
    population: 5000,
    employment: 4500,
    unemploymentRate: 10,
    averageLandValue: 75,
    totalTax: 12000,
    totalUpkeep: 8000,
    ...overrides,
  };
}

describe('InspectorManager', () => {
  let mgr: InspectorManager;

  beforeEach(() => {
    mgr = new InspectorManager();
  });

  // --- Constructor ---

  it('constructor starts with None type', () => {
    expect(mgr.getActiveType()).toBe(InspectorType.None);
    expect(mgr.getBuildingInfo()).toBeNull();
    expect(mgr.getTileInfo()).toBeNull();
    expect(mgr.getDistrictInfo()).toBeNull();
    expect(mgr.isPinned()).toBe(false);
  });

  // --- inspectBuilding ---

  it('inspectBuilding sets active type and info', () => {
    const info = makeBuildingInfo();
    mgr.inspectBuilding(info);
    expect(mgr.getActiveType()).toBe(InspectorType.Building);
    expect(mgr.getBuildingInfo()).toEqual(info);
    expect(mgr.getTileInfo()).toBeNull();
    expect(mgr.getDistrictInfo()).toBeNull();
  });

  // --- inspectTile ---

  it('inspectTile sets active type and info', () => {
    const info = makeTileInfo();
    mgr.inspectTile(info);
    expect(mgr.getActiveType()).toBe(InspectorType.Tile);
    expect(mgr.getTileInfo()).toEqual(info);
    expect(mgr.getBuildingInfo()).toBeNull();
    expect(mgr.getDistrictInfo()).toBeNull();
  });

  // --- inspectDistrict ---

  it('inspectDistrict sets active type and info', () => {
    const info = makeDistrictInfo();
    mgr.inspectDistrict(info);
    expect(mgr.getActiveType()).toBe(InspectorType.District);
    expect(mgr.getDistrictInfo()).toEqual(info);
    expect(mgr.getBuildingInfo()).toBeNull();
    expect(mgr.getTileInfo()).toBeNull();
  });

  // --- Getters return copies ---

  it('getBuildingInfo returns a copy', () => {
    const info = makeBuildingInfo();
    mgr.inspectBuilding(info);
    const copy = mgr.getBuildingInfo()!;
    copy.name = 'MUTATED';
    expect(mgr.getBuildingInfo()!.name).toBe('House');
  });

  it('getTileInfo returns a copy', () => {
    const info = makeTileInfo();
    mgr.inspectTile(info);
    const copy = mgr.getTileInfo()!;
    copy.terrainType = 'MUTATED';
    expect(mgr.getTileInfo()!.terrainType).toBe('grass');
  });

  // --- close ---

  it('close clears inspector', () => {
    mgr.inspectBuilding(makeBuildingInfo());
    mgr.close();
    expect(mgr.getActiveType()).toBe(InspectorType.None);
    expect(mgr.getBuildingInfo()).toBeNull();
  });

  it('close does not close when pinned', () => {
    mgr.inspectBuilding(makeBuildingInfo());
    mgr.pin();
    mgr.close();
    expect(mgr.getActiveType()).toBe(InspectorType.Building);
    expect(mgr.getBuildingInfo()).not.toBeNull();
  });

  // --- forceClose ---

  it('forceClose closes even when pinned', () => {
    mgr.inspectBuilding(makeBuildingInfo());
    mgr.pin();
    mgr.forceClose();
    expect(mgr.getActiveType()).toBe(InspectorType.None);
    expect(mgr.getBuildingInfo()).toBeNull();
    expect(mgr.isPinned()).toBe(false);
  });

  // --- pin / unpin / togglePin ---

  it('pin sets pinned', () => {
    mgr.pin();
    expect(mgr.isPinned()).toBe(true);
  });

  it('unpin unsets pinned', () => {
    mgr.pin();
    mgr.unpin();
    expect(mgr.isPinned()).toBe(false);
  });

  it('togglePin toggles pinned state', () => {
    expect(mgr.isPinned()).toBe(false);
    mgr.togglePin();
    expect(mgr.isPinned()).toBe(true);
    mgr.togglePin();
    expect(mgr.isPinned()).toBe(false);
  });

  // --- updateBuildingInfo ---

  it('updateBuildingInfo updates partial fields', () => {
    mgr.inspectBuilding(makeBuildingInfo({ occupants: 4 }));
    mgr.updateBuildingInfo({ occupants: 7, level: 2 });
    const info = mgr.getBuildingInfo()!;
    expect(info.occupants).toBe(7);
    expect(info.level).toBe(2);
    // unchanged field
    expect(info.name).toBe('House');
  });

  // --- updateTileInfo ---

  it('updateTileInfo updates partial fields', () => {
    mgr.inspectTile(makeTileInfo({ pollution: 10 }));
    mgr.updateTileInfo({ pollution: 50, crime: 30 });
    const info = mgr.getTileInfo()!;
    expect(info.pollution).toBe(50);
    expect(info.crime).toBe(30);
    // unchanged field
    expect(info.terrainType).toBe('grass');
  });

  // --- requestUpgrade ---

  it('requestUpgrade emits upgrade event', () => {
    const handler = vi.fn();
    mgr.addEventListener(handler);
    mgr.inspectBuilding(makeBuildingInfo({ entityId: 42 }));
    // clear events from inspectBuilding
    handler.mockClear();
    mgr.requestUpgrade();
    expect(handler).toHaveBeenCalledWith('upgrade', { entityId: 42 });
  });

  // --- requestDemolish ---

  it('requestDemolish emits demolish event', () => {
    const handler = vi.fn();
    mgr.addEventListener(handler);
    mgr.inspectBuilding(makeBuildingInfo({ entityId: 99 }));
    handler.mockClear();
    mgr.requestDemolish();
    expect(handler).toHaveBeenCalledWith('demolish', { entityId: 99 });
  });

  // --- formatUpkeep ---

  it('formatUpkeep formats correctly', () => {
    // 1 cent/tick * 72000 / 100 = $720/mo
    expect(mgr.formatUpkeep(1)).toBe('$720/mo');
  });

  it('formatUpkeep formats thousands', () => {
    // 2 cents/tick * 72000 / 100 = $1440 -> $1.4K/mo
    expect(mgr.formatUpkeep(2)).toBe('$1.4K/mo');
  });

  // --- formatCapacity ---

  it('formatCapacity formats X / Y', () => {
    expect(mgr.formatCapacity(4, 8)).toBe('4 / 8');
  });

  // --- formatLevel ---

  it('formatLevel formats correctly', () => {
    expect(mgr.formatLevel(2, 5)).toBe('Level 2 / 5');
  });

  // --- formatPercent ---

  it('formatPercent rounds', () => {
    expect(mgr.formatPercent(33.3)).toBe('33%');
    expect(mgr.formatPercent(66.7)).toBe('67%');
    expect(mgr.formatPercent(100)).toBe('100%');
  });

  // --- getBuildingStatus ---

  it('getBuildingStatus shows construction', () => {
    const info = makeBuildingInfo({ underConstruction: true, constructionPercent: 45 });
    const statuses = mgr.getBuildingStatus(info);
    expect(statuses).toContain('Building... 45%');
  });

  it('getBuildingStatus shows fire', () => {
    const info = makeBuildingInfo({ onFire: true });
    const statuses = mgr.getBuildingStatus(info);
    expect(statuses).toContain('On Fire!');
  });

  it('getBuildingStatus shows operating when all good', () => {
    const info = makeBuildingInfo();
    const statuses = mgr.getBuildingStatus(info);
    expect(statuses).toEqual(['Operating']);
  });

  it('getBuildingStatus shows multiple issues', () => {
    const info = makeBuildingInfo({
      onFire: true,
      hasPower: false,
      hasWater: false,
      isStaffed: false,
    });
    const statuses = mgr.getBuildingStatus(info);
    expect(statuses).toContain('On Fire!');
    expect(statuses).toContain('No Power');
    expect(statuses).toContain('No Water');
    expect(statuses).toContain('Understaffed');
    expect(statuses.length).toBe(4);
  });

  // --- Events ---

  it('addEventListener receives events', () => {
    const handler = vi.fn();
    mgr.addEventListener(handler);
    mgr.inspectBuilding(makeBuildingInfo());
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('open', expect.objectContaining({ type: InspectorType.Building }));
  });

  it('removeEventListener stops receiving events', () => {
    const handler = vi.fn();
    mgr.addEventListener(handler);
    mgr.removeEventListener(handler);
    mgr.inspectBuilding(makeBuildingInfo());
    expect(handler).not.toHaveBeenCalled();
  });
});
