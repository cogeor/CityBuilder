// @townbuilder/ui — ToolCommandDispatcher
// Bridges ToolManager commandGenerated events to engine command delivery.
// Uses dependency-injection callbacks to avoid a hard cross-package dependency
// on @townbuilder/runtime while still calling translateToolInteraction() +
// RuntimeFacade.sendCommand() inside this class.

import type { ToolManager, ToolCommand, ToolEventPayloads } from "./tool_manager.js";

/**
 * Translates a single ToolCommand into zero or more engine commands.
 * Wire in translateToolInteraction() from @townbuilder/runtime/engine/interaction_bridge.
 */
export type TranslateFn = (cmd: ToolCommand) => readonly object[];

/**
 * Delivers a single engine command to the simulation.
 * Wire in RuntimeFacade.sendCommand() (bound to the facade instance).
 */
export type SendCommandFn = (cmd: object) => void;

/**
 * Subscribes to ToolManager.commandGenerated events, translates each ToolCommand
 * into one or more engine commands via the provided translate function, then
 * delivers them via sendCommand. No caller loop required.
 *
 * Usage:
 * ```ts
 * const dispatcher = new ToolCommandDispatcher(
 *   toolManager,
 *   (cmd) => translateToolInteraction(cmd as ToolInteractionCommand),
 *   (cmd) => facade.sendCommand(cmd as EngineCommand),
 * );
 * ```
 */
export class ToolCommandDispatcher {
  private readonly _toolManager: ToolManager;
  private readonly _translate: TranslateFn;
  private readonly _send: SendCommandFn;
  private readonly _handler: (type: string, payload: ToolEventPayloads[keyof ToolEventPayloads]) => void;

  constructor(toolManager: ToolManager, translate: TranslateFn, send: SendCommandFn) {
    this._toolManager = toolManager;
    this._translate = translate;
    this._send = send;

    this._handler = (type, payload) => {
      if (type !== "commandGenerated") return;
      const { command } = payload as ToolEventPayloads["commandGenerated"];
      const engineCommands = this._translate(command);
      for (const cmd of engineCommands) {
        try {
          this._send(cmd);
        } catch {
          // sendCommand throws when runtime is not Running; swallow to avoid crashing UI
        }
      }
    };

    this._toolManager.addEventListener(this._handler as any);
  }

  /** Detach this dispatcher from the ToolManager. */
  dispose(): void {
    this._toolManager.removeEventListener(this._handler as any);
  }
}
