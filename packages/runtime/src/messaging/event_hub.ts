// @townbuilder/runtime — EventHub: Observer-pattern message subscription system
// Provides type-based publish/subscribe with one-shot support and safe iteration.

// ---- EventSubscription Interface ----

/** A single subscription registration. */
export interface EventSubscription {
  readonly id: number;
  readonly type: string;
  readonly callback: (data: unknown) => void;
  readonly once: boolean;
}

// ---- EventHub Class ----

/**
 * A lightweight publish/subscribe hub for decoupled message routing.
 *
 * Subscribers register for a specific event type string and receive
 * callbacks when that type is published.  Supports one-shot listeners
 * that auto-remove after their first invocation.
 *
 * Unsubscribing during a publish is safe -- the hub iterates a snapshot
 * of the subscription list so mutations do not corrupt the iteration.
 */
export class EventHub {
  private _subscriptions: Map<string, EventSubscription[]>;
  private _nextId: number;

  constructor() {
    this._subscriptions = new Map();
    this._nextId = 1;
  }

  /**
   * Subscribe to events of the given type.
   *
   * @param type     Event type discriminator string.
   * @param callback Function invoked with the event data.
   * @returns An unsubscribe function -- call it to remove this subscription.
   */
  subscribe(type: string, callback: (data: unknown) => void): () => void {
    const id = this._nextId++;
    const sub: EventSubscription = { id, type, callback, once: false };

    let list = this._subscriptions.get(type);
    if (!list) {
      list = [];
      this._subscriptions.set(type, list);
    }
    list.push(sub);

    return () => this._removeById(type, id);
  }

  /**
   * Subscribe to a single event of the given type.
   * The subscription is automatically removed after the first invocation.
   *
   * @param type     Event type discriminator string.
   * @param callback Function invoked with the event data.
   * @returns An unsubscribe function (can be called before the event fires).
   */
  subscribeOnce(type: string, callback: (data: unknown) => void): () => void {
    const id = this._nextId++;
    const sub: EventSubscription = { id, type, callback, once: true };

    let list = this._subscriptions.get(type);
    if (!list) {
      list = [];
      this._subscriptions.set(type, list);
    }
    list.push(sub);

    return () => this._removeById(type, id);
  }

  /**
   * Publish an event, invoking all subscribers registered for the given type.
   *
   * Callbacks fire in FIFO registration order.  One-shot listeners are removed
   * after firing.  Iteration is safe even if a callback unsubscribes itself or
   * other listeners.
   *
   * @param type Event type discriminator string.
   * @param data Arbitrary payload forwarded to each callback.
   */
  publish(type: string, data: unknown): void {
    const list = this._subscriptions.get(type);
    if (!list || list.length === 0) {
      return;
    }

    // Snapshot the current list so mutations during iteration are safe.
    const snapshot = [...list];
    const toRemove: number[] = [];

    for (const sub of snapshot) {
      sub.callback(data);
      if (sub.once) {
        toRemove.push(sub.id);
      }
    }

    // Remove one-shot subscriptions after all callbacks have fired.
    if (toRemove.length > 0) {
      const removeSet = new Set(toRemove);
      const remaining = list.filter((s) => !removeSet.has(s.id));
      if (remaining.length === 0) {
        this._subscriptions.delete(type);
      } else {
        this._subscriptions.set(type, remaining);
      }
    }
  }

  /**
   * Remove all subscriptions.
   *
   * @param type If provided, only subscriptions for that type are cleared.
   *             If omitted, every subscription across all types is removed.
   */
  unsubscribeAll(type?: string): void {
    if (type !== undefined) {
      this._subscriptions.delete(type);
    } else {
      this._subscriptions.clear();
    }
  }

  /**
   * Count active subscriptions.
   *
   * @param type If provided, count only subscriptions for that type.
   *             If omitted, count all subscriptions across every type.
   */
  getSubscriptionCount(type?: string): number {
    if (type !== undefined) {
      const list = this._subscriptions.get(type);
      return list ? list.length : 0;
    }

    let total = 0;
    for (const list of this._subscriptions.values()) {
      total += list.length;
    }
    return total;
  }

  /**
   * Check whether any subscribers exist for the given type.
   *
   * @param type Event type discriminator string.
   */
  hasSubscribers(type: string): boolean {
    const list = this._subscriptions.get(type);
    return list !== undefined && list.length > 0;
  }

  // ---- Private helpers ----

  /** Remove a single subscription by its unique id. */
  private _removeById(type: string, id: number): void {
    const list = this._subscriptions.get(type);
    if (!list) {
      return;
    }

    const idx = list.findIndex((s) => s.id === id);
    if (idx !== -1) {
      list.splice(idx, 1);
      if (list.length === 0) {
        this._subscriptions.delete(type);
      }
    }
  }
}
