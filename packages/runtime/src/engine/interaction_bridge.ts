import type { EngineCommand, RoadTypeName, SimSpeedName, TerrainTypeName, ZoneTypeName } from "./commands.js";

export interface ToolTile {
  x: number;
  y: number;
}

export interface ToolInteractionCommand {
  type: "place" | "zone" | "bulldoze" | "road" | "terrain" | "speed";
  tiles: ToolTile[];
  archetypeId?: number;
  zoneType?: number;
  terrainType?: number;
  roadType?: number;
  rotation?: number;
  /** Used when type is "speed". Must be a SimSpeedName. */
  simSpeed?: SimSpeedName;
}

function boundsFromTiles(tiles: ToolTile[]): { x: number; y: number; w: number; h: number } {
  if (tiles.length === 0) {
    return { x: 0, y: 0, w: 0, h: 0 };
  }
  let minX = tiles[0].x;
  let minY = tiles[0].y;
  let maxX = tiles[0].x;
  let maxY = tiles[0].y;
  for (const tile of tiles) {
    if (tile.x < minX) minX = tile.x;
    if (tile.y < minY) minY = tile.y;
    if (tile.x > maxX) maxX = tile.x;
    if (tile.y > maxY) maxY = tile.y;
  }
  return {
    x: minX,
    y: minY,
    w: maxX - minX + 1,
    h: maxY - minY + 1,
  };
}

/** Explicit mapping of Rust ZoneType repr(u8) codes to serde string names. */
const ZONE_CODE_MAP: Record<number, ZoneTypeName> = {
  1: "Residential",
  2: "Commercial",
  3: "Industrial",
  4: "Civic",
};

/**
 * Map a numeric zone code (Rust ZoneType repr(u8)) to its serde name.
 * Isolated here so the numeric coupling is explicit and easy to update.
 */
export function mapZoneTypeFromCode(code: number | undefined): ZoneTypeName {
  if (code === undefined) return "None";
  return ZONE_CODE_MAP[code] ?? "None";
}

/** Map a zone type — accepts either a string name (pass-through) or numeric code. */
export function mapZoneType(zoneType: number | undefined): ZoneTypeName {
  return mapZoneTypeFromCode(zoneType);
}

export function mapTerrainType(terrainType: number | undefined): TerrainTypeName {
  switch (terrainType) {
    case 1:
      return "Water";
    case 2:
      return "Sand";
    case 3:
      return "Forest";
    case 4:
      return "Rock";
    default:
      return "Grass";
  }
}

export function mapRoadType(roadType: number | undefined): RoadTypeName {
  switch (roadType) {
    case 2:
      return "Collector";
    case 3:
      return "Arterial";
    case 4:
      return "Highway";
    default:
      return "Local";
  }
}

// Convert UI tool intent into canonical Rust engine mutation commands.
export function translateToolInteraction(
  command: ToolInteractionCommand,
): EngineCommand[] {
  if (command.type === "speed") {
    const speed: SimSpeedName = command.simSpeed ?? "Normal";
    return [{ SetSimSpeed: { speed } }];
  }

  if (command.type === "place") {
    const first = command.tiles[0];
    if (!first) return [];
    return [
      {
        PlaceEntity: {
          archetype_id: command.archetypeId ?? 0,
          x: first.x,
          y: first.y,
          rotation: command.rotation ?? 0,
        },
      },
    ];
  }

  const rect = boundsFromTiles(command.tiles);
  if (rect.w <= 0 || rect.h <= 0) return [];

  if (command.type === "zone") {
    return [
      {
        SetZoning: {
          x: rect.x,
          y: rect.y,
          w: rect.w,
          h: rect.h,
          zone: mapZoneType(command.zoneType),
        },
      },
    ];
  }

  if (command.type === "terrain") {
    return [
      {
        SetTerrain: {
          x: rect.x,
          y: rect.y,
          w: rect.w,
          h: rect.h,
          terrain: mapTerrainType(command.terrainType),
        },
      },
    ];
  }

  if (command.type === "road") {
    if (command.tiles.length === 0) {
      return [];
    }
    const first = command.tiles[0];
    const last = command.tiles[command.tiles.length - 1];
    const dx = Math.abs(last.x - first.x);
    const dy = Math.abs(last.y - first.y);
    let x1 = last.x;
    let y1 = last.y;
    // Snap non-axis-aligned input to the dominant axis.
    if (dx !== 0 && dy !== 0) {
      if (dx >= dy) {
        // Horizontal dominant — clamp to same row.
        y1 = first.y;
      } else {
        // Vertical dominant — clamp to same column.
        x1 = first.x;
      }
    }
    return [
      {
        SetRoadLine: {
          x0: first.x,
          y0: first.y,
          x1,
          y1,
          road_type: mapRoadType(command.roadType),
        },
      },
    ];
  }

  return [
    {
      Bulldoze: {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
      },
    },
  ];
}

// Backward-compatible export name.
export const mapToolInteractionToEngineCommands = translateToolInteraction;
