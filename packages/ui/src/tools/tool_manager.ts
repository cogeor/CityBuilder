import { ToolType } from '../shell/shell.js';
import { TypedEventHub, type EventListener } from '../shared/typed_events.js';

/** Tool state for the active tool */
export enum ToolState {
  Idle = 'idle',           // No action in progress
  Previewing = 'previewing', // Mouse hovering, showing ghost
  Dragging = 'dragging',   // Mouse down, dragging
}

/** Tile coordinate */
export interface TileCoord {
  x: number;
  y: number;
}

/** Placement validity */
export enum PlacementValidity {
  Valid = 'valid',
  InvalidTerrain = 'invalid_terrain',
  Occupied = 'occupied',
  InsufficientFunds = 'insufficient_funds',
  OutOfBounds = 'out_of_bounds',
}

/** A command to be sent to the simulation */
export interface ToolCommand {
  type: 'place' | 'zone' | 'bulldoze' | 'road' | 'terrain';
  /** Tile coordinates affected */
  tiles: TileCoord[];
  /** Archetype ID for placement */
  archetypeId?: number;
  /** Zone type for zoning */
  zoneType?: number;
  /** Road type for road tool */
  roadType?: number;
  /** Terrain type for terrain tool */
  terrainType?: number;
  /** Estimated cost in cents */
  estimatedCost: number;
}

/** Drag rectangle for zoning/bulldoze */
export interface DragRect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

/** Callback for validating placement */
export type ValidateCallback = (x: number, y: number, archetypeId?: number) => PlacementValidity;

/** Callback for computing cost */
export type CostCallback = (command: ToolCommand) => number;

/** Tool event types */
export type ToolEventType = 'commandGenerated' | 'stateChanged' | 'previewUpdated';
export interface ToolEventPayloads {
  commandGenerated: { command: ToolCommand };
  stateChanged: { state: ToolState };
  previewUpdated: { tiles: TileCoord[]; validity: PlacementValidity };
}
export type ToolEventHandler = EventListener<ToolEventPayloads>;

/**
 * ToolManager -- state machine for all building/zoning tools.
 *
 * Flow:
 * 1. User selects tool via HudShell
 * 2. Mouse hover -> Previewing (show ghost sprite / highlight)
 * 3. Mouse down -> Dragging (for zone/bulldoze/road)
 * 4. Mouse up -> Generate command
 */
export class ToolManager {
  private activeTool: ToolType;
  private state: ToolState;
  private selectedArchetype: number;
  private selectedZoneType: number;
  private selectedRoadType: number;
  private selectedTerrainType: number;
  private hoverTile: TileCoord | null;
  private dragRect: DragRect | null;
  private previewTiles: TileCoord[];
  private previewValidity: PlacementValidity;
  private validateFn: ValidateCallback | null;
  private costFn: CostCallback | null;
  private readonly events: TypedEventHub<ToolEventPayloads>;

  constructor() {
    this.activeTool = ToolType.Select;
    this.state = ToolState.Idle;
    this.selectedArchetype = 0;
    this.selectedZoneType = 0;
    this.selectedRoadType = 1;
    this.selectedTerrainType = 0;
    this.hoverTile = null;
    this.dragRect = null;
    this.previewTiles = [];
    this.previewValidity = PlacementValidity.Valid;
    this.validateFn = null;
    this.costFn = null;
    this.events = new TypedEventHub<ToolEventPayloads>();
  }

  // --- Getters ---
  getActiveTool(): ToolType { return this.activeTool; }
  getState(): ToolState { return this.state; }
  getHoverTile(): TileCoord | null { return this.hoverTile ? { ...this.hoverTile } : null; }
  getDragRect(): DragRect | null { return this.dragRect ? { ...this.dragRect } : null; }
  getPreviewTiles(): TileCoord[] { return [...this.previewTiles]; }
  getPreviewValidity(): PlacementValidity { return this.previewValidity; }
  getSelectedArchetype(): number { return this.selectedArchetype; }
  getSelectedZoneType(): number { return this.selectedZoneType; }
  getSelectedRoadType(): number { return this.selectedRoadType; }
  getSelectedTerrainType(): number { return this.selectedTerrainType; }

  // --- Configuration ---
  setValidateCallback(fn: ValidateCallback): void { this.validateFn = fn; }
  setCostCallback(fn: CostCallback): void { this.costFn = fn; }
  addEventListener(handler: ToolEventHandler): void { this.events.on(handler); }
  removeEventListener(handler: ToolEventHandler): void { this.events.off(handler); }

  // --- Tool Selection ---
  setTool(tool: ToolType, archetypeId?: number, zoneType?: number, roadType?: number, terrainType?: number): void {
    this.activeTool = tool;
    if (archetypeId !== undefined) this.selectedArchetype = archetypeId;
    if (zoneType !== undefined) this.selectedZoneType = zoneType;
    if (roadType !== undefined) this.selectedRoadType = roadType;
    if (terrainType !== undefined) this.selectedTerrainType = terrainType;
    this.cancelAction();
  }

  // --- Mouse Events ---
  onHover(tileX: number, tileY: number): void {
    this.hoverTile = { x: tileX, y: tileY };

    if (this.activeTool === ToolType.Select) return;

    if (this.state === ToolState.Idle) {
      this.state = ToolState.Previewing;
    }

    // Update preview based on tool type
    if (this.state === ToolState.Previewing) {
      this.updatePreview();
    } else if (this.state === ToolState.Dragging && this.dragRect) {
      this.dragRect.endX = tileX;
      this.dragRect.endY = tileY;
      this.updateDragPreview();
    }

    this.emit('previewUpdated', {
      tiles: this.previewTiles,
      validity: this.previewValidity,
    });
  }

