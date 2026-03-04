import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  ToolManager,
  ToolState,
  PlacementValidity,
  type ToolEventType,
  type ToolCommand,
} from '../tool_manager.js';
import { ToolType } from '../../shell/shell.js';

describe('ToolManager', () => {
  let tm: ToolManager;

  beforeEach(() => {
    tm = new ToolManager();
  });

  // 1. Constructor starts with Select tool and Idle state
  it('constructor starts with Select tool and Idle state', () => {
    expect(tm.getActiveTool()).toBe(ToolType.Select);
    expect(tm.getState()).toBe(ToolState.Idle);
    expect(tm.getHoverTile()).toBeNull();
    expect(tm.getDragRect()).toBeNull();
    expect(tm.getPreviewTiles()).toEqual([]);
    expect(tm.getPreviewValidity()).toBe(PlacementValidity.Valid);
    expect(tm.getSelectedArchetype()).toBe(0);
    expect(tm.getSelectedZoneType()).toBe(0);
    expect(tm.getSelectedRoadType()).toBe(1);
  });

  // 2. setTool changes active tool
  it('setTool changes active tool', () => {
    tm.setTool(ToolType.Place);
    expect(tm.getActiveTool()).toBe(ToolType.Place);
  });

  // 3. setTool with archetypeId sets selected archetype
  it('setTool with archetypeId sets selected archetype', () => {
    tm.setTool(ToolType.Place, 42);
    expect(tm.getSelectedArchetype()).toBe(42);
  });

  // 4. setTool cancels current action
  it('setTool cancels current action', () => {
    tm.setTool(ToolType.Zone);
    tm.onHover(5, 5);
    tm.onMouseDown(5, 5);
    expect(tm.getState()).toBe(ToolState.Dragging);

    tm.setTool(ToolType.Bulldoze);
    expect(tm.getState()).toBe(ToolState.Idle);
    expect(tm.getDragRect()).toBeNull();
    expect(tm.getPreviewTiles()).toEqual([]);
  });

  // 5. onHover sets hover tile
  it('onHover sets hover tile', () => {
    tm.setTool(ToolType.Place);
    tm.onHover(3, 7);
    const hover = tm.getHoverTile();
    expect(hover).toEqual({ x: 3, y: 7 });
  });

  // 6. onHover enters Previewing state for Place tool
  it('onHover enters Previewing state for Place tool', () => {
    tm.setTool(ToolType.Place);
    tm.onHover(1, 1);
    expect(tm.getState()).toBe(ToolState.Previewing);
  });

  // 7. onHover stays Idle for Select tool
  it('onHover stays Idle for Select tool', () => {
    tm.onHover(1, 1);
    expect(tm.getState()).toBe(ToolState.Idle);
  });

  // 8. onMouseDown with Place tool generates command
  it('onMouseDown with Place tool generates command', () => {
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.setTool(ToolType.Place, 10);
    tm.onMouseDown(5, 5);

    const commandCall = handler.mock.calls.find(
      (call: any[]) => call[0] === 'commandGenerated'
    );
    expect(commandCall).toBeDefined();
    const command = commandCall![1].command as ToolCommand;
    expect(command.type).toBe('place');
    expect(command.tiles).toEqual([{ x: 5, y: 5 }]);
    expect(command.archetypeId).toBe(10);
  });

  // 9. onMouseDown with Zone tool starts drag
  it('onMouseDown with Zone tool starts drag', () => {
    tm.setTool(ToolType.Zone);
    tm.onMouseDown(3, 4);
    expect(tm.getState()).toBe(ToolState.Dragging);
    const rect = tm.getDragRect();
    expect(rect).toEqual({ startX: 3, startY: 4, endX: 3, endY: 4 });
  });

  // 10. onMouseDown with Bulldoze tool starts drag
  it('onMouseDown with Bulldoze tool starts drag', () => {
    tm.setTool(ToolType.Bulldoze);
    tm.onMouseDown(2, 6);
    expect(tm.getState()).toBe(ToolState.Dragging);
    expect(tm.getDragRect()).toEqual({ startX: 2, startY: 6, endX: 2, endY: 6 });
  });

  // 11. onMouseDown with Road tool starts drag
  it('onMouseDown with Road tool starts drag', () => {
    tm.setTool(ToolType.Road);
    tm.onMouseDown(0, 0);
    expect(tm.getState()).toBe(ToolState.Dragging);
    expect(tm.getDragRect()).toEqual({ startX: 0, startY: 0, endX: 0, endY: 0 });
  });

  // 12. onMouseUp generates zone command from drag rect
  it('onMouseUp generates zone command from drag rect', () => {
    tm.setTool(ToolType.Zone, undefined, 2);
    tm.onMouseDown(1, 1);
    tm.onHover(3, 3);
    const command = tm.onMouseUp();

    expect(command).not.toBeNull();
    expect(command!.type).toBe('zone');
    expect(command!.zoneType).toBe(2);
    expect(command!.tiles.length).toBe(9); // 3x3
  });

  // 13. onMouseUp generates bulldoze command from drag rect
  it('onMouseUp generates bulldoze command from drag rect', () => {
    tm.setTool(ToolType.Bulldoze);
    tm.onMouseDown(0, 0);
    tm.onHover(1, 1);
    const command = tm.onMouseUp();

    expect(command).not.toBeNull();
    expect(command!.type).toBe('bulldoze');
    expect(command!.tiles.length).toBe(4); // 2x2
  });

  // 14. onMouseUp returns null when not dragging
  it('onMouseUp returns null when not dragging', () => {
    const result = tm.onMouseUp();
    expect(result).toBeNull();
  });

  // 15. computeRectTiles returns correct tiles
  it('computeRectTiles returns correct tiles', () => {
    const tiles = tm.computeRectTiles({ startX: 0, startY: 0, endX: 2, endY: 1 });
    expect(tiles).toEqual([
      { x: 0, y: 0 }, { x: 1, y: 0 }, { x: 2, y: 0 },
      { x: 0, y: 1 }, { x: 1, y: 1 }, { x: 2, y: 1 },
    ]);
  });

  // 16. computeRectTiles handles reversed coordinates
  it('computeRectTiles handles reversed coordinates', () => {
    const tiles = tm.computeRectTiles({ startX: 3, startY: 3, endX: 1, endY: 1 });
    expect(tiles).toEqual([
      { x: 1, y: 1 }, { x: 2, y: 1 }, { x: 3, y: 1 },
      { x: 1, y: 2 }, { x: 2, y: 2 }, { x: 3, y: 2 },
      { x: 1, y: 3 }, { x: 2, y: 3 }, { x: 3, y: 3 },
    ]);
  });

  // 17. computeRoadTiles horizontal dominant
  it('computeRoadTiles horizontal dominant', () => {
    const tiles = tm.computeRoadTiles({ x: 0, y: 5 }, { x: 4, y: 5 });
    expect(tiles).toEqual([
      { x: 0, y: 5 }, { x: 1, y: 5 }, { x: 2, y: 5 },
      { x: 3, y: 5 }, { x: 4, y: 5 },
    ]);
  });

  // 18. computeRoadTiles vertical dominant
  it('computeRoadTiles vertical dominant', () => {
    const tiles = tm.computeRoadTiles({ x: 3, y: 0 }, { x: 3, y: 3 });
    expect(tiles).toEqual([
      { x: 3, y: 0 }, { x: 3, y: 1 }, { x: 3, y: 2 }, { x: 3, y: 3 },
    ]);
  });

  // 19. cancelAction resets state
  it('cancelAction resets state', () => {
    tm.setTool(ToolType.Zone);
    tm.onHover(5, 5);
    tm.onMouseDown(5, 5);
    expect(tm.getState()).toBe(ToolState.Dragging);

    tm.cancelAction();
    expect(tm.getState()).toBe(ToolState.Idle);
    expect(tm.getDragRect()).toBeNull();
    expect(tm.getPreviewTiles()).toEqual([]);
    expect(tm.getHoverTile()).toBeNull();
  });

  // 20. ValidateCallback prevents invalid placement
  it('ValidateCallback prevents invalid placement', () => {
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.setValidateCallback(() => PlacementValidity.Occupied);
    tm.setTool(ToolType.Place, 1);
    tm.onMouseDown(5, 5);

    // Should not generate a command because validation failed
    const commandCall = handler.mock.calls.find(
      (call: any[]) => call[0] === 'commandGenerated'
    );
    expect(commandCall).toBeUndefined();
  });

  // 21. CostCallback computes cost
  it('CostCallback computes cost', () => {
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.setCostCallback((cmd) => cmd.tiles.length * 100);
    tm.setTool(ToolType.Place, 5);
    tm.onMouseDown(2, 2);

    const commandCall = handler.mock.calls.find(
      (call: any[]) => call[0] === 'commandGenerated'
    );
    expect(commandCall).toBeDefined();
    expect(commandCall![1].command.estimatedCost).toBe(100);
  });

  // 22. addEventListener receives events
  it('addEventListener receives events', () => {
    tm.setTool(ToolType.Zone);
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.onMouseDown(1, 1);

    expect(handler).toHaveBeenCalled();
    const stateCall = handler.mock.calls.find(
      (call: any[]) => call[0] === 'stateChanged'
    );
    expect(stateCall).toBeDefined();
    expect(stateCall![1].state).toBe(ToolState.Dragging);
  });

  // 23. removeEventListener stops receiving
  it('removeEventListener stops receiving events', () => {
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.removeEventListener(handler);
    tm.setTool(ToolType.Zone);
    tm.onMouseDown(1, 1);

    // Only stateChanged from cancelAction inside setTool would fire,
    // but handler was removed so nothing should be recorded
    expect(handler).not.toHaveBeenCalled();
  });

  // 24. Drag updates preview tiles
  it('drag updates preview tiles during hover', () => {
    tm.setTool(ToolType.Zone);
    tm.onMouseDown(0, 0);
    expect(tm.getState()).toBe(ToolState.Dragging);

    tm.onHover(2, 2);
    const preview = tm.getPreviewTiles();
    expect(preview.length).toBe(9); // 3x3 rectangle from (0,0) to (2,2)
    expect(preview).toContainEqual({ x: 0, y: 0 });
    expect(preview).toContainEqual({ x: 2, y: 2 });
    expect(preview).toContainEqual({ x: 1, y: 1 });
  });

  // --- Additional edge case tests ---

  it('getHoverTile returns a copy', () => {
    tm.setTool(ToolType.Place);
    tm.onHover(5, 5);
    const hover = tm.getHoverTile();
    hover!.x = 999;
    expect(tm.getHoverTile()!.x).toBe(5);
  });

  it('getDragRect returns a copy', () => {
    tm.setTool(ToolType.Zone);
    tm.onMouseDown(1, 1);
    const rect = tm.getDragRect();
    rect!.startX = 999;
    expect(tm.getDragRect()!.startX).toBe(1);
  });

  it('setTool with zoneType and roadType sets values', () => {
    tm.setTool(ToolType.Zone, undefined, 3);
    expect(tm.getSelectedZoneType()).toBe(3);

    tm.setTool(ToolType.Road, undefined, undefined, 5);
    expect(tm.getSelectedRoadType()).toBe(5);
  });

  it('onMouseDown with Select tool does nothing', () => {
    const handler = vi.fn();
    tm.addEventListener(handler);
    tm.onMouseDown(5, 5);
    // Only the cancelAction from constructor setTool - no additional events
    expect(tm.getState()).toBe(ToolState.Idle);
  });

  it('onMouseUp transitions from Dragging to Previewing', () => {
    tm.setTool(ToolType.Zone, undefined, 1);
    tm.onMouseDown(0, 0);
    tm.onHover(1, 1);
    expect(tm.getState()).toBe(ToolState.Dragging);

    tm.onMouseUp();
    expect(tm.getState()).toBe(ToolState.Previewing);
    expect(tm.getDragRect()).toBeNull();
  });

  it('preview shows validity from validate callback on hover', () => {
    tm.setValidateCallback(() => PlacementValidity.InvalidTerrain);
    tm.setTool(ToolType.Place, 1);
    tm.onHover(3, 3);
    expect(tm.getPreviewValidity()).toBe(PlacementValidity.InvalidTerrain);
    expect(tm.getPreviewTiles()).toEqual([{ x: 3, y: 3 }]);
  });

  it('computeRoadTiles with diagonal favors horizontal when dx >= dy', () => {
    const tiles = tm.computeRoadTiles({ x: 0, y: 0 }, { x: 3, y: 2 });
    // dx=3 >= dy=2, so horizontal: y stays at start.y=0
    expect(tiles.length).toBe(4);
    for (const t of tiles) {
      expect(t.y).toBe(0);
    }
  });

  it('road drag uses computeRoadTiles for preview', () => {
    tm.setTool(ToolType.Road, undefined, undefined, 2);
    tm.onMouseDown(0, 0);
    tm.onHover(0, 4); // Vertical: dy=4 > dx=0
    const preview = tm.getPreviewTiles();
    expect(preview.length).toBe(5);
    for (const t of preview) {
      expect(t.x).toBe(0);
    }
  });

  it('ToolState enum has correct values', () => {
    expect(ToolState.Idle).toBe('idle');
    expect(ToolState.Previewing).toBe('previewing');
    expect(ToolState.Dragging).toBe('dragging');
    expect(ToolState.Confirming).toBe('confirming');
  });

  it('PlacementValidity enum has correct values', () => {
    expect(PlacementValidity.Valid).toBe('valid');
    expect(PlacementValidity.InvalidTerrain).toBe('invalid_terrain');
    expect(PlacementValidity.Occupied).toBe('occupied');
    expect(PlacementValidity.InsufficientFunds).toBe('insufficient_funds');
    expect(PlacementValidity.OutOfBounds).toBe('out_of_bounds');
  });
});
