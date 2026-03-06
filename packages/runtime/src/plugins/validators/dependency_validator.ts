// @townbuilder/runtime — Dependency validator
// Checks that all declared plugin dependencies exist in the available set.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";
import { normalizeForValidation } from "./manifest_input.js";

/** Validates that a manifest's declared dependencies are all available. */
export class DependencyValidator implements IPluginValidator {
  readonly stage = ValidationStage.Dependency;
  private readonly availableIds: ReadonlySet<string>;

  /**
   * @param availableIds - Set of plugin IDs that are currently available/registered.
   */
  constructor(availableIds: Iterable<string>) {
    this.availableIds = new Set(availableIds);
  }

  validate(manifest: unknown): ValidationResult {
    const input = normalizeForValidation(this.stage, manifest);
    if ("error" in input) return input.error;
    const m = input.normalized;

    const missing: string[] = [];
    for (const dep of m.dependencies) {
      if (!this.availableIds.has(dep)) {
        missing.push(dep);
      }
    }

    if (missing.length > 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Missing dependencies: ${missing.join(", ")}`,
        metadata: { missing },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: "All dependencies satisfied",
    };
  }
}
