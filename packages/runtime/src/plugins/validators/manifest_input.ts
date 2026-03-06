// @townbuilder/runtime -- Validator manifest helpers

import {
  normalizeManifest,
  type PluginManifest,
} from "../manifest.js";
import {
  ValidationSeverity,
  type ValidationResult,
  type ValidationStage,
} from "./types.js";

export function requireObjectManifest(
  stage: ValidationStage,
  manifest: unknown,
): ValidationResult | null {
  if (manifest === null || manifest === undefined || typeof manifest !== "object") {
    return {
      stage,
      valid: false,
      severity: ValidationSeverity.Error,
      message: "Manifest must be a non-null object",
    };
  }
  return null;
}

export function normalizeForValidation(
  stage: ValidationStage,
  manifest: unknown,
): { normalized: PluginManifest } | { error: ValidationResult } {
  const objectError = requireObjectManifest(stage, manifest);
  if (objectError) {
    return { error: objectError };
  }

  const normalized = normalizeManifest(manifest);
  if (!normalized) {
    return {
      error: {
        stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest normalization failed",
      },
    };
  }

  return { normalized };
}
