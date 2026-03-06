// @townbuilder/runtime — Schema validator
// Checks basic structural validity of a plugin manifest:
// non-empty id, semver-like version, and name presence.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";
import { normalizeForValidation } from "./manifest_input.js";

/** Semver pattern: major.minor.patch with optional pre-release. */
const SEMVER_PATTERN = /^\d+\.\d+\.\d+(-[\w.]+)?$/;

/** Validates the structural schema of a plugin manifest. */
export class SchemaValidator implements IPluginValidator {
  readonly stage = ValidationStage.Schema;

  validate(manifest: unknown): ValidationResult {
    const input = normalizeForValidation(this.stage, manifest);
    if ("error" in input) return input.error;
    const m = input.normalized;

    // Check id
    if (m.id.length === 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest 'id' must be a non-empty string",
        metadata: { field: "id", value: m.id },
      };
    }

    // Check version (semver pattern)
    if (!SEMVER_PATTERN.test(m.version)) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest 'version' must match semver pattern (e.g. 1.0.0)",
        metadata: { field: "version", value: m.version },
      };
    }

    // Check name
    if (m.name.length === 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest 'name' must be a non-empty string",
        metadata: { field: "name", value: m.name },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: "Schema validation passed",
    };
  }
}
