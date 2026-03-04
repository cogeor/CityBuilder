// @townbuilder/runtime — Safety validator
// Checks plugin manifests for determinism safety by scanning for
// references to non-deterministic APIs like Math.random and Date.now
// in any code strings embedded in the manifest.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

/** Patterns that indicate non-deterministic behavior. */
const UNSAFE_PATTERNS: ReadonlyArray<{ pattern: RegExp; description: string }> = [
  { pattern: /\bMath\.random\b/, description: "Math.random" },
  { pattern: /\bDate\.now\b/, description: "Date.now" },
  { pattern: /\bnew\s+Date\b/, description: "new Date()" },
  { pattern: /\bcrypto\.getRandomValues\b/, description: "crypto.getRandomValues" },
];

/**
 * Recursively collect all string values from an object.
 * Used to scan manifest fields for unsafe code patterns.
 */
function collectStrings(value: unknown): string[] {
  if (typeof value === "string") {
    return [value];
  }
  if (Array.isArray(value)) {
    const result: string[] = [];
    for (const item of value) {
      result.push(...collectStrings(item));
    }
    return result;
  }
  if (value !== null && typeof value === "object") {
    const result: string[] = [];
    for (const key of Object.keys(value as Record<string, unknown>)) {
      result.push(...collectStrings((value as Record<string, unknown>)[key]));
    }
    return result;
  }
  return [];
}

/**
 * Validates that a plugin manifest does not contain references to
 * non-deterministic APIs in its string fields, ensuring simulation
 * determinism is preserved.
 */
export class SafetyValidator implements IPluginValidator {
  readonly stage = ValidationStage.Trust;

  validate(manifest: unknown): ValidationResult {
    if (manifest === null || manifest === undefined || typeof manifest !== "object") {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest must be a non-null object",
      };
    }

    const strings = collectStrings(manifest);
    const violations: string[] = [];

    for (const str of strings) {
      for (const { pattern, description } of UNSAFE_PATTERNS) {
        if (pattern.test(str)) {
          if (!violations.includes(description)) {
            violations.push(description);
          }
        }
      }
    }

    if (violations.length > 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Non-deterministic API references found: ${violations.join(", ")}`,
        metadata: { violations },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: "Safety validation passed: no non-deterministic references found",
    };
  }
}
