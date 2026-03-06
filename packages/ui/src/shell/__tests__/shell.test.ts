import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  HudShell,
  SimSpeed,
  ToolType,
  DEFAULT_CITY_STATS,
  type ShellEventType,
} from '../shell.js';

describe('HudShell', () => {
  let shell: HudShell;

  beforeEach(() => {
    shell = new HudShell();
  });

  // --- Constructor ---

  it('creates with default stats', () => {
    const stats = shell.getStats();
    expect(stats.cityName).toBe('New City');
    expect(stats.population).toBe(0);
    expect(stats.treasury).toBe(100000);
    expect(stats.date).toBe('Day 1');
    expect(stats.time).toBe('08:00');
  });

  it('merges initial stats over defaults', () => {
    const custom = new HudShell({ cityName: 'Metropolis', population: 5000 });
    const stats = custom.getStats();
    expect(stats.cityName).toBe('Metropolis');
    expect(stats.population).toBe(5000);
    // defaults preserved for unset fields
    expect(stats.treasury).toBe(100000);
    expect(stats.date).toBe('Day 1');
  });

  // --- Stats ---

  it('getStats returns a copy (not the internal object)', () => {
    const a = shell.getStats();
    const b = shell.getStats();
    expect(a).toEqual(b);
    a.population = 999;
    expect(shell.getStats().population).toBe(0);
  });

  it('updateStats updates specific fields', () => {
    shell.updateStats({ population: 1234, cityName: 'TestVille' });
    const stats = shell.getStats();
    expect(stats.population).toBe(1234);
    expect(stats.cityName).toBe('TestVille');
    // unchanged
    expect(stats.treasury).toBe(100000);
  });

  // --- formatTreasury ---

  it('formatTreasury formats small dollar amounts', () => {
    shell.updateStats({ treasury: 50000 }); // $500
    expect(shell.formatTreasury()).toBe('$500');
  });

  it('formatTreasury formats thousands with K suffix', () => {
    shell.updateStats({ treasury: 500000 }); // $5,000
    expect(shell.formatTreasury()).toBe('$5.0K');
  });

  it('formatTreasury formats millions with M suffix', () => {
    shell.updateStats({ treasury: 250000000 }); // $2,500,000
    expect(shell.formatTreasury()).toBe('$2.5M');
  });

  it('formatTreasury handles negative values', () => {
    shell.updateStats({ treasury: -500000 }); // -$5,000
    expect(shell.formatTreasury()).toBe('$-5.0K');
  });

  // --- formatPopulation ---

  it('formatPopulation formats small numbers', () => {
    shell.updateStats({ population: 42 });
    expect(shell.formatPopulation()).toBe('42');
  });

  it('formatPopulation formats thousands', () => {
    shell.updateStats({ population: 12500 });
    expect(shell.formatPopulation()).toBe('12.5K');
  });

  it('formatPopulation formats millions', () => {
    shell.updateStats({ population: 1500000 });
    expect(shell.formatPopulation()).toBe('1.5M');
  });

  // --- Speed Controls ---

  it('getSpeed starts at Normal', () => {
    expect(shell.getSpeed()).toBe(SimSpeed.Normal);
  });

  it('setSpeed changes speed and emits event', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.setSpeed(SimSpeed.Fast);
    expect(shell.getSpeed()).toBe(SimSpeed.Fast);
    expect(handler).toHaveBeenCalledWith('speedChange', { speed: SimSpeed.Fast });
  });

  it('togglePause pauses when running', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.togglePause();
    expect(shell.getSpeed()).toBe(SimSpeed.Paused);
    expect(handler).toHaveBeenCalledWith('speedChange', { speed: SimSpeed.Paused });
  });

  it('togglePause resumes to Normal when paused', () => {
    shell.setSpeed(SimSpeed.Paused);
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.togglePause();
    expect(shell.getSpeed()).toBe(SimSpeed.Normal);
    expect(handler).toHaveBeenCalledWith('speedChange', { speed: SimSpeed.Normal });
  });

  // --- Tool Selection ---

  it('getActiveTool starts at Select', () => {
    expect(shell.getActiveTool()).toBe(ToolType.Select);
  });

  it('setActiveTool changes tool and emits event', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.setActiveTool(ToolType.Bulldoze);
    expect(shell.getActiveTool()).toBe(ToolType.Bulldoze);
    expect(handler).toHaveBeenCalledWith('toolChange', { tool: ToolType.Bulldoze });
  });

  // --- Panel Management ---

  it('registerPanel creates a new panel', () => {
    shell.registerPanel('budget', 'Budget');
    const panel = shell.getPanel('budget');
    expect(panel).toBeDefined();
    expect(panel!.title).toBe('Budget');
    expect(panel!.visible).toBe(false);
    expect(panel!.pinned).toBe(false);
  });

  it('showPanel makes a panel visible and emits event', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.registerPanel('budget', 'Budget');
    shell.showPanel('budget');
    expect(shell.getPanel('budget')!.visible).toBe(true);
    expect(handler).toHaveBeenCalledWith('panelToggle', { id: 'budget', visible: true });
  });

  it('hidePanel hides a non-pinned panel', () => {
    shell.registerPanel('budget', 'Budget');
    shell.showPanel('budget');
    shell.hidePanel('budget');
    expect(shell.getPanel('budget')!.visible).toBe(false);
  });

  it('hidePanel respects pinned state', () => {
    shell.registerPanel('budget', 'Budget');
    shell.showPanel('budget');
    shell.pinPanel('budget');
    shell.hidePanel('budget');
    // still visible because it is pinned
    expect(shell.getPanel('budget')!.visible).toBe(true);
  });

  it('togglePanel toggles visibility on', () => {
    shell.registerPanel('budget', 'Budget');
    shell.togglePanel('budget');
    expect(shell.getPanel('budget')!.visible).toBe(true);
  });

  it('togglePanel toggles visibility off', () => {
    shell.registerPanel('budget', 'Budget');
    shell.showPanel('budget');
    shell.togglePanel('budget');
    expect(shell.getPanel('budget')!.visible).toBe(false);
  });

  it('unpinPanel allows hiding again', () => {
    shell.registerPanel('budget', 'Budget');
    shell.showPanel('budget');
    shell.pinPanel('budget');
    shell.unpinPanel('budget');
    shell.hidePanel('budget');
    expect(shell.getPanel('budget')!.visible).toBe(false);
  });

  it('getPanel returns undefined for unknown id', () => {
    expect(shell.getPanel('nonexistent')).toBeUndefined();
  });

  it('getPanel returns a copy', () => {
    shell.registerPanel('budget', 'Budget');
    const p1 = shell.getPanel('budget')!;
    p1.visible = true;
    expect(shell.getPanel('budget')!.visible).toBe(false);
  });

  it('getVisiblePanels returns only visible panels', () => {
    shell.registerPanel('budget', 'Budget');
    shell.registerPanel('stats', 'Statistics');
    shell.showPanel('budget');
    const visible = shell.getVisiblePanels();
    expect(visible).toHaveLength(1);
    expect(visible[0].id).toBe('budget');
  });

  // --- Notifications ---

  it('addNotification adds and returns an id', () => {
    const id = shell.addNotification('Hello world');
    expect(id).toBe(1);
    const notifs = shell.getNotifications();
    expect(notifs).toHaveLength(1);
    expect(notifs[0].message).toBe('Hello world');
    expect(notifs[0].type).toBe('info');
  });

  it('addNotification supports custom types', () => {
    shell.addNotification('Fire!', 'error');
    const notifs = shell.getNotifications();
    expect(notifs[0].type).toBe('error');
  });

  it('dismissNotification marks as dismissed and emits event', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    const id = shell.addNotification('Test');
    shell.dismissNotification(id);
    expect(shell.getNotifications()).toHaveLength(0);
    expect(handler).toHaveBeenCalledWith('notificationDismiss', { id });
  });

  it('getNotifications filters dismissed notifications', () => {
    const id1 = shell.addNotification('A');
    shell.addNotification('B');
    shell.dismissNotification(id1);
    const notifs = shell.getNotifications();
    expect(notifs).toHaveLength(1);
    expect(notifs[0].message).toBe('B');
  });

  it('clearNotifications removes all', () => {
    shell.addNotification('A');
    shell.addNotification('B');
    shell.clearNotifications();
    expect(shell.getNotifications()).toHaveLength(0);
  });

  it('notification limit is enforced', () => {
    for (let i = 0; i < 15; i++) {
      shell.addNotification(`msg-${i}`);
    }
    // maxNotifications is 10, so oldest are trimmed
    const notifs = shell.getNotifications();
    expect(notifs.length).toBeLessThanOrEqual(10);
    // first message should be msg-5 (0-4 were trimmed)
    expect(notifs[0].message).toBe('msg-5');
  });

  it('getNotifications returns copies', () => {
    shell.addNotification('A');
    const notifs = shell.getNotifications();
    notifs[0].message = 'MUTATED';
    expect(shell.getNotifications()[0].message).toBe('A');
  });

  // --- Events ---

  it('addEventListener receives events', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.setSpeed(SimSpeed.VeryFast);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('speedChange', { speed: SimSpeed.VeryFast });
  });

  it('removeEventListener stops receiving events', () => {
    const handler = vi.fn();
    shell.addEventListener(handler);
    shell.removeEventListener(handler);
    shell.setSpeed(SimSpeed.Fast);
    expect(handler).not.toHaveBeenCalled();
  });

  it('multiple event handlers all receive events', () => {
    const h1 = vi.fn();
    const h2 = vi.fn();
    shell.addEventListener(h1);
    shell.addEventListener(h2);
    shell.setActiveTool(ToolType.Road);
    expect(h1).toHaveBeenCalledOnce();
    expect(h2).toHaveBeenCalledOnce();
  });

  // --- Enum Values ---

  it('SimSpeed enum has correct values', () => {
    expect(SimSpeed.Paused).toBe(0);
    expect(SimSpeed.Normal).toBe(1);
    expect(SimSpeed.Fast).toBe(2);
    expect(SimSpeed.VeryFast).toBe(3);
  });

  it('ToolType enum has correct string values', () => {
    expect(ToolType.Select).toBe('select');
    expect(ToolType.Place).toBe('place');
    expect(ToolType.Zone).toBe('zone');
    expect(ToolType.Bulldoze).toBe('bulldoze');
    expect(ToolType.Road).toBe('road');
    expect(ToolType.Terrain).toBe('terrain');
  });

  it('DEFAULT_CITY_STATS is frozen reference', () => {
    expect(DEFAULT_CITY_STATS.cityName).toBe('New City');
    expect(DEFAULT_CITY_STATS.population).toBe(0);
    expect(DEFAULT_CITY_STATS.treasury).toBe(100000);
  });
});
