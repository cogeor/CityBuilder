import { describe, expect, it } from "vitest";
import {
  mapZoneTypeFromCode,
  translateToolInteraction,
  ZONE_TYPE_IDS,
} from "./interaction_bridge.js";

describe("interaction_bridge", () => {
  it("translates road tool into SetRoadLine", () => {
    const commands = translateToolInteraction({
      type: "road",
      tiles: [
        { x: 2, y: 3 },
        { x: 5, y: 3 },
      ],
      roadType: 3,
    });

    expect(commands).toEqual([
      {
        SetRoadLine: {
          x0: 2,
          y0: 3,
          x1: 5,
          y1: 3,
          road_type: "Arterial",
        },
      },
    ]);
  });

  it("translates terrain drag into SetTerrain", () => {
    const commands = translateToolInteraction({
      type: "terrain",
      tiles: [
        { x: 10, y: 10 },
        { x: 11, y: 11 },
      ],
      terrainType: 1,
    });

    expect(commands).toEqual([
      {
        SetTerrain: {
          x: 10,
          y: 10,
          w: 2,
          h: 2,
          terrain: "Water",
        },
      },
    ]);
  });

  // ── zone ──────────────────────────────────────────────────────────────────

  it("translates single-tile zone into SetZoning", () => {
    const commands = translateToolInteraction({
      type: "zone",
      tiles: [{ x: 3, y: 4 }],
      zoneType: 1,
    });
    expect(commands).toEqual([
      { SetZoning: { x: 3, y: 4, w: 1, h: 1, zone: "Residential" } },
    ]);
  });

  it("translates multi-tile zone rect into SetZoning", () => {
    const commands = translateToolInteraction({
      type: "zone",
      tiles: [
        { x: 2, y: 2 },
        { x: 5, y: 5 },
      ],
      zoneType: 2,
    });
    expect(commands).toEqual([
      { SetZoning: { x: 2, y: 2, w: 4, h: 4, zone: "Commercial" } },
    ]);
  });

  it("unknown zone code defaults to None", () => {
    expect(mapZoneTypeFromCode(99)).toBe("None");
    expect(mapZoneTypeFromCode(undefined)).toBe("None");
  });

  // ── place ─────────────────────────────────────────────────────────────────

  it("translates place tool into PlaceEntity", () => {
    const commands = translateToolInteraction({
      type: "place",
      tiles: [{ x: 7, y: 8 }],
      archetypeId: 42,
      rotation: 2,
    });
    expect(commands).toEqual([
      { PlaceEntity: { archetype_id: 42, x: 7, y: 8, rotation: 2 } },
    ]);
  });

  it("place tool defaults rotation to 0", () => {
    const commands = translateToolInteraction({
      type: "place",
      tiles: [{ x: 0, y: 0 }],
      archetypeId: 1,
    });
    expect(commands[0]).toMatchObject({ PlaceEntity: { rotation: 0 } });
  });

  // ── bulldoze ──────────────────────────────────────────────────────────────

  it("translates bulldoze into Bulldoze w=1 h=1 for single tile", () => {
    const commands = translateToolInteraction({
      type: "bulldoze",
      tiles: [{ x: 5, y: 6 }],
    });
    expect(commands).toEqual([{ Bulldoze: { x: 5, y: 6, w: 1, h: 1 } }]);
  });

  // ── speed ─────────────────────────────────────────────────────────────────

  it("translates speed interaction into SetSimSpeed Paused", () => {
    const commands = translateToolInteraction({
      type: "speed",
      tiles: [],
      simSpeed: "Paused",
    });
    expect(commands).toEqual([{ SetSimSpeed: { speed: "Paused" } }]);
  });

  it("translates speed interaction into SetSimSpeed Fast", () => {
    const commands = translateToolInteraction({
      type: "speed",
      tiles: [],
      simSpeed: "Fast",
    });
    expect(commands).toEqual([{ SetSimSpeed: { speed: "Fast" } }]);
  });

  it("speed interaction defaults to Normal when simSpeed omitted", () => {
    const commands = translateToolInteraction({
      type: "speed",
      tiles: [],
    });
    expect(commands).toEqual([{ SetSimSpeed: { speed: "Normal" } }]);
  });

  // ── road snapping ─────────────────────────────────────────────────────────

  it("snaps diagonal road (dx > dy) to horizontal axis", () => {
    const commands = translateToolInteraction({
      type: "road",
      tiles: [
        { x: 0, y: 0 },
        { x: 5, y: 2 }, // dx=5 > dy=2, snap to horizontal
      ],
    });
    expect(commands).toEqual([
      {
        SetRoadLine: {
          x0: 0,
          y0: 0,
          x1: 5,
          y1: 0, // y snapped to first.y
          road_type: "Local",
        },
      },
    ]);
  });

  it("snaps diagonal road (dy > dx) to vertical axis", () => {
    const commands = translateToolInteraction({
      type: "road",
      tiles: [
        { x: 3, y: 0 },
        { x: 5, y: 6 }, // dy=6 > dx=2, snap to vertical
      ],
    });
    expect(commands).toEqual([
      {
        SetRoadLine: {
          x0: 3,
          y0: 0,
          x1: 3, // x snapped to first.x
          y1: 6,
          road_type: "Local",
        },
      },
    ]);
  });

  // ── ZONE_TYPE_IDS constant ────────────────────────────────────────────────

  it("ZONE_TYPE_IDS provides canonical zone code constants", () => {
    expect(ZONE_TYPE_IDS.None).toBe(0);
    expect(ZONE_TYPE_IDS.Residential).toBe(1);
    expect(ZONE_TYPE_IDS.Commercial).toBe(2);
    expect(ZONE_TYPE_IDS.Industrial).toBe(3);
    expect(ZONE_TYPE_IDS.Civic).toBe(4);
  });

  it("ZONE_TYPE_IDS codes map to correct ZoneTypeName via mapZoneTypeFromCode", () => {
    expect(mapZoneTypeFromCode(ZONE_TYPE_IDS.Residential)).toBe("Residential");
    expect(mapZoneTypeFromCode(ZONE_TYPE_IDS.Commercial)).toBe("Commercial");
    expect(mapZoneTypeFromCode(ZONE_TYPE_IDS.Industrial)).toBe("Industrial");
    expect(mapZoneTypeFromCode(ZONE_TYPE_IDS.Civic)).toBe("Civic");
    expect(mapZoneTypeFromCode(ZONE_TYPE_IDS.None)).toBe("None");
  });

  // ── boundsFromTiles edge cases (tested indirectly via translateToolInteraction) ──

  it("empty tiles array returns no commands for zone", () => {
    const commands = translateToolInteraction({ type: "zone", tiles: [] });
    expect(commands).toEqual([]);
  });

  it("single-tile zone produces w=1 h=1 bounds", () => {
    const commands = translateToolInteraction({
      type: "zone",
      tiles: [{ x: 7, y: 9 }],
      zoneType: ZONE_TYPE_IDS.Civic,
    });
    expect(commands).toEqual([{ SetZoning: { x: 7, y: 9, w: 1, h: 1, zone: "Civic" } }]);
  });

  it("non-contiguous tiles use bounding-box for zone", () => {
    // Tiles at corners of a 5x5 area — bounding box is (0,0) to (4,4) → w=5, h=5
    const commands = translateToolInteraction({
      type: "zone",
      tiles: [
        { x: 0, y: 0 },
        { x: 4, y: 4 },
        { x: 0, y: 4 },
        { x: 4, y: 0 },
      ],
      zoneType: ZONE_TYPE_IDS.Industrial,
    });
    expect(commands).toEqual([
      { SetZoning: { x: 0, y: 0, w: 5, h: 5, zone: "Industrial" } },
    ]);
  });

  it("empty tiles array returns no commands for bulldoze", () => {
    const commands = translateToolInteraction({ type: "bulldoze", tiles: [] });
    expect(commands).toEqual([]);
  });

  it("empty tiles array returns no commands for road", () => {
    const commands = translateToolInteraction({ type: "road", tiles: [] });
    expect(commands).toEqual([]);
  });
});
