// @townbuilder/runtime — Plugin validation types
// Chain of Responsibility pattern for validating plugin manifests
// through a configurable pipeline of validators.

// ---- Validation Stage ----

/** The stage at which validation occurs. */
export enum ValidationStage {
  Schema = "Schema",
  Dependency = "Dependency",
  EngineVersion = "EngineVersion",
  Trust = "Trust",
}

// ---- Validation Severity ----

/** How severe a validation finding is. */
export enum ValidationSeverity {
  Error = "Error",
  Warning = "Warning",
  Info = "Info",
}

// ---- Validation Result ----

/** The outcome of a single validation check. */
export interface ValidationResult {
  /** Which stage produced this result. */
  readonly stage: ValidationStage;
  /** Whether the manifest passed this check. */
  readonly valid: boolean;
  /** Severity of the finding. */
  readonly severity: ValidationSeverity;
  /** Human-readable description of the result. */
  readonly message: string;
  /** Optional structured data about the finding. */
  readonly metadata?: Record<string, unknown>;
}

// ---- Validator Interface ----

/** A single validator in the chain of responsibility. */
export interface IPluginValidator {
  /** Which pipeline stage this validator belongs to. */
  readonly stage: ValidationStage;
  /** Validate an unknown manifest value, returning a result. */
  validate(manifest: unknown): ValidationResult;
}
