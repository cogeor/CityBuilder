import type { EngineCommand, ZoneTypeName } from "./commands.js";

export interface ToolTile {
  x: number;
  y: number;
}

export interface ToolInteractionCommand {
  type: "place" | "zone" | "bulldoze";
  tiles: ToolTile[];
  archetypeId?: number;
  zoneType?: number;
  rotation?: number;
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

export function mapZoneType(zoneType: number | undefined): ZoneTypeName {
  switch (zoneType) {
    case 1:
      return "Residential";
    case 2:
      return "Commercial";
    case 3:
      return "Industrial";
    case 4:
      return "Civic";
    default:
      return "None";
  }
}

// Convert UI tool intent into canonical Rust engine mutation commands.
export function translateToolInteraction(
  command: ToolInteractionCommand,
): EngineCommand[] {
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
