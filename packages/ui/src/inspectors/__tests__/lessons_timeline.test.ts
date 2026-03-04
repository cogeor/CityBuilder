import { describe, it, expect, beforeEach } from 'vitest';
import {
  LessonType,
  LessonsTimeline,
} from '../lessons_timeline.js';

describe('LessonsTimeline', () => {
  let timeline: LessonsTimeline;

  beforeEach(() => {
    timeline = new LessonsTimeline();
  });

  // --- addEvent ---

  it('addEvent creates event with incremented id', () => {
    const e1 = timeline.addEvent(100, LessonType.Decision, 'Built park', 'Placed central park');
    const e2 = timeline.addEvent(200, LessonType.Milestone, 'Pop 1000', 'Population reached 1000');

    expect(e1.id).toBe(1);
    expect(e2.id).toBe(2);
    expect(e1.tick).toBe(100);
    expect(e1.type).toBe(LessonType.Decision);
    expect(e1.title).toBe('Built park');
    expect(e1.description).toBe('Placed central park');
    expect(e1.impact).toBeUndefined();
    expect(e1.impactTick).toBeUndefined();
  });

  // --- getEvents ---

  it('getEvents returns all events', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc a');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc b');
    timeline.addEvent(300, LessonType.Warning, 'C', 'desc c');

    const all = timeline.getEvents();
    expect(all.length).toBe(3);
  });

  it('getEvents filters by type', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    timeline.addEvent(300, LessonType.Decision, 'C', 'desc');
    timeline.addEvent(400, LessonType.Achievement, 'D', 'desc');

    const decisions = timeline.getEvents(LessonType.Decision);
    expect(decisions.length).toBe(2);
    for (const e of decisions) {
      expect(e.type).toBe(LessonType.Decision);
    }

    const achievements = timeline.getEvents(LessonType.Achievement);
    expect(achievements.length).toBe(1);
    expect(achievements[0].title).toBe('D');

    const warnings = timeline.getEvents(LessonType.Warning);
    expect(warnings.length).toBe(0);
  });

  // --- addImpact ---

  it('addImpact attaches impact to existing event', () => {
    const e = timeline.addEvent(100, LessonType.Decision, 'Tax hike', 'Raised taxes');
    const result = timeline.addImpact(e.id, 'Population declined 5%', 500);

    expect(result).toBe(true);
    const updated = timeline.getEventById(e.id);
    expect(updated).toBeDefined();
    expect(updated!.impact).toBe('Population declined 5%');
    expect(updated!.impactTick).toBe(500);
  });

  it('addImpact returns false for unknown id', () => {
    const result = timeline.addImpact(999, 'No effect', 100);
    expect(result).toBe(false);
  });

  // --- getRecentEvents ---

  it('getRecentEvents returns last N events', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    timeline.addEvent(300, LessonType.Warning, 'C', 'desc');
    timeline.addEvent(400, LessonType.Achievement, 'D', 'desc');

    const recent = timeline.getRecentEvents(2);
    expect(recent.length).toBe(2);
    expect(recent[0].title).toBe('C');
    expect(recent[1].title).toBe('D');
  });

  it('getRecentEvents returns all events if count exceeds total', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');

    const recent = timeline.getRecentEvents(10);
    expect(recent.length).toBe(2);
  });

  // --- getEventsInRange ---

  it('getEventsInRange filters by tick range', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    timeline.addEvent(300, LessonType.Warning, 'C', 'desc');
    timeline.addEvent(400, LessonType.Achievement, 'D', 'desc');

    const range = timeline.getEventsInRange(150, 350);
    expect(range.length).toBe(2);
    expect(range[0].title).toBe('B');
    expect(range[1].title).toBe('C');
  });

  it('getEventsInRange is inclusive of boundaries', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    timeline.addEvent(300, LessonType.Warning, 'C', 'desc');

    const range = timeline.getEventsInRange(100, 300);
    expect(range.length).toBe(3);
  });

  // --- maxEvents ---

  it('maxEvents enforced (oldest dropped)', () => {
    const small = new LessonsTimeline(3);

    small.addEvent(100, LessonType.Decision, 'A', 'desc');
    small.addEvent(200, LessonType.Milestone, 'B', 'desc');
    small.addEvent(300, LessonType.Warning, 'C', 'desc');
    small.addEvent(400, LessonType.Achievement, 'D', 'desc');

    expect(small.getEventCount()).toBe(3);
    const events = small.getEvents();
    // Oldest (A) should have been dropped
    expect(events[0].title).toBe('B');
    expect(events[1].title).toBe('C');
    expect(events[2].title).toBe('D');
  });

  // --- clear ---

  it('clear removes all events', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    expect(timeline.getEventCount()).toBe(2);

    timeline.clear();
    expect(timeline.getEventCount()).toBe(0);
    expect(timeline.getEvents().length).toBe(0);
  });

  it('clear resets id counter', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    timeline.clear();

    const e = timeline.addEvent(300, LessonType.Warning, 'C', 'desc');
    expect(e.id).toBe(1);
  });

  // --- getEventById ---

  it('getEventById returns correct event', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc a');
    const e2 = timeline.addEvent(200, LessonType.Milestone, 'B', 'desc b');
    timeline.addEvent(300, LessonType.Warning, 'C', 'desc c');

    const found = timeline.getEventById(e2.id);
    expect(found).toBeDefined();
    expect(found!.title).toBe('B');
    expect(found!.description).toBe('desc b');
  });

  it('getEventById returns undefined for unknown id', () => {
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    expect(timeline.getEventById(999)).toBeUndefined();
  });

  // --- getEventCount ---

  it('getEventCount returns correct count', () => {
    expect(timeline.getEventCount()).toBe(0);
    timeline.addEvent(100, LessonType.Decision, 'A', 'desc');
    expect(timeline.getEventCount()).toBe(1);
    timeline.addEvent(200, LessonType.Milestone, 'B', 'desc');
    expect(timeline.getEventCount()).toBe(2);
  });
});
