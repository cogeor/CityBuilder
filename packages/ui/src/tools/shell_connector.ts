// @townbuilder/ui — Shell-to-ToolManager connector
// Listens to HudShell.toolChange events and forwards them to ToolManager.setTool()
// with optional sub-type lookup callbacks for zone type, archetype, and road tier.

import type { HudShell } from "../shell/shell.js";
import { ToolType } from "../shell/shell.js";
import type { ToolManager } from "./tool_manager.js";

export interface ShellConnectorOptions {
  /** Called when tool changes to Place — return archetype ID to use. */
  getArchetypeId?: () => number | undefined;
  /** Called when tool changes to Zone — return zone type code to use. */
  getZoneType?: () => number | undefined;
  /** Called when tool changes to Road — return road tier code to use. */
  getRoadType?: () => number | undefined;
}

/**
 * Connects a HudShell to a ToolManager so that tool changes in the HUD
 * automatically propagate to the tool manager.
 *
 * Returns an unsubscribe function that detaches the listener.
 *
 * Usage:
 * ```ts
 * const disconnect = connectShellToToolManager(shell, toolManager, {
 *   getArchetypeId: () => selectedArchetypeId,
 *   getZoneType: () => selectedZoneCode,
 * });
 * // Later:
 * disconnect();
 * ```
 */
export function connectShellToToolManager(
  shell: HudShell,
  toolManager: ToolManager,
  opts?: ShellConnectorOptions,
): () => void {
  const handler = (type: string, payload: { tool: ToolType }) => {
    if (type !== "toolChange") return;
    const tool = payload.tool;

    const archetypeId = tool === ToolType.Place ? opts?.getArchetypeId?.() : undefined;
    const zoneType = tool === ToolType.Zone ? opts?.getZoneType?.() : undefined;
    const roadType = tool === ToolType.Road ? opts?.getRoadType?.() : undefined;

    toolManager.setTool(tool, archetypeId, zoneType, roadType);
  };

  shell.addEventListener(handler as any);

  return () => shell.removeEventListener(handler as any);
}