  onMouseDown(tileX: number, tileY: number): ToolCommand | null {
    if (this.activeTool === ToolType.Select) return null;

    if (this.activeTool === ToolType.Place) {
      // Place tool: immediate command on click
      return this.generatePlaceCommand(tileX, tileY);
    } else {
      // Zone, bulldoze, road, terrain: start drag
      this.state = ToolState.Dragging;
      this.dragRect = { startX: tileX, startY: tileY, endX: tileX, endY: tileY };
      this.emit('stateChanged', { state: this.state });
      return null;
    }
  }

  /**
   * Handle canvas mouse leave — resets hoverTile and transitions Previewing → Idle.
   * Prevents a stale ghost tile being shown after the cursor leaves the viewport.
   */
  onMouseLeave(): void {
    this.hoverTile = null;
    if (this.state === ToolState.Previewing) {
      this.state = ToolState.Idle;
      this.previewTiles = [];
      this.emit('stateChanged', { state: this.state });
    }
  }

  onMouseUp(): ToolCommand | null {
    if (this.state !== ToolState.Dragging || !this.dragRect) return null;

    const command = this.generateDragCommand();
    this.state = ToolState.Previewing;
    this.dragRect = null;
    this.emit('stateChanged', { state: this.state });

    return command;
  }

  // --- Command Generation ---
  private generatePlaceCommand(tileX: number, tileY: number): ToolCommand | null {
    if (this.validateFn) {
      const validity = this.validateFn(tileX, tileY, this.selectedArchetype);
      if (validity !== PlacementValidity.Valid) return null;
    }

    const command: ToolCommand = {
      type: 'place',
      tiles: [{ x: tileX, y: tileY }],
      archetypeId: this.selectedArchetype,
      estimatedCost: 0,
    };

    if (this.costFn) {
      command.estimatedCost = this.costFn(command);
    }

    this.emit('commandGenerated', { command });
    return command;
  }

  private generateDragCommand(): ToolCommand | null {
    if (!this.dragRect) return null;

    const tiles = this.computeRectTiles(this.dragRect);
    if (tiles.length === 0) return null;

    let type: ToolCommand['type'];
    switch (this.activeTool) {
      case ToolType.Zone: type = 'zone'; break;
      case ToolType.Bulldoze: type = 'bulldoze'; break;
      case ToolType.Road: type = 'road'; break;
      case ToolType.Terrain: type = 'terrain'; break;
      default: return null;
    }

    const command: ToolCommand = {
      type,
      tiles,
      estimatedCost: 0,
    };

    if (type === 'zone') command.zoneType = this.selectedZoneType;
    if (type === 'road') command.roadType = this.selectedRoadType;
    if (type === 'terrain') command.terrainType = this.selectedTerrainType;
    if (this.costFn) command.estimatedCost = this.costFn(command);

    this.emit('commandGenerated', { command });
    return command;
  }

  /** Compute tiles within a rectangle */
  computeRectTiles(rect: DragRect): TileCoord[] {
    const minX = Math.min(rect.startX, rect.endX);
    const maxX = Math.max(rect.startX, rect.endX);
    const minY = Math.min(rect.startY, rect.endY);
    const maxY = Math.max(rect.startY, rect.endY);

    const tiles: TileCoord[] = [];
    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        tiles.push({ x, y });
      }
    }
    return tiles;
  }

  /** Compute tiles for road line (Bresenham-like, axis-aligned) */
  computeRoadTiles(start: TileCoord, end: TileCoord): TileCoord[] {
    const tiles: TileCoord[] = [];
    const dx = Math.abs(end.x - start.x);
    const dy = Math.abs(end.y - start.y);

    if (dx >= dy) {
      // Horizontal dominant
      const minX = Math.min(start.x, end.x);
      const maxX = Math.max(start.x, end.x);
      const y = start.y;
      for (let x = minX; x <= maxX; x++) {
        tiles.push({ x, y });
      }
    } else {
      // Vertical dominant
      const minY = Math.min(start.y, end.y);
      const maxY = Math.max(start.y, end.y);
      const x = start.x;
      for (let y = minY; y <= maxY; y++) {
        tiles.push({ x, y });
      }
    }
    return tiles;
  }

  // --- Preview ---
  private updatePreview(): void {
    if (!this.hoverTile) return;

    this.previewTiles = [{ ...this.hoverTile }];

    if (this.validateFn) {
      const archetypeId = this.activeTool === ToolType.Place ? this.selectedArchetype : undefined;
      this.previewValidity = this.validateFn(this.hoverTile.x, this.hoverTile.y, archetypeId);
    } else {
      this.previewValidity = PlacementValidity.Valid;
    }
  }

  private updateDragPreview(): void {
    if (!this.dragRect) return;

    if (this.activeTool === ToolType.Road) {
      this.previewTiles = this.computeRoadTiles(
        { x: this.dragRect.startX, y: this.dragRect.startY },
        { x: this.dragRect.endX, y: this.dragRect.endY }
      );
    } else {
      this.previewTiles = this.computeRectTiles(this.dragRect);
    }
  }

  /** Cancel current action and return to idle */
  cancelAction(): void {
    this.state = ToolState.Idle;
    this.dragRect = null;
    this.previewTiles = [];
    this.hoverTile = null;
    this.emit('stateChanged', { state: this.state });
  }

  private emit<K extends ToolEventType>(type: K, data: ToolEventPayloads[K]): void {
    this.events.emit(type, data);
  }
}
