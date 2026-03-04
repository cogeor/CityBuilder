// @townbuilder/runtime — Tests for WorkerStateMachine
import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  WorkerLifecycleState,
  WorkerStateMachine,
  VALID_TRANSITIONS,
  type StateTransition,
  type TransitionGuard,
} from "../state_machine.js";

describe("WorkerStateMachine", () => {
  let sm: WorkerStateMachine;

  beforeEach(() => {
    sm = new WorkerStateMachine();
  });

  // ---- Test 1: starts in Uninitialized ----
  it("starts in Uninitialized state", () => {
    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
  });

  // ---- Test 2: valid transition succeeds ----
  it("valid transition succeeds and updates state", () => {
    const result = sm.transition(WorkerLifecycleState.Ready);

    expect(result).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Ready);
  });

  // ---- Test 3: invalid transition returns false and stays ----
  it("invalid transition returns false and stays in current state", () => {
    // Uninitialized -> Running is not valid
    const result = sm.transition(WorkerLifecycleState.Running);

    expect(result).toBe(false);
    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
  });

  // ---- Test 4: transition history is recorded ----
  it("records transition history", () => {
    sm.transition(WorkerLifecycleState.Ready);
    sm.transition(WorkerLifecycleState.Running);

    const history = sm.getHistory();
    expect(history).toHaveLength(2);
    expect(history[0].from).toBe(WorkerLifecycleState.Uninitialized);
    expect(history[0].to).toBe(WorkerLifecycleState.Ready);
    expect(history[1].from).toBe(WorkerLifecycleState.Ready);
    expect(history[1].to).toBe(WorkerLifecycleState.Running);
    expect(typeof history[0].timestamp).toBe("number");
  });

  // ---- Test 5: guards can block transitions ----
  it("guards can block transitions", () => {
    const blockGuard: TransitionGuard = (_from, _to) => false;
    sm.addGuard(blockGuard);

    const result = sm.transition(WorkerLifecycleState.Ready);

    expect(result).toBe(false);
    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
  });

  // ---- Test 6: onTransition callback fires ----
  it("onTransition callback fires on successful transition", () => {
    const fired: StateTransition[] = [];
    sm.onTransition((t) => fired.push(t));

    sm.transition(WorkerLifecycleState.Ready);

    expect(fired).toHaveLength(1);
    expect(fired[0].from).toBe(WorkerLifecycleState.Uninitialized);
    expect(fired[0].to).toBe(WorkerLifecycleState.Ready);
  });

  // ---- Test 7: onTransition callback does NOT fire on failed transition ----
  it("onTransition callback does not fire on failed transition", () => {
    const fired: StateTransition[] = [];
    sm.onTransition((t) => fired.push(t));

    sm.transition(WorkerLifecycleState.Running); // invalid from Uninitialized

    expect(fired).toHaveLength(0);
  });

  // ---- Test 8: canTransition checks without applying ----
  it("canTransition checks without applying", () => {
    expect(sm.canTransition(WorkerLifecycleState.Ready)).toBe(true);
    expect(sm.canTransition(WorkerLifecycleState.Running)).toBe(false);

    // State should not have changed
    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
  });

  // ---- Test 9: reset clears state and history ----
  it("reset clears state and history", () => {
    sm.transition(WorkerLifecycleState.Ready);
    sm.transition(WorkerLifecycleState.Running);

    expect(sm.getState()).toBe(WorkerLifecycleState.Running);
    expect(sm.getHistory()).toHaveLength(2);

    sm.reset();

    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
    expect(sm.getHistory()).toHaveLength(0);
  });

  // ---- Test 10: all valid transitions work ----
  it("all valid transitions from VALID_TRANSITIONS succeed", () => {
    for (const [from, targets] of VALID_TRANSITIONS) {
      for (const to of targets) {
        const machine = new WorkerStateMachine();

        // Navigate to the "from" state first
        navigateTo(machine, from);
        expect(machine.getState()).toBe(from);

        const result = machine.transition(to);
        expect(result).toBe(true);
        expect(machine.getState()).toBe(to);
      }
    }
  });

  // ---- Test 11: Error -> Stopping -> Stopped path ----
  it("Error -> Stopping -> Stopped path works", () => {
    sm.transition(WorkerLifecycleState.Ready);
    sm.transition(WorkerLifecycleState.Running);
    sm.transition(WorkerLifecycleState.Error);

    expect(sm.getState()).toBe(WorkerLifecycleState.Error);

    expect(sm.transition(WorkerLifecycleState.Stopping)).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Stopping);

    expect(sm.transition(WorkerLifecycleState.Stopped)).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Stopped);
  });

  // ---- Test 12: Paused -> Running -> Paused cycle ----
  it("Paused -> Running -> Paused cycle works", () => {
    sm.transition(WorkerLifecycleState.Ready);
    sm.transition(WorkerLifecycleState.Running);
    sm.transition(WorkerLifecycleState.Paused);

    expect(sm.getState()).toBe(WorkerLifecycleState.Paused);

    expect(sm.transition(WorkerLifecycleState.Running)).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Running);

    expect(sm.transition(WorkerLifecycleState.Paused)).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Paused);

    expect(sm.transition(WorkerLifecycleState.Running)).toBe(true);
    expect(sm.getState()).toBe(WorkerLifecycleState.Running);
  });

  // ---- Test 13: multiple guards, all must pass ----
  it("multiple guards must all pass for transition to succeed", () => {
    const allowGuard: TransitionGuard = () => true;
    const blockGuard: TransitionGuard = (_from, to) =>
      to !== WorkerLifecycleState.Ready;

    sm.addGuard(allowGuard);
    sm.addGuard(blockGuard);

    // blockGuard blocks Ready
    expect(sm.transition(WorkerLifecycleState.Ready)).toBe(false);
    expect(sm.getState()).toBe(WorkerLifecycleState.Uninitialized);
  });

  // ---- Test 14: getHistory returns a copy, not a reference ----
  it("getHistory returns a copy, not a reference", () => {
    sm.transition(WorkerLifecycleState.Ready);

    const history1 = sm.getHistory();
    const history2 = sm.getHistory();

    expect(history1).toEqual(history2);
    expect(history1).not.toBe(history2);
  });

  // ---- Test 15: Stopped is a terminal state ----
  it("Stopped is a terminal state with no valid transitions", () => {
    const targets = VALID_TRANSITIONS.get(WorkerLifecycleState.Stopped);
    expect(targets).toBeUndefined();

    sm.transition(WorkerLifecycleState.Ready);
    sm.transition(WorkerLifecycleState.Stopping);
    sm.transition(WorkerLifecycleState.Stopped);

    expect(sm.getState()).toBe(WorkerLifecycleState.Stopped);

    // Cannot go anywhere from Stopped
    for (const state of Object.values(WorkerLifecycleState)) {
      expect(sm.canTransition(state)).toBe(false);
    }
  });
});

