/** Overlay type identifiers (mirrors renderer OverlayType) */
export enum UIOverlayType {
  None = 0,
  Traffic = 1,
  Power = 2,
  Water = 3,
  Pollution = 4,
  Crime = 5,
  Noise = 6,
  LandValue = 7,
  Desirability = 8,
  Zoning = 9,
}

/** Overlay button configuration */
export interface OverlayButton {
  type: UIOverlayType;
  label: string;
  shortcut: string;  // keyboard shortcut hint
  icon: string;      // icon name/identifier
  enabled: boolean;
}

/** Legend color entry */
export interface LegendEntry {
  label: string;
  color: { r: number; g: number; b: number; a: number };
}

/** Overlay legend */
export interface OverlayLegend {
  title: string;
  entries: LegendEntry[];
  minLabel: string;  // e.g. "Low"
  maxLabel: string;  // e.g. "High"
}

/** Minimap configuration */
export interface MinimapConfig {
  width: number;    // pixels
  height: number;   // pixels
  mapWidth: number;  // tiles
  mapHeight: number; // tiles
}

/** Minimap viewport indicator */
export interface MinimapViewport {
  x: number;      // camera center in tiles
  y: number;
  viewW: number;  // visible width in tiles
  viewH: number;  // visible height in tiles
}

/** Overlay panel events */
export type OverlayPanelEventType = 'overlayChanged' | 'minimapClick';
export type OverlayPanelEventHandler = (type: OverlayPanelEventType, data: any) => void;

/** Built-in overlay button definitions */
export const OVERLAY_BUTTONS: OverlayButton[] = [
  { type: UIOverlayType.Traffic, label: 'Traffic', shortcut: 'T', icon: 'traffic', enabled: true },
  { type: UIOverlayType.Power, label: 'Power', shortcut: 'P', icon: 'power', enabled: true },
  { type: UIOverlayType.Water, label: 'Water', shortcut: 'W', icon: 'water', enabled: true },
  { type: UIOverlayType.Pollution, label: 'Pollution', shortcut: 'U', icon: 'pollution', enabled: true },
  { type: UIOverlayType.Crime, label: 'Crime', shortcut: 'C', icon: 'crime', enabled: true },
  { type: UIOverlayType.Noise, label: 'Noise', shortcut: 'N', icon: 'noise', enabled: true },
  { type: UIOverlayType.LandValue, label: 'Land Value', shortcut: 'V', icon: 'land_value', enabled: true },
  { type: UIOverlayType.Desirability, label: 'Desirability', shortcut: 'D', icon: 'desirability', enabled: true },
  { type: UIOverlayType.Zoning, label: 'Zoning', shortcut: 'Z', icon: 'zoning', enabled: true },
];

/** Built-in legend definitions */
export const OVERLAY_LEGENDS: Partial<Record<UIOverlayType, OverlayLegend>> = {
  [UIOverlayType.Traffic]: {
    title: 'Traffic Density',
    minLabel: 'Low', maxLabel: 'High',
    entries: [
      { label: 'Free Flow', color: { r: 0, g: 200, b: 0, a: 255 } },
      { label: 'Moderate', color: { r: 200, g: 200, b: 0, a: 255 } },
      { label: 'Congested', color: { r: 200, g: 0, b: 0, a: 255 } },
    ],
  },
  [UIOverlayType.Power]: {
    title: 'Power Coverage',
    minLabel: 'None', maxLabel: 'Full',
    entries: [
      { label: 'Unpowered', color: { r: 50, g: 50, b: 200, a: 255 } },
      { label: 'Partial', color: { r: 200, g: 200, b: 50, a: 255 } },
      { label: 'Full Power', color: { r: 200, g: 50, b: 50, a: 255 } },
    ],
  },
  [UIOverlayType.Pollution]: {
    title: 'Pollution Level',
    minLabel: 'Clean', maxLabel: 'Polluted',
    entries: [
      { label: 'Clean', color: { r: 0, g: 200, b: 0, a: 255 } },
      { label: 'Moderate', color: { r: 200, g: 200, b: 0, a: 255 } },
      { label: 'Heavy', color: { r: 200, g: 0, b: 0, a: 255 } },
    ],
  },
  [UIOverlayType.Zoning]: {
    title: 'Zone Types',
    minLabel: '', maxLabel: '',
    entries: [
      { label: 'Residential', color: { r: 50, g: 200, b: 50, a: 255 } },
      { label: 'Commercial', color: { r: 50, g: 50, b: 200, a: 255 } },
      { label: 'Industrial', color: { r: 200, g: 200, b: 50, a: 255 } },
    ],
  },
};

