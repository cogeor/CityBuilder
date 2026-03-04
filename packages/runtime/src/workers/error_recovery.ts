// @townbuilder/runtime -- WASM error recovery manager
// Mirrors the Rust error_boundary module on the JS side, providing
// error tracking, recommended recovery actions, and callback hooks.

// ---------------------------------------------------------------------------
// RecoveryAction enum
// ---------------------------------------------------------------------------

/** Action the runtime should take after a WASM error. */
export enum RecoveryAction {
  Restart = "restart",
  LoadLastSave = "load_last_save",
  Abort = "abort",
}

// ---------------------------------------------------------------------------
// ErrorReport interface
// ---------------------------------------------------------------------------

/** A single error report produced by the recovery manager. */
export interface ErrorReport {
  /** Numeric error code matching WasmError::code() on the Rust side. */
  code: number;
  /** Human-readable error message. */
  message: string;
  /** Unix timestamp (ms) when the error was recorded. */
  timestamp: number;
  /** The recovery action assigned to this error. */
  recoveryAction: RecoveryAction;
}

// ---------------------------------------------------------------------------
// ErrorRecoveryManager
// ---------------------------------------------------------------------------

/**
 * Tracks WASM errors on the JS side and recommends recovery actions.
 *
 * - Under `maxErrors` total errors -> recommend `Restart`.
 * - At `maxErrors` -> recommend `LoadLastSave`.
 * - Over `maxErrors` -> recommend `Abort`.
 */
export class ErrorRecoveryManager {
  private errorLog: ErrorReport[] = [];
  private maxErrors: number;
  private onErrorCallbacks: Array<(report: ErrorReport) => void> = [];

  constructor(maxErrors: number = 3) {
    this.maxErrors = maxErrors;
  }

  /**
   * Record a new error and return the generated report.
   *
   * The recommended recovery action is computed based on the current
   * error count *after* adding this error.
   */
  reportError(code: number, message: string): ErrorReport {
    const action = this.getRecommendedActionForCount(this.errorLog.length + 1);

    const report: ErrorReport = {
      code,
      message,
      timestamp: Date.now(),
      recoveryAction: action,
    };

    this.errorLog.push(report);

    // Notify subscribers.
    for (const cb of this.onErrorCallbacks) {
      try {
        cb(report);
      } catch {
        // Swallow callback errors to avoid cascading failures.
      }
    }

    return report;
  }

  /**
   * Get the recommended recovery action based on the *current* error count.
   *
   * - 0 errors -> Restart (nothing has happened yet, fresh start).
   * - < maxErrors -> Restart.
   * - === maxErrors -> LoadLastSave.
   * - > maxErrors -> Abort.
   */
  getRecommendedAction(): RecoveryAction {
    return this.getRecommendedActionForCount(this.errorLog.length);
  }

  /** Return a copy of the full error log. */
  getErrorLog(): ErrorReport[] {
    return [...this.errorLog];
  }

  /** Number of errors recorded so far. */
  getErrorCount(): number {
    return this.errorLog.length;
  }

  /**
   * Subscribe to error events.
   * Returns an unsubscribe function.
   */
  onError(callback: (report: ErrorReport) => void): () => void {
    this.onErrorCallbacks.push(callback);
    return () => {
      const idx = this.onErrorCallbacks.indexOf(callback);
      if (idx !== -1) {
        this.onErrorCallbacks.splice(idx, 1);
      }
    };
  }

  /** Clear all recorded errors and reset state. */
  clear(): void {
    this.errorLog = [];
  }

  // -- Private helpers ------------------------------------------------------

  private getRecommendedActionForCount(count: number): RecoveryAction {
    if (count < this.maxErrors) {
      return RecoveryAction.Restart;
    } else if (count === this.maxErrors) {
      return RecoveryAction.LoadLastSave;
    }
    return RecoveryAction.Abort;
  }
}
