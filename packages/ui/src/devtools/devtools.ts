import { TypedEventHub, type EventListener } from "../shared/typed_events.js";

/** Performance metrics */
export interface PerformanceMetrics {
  fps: number;
  frameTimeMs: number;
  tickTimeMs: number;
  simTickTimeMs: number;
  memoryUsageMb: number;
  entityCount: number;
  tileCount: number;
}

/** Entity debug info */
export interface EntityDebugInfo {
  id: number;
  archetype: number;
  archetypeName: string;
  tileX: number;
  tileY: number;
  flags: number;
  flagNames: string[];
  level: number;
  constructionProgress: number;
}

/** Cache stats */
export interface CacheStats {
  dirtyChunks: number;
  totalChunks: number;
  rebuiltThisFrame: number;
  distanceFieldUpdates: number;
  analysisMapsVersion: number;
}

/** Phase wheel status */
export interface PhaseWheelStatus {
  currentPhase: string;
  currentTick: number;
  scanFraction: number;
  adaptedDenominator: number;
}

/** Dev tools panel types */
export enum DevPanel {
  Performance = 'performance',
  Entity = 'entity',
  Tile = 'tile',
  PhaseWheel = 'phase_wheel',
  Cache = 'cache',
  Console = 'console',
}

/** Dev tools event types */
export type DevToolsEventType = 'toggle' | 'panelChanged' | 'command';
export interface DevToolsEventPayloads {
  toggle: { visible: boolean };
  panelChanged: { panel: DevPanel };
  command: { command: string };
}
export type DevToolsEventHandler = EventListener<DevToolsEventPayloads>;

/** Console log entry */
export interface ConsoleEntry {
  timestamp: number;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  source: string;
}

/**
 * DevTools — debug overlay with performance monitoring and entity inspection.
 * Toggle with F12 or debug key.
 */
export class DevTools {
  private visible: boolean;
  private activePanel: DevPanel;
  private perfMetrics: PerformanceMetrics;
  private cacheStats: CacheStats;
  private phaseStatus: PhaseWheelStatus;
  private inspectedEntity: EntityDebugInfo | null;
  private inspectedTile: { x: number; y: number; data: Record<string, unknown> } | null;
  private consoleLog: ConsoleEntry[];
  private maxConsoleEntries: number;
  private readonly events: TypedEventHub<DevToolsEventPayloads>;
  private fpsHistory: number[];
  private maxFpsHistory: number;

  constructor() {
    this.visible = false;
    this.activePanel = DevPanel.Performance;
    this.perfMetrics = {
      fps: 0, frameTimeMs: 0, tickTimeMs: 0, simTickTimeMs: 0,
      memoryUsageMb: 0, entityCount: 0, tileCount: 0,
    };
    this.cacheStats = {
      dirtyChunks: 0, totalChunks: 0, rebuiltThisFrame: 0,
      distanceFieldUpdates: 0, analysisMapsVersion: 0,
    };
    this.phaseStatus = {
      currentPhase: 'Transport', currentTick: 0,
      scanFraction: 8, adaptedDenominator: 8,
    };
    this.inspectedEntity = null;
    this.inspectedTile = null;
    this.consoleLog = [];
    this.maxConsoleEntries = 200;
    this.events = new TypedEventHub<DevToolsEventPayloads>();
    this.fpsHistory = [];
    this.maxFpsHistory = 120; // 2 seconds at 60fps
  }

  // --- Visibility ---
  isVisible(): boolean { return this.visible; }

  toggle(): void {
    this.visible = !this.visible;
    this.emit('toggle', { visible: this.visible });
  }

  show(): void {
    this.visible = true;
    this.emit('toggle', { visible: true });
  }

  hide(): void {
    this.visible = false;
    this.emit('toggle', { visible: false });
  }

  // --- Panel ---
  getActivePanel(): DevPanel { return this.activePanel; }

  setPanel(panel: DevPanel): void {
    this.activePanel = panel;
    this.emit('panelChanged', { panel });
  }

  // --- Performance ---
  updatePerformance(metrics: Partial<PerformanceMetrics>): void {
    Object.assign(this.perfMetrics, metrics);
    if (metrics.fps !== undefined) {
      this.fpsHistory.push(metrics.fps);
      while (this.fpsHistory.length > this.maxFpsHistory) {
        this.fpsHistory.shift();
      }
    }
  }

