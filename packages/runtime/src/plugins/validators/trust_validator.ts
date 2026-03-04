// @townbuilder/runtime — Trust validator
// Checks that a manifest's source/author is in the trusted sources list.
// If the trusted list is empty, all sources are trusted.

import {
  ValidationStage,
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

/** Validates that a manifest's author is in the trusted sources list. */
export class TrustValidator implements IPluginValidator {
  readonly stage = ValidationStage.Trust;
  private readonly trustedSources: ReadonlySet<string>;

  /**
   * @param trustedSources - List of trusted author/source names.
   *   If empty, all sources are trusted.
   */
  constructor(trustedSources: Iterable<string>) {
    this.trustedSources = new Set(trustedSources);
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

    // If no trusted sources defined, trust everything
    if (this.trustedSources.size === 0) {
      return {
        stage: this.stage,
        valid: true,
        severity: ValidationSeverity.Info,
        message: "No trust restrictions configured; all sources trusted",
      };
    }

    const m = manifest as Record<string, unknown>;
    const author = typeof m.author === "string" ? m.author : "";

    if (author.length === 0) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Warning,
        message: "Manifest has no author specified",
        metadata: { field: "author" },
      };
    }

    if (!this.trustedSources.has(author)) {
      return {
        stage: this.stage,
        valid: false,
        severity: ValidationSeverity.Error,
        message: `Author "${author}" is not in the trusted sources list`,
        metadata: { author, trusted: Array.from(this.trustedSources) },
      };
    }

    return {
      stage: this.stage,
      valid: true,
      severity: ValidationSeverity.Info,
      message: `Author "${author}" is trusted`,
    };
  }
}
