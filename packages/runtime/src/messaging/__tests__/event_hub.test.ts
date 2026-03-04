// @townbuilder/runtime — Tests for EventHub
import { describe, it, expect, beforeEach } from "vitest";
import { EventHub } from "../event_hub.js";

describe("EventHub", () => {
  let hub: EventHub;

  beforeEach(() => {
    hub = new EventHub();
  });

  // ---- Test 1: subscribe receives published messages ----
  it("subscribe receives published messages", () => {
    const received: unknown[] = [];
    hub.subscribe("test", (data) => received.push(data));

    hub.publish("test", { value: 42 });
    hub.publish("test", "hello");

    expect(received).toHaveLength(2);
    expect(received[0]).toEqual({ value: 42 });
    expect(received[1]).toBe("hello");
  });

  // ---- Test 2: multiple subscribers for same type all fire ----
  it("multiple subscribers for same type all fire", () => {
    let count1 = 0;
    let count2 = 0;
    let count3 = 0;

    hub.subscribe("event", () => { count1++; });
    hub.subscribe("event", () => { count2++; });
    hub.subscribe("event", () => { count3++; });

    hub.publish("event", null);

    expect(count1).toBe(1);
    expect(count2).toBe(1);
    expect(count3).toBe(1);
  });

  // ---- Test 3: unsubscribe prevents further callbacks ----
  it("unsubscribe prevents further callbacks", () => {
    let callCount = 0;
    const unsub = hub.subscribe("tick", () => { callCount++; });

    hub.publish("tick", null);
    expect(callCount).toBe(1);

    unsub();

    hub.publish("tick", null);
    expect(callCount).toBe(1); // not called again
  });

  // ---- Test 4: subscribeOnce fires exactly once then auto-removes ----
  it("subscribeOnce fires exactly once then auto-removes", () => {
    let callCount = 0;
    hub.subscribeOnce("once-event", () => { callCount++; });

    hub.publish("once-event", "first");
    hub.publish("once-event", "second");
    hub.publish("once-event", "third");

    expect(callCount).toBe(1);
    expect(hub.hasSubscribers("once-event")).toBe(false);
  });

  // ---- Test 5: unsubscribeAll clears all for a type ----
  it("unsubscribeAll clears all subscriptions for a type", () => {
    let count1 = 0;
    let count2 = 0;

    hub.subscribe("alpha", () => { count1++; });
    hub.subscribe("alpha", () => { count2++; });
    hub.subscribe("beta", () => {});

    expect(hub.getSubscriptionCount("alpha")).toBe(2);

    hub.unsubscribeAll("alpha");

    expect(hub.getSubscriptionCount("alpha")).toBe(0);
    expect(hub.hasSubscribers("alpha")).toBe(false);
    // beta is untouched
    expect(hub.getSubscriptionCount("beta")).toBe(1);

    hub.publish("alpha", null);
    expect(count1).toBe(0);
    expect(count2).toBe(0);
  });

  // ---- Test 6: unsubscribeAll with no args clears everything ----
  it("unsubscribeAll with no args clears everything", () => {
    hub.subscribe("a", () => {});
    hub.subscribe("b", () => {});
    hub.subscribe("c", () => {});

    expect(hub.getSubscriptionCount()).toBe(3);

    hub.unsubscribeAll();

    expect(hub.getSubscriptionCount()).toBe(0);
    expect(hub.hasSubscribers("a")).toBe(false);
    expect(hub.hasSubscribers("b")).toBe(false);
    expect(hub.hasSubscribers("c")).toBe(false);
  });

  // ---- Test 7: publish with no subscribers does not crash ----
  it("publish with no subscribers does not crash", () => {
    expect(() => {
      hub.publish("nonexistent", { anything: true });
    }).not.toThrow();
  });

  // ---- Test 8: getSubscriptionCount accurate ----
  it("getSubscriptionCount returns accurate counts", () => {
    expect(hub.getSubscriptionCount()).toBe(0);
    expect(hub.getSubscriptionCount("foo")).toBe(0);

    hub.subscribe("foo", () => {});
    hub.subscribe("foo", () => {});
    hub.subscribe("bar", () => {});

    expect(hub.getSubscriptionCount("foo")).toBe(2);
    expect(hub.getSubscriptionCount("bar")).toBe(1);
    expect(hub.getSubscriptionCount()).toBe(3);
    expect(hub.getSubscriptionCount("unknown")).toBe(0);
  });

  // ---- Test 9: hasSubscribers correct ----
  it("hasSubscribers returns correct boolean", () => {
    expect(hub.hasSubscribers("test")).toBe(false);

    const unsub = hub.subscribe("test", () => {});

    expect(hub.hasSubscribers("test")).toBe(true);
    expect(hub.hasSubscribers("other")).toBe(false);

    unsub();

    expect(hub.hasSubscribers("test")).toBe(false);
  });

  // ---- Test 10: subscription ordering preserved (FIFO) ----
  it("subscription ordering preserved (FIFO)", () => {
    const order: number[] = [];

    hub.subscribe("ordered", () => order.push(1));
    hub.subscribe("ordered", () => order.push(2));
    hub.subscribe("ordered", () => order.push(3));

    hub.publish("ordered", null);

    expect(order).toEqual([1, 2, 3]);
  });

  // ---- Test 11: unsubscribe during publish is safe ----
  it("unsubscribe during publish is safe", () => {
    const order: string[] = [];
    let unsub2: () => void;

    hub.subscribe("safe", () => {
      order.push("first");
      // Unsubscribe the second listener mid-iteration
      unsub2();
    });

    unsub2 = hub.subscribe("safe", () => {
      order.push("second");
    });

    hub.subscribe("safe", () => {
      order.push("third");
    });

    hub.publish("safe", null);

    // All three fire because publish uses a snapshot
    expect(order).toEqual(["first", "second", "third"]);

    // But the second listener was removed, so next publish skips it
    order.length = 0;
    hub.publish("safe", null);

    expect(order).toEqual(["first", "third"]);
  });

  // ---- Test 12: different types do not interfere ----
  it("different types do not interfere", () => {
    const alphaData: unknown[] = [];
    const betaData: unknown[] = [];

    hub.subscribe("alpha", (d) => alphaData.push(d));
    hub.subscribe("beta", (d) => betaData.push(d));

    hub.publish("alpha", "a1");
    hub.publish("beta", "b1");
    hub.publish("alpha", "a2");

    expect(alphaData).toEqual(["a1", "a2"]);
    expect(betaData).toEqual(["b1"]);
  });

  // ---- Test 13: subscribeOnce can be manually unsubscribed before firing ----
  it("subscribeOnce can be manually unsubscribed before firing", () => {
    let fired = false;
    const unsub = hub.subscribeOnce("pre-unsub", () => { fired = true; });

    unsub();

    hub.publish("pre-unsub", null);
    expect(fired).toBe(false);
  });

  // ---- Test 14: multiple subscribeOnce on same type all fire once ----
  it("multiple subscribeOnce on same type all fire exactly once", () => {
    let count1 = 0;
    let count2 = 0;

    hub.subscribeOnce("multi-once", () => { count1++; });
    hub.subscribeOnce("multi-once", () => { count2++; });

    hub.publish("multi-once", null);
    hub.publish("multi-once", null);

    expect(count1).toBe(1);
    expect(count2).toBe(1);
    expect(hub.hasSubscribers("multi-once")).toBe(false);
  });
});
