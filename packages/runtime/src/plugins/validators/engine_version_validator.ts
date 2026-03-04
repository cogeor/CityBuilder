// @townbuilder/runtime — Engine version validator
// Checks that a manifest's required engine version range is compatible
// with the running engine version (simple major version comparison).

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

/**
 * Parse the major version number from a version string.
 * Returns NaN if the string doesn't start with a valid number.
 */
function parseMajor(version: string): number {
  // Strip leading comparison operators (>=, ^, ~, etc.)
  const cleaned = version.replace(/^[^0-9]*/, "");
  const dotIndex = cleaned.indexOf(".");
  const majorStr = dotIndex === -1 ? cleaned : cleaned.slice(0, dotIndex);
  return parseInt(majorStr, 10);
}

/** Validates that a manifest's engine version is compatible with the running engine. */
export class EngineVersionValidator implements IPluginValidator {
  readonly stage = ValidationStage.EngineVersion;
  private readonly engineMajor: number;
  private readonly engineVersion: string;

  /**
   * @param engineVersion - The current engine version string (e.g. "2.3.1").
   */
  constructor(engineVersion: string) {
    this.engineVersion = engineVersion;
    this.engineMajor = parseMajor(engineVersion);
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

    // If no engine field, treat as compatible (no constraint declared)
    if (m.engine === undefined || m.engine === null) {
      return {
        stage: this.stage,
        valid: true,
        severity: ValidationSeverity.Info,
        message: "No engine version constraint declared",
      };
    }

    if (typeof m.engine !== "string" || m.engine.length === 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest 'engine' must be a non-empty string if specified",
        metadata: { field: "engine", value: m.engine },
      };
    }

    const requiredMajor = parseMajor(m.engine);

    if (isNaN(requiredMajor)) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Cannot parse engine version from "${m.engine}"`,
        metadata: { engine: m.engine },
      };
    }

    if (requiredMajor !== this.engineMajor) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Engine major version mismatch: plugin requires ${requiredMajor}.x, engine is ${this.engineVersion}`,
        metadata: {
          required: m.engine,
          actual: this.engineVersion,
          requiredMajor,
          actualMajor: this.engineMajor,
        },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: `Engine version compatible (major ${this.engineMajor})`,
    };
  }
}
