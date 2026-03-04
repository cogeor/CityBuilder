// @townbuilder/runtime — Dependency validator
// Checks that all declared plugin dependencies exist in the available set.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

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
    if (manifest === null || manifest === undefined || typeof manifest !== "object") {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest must be a non-null object",
      };
    }

    const m = manifest as Record<string, unknown>;

    // If no dependencies array, pass (schema validator handles structure)
    if (!Array.isArray(m.dependencies)) {
      return {
        stage: this.stage,
        valid: true,
        severity: ValidationSeverity.Info,
        message: "No dependencies declared",
      };
    }

    const missing: string[] = [];
    for (const dep of m.dependencies) {
      if (typeof dep === "string" && !this.availableIds.has(dep)) {
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
