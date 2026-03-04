// @townbuilder/runtime — Validation pipeline
// Runs a chain of IPluginValidator instances against a manifest,
// collecting results and optionally short-circuiting on first error.

import {
  ValidationSeverity,
  type ValidationResult,
  type IPluginValidator,
} from "./types.js";

/** Orchestrates a chain of plugin validators. */
export class ValidationPipeline {
  private readonly validators: IPluginValidator[] = [];

  /** Add a validator to the end of the chain. */
  addValidator(validator: IPluginValidator): void {
    this.validators.push(validator);
  }

  /**
   * Run all validators in order, short-circuiting on the first Error result.
   * Returns accumulated results up to and including the first error.
   */
  validate(manifest: unknown): ValidationResult[] {
    const results: ValidationResult[] = [];

    for (const validator of this.validators) {
      const result = validator.validate(manifest);
      results.push(result);

      if (!result.valid && result.severity === ValidationSeverity.Error) {
        break;
      }
    }

    return results;
  }

  /**
   * Run all validators in order without short-circuiting.
   * Returns results from every validator regardless of errors.
   */
  validateAll(manifest: unknown): ValidationResult[] {
    const results: ValidationResult[] = [];

    for (const validator of this.validators) {
      results.push(validator.validate(manifest));
    }

    return results;
  }

  /** Return a copy of the current validator chain. */
  getValidators(): IPluginValidator[] {
    return [...this.validators];
  }
}
