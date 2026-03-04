// @townbuilder/runtime -- Tests for ErrorRecoveryManager
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  RecoveryAction,
  ErrorRecoveryManager,
  type ErrorReport,
} from "../error_recovery.js";

describe("ErrorRecoveryManager", () => {
  let manager: ErrorRecoveryManager;

  beforeEach(() => {
    manager = new ErrorRecoveryManager(3);
  });

  // ---- Test 1: reportError adds to log ----
  it("reportError adds to log", () => {
    expect(manager.getErrorLog()).toHaveLength(0);

    manager.reportError(1, "WASM panic: test");

    const log = manager.getErrorLog();
    expect(log).toHaveLength(1);
    expect(log[0].code).toBe(1);
    expect(log[0].message).toBe("WASM panic: test");
    expect(log[0].timestamp).toBeGreaterThan(0);
  });

  // ---- Test 2: getRecommendedAction returns Restart initially ----
  it("getRecommendedAction returns Restart when no errors", () => {
    expect(manager.getRecommendedAction()).toBe(RecoveryAction.Restart);
  });

  // ---- Test 3: getRecommendedAction returns Restart under max ----
  it("getRecommendedAction returns Restart under max errors", () => {
    manager.reportError(1, "error 1");
    manager.reportError(1, "error 2");

    // 2 errors < 3 max -> Restart
    expect(manager.getRecommendedAction()).toBe(RecoveryAction.Restart);
  });

  // ---- Test 4: getRecommendedAction returns LoadLastSave at max ----
  it("getRecommendedAction returns LoadLastSave at max errors", () => {
    manager.reportError(1, "error 1");
    manager.reportError(1, "error 2");
    manager.reportError(1, "error 3");

    // 3 errors === 3 max -> LoadLastSave
    expect(manager.getRecommendedAction()).toBe(RecoveryAction.LoadLastSave);
  });

  // ---- Test 5: getRecommendedAction returns Abort after max errors ----
  it("getRecommendedAction returns Abort after exceeding max errors", () => {
    manager.reportError(1, "error 1");
    manager.reportError(1, "error 2");
    manager.reportError(1, "error 3");
    manager.reportError(1, "error 4");

    // 4 errors > 3 max -> Abort
    expect(manager.getRecommendedAction()).toBe(RecoveryAction.Abort);
  });

  // ---- Test 6: onError callback fires ----
  it("onError callback fires on reportError", () => {
    const spy = vi.fn<(report: ErrorReport) => void>();
    manager.onError(spy);

    manager.reportError(2, "Out of WASM memory");

    expect(spy).toHaveBeenCalledTimes(1);
    const report = spy.mock.calls[0][0];
    expect(report.code).toBe(2);
    expect(report.message).toBe("Out of WASM memory");
    expect(report.recoveryAction).toBe(RecoveryAction.Restart);
  });

  // ---- Test 7: clear resets log ----
  it("clear resets the error log", () => {
    manager.reportError(1, "error 1");
    manager.reportError(1, "error 2");
    expect(manager.getErrorCount()).toBe(2);

    manager.clear();

    expect(manager.getErrorCount()).toBe(0);
    expect(manager.getErrorLog()).toHaveLength(0);
    expect(manager.getRecommendedAction()).toBe(RecoveryAction.Restart);
  });

  // ---- Test 8: getErrorCount accurate ----
  it("getErrorCount returns correct count", () => {
    expect(manager.getErrorCount()).toBe(0);
    manager.reportError(1, "a");
    expect(manager.getErrorCount()).toBe(1);
    manager.reportError(2, "b");
    expect(manager.getErrorCount()).toBe(2);
    manager.reportError(99, "c");
    expect(manager.getErrorCount()).toBe(3);
  });

  // ---- Test 9: unsubscribe stops callback ----
  it("unsubscribe prevents further callback invocations", () => {
    const spy = vi.fn<(report: ErrorReport) => void>();
    const unsub = manager.onError(spy);

    manager.reportError(1, "first");
    expect(spy).toHaveBeenCalledTimes(1);

    unsub();

    manager.reportError(1, "second");
    expect(spy).toHaveBeenCalledTimes(1); // still 1
  });

  // ---- Test 10: reportError assigns correct recovery action ----
  it("reportError assigns the correct recovery action per error", () => {
    // maxErrors = 3
    const r1 = manager.reportError(1, "e1"); // count becomes 1 < 3
    expect(r1.recoveryAction).toBe(RecoveryAction.Restart);

    const r2 = manager.reportError(1, "e2"); // count becomes 2 < 3
    expect(r2.recoveryAction).toBe(RecoveryAction.Restart);

    const r3 = manager.reportError(1, "e3"); // count becomes 3 === 3
    expect(r3.recoveryAction).toBe(RecoveryAction.LoadLastSave);

    const r4 = manager.reportError(1, "e4"); // count becomes 4 > 3
    expect(r4.recoveryAction).toBe(RecoveryAction.Abort);
  });

  // ---- Test 11: getErrorLog returns a copy ----
  it("getErrorLog returns a copy, not the internal array", () => {
    manager.reportError(1, "a");
    const log = manager.getErrorLog();
    log.push({ code: 999, message: "fake", timestamp: 0, recoveryAction: RecoveryAction.Abort });

    expect(manager.getErrorCount()).toBe(1); // internal unaffected
  });

  // ---- Test 12: default maxErrors is 3 ----
  it("uses default maxErrors of 3 when no argument provided", () => {
    const defaultManager = new ErrorRecoveryManager();

    defaultManager.reportError(1, "a");
    defaultManager.reportError(1, "b");
    expect(defaultManager.getRecommendedAction()).toBe(RecoveryAction.Restart);

    defaultManager.reportError(1, "c");
    expect(defaultManager.getRecommendedAction()).toBe(RecoveryAction.LoadLastSave);
  });

  // ---- Test 13: callback error does not break other callbacks ----
  it("callback throwing does not prevent other callbacks from firing", () => {
    const spy = vi.fn<(report: ErrorReport) => void>();
    manager.onError(() => {
      throw new Error("callback failure");
    });
    manager.onError(spy);

    manager.reportError(1, "test");

    expect(spy).toHaveBeenCalledTimes(1);
  });
});
