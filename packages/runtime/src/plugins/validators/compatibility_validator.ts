// @townbuilder/runtime — Compatibility validator
// Checks that a new plugin manifest does not conflict with already-active
// plugins. Detects duplicate IDs and overlapping exclusive content types.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";
import { normalizeForValidation } from "./manifest_input.js";

/** Content types where only one plugin should be active at a time. */
const EXCLUSIVE_CONTENT_TYPES: ReadonlySet<string> = new Set([
  "economy",
  "terrain",
  "governance",
]);

/**
 * Validates that a new plugin is compatible with the currently active
 * plugin set. Checks for:
 * - Duplicate plugin IDs
 * - Conflicting exclusive content types
 */
export class CompatibilityValidator implements IPluginValidator {
  readonly stage = ValidationStage.Dependency;
  private readonly activePlugins: ReadonlyArray<string>;

  /**
   * @param activePlugins - IDs of currently active plugins.
   *   To also check content type conflicts, pass manifests via
   *   the optional second parameter.
   * @param activeContentTypes - Content types claimed by active plugins.
   */
  constructor(
    activePlugins: string[],
    private readonly activeContentTypes: string[] = [],
  ) {
    this.activePlugins = [...activePlugins];
  }

  validate(manifest: unknown): ValidationResult {
    const input = normalizeForValidation(this.stage, manifest);
    if ("error" in input) return input.error;
    const m = input.normalized;
    const conflicts: string[] = [];

    // Check for duplicate plugin ID
    if (this.activePlugins.includes(m.id)) {
      conflicts.push(`Plugin "${m.id}" is already active`);
    }

    // Check conflicts against canonical contributes keys.
    for (const ct of Object.keys(m.contributes)) {
      if (
        EXCLUSIVE_CONTENT_TYPES.has(ct) &&
        this.activeContentTypes.includes(ct)
      ) {
        conflicts.push(
          `Content type "${ct}" is exclusive and already claimed by an active plugin`,
        );
      }
    }

    if (conflicts.length > 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Compatibility conflicts: ${conflicts.join("; ")}`,
        metadata: { conflicts },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: "Compatibility validation passed",
    };
  }
}