  getPerformance(): PerformanceMetrics { return { ...this.perfMetrics }; }

  getAverageFps(): number {
    if (this.fpsHistory.length === 0) return 0;
    const sum = this.fpsHistory.reduce((a, b) => a + b, 0);
    return Math.round(sum / this.fpsHistory.length);
  }

  getMinFps(): number {
    if (this.fpsHistory.length === 0) return 0;
    return Math.min(...this.fpsHistory);
  }

  getMaxFps(): number {
    if (this.fpsHistory.length === 0) return 0;
    return Math.max(...this.fpsHistory);
  }

  getFpsHistory(): number[] { return [...this.fpsHistory]; }

  // --- Cache ---
  updateCacheStats(stats: Partial<CacheStats>): void {
    Object.assign(this.cacheStats, stats);
  }

  getCacheStats(): CacheStats { return { ...this.cacheStats }; }

  // --- Phase Wheel ---
  updatePhaseStatus(status: Partial<PhaseWheelStatus>): void {
    Object.assign(this.phaseStatus, status);
  }

  getPhaseStatus(): PhaseWheelStatus { return { ...this.phaseStatus }; }

  // --- Entity Inspector ---
  inspectEntity(info: EntityDebugInfo): void {
    this.inspectedEntity = { ...info };
  }

  getInspectedEntity(): EntityDebugInfo | null {
    return this.inspectedEntity ? { ...this.inspectedEntity } : null;
  }

  clearEntityInspection(): void {
    this.inspectedEntity = null;
  }

  // --- Tile Inspector ---
  inspectTile(x: number, y: number, data: Record<string, unknown>): void {
    this.inspectedTile = { x, y, data: { ...data } };
  }

  getInspectedTile(): { x: number; y: number; data: Record<string, unknown> } | null {
    return this.inspectedTile ? { ...this.inspectedTile, data: { ...this.inspectedTile.data } } : null;
  }

  clearTileInspection(): void {
    this.inspectedTile = null;
  }

  // --- Console ---
  log(level: ConsoleEntry['level'], message: string, source: string = 'system'): void {
    this.consoleLog.push({
      timestamp: Date.now(),
      level,
      message,
      source,
    });
    while (this.consoleLog.length > this.maxConsoleEntries) {
      this.consoleLog.shift();
    }
  }

  getConsoleLog(level?: ConsoleEntry['level']): ConsoleEntry[] {
    const entries = level
      ? this.consoleLog.filter(e => e.level === level)
      : this.consoleLog;
    return entries.map(e => ({ ...e }));
  }

  clearConsole(): void {
    this.consoleLog = [];
  }

  /** Execute a debug command */
  executeCommand(command: string): string {
    this.log('info', `> ${command}`, 'console');
    this.emit('command', { command });
    // Return acknowledgement — actual handling via event
    return `Executed: ${command}`;
  }

  // --- Display Helpers ---
  formatFps(fps: number): string {
    return `${fps} FPS`;
  }

  formatMemory(mb: number): string {
    return `${mb.toFixed(1)} MB`;
  }

  formatTime(ms: number): string {
    if (ms < 1) return `${(ms * 1000).toFixed(0)}us`;
    return `${ms.toFixed(1)}ms`;
  }

  formatFlags(flags: number): string {
    const names: string[] = [];
    if (flags & 0x01) names.push('POWERED');
    if (flags & 0x02) names.push('HAS_WATER');
    if (flags & 0x04) names.push('STAFFED');
    if (flags & 0x08) names.push('UNDER_CONSTRUCTION');
    if (flags & 0x10) names.push('ON_FIRE');
    if (flags & 0x20) names.push('DAMAGED');
    return names.length > 0 ? names.join(' | ') : 'NONE';
  }

  // --- Events ---
  addEventListener(handler: DevToolsEventHandler): void {
    this.events.on(handler);
  }
  removeEventListener(handler: DevToolsEventHandler): void {
    this.events.off(handler);
  }
  private emit<K extends DevToolsEventType>(type: K, data: DevToolsEventPayloads[K]): void {
    this.events.emit(type, data);
  }
}
