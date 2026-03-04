// @townbuilder/runtime — Compatibility validator
// Checks that a new plugin manifest does not conflict with already-active
// plugins. Detects duplicate IDs and overlapping exclusive content types.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

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
    if (manifest === null || manifest === undefined || typeof manifest !== "object") {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: "Manifest must be a non-null object",
      };
    }

    const m = manifest as Record<string, unknown>;
    const conflicts: string[] = [];

    // Check for duplicate plugin ID
    if (typeof m.id === "string" && this.activePlugins.includes(m.id)) {
      conflicts.push(`Plugin "${m.id}" is already active`);
    }

    // Check content_type conflicts (v1 manifests use content_type)
    if (typeof m.content_type === "string") {
      if (
        EXCLUSIVE_CONTENT_TYPES.has(m.content_type) &&
        this.activeContentTypes.includes(m.content_type)
      ) {
        conflicts.push(
          `Content type "${m.content_type}" is exclusive and already claimed by an active plugin`,
        );
      }
    }

    // Check contentTypes conflicts (v2 manifests use contentTypes array)
    if (Array.isArray(m.contentTypes)) {
      for (const ct of m.contentTypes) {
        if (
          typeof ct === "string" &&
          EXCLUSIVE_CONTENT_TYPES.has(ct) &&
          this.activeContentTypes.includes(ct)
        ) {
          conflicts.push(
            `Content type "${ct}" is exclusive and already claimed by an active plugin`,
          );
        }
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
