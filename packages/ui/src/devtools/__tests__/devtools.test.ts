import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  DevTools,
  DevPanel,
  type PerformanceMetrics,
  type EntityDebugInfo,
  type CacheStats,
  type PhaseWheelStatus,
  type DevToolsEventType,
} from '../devtools.js';

describe('DevTools', () => {
  let devtools: DevTools;

  beforeEach(() => {
    devtools = new DevTools();
  });

  // --- Constructor ---

  it('1. constructor starts hidden', () => {
    expect(devtools.isVisible()).toBe(false);
  });

  // --- Visibility ---

  it('2. toggle shows when hidden', () => {
    devtools.toggle();
    expect(devtools.isVisible()).toBe(true);
  });

  it('2b. toggle hides when visible', () => {
    devtools.toggle(); // show
    devtools.toggle(); // hide
    expect(devtools.isVisible()).toBe(false);
  });

  it('3. show explicitly sets visibility to true', () => {
    devtools.show();
    expect(devtools.isVisible()).toBe(true);
  });

  it('3b. hide explicitly sets visibility to false', () => {
    devtools.show();
    devtools.hide();
    expect(devtools.isVisible()).toBe(false);
  });

  // --- Panel ---

  it('4. getActivePanel starts at Performance', () => {
    expect(devtools.getActivePanel()).toBe(DevPanel.Performance);
  });

  it('5. setPanel changes the active panel', () => {
    devtools.setPanel(DevPanel.Console);
    expect(devtools.getActivePanel()).toBe(DevPanel.Console);
  });

  // --- Performance ---

  it('6. updatePerformance updates metrics', () => {
    devtools.updatePerformance({ fps: 60, frameTimeMs: 16.6 });
    const perf = devtools.getPerformance();
    expect(perf.fps).toBe(60);
    expect(perf.frameTimeMs).toBe(16.6);
  });

  it('7. getPerformance returns a copy', () => {
    devtools.updatePerformance({ fps: 60 });
    const a = devtools.getPerformance();
    const b = devtools.getPerformance();
    expect(a).toEqual(b);
    a.fps = 999;
    expect(devtools.getPerformance().fps).toBe(60);
  });

  it('8. getAverageFps computes average of history', () => {
    devtools.updatePerformance({ fps: 60 });
    devtools.updatePerformance({ fps: 30 });
    devtools.updatePerformance({ fps: 90 });
    expect(devtools.getAverageFps()).toBe(60); // (60+30+90)/3 = 60
  });

  it('8b. getAverageFps returns 0 when no history', () => {
    expect(devtools.getAverageFps()).toBe(0);
  });

  it('9. getMinFps returns minimum FPS', () => {
    devtools.updatePerformance({ fps: 60 });
    devtools.updatePerformance({ fps: 30 });
    devtools.updatePerformance({ fps: 90 });
    expect(devtools.getMinFps()).toBe(30);
  });

  it('9b. getMinFps returns 0 when no history', () => {
    expect(devtools.getMinFps()).toBe(0);
  });

  it('10. getMaxFps returns maximum FPS', () => {
    devtools.updatePerformance({ fps: 60 });
    devtools.updatePerformance({ fps: 30 });
    devtools.updatePerformance({ fps: 90 });
    expect(devtools.getMaxFps()).toBe(90);
  });

  it('10b. getMaxFps returns 0 when no history', () => {
    expect(devtools.getMaxFps()).toBe(0);
  });

  it('11. getFpsHistory returns a copy', () => {
    devtools.updatePerformance({ fps: 60 });
    devtools.updatePerformance({ fps: 30 });
    const history = devtools.getFpsHistory();
    expect(history).toEqual([60, 30]);
    history.push(999);
    expect(devtools.getFpsHistory()).toEqual([60, 30]);
  });

  it('12. FPS history trims at max entries', () => {
    // maxFpsHistory is 120
    for (let i = 0; i < 130; i++) {
      devtools.updatePerformance({ fps: i });
    }
    const history = devtools.getFpsHistory();
    expect(history.length).toBe(120);
    // first entry should be 10 (0-9 were trimmed)
    expect(history[0]).toBe(10);
    expect(history[119]).toBe(129);
  });

  // --- Cache ---

  it('13. updateCacheStats updates stats', () => {
    devtools.updateCacheStats({ dirtyChunks: 5, totalChunks: 100 });
    const stats = devtools.getCacheStats();
    expect(stats.dirtyChunks).toBe(5);
    expect(stats.totalChunks).toBe(100);
  });

  it('14. getCacheStats returns a copy', () => {
    devtools.updateCacheStats({ dirtyChunks: 5 });
    const a = devtools.getCacheStats();
    const b = devtools.getCacheStats();
    expect(a).toEqual(b);
    a.dirtyChunks = 999;
    expect(devtools.getCacheStats().dirtyChunks).toBe(5);
  });

  // --- Phase Wheel ---

  it('15. updatePhaseStatus updates status', () => {
    devtools.updatePhaseStatus({ currentPhase: 'Population', currentTick: 42 });
    const status = devtools.getPhaseStatus();
    expect(status.currentPhase).toBe('Population');
    expect(status.currentTick).toBe(42);
  });

  it('16. getPhaseStatus returns a copy', () => {
    devtools.updatePhaseStatus({ currentPhase: 'Power' });
    const a = devtools.getPhaseStatus();
    const b = devtools.getPhaseStatus();
    expect(a).toEqual(b);
    a.currentPhase = 'MUTATED';
    expect(devtools.getPhaseStatus().currentPhase).toBe('Power');
  });

  // --- Entity Inspector ---

  it('17. inspectEntity stores entity debug info', () => {
    const entity: EntityDebugInfo = {
      id: 1, archetype: 10, archetypeName: 'House',
      tileX: 5, tileY: 8, flags: 0x05,
      flagNames: ['POWERED', 'STAFFED'], level: 2,
      constructionProgress: 100,
    };
    devtools.inspectEntity(entity);
    const result = devtools.getInspectedEntity();
    expect(result).toBeDefined();
    expect(result!.id).toBe(1);
    expect(result!.archetypeName).toBe('House');
  });

  it('18. getInspectedEntity returns a copy', () => {
    const entity: EntityDebugInfo = {
      id: 1, archetype: 10, archetypeName: 'House',
      tileX: 5, tileY: 8, flags: 0x05,
      flagNames: ['POWERED', 'STAFFED'], level: 2,
      constructionProgress: 100,
    };
    devtools.inspectEntity(entity);
    const a = devtools.getInspectedEntity()!;
    a.id = 999;
    expect(devtools.getInspectedEntity()!.id).toBe(1);
  });

  it('18b. getInspectedEntity returns null when nothing inspected', () => {
    expect(devtools.getInspectedEntity()).toBeNull();
  });

  it('19. clearEntityInspection clears the inspected entity', () => {
    const entity: EntityDebugInfo = {
      id: 1, archetype: 10, archetypeName: 'House',
      tileX: 5, tileY: 8, flags: 0x05,
      flagNames: ['POWERED', 'STAFFED'], level: 2,
      constructionProgress: 100,
    };
    devtools.inspectEntity(entity);
    devtools.clearEntityInspection();
    expect(devtools.getInspectedEntity()).toBeNull();
  });

  // --- Tile Inspector ---

  it('20. inspectTile stores tile data', () => {
    devtools.inspectTile(10, 20, { terrain: 'grass', elevation: 5 });
    const tile = devtools.getInspectedTile();
    expect(tile).toBeDefined();
    expect(tile!.x).toBe(10);
    expect(tile!.y).toBe(20);
    expect(tile!.data.terrain).toBe('grass');
  });

  it('21. getInspectedTile returns a copy', () => {
    devtools.inspectTile(10, 20, { terrain: 'grass' });
    const a = devtools.getInspectedTile()!;
    a.x = 999;
    a.data.terrain = 'MUTATED';
    const b = devtools.getInspectedTile()!;
    expect(b.x).toBe(10);
    expect(b.data.terrain).toBe('grass');
  });

  it('21b. getInspectedTile returns null when nothing inspected', () => {
    expect(devtools.getInspectedTile()).toBeNull();
  });

  it('22. clearTileInspection clears the inspected tile', () => {
    devtools.inspectTile(10, 20, { terrain: 'grass' });
    devtools.clearTileInspection();
    expect(devtools.getInspectedTile()).toBeNull();
  });

  // --- Console ---

  it('23. log adds a console entry', () => {
    devtools.log('info', 'Hello world', 'test');
    const log = devtools.getConsoleLog();
    expect(log.length).toBe(1);
    expect(log[0].level).toBe('info');
    expect(log[0].message).toBe('Hello world');
    expect(log[0].source).toBe('test');
    expect(log[0].timestamp).toBeGreaterThan(0);
  });

  it('23b. log defaults source to system', () => {
    devtools.log('debug', 'test message');
    const log = devtools.getConsoleLog();
    expect(log[0].source).toBe('system');
  });

  it('24. getConsoleLog returns all entries', () => {
    devtools.log('info', 'msg1');
    devtools.log('warn', 'msg2');
    devtools.log('error', 'msg3');
    const log = devtools.getConsoleLog();
    expect(log.length).toBe(3);
  });

  it('25. getConsoleLog filters by level', () => {
    devtools.log('info', 'info message');
    devtools.log('warn', 'warn message');
    devtools.log('error', 'error message');
    devtools.log('info', 'another info');
    const infoOnly = devtools.getConsoleLog('info');
    expect(infoOnly.length).toBe(2);
    expect(infoOnly.every(e => e.level === 'info')).toBe(true);
  });

  it('26. clearConsole removes all entries', () => {
    devtools.log('info', 'msg1');
    devtools.log('warn', 'msg2');
    devtools.clearConsole();
    expect(devtools.getConsoleLog().length).toBe(0);
  });

  it('27. console trims at max entries', () => {
    // maxConsoleEntries is 200
    for (let i = 0; i < 210; i++) {
      devtools.log('info', `msg-${i}`);
    }
    const log = devtools.getConsoleLog();
    expect(log.length).toBe(200);
    // first entry should be msg-10 (0-9 were trimmed)
    expect(log[0].message).toBe('msg-10');
  });

  it('28. executeCommand logs and emits command event', () => {
    const handler = vi.fn();
    devtools.addEventListener(handler);
    const result = devtools.executeCommand('spawn entity 42');
    expect(result).toBe('Executed: spawn entity 42');
    // check it was logged
    const log = devtools.getConsoleLog();
    expect(log.length).toBe(1);
    expect(log[0].message).toBe('> spawn entity 42');
    expect(log[0].source).toBe('console');
    // check event was emitted
    expect(handler).toHaveBeenCalledWith('command', { command: 'spawn entity 42' });
  });

  // --- Display Helpers ---

  it('29. formatFps formats correctly', () => {
    expect(devtools.formatFps(60)).toBe('60 FPS');
    expect(devtools.formatFps(0)).toBe('0 FPS');
    expect(devtools.formatFps(144)).toBe('144 FPS');
  });

  it('30. formatMemory formats correctly', () => {
    expect(devtools.formatMemory(128.456)).toBe('128.5 MB');
    expect(devtools.formatMemory(0)).toBe('0.0 MB');
    expect(devtools.formatMemory(1024.1)).toBe('1024.1 MB');
  });

  it('31. formatTime formats ms and us', () => {
    expect(devtools.formatTime(16.6)).toBe('16.6ms');
    expect(devtools.formatTime(0.5)).toBe('500us');
    expect(devtools.formatTime(1.0)).toBe('1.0ms');
    expect(devtools.formatTime(0.001)).toBe('1us');
  });

  it('32. formatFlags decodes bitflags', () => {
    expect(devtools.formatFlags(0x01)).toBe('POWERED');
    expect(devtools.formatFlags(0x02)).toBe('HAS_WATER');
    expect(devtools.formatFlags(0x03)).toBe('POWERED | HAS_WATER');
    expect(devtools.formatFlags(0x05)).toBe('POWERED | STAFFED');
    expect(devtools.formatFlags(0x08)).toBe('UNDER_CONSTRUCTION');
    expect(devtools.formatFlags(0x10)).toBe('ON_FIRE');
    expect(devtools.formatFlags(0x20)).toBe('DAMAGED');
    expect(devtools.formatFlags(0x3F)).toBe('POWERED | HAS_WATER | STAFFED | UNDER_CONSTRUCTION | ON_FIRE | DAMAGED');
    expect(devtools.formatFlags(0x00)).toBe('NONE');
  });

  // --- Events ---

  it('33. addEventListener receives events', () => {
    const handler = vi.fn();
    devtools.addEventListener(handler);
    devtools.toggle();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('toggle', { visible: true });
  });

  it('33b. addEventListener receives panel change events', () => {
    const handler = vi.fn();
    devtools.addEventListener(handler);
    devtools.setPanel(DevPanel.Entity);
    expect(handler).toHaveBeenCalledWith('panelChanged', { panel: DevPanel.Entity });
  });

  it('34. removeEventListener stops receiving events', () => {
    const handler = vi.fn();
    devtools.addEventListener(handler);
    devtools.removeEventListener(handler);
    devtools.toggle();
    expect(handler).not.toHaveBeenCalled();
  });

  it('34b. multiple handlers all receive events', () => {
    const h1 = vi.fn();
    const h2 = vi.fn();
    devtools.addEventListener(h1);
    devtools.addEventListener(h2);
    devtools.show();
    expect(h1).toHaveBeenCalledOnce();
    expect(h2).toHaveBeenCalledOnce();
  });

  // --- DevPanel enum ---

  it('DevPanel enum has correct values', () => {
    expect(DevPanel.Performance).toBe('performance');
    expect(DevPanel.Entity).toBe('entity');
    expect(DevPanel.Tile).toBe('tile');
    expect(DevPanel.PhaseWheel).toBe('phase_wheel');
    expect(DevPanel.Cache).toBe('cache');
    expect(DevPanel.Console).toBe('console');
  });

  // --- Edge cases ---

  it('updatePerformance partial update preserves other fields', () => {
    devtools.updatePerformance({ fps: 60, entityCount: 500 });
    devtools.updatePerformance({ fps: 55 });
    const perf = devtools.getPerformance();
    expect(perf.fps).toBe(55);
    expect(perf.entityCount).toBe(500);
  });

  it('getConsoleLog returns copies of entries', () => {
    devtools.log('info', 'original');
    const log = devtools.getConsoleLog();
    log[0].message = 'MUTATED';
    expect(devtools.getConsoleLog()[0].message).toBe('original');
  });
});
