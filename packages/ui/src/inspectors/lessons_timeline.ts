/** Types of lessons / events recorded on the city timeline */
export enum LessonType {
  Decision = "decision",
  Milestone = "milestone",
  Warning = "warning",
  Achievement = "achievement",
}

/** A single event on the city lessons timeline */
export interface TimelineEvent {
  id: number;
  tick: number;
  type: LessonType;
  title: string;
  description: string;
  impact?: string;        // delayed impact attribution
  impactTick?: number;    // when the impact was realized
}

/**
 * LessonsTimeline -- records and queries a chronological list of
 * city events such as decisions, milestones, warnings, and achievements.
 *
 * When the event cap is reached, the oldest events are dropped.
 */
export class LessonsTimeline {
  private events: TimelineEvent[];
  private nextId: number;
  private maxEvents: number;

  constructor(maxEvents: number = 200) {
    this.events = [];
    this.nextId = 1;
    this.maxEvents = maxEvents;
  }

  /** Add a new event and return it. Drops oldest if over capacity. */
  addEvent(tick: number, type: LessonType, title: string, description: string): TimelineEvent {
    const event: TimelineEvent = {
      id: this.nextId++,
      tick,
      type,
      title,
      description,
    };
    this.events.push(event);

    // Enforce capacity by dropping the oldest events
    while (this.events.length > this.maxEvents) {
      this.events.shift();
    }

    return event;
  }

  /** Attach a delayed impact description to an existing event. Returns false if id not found. */
  addImpact(eventId: number, impact: string, impactTick: number): boolean {
    const event = this.events.find(e => e.id === eventId);
    if (!event) {
      return false;
    }
    event.impact = impact;
    event.impactTick = impactTick;
    return true;
  }

  /** Get all events, optionally filtered by type */
  getEvents(type?: LessonType): TimelineEvent[] {
    if (type === undefined) {
      return [...this.events];
    }
    return this.events.filter(e => e.type === type);
  }

  /** Get the most recent N events (ordered oldest to newest) */
  getRecentEvents(count: number): TimelineEvent[] {
    return this.events.slice(-count);
  }

  /** Find an event by its ID */
  getEventById(id: number): TimelineEvent | undefined {
    return this.events.find(e => e.id === id);
  }

  /** Get all events within a tick range (inclusive) */
  getEventsInRange(startTick: number, endTick: number): TimelineEvent[] {
    return this.events.filter(e => e.tick >= startTick && e.tick <= endTick);
  }

  /** Total number of events stored */
  getEventCount(): number {
    return this.events.length;
  }

  /** Remove all events and reset the ID counter */
  clear(): void {
    this.events = [];
    this.nextId = 1;
  }
}