/**
 * OverlayPanel -- manages overlay toggles and minimap display.
 */
export class OverlayPanel {
  private activeOverlay: UIOverlayType;
  private buttons: OverlayButton[];
  private minimapConfig: MinimapConfig;
  private minimapViewport: MinimapViewport;
  private eventHandlers: OverlayPanelEventHandler[];

  constructor(minimapConfig?: Partial<MinimapConfig>) {
    this.activeOverlay = UIOverlayType.None;
    this.buttons = OVERLAY_BUTTONS.map(b => ({ ...b }));
    this.minimapConfig = {
      width: 200,
      height: 200,
      mapWidth: 256,
      mapHeight: 256,
      ...minimapConfig,
    };
    this.minimapViewport = { x: 128, y: 128, viewW: 30, viewH: 20 };
    this.eventHandlers = [];
  }

  // --- Overlay ---
  getActiveOverlay(): UIOverlayType { return this.activeOverlay; }

  setOverlay(type: UIOverlayType): void {
    this.activeOverlay = type;
    this.emit('overlayChanged', { overlay: type });
  }

  toggleOverlay(type: UIOverlayType): void {
    this.activeOverlay = this.activeOverlay === type ? UIOverlayType.None : type;
    this.emit('overlayChanged', { overlay: this.activeOverlay });
  }

  getButtons(): OverlayButton[] {
    return this.buttons.map(b => ({ ...b }));
  }

  setButtonEnabled(type: UIOverlayType, enabled: boolean): void {
    const btn = this.buttons.find(b => b.type === type);
    if (btn) btn.enabled = enabled;
  }

  /** Get legend for active overlay */
  getActiveLegend(): OverlayLegend | null {
    if (this.activeOverlay === UIOverlayType.None) return null;
    return OVERLAY_LEGENDS[this.activeOverlay] ?? null;
  }

  /** Handle keyboard shortcut */
  handleShortcut(key: string): boolean {
    const btn = this.buttons.find(b => b.shortcut.toLowerCase() === key.toLowerCase() && b.enabled);
    if (btn) {
      this.toggleOverlay(btn.type);
      return true;
    }
    return false;
  }

  // --- Minimap ---
  getMinimapConfig(): MinimapConfig { return { ...this.minimapConfig }; }

  updateMinimapViewport(viewport: Partial<MinimapViewport>): void {
    Object.assign(this.minimapViewport, viewport);
  }

  getMinimapViewport(): MinimapViewport { return { ...this.minimapViewport }; }

  /** Convert minimap pixel coords to tile coords */
  minimapToTile(pixelX: number, pixelY: number): { tileX: number; tileY: number } {
    const tileX = Math.floor((pixelX / this.minimapConfig.width) * this.minimapConfig.mapWidth);
    const tileY = Math.floor((pixelY / this.minimapConfig.height) * this.minimapConfig.mapHeight);
    return { tileX, tileY };
  }

  /** Handle minimap click */
  onMinimapClick(pixelX: number, pixelY: number): void {
    const { tileX, tileY } = this.minimapToTile(pixelX, pixelY);
    this.emit('minimapClick', { tileX, tileY });
  }

  /** Get the viewport rectangle in minimap pixel coordinates */
  getViewportRect(): { x: number; y: number; w: number; h: number } {
    const scaleX = this.minimapConfig.width / this.minimapConfig.mapWidth;
    const scaleY = this.minimapConfig.height / this.minimapConfig.mapHeight;
    return {
      x: (this.minimapViewport.x - this.minimapViewport.viewW / 2) * scaleX,
      y: (this.minimapViewport.y - this.minimapViewport.viewH / 2) * scaleY,
      w: this.minimapViewport.viewW * scaleX,
      h: this.minimapViewport.viewH * scaleY,
    };
  }

  // --- Events ---
  addEventListener(handler: OverlayPanelEventHandler): void {
    this.eventHandlers.push(handler);
  }
  removeEventListener(handler: OverlayPanelEventHandler): void {
    const idx = this.eventHandlers.indexOf(handler);
    if (idx >= 0) this.eventHandlers.splice(idx, 1);
  }
  private emit(type: OverlayPanelEventType, data: any): void {
    for (const handler of this.eventHandlers) handler(type, data);
  }
}