// ---- Helper ----

/**
 * Navigate a fresh state machine to the given target state by following
 * the shortest valid path from Uninitialized.
 */
function navigateTo(
  machine: WorkerStateMachine,
  target: WorkerLifecycleState,
): void {
  const paths: Record<string, WorkerLifecycleState[]> = {
    [WorkerLifecycleState.Uninitialized]: [],
    [WorkerLifecycleState.Ready]: [WorkerLifecycleState.Ready],
    [WorkerLifecycleState.Running]: [
      WorkerLifecycleState.Ready,
      WorkerLifecycleState.Running,
    ],
    [WorkerLifecycleState.Paused]: [
      WorkerLifecycleState.Ready,
      WorkerLifecycleState.Running,
      WorkerLifecycleState.Paused,
    ],
    [WorkerLifecycleState.Stopping]: [
      WorkerLifecycleState.Ready,
      WorkerLifecycleState.Stopping,
    ],
    [WorkerLifecycleState.Stopped]: [
      WorkerLifecycleState.Ready,
      WorkerLifecycleState.Stopping,
      WorkerLifecycleState.Stopped,
    ],
    [WorkerLifecycleState.Error]: [
      WorkerLifecycleState.Ready,
      WorkerLifecycleState.Running,
      WorkerLifecycleState.Error,
    ],
  };

  const path = paths[target];
  for (const step of path) {
    machine.transition(step);
  }
}
