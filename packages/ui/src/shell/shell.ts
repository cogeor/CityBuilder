import { TypedEventHub, type EventListener } from "../shared/typed_events.js";

/** Simulation speed levels */
export enum SimSpeed {
  Paused = 0,
  Normal = 1,
  Fast = 2,
  VeryFast = 3,
}

/** Tool type identifiers */
export enum ToolType {
  Select = 'select',
  Place = 'place',
  Zone = 'zone',
  Bulldoze = 'bulldoze',
  Road = 'road',
  Terrain = 'terrain',
}

/** City stats displayed in top bar */
export interface CityStats {
  cityName: string;
  population: number;
  treasury: number;    // in cents
  date: string;        // formatted game date
  time: string;        // formatted game time
}

/** Notification for event alerts */
export interface Notification {
  id: number;
  message: string;
  type: 'info' | 'warning' | 'error' | 'success';
  timestamp: number;
  dismissed: boolean;
}

/** Panel state (for side panels) */
export interface PanelState {
  id: string;
  title: string;
  visible: boolean;
  pinned: boolean;
}

/** Shell event types */
export type ShellEventType =
  | 'speedChange'
  | 'toolChange'
  | 'panelToggle'
  | 'notificationDismiss';

export interface ShellEventPayloads {
  speedChange: { speed: SimSpeed };
  toolChange: { tool: ToolType };
  panelToggle: { id: string; visible: boolean };
  notificationDismiss: { id: number };
}

/** Shell event handler */
export type ShellEventHandler = EventListener<ShellEventPayloads>;

/** Default city stats */
export const DEFAULT_CITY_STATS: CityStats = {
  cityName: 'New City',
  population: 0,
  treasury: 100000,
  date: 'Day 1',
  time: '08:00',
};

/**
 * HUD Shell -- manages the overall UI layout.
 *
 * Layout:
 * +----------------------------------------------+
 * | Top Bar: City Name | Pop | $ | Date/Time     |
 * +----------+-----------------------------------+
 * | Side     |                                   |
 * | Panel    |      Game Canvas                  |
 * |          |                                   |
 * +----------+-----------------------------------+
 * | Bottom: Tools | Speed Controls               |
 * +----------------------------------------------+
 */
export class HudShell {
  private stats: CityStats;
  private speed: SimSpeed;
  private activeTool: ToolType;
  private panels: Map<string, PanelState>;
  private notifications: Notification[];
  private nextNotificationId: number;
  private readonly events: TypedEventHub<ShellEventPayloads>;
  private maxNotifications: number;

  constructor(initialStats?: Partial<CityStats>) {
    this.stats = { ...DEFAULT_CITY_STATS, ...initialStats };
    this.speed = SimSpeed.Normal;
    this.activeTool = ToolType.Select;
    this.panels = new Map();
    this.notifications = [];
    this.nextNotificationId = 1;
    this.events = new TypedEventHub<ShellEventPayloads>();
    this.maxNotifications = 10;
  }

  // --- Stats ---
  getStats(): CityStats { return { ...this.stats }; }

  updateStats(stats: Partial<CityStats>): void {
    Object.assign(this.stats, stats);
  }

  /** Format treasury as currency string */
  formatTreasury(): string {
    const dollars = this.stats.treasury / 100;
    if (Math.abs(dollars) >= 1_000_000) {
      return `$${(dollars / 1_000_000).toFixed(1)}M`;
    }
    if (Math.abs(dollars) >= 1_000) {
      return `$${(dollars / 1_000).toFixed(1)}K`;
    }
    return `$${dollars.toFixed(0)}`;
  }

  /** Format population with K/M suffixes */
  formatPopulation(): string {
    const pop = this.stats.population;
    if (pop >= 1_000_000) return `${(pop / 1_000_000).toFixed(1)}M`;
    if (pop >= 1_000) return `${(pop / 1_000).toFixed(1)}K`;
    return `${pop}`;
  }

  // --- Speed Controls ---
  getSpeed(): SimSpeed { return this.speed; }

  setSpeed(speed: SimSpeed): void {
    this.speed = speed;
    this.emit('speedChange', { speed });
  }

  togglePause(): void {
    if (this.speed === SimSpeed.Paused) {
      this.speed = SimSpeed.Normal;
    } else {
      this.speed = SimSpeed.Paused;
    }
    this.emit('speedChange', { speed: this.speed });
  }

  // --- Tool Selection ---
  getActiveTool(): ToolType { return this.activeTool; }

  setActiveTool(tool: ToolType): void {
    this.activeTool = tool;
    this.emit('toolChange', { tool });
  }

  // --- Panel Management ---
  registerPanel(id: string, title: string): void {
    this.panels.set(id, { id, title, visible: false, pinned: false });
  }

  showPanel(id: string): void {
    const panel = this.panels.get(id);
    if (panel) {
      panel.visible = true;
      this.emit('panelToggle', { id, visible: true });
    }
  }

  hidePanel(id: string): void {
    const panel = this.panels.get(id);
    if (panel && !panel.pinned) {
      panel.visible = false;
      this.emit('panelToggle', { id, visible: false });
    }
  }

  togglePanel(id: string): void {
    const panel = this.panels.get(id);
    if (panel) {
      if (panel.visible) {
        this.hidePanel(id);
      } else {
        this.showPanel(id);
      }
    }
  }

  pinPanel(id: string): void {
    const panel = this.panels.get(id);
    if (panel) panel.pinned = true;
  }

  unpinPanel(id: string): void {
    const panel = this.panels.get(id);
    if (panel) panel.pinned = false;
  }

  getPanel(id: string): PanelState | undefined {
    const panel = this.panels.get(id);
    return panel ? { ...panel } : undefined;
  }

  getVisiblePanels(): PanelState[] {
    return Array.from(this.panels.values()).filter(p => p.visible).map(p => ({ ...p }));
  }

  // --- Notifications ---
  addNotification(message: string, type: Notification['type'] = 'info'): number {
    const id = this.nextNotificationId++;
    this.notifications.push({
      id,
      message,
      type,
      timestamp: Date.now(),
      dismissed: false,
    });
    // Trim old notifications
    while (this.notifications.length > this.maxNotifications) {
      this.notifications.shift();
    }
    return id;
  }

  dismissNotification(id: number): void {
    const notification = this.notifications.find(n => n.id === id);
    if (notification) {
      notification.dismissed = true;
      this.emit('notificationDismiss', { id });
    }
  }

  getNotifications(): Notification[] {
    return this.notifications.filter(n => !n.dismissed).map(n => ({ ...n }));
  }

  clearNotifications(): void {
    this.notifications = [];
  }

  // --- Events ---
  addEventListener(handler: ShellEventHandler): void {
    this.events.on(handler);
  }

  removeEventListener(handler: ShellEventHandler): void {
    this.events.off(handler);
  }

  private emit<K extends ShellEventType>(type: K, data: ShellEventPayloads[K]): void {
    this.events.emit(type, data);
  }
}
