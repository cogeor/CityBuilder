// @townbuilder/runtime — Worker lifecycle state machine
// Implements the State pattern for managing worker lifecycle transitions.

// ---- WorkerLifecycleState Enum ----

/** Possible lifecycle states for a worker. */
export enum WorkerLifecycleState {
  Uninitialized = "Uninitialized",
  Ready = "Ready",
  Running = "Running",
  Paused = "Paused",
  Stopping = "Stopping",
  Stopped = "Stopped",
  Error = "Error",
}

// ---- StateTransition Interface ----

/** Record of a state transition. */
export interface StateTransition {
  readonly from: WorkerLifecycleState;
  readonly to: WorkerLifecycleState;
  readonly timestamp: number;
}

// ---- TransitionGuard Type ----

/**
 * A guard function that decides whether a transition is allowed.
 * Returns true to allow, false to block.
 */
export type TransitionGuard = (
  from: WorkerLifecycleState,
  to: WorkerLifecycleState,
) => boolean;

// ---- Valid Transitions ----

/**
 * Map of allowed transitions. Each key is a source state, and
 * its value is the set of valid target states.
 */
export const VALID_TRANSITIONS: ReadonlyMap<
  WorkerLifecycleState,
  ReadonlySet<WorkerLifecycleState>
> = new Map<WorkerLifecycleState, Set<WorkerLifecycleState>>([
  [
    WorkerLifecycleState.Uninitialized,
    new Set([WorkerLifecycleState.Ready]),
  ],
  [
    WorkerLifecycleState.Ready,
    new Set([WorkerLifecycleState.Running, WorkerLifecycleState.Stopping]),
  ],
  [
    WorkerLifecycleState.Running,
    new Set([
      WorkerLifecycleState.Paused,
      WorkerLifecycleState.Stopping,
      WorkerLifecycleState.Error,
    ]),
  ],
  [
    WorkerLifecycleState.Paused,
    new Set([WorkerLifecycleState.Running, WorkerLifecycleState.Stopping]),
  ],
  [
    WorkerLifecycleState.Stopping,
    new Set([WorkerLifecycleState.Stopped]),
  ],
  [
    WorkerLifecycleState.Error,
    new Set([WorkerLifecycleState.Stopping]),
  ],
]);

// ---- WorkerStateMachine Class ----

/** Transition event callback type. */
type TransitionCallback = (transition: StateTransition) => void;

/**
 * State machine that enforces valid lifecycle transitions for workers.
 *
 * Provides guard registration, transition history, and event callbacks.
 */
export class WorkerStateMachine {
  private _state: WorkerLifecycleState;
  private _history: StateTransition[];
  private _guards: TransitionGuard[];
  private _listeners: TransitionCallback[];

  constructor() {
    this._state = WorkerLifecycleState.Uninitialized;
    this._history = [];
    this._guards = [];
    this._listeners = [];
  }

  /** Returns the current lifecycle state. */
  getState(): WorkerLifecycleState {
    return this._state;
  }

  /**
   * Attempt to transition to the given state.
   *
   * Returns true if the transition succeeded, false if it was invalid
   * or blocked by a guard.
   */
  transition(to: WorkerLifecycleState): boolean {
    if (!this.canTransition(to)) {
      return false;
    }

    const from = this._state;
    const record: StateTransition = {
      from,
      to,
      timestamp: Date.now(),
    };

    this._state = to;
    this._history.push(record);

    // Notify listeners
    for (const cb of this._listeners) {
      cb(record);
    }

    return true;
  }

  /**
   * Register a guard function. All registered guards must return true
   * for a transition to proceed.
   */
  addGuard(guard: TransitionGuard): void {
    this._guards.push(guard);
  }

  /**
   * Check whether a transition to the given state is currently possible,
   * without actually applying it. Evaluates both the transition map and
   * all registered guards.
   */
  canTransition(to: WorkerLifecycleState): boolean {
    const allowed = VALID_TRANSITIONS.get(this._state);
    if (!allowed || !allowed.has(to)) {
      return false;
    }

    // All guards must pass
    for (const guard of this._guards) {
      if (!guard(this._state, to)) {
        return false;
      }
    }

    return true;
  }

  /** Returns a copy of the transition history. */
  getHistory(): StateTransition[] {
    return [...this._history];
  }

  /** Register a callback that fires after every successful transition. */
  onTransition(callback: TransitionCallback): void {
    this._listeners.push(callback);
  }

  /** Reset to Uninitialized, clearing all history (guards and listeners are preserved). */
  reset(): void {
    this._state = WorkerLifecycleState.Uninitialized;
    this._history = [];
  }
}
