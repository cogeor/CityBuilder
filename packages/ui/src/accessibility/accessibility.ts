/** Color-blind simulation modes */
export enum ColorBlindMode {
  None = "none",
  Protanopia = "protanopia",
  Deuteranopia = "deuteranopia",
  Tritanopia = "tritanopia",
}

/** Full accessibility configuration */
export interface AccessibilityConfig {
  colorBlindMode: ColorBlindMode;
  uiScale: number;          // 0.75 - 2.0
  highContrast: boolean;
  reducedMotion: boolean;
  keyboardNavigation: boolean;
  fontSize: number;          // base font size in px (12-24)
}

/** Default accessibility settings */
export const DEFAULT_ACCESSIBILITY: AccessibilityConfig = {
  colorBlindMode: ColorBlindMode.None,
  uiScale: 1.0,
  highContrast: false,
  reducedMotion: false,
  keyboardNavigation: false,
  fontSize: 16,
};

/** Color-blind safe overlay palettes */
export interface ColorPalette {
  low: string;      // hex color
  medium: string;
  high: string;
  critical: string;
}

export const STANDARD_PALETTE: ColorPalette = {
  low: "#00ff00",
  medium: "#ffff00",
  high: "#ff8800",
  critical: "#ff0000",
};

export const PROTANOPIA_PALETTE: ColorPalette = {
  low: "#0077bb",
  medium: "#ddcc77",
  high: "#cc6677",
  critical: "#882255",
};

export const DEUTERANOPIA_PALETTE: ColorPalette = {
  low: "#332288",
  medium: "#88ccee",
  high: "#cc6677",
  critical: "#882255",
};

export const TRITANOPIA_PALETTE: ColorPalette = {
  low: "#004488",
  medium: "#ddaa33",
  high: "#bb5566",
  critical: "#000000",
};

/** Get the color palette appropriate for a given color-blind mode */
export function getPaletteForMode(mode: ColorBlindMode): ColorPalette {
  switch (mode) {
    case ColorBlindMode.None:
      return STANDARD_PALETTE;
    case ColorBlindMode.Protanopia:
      return PROTANOPIA_PALETTE;
    case ColorBlindMode.Deuteranopia:
      return DEUTERANOPIA_PALETTE;
    case ColorBlindMode.Tritanopia:
      return TRITANOPIA_PALETTE;
  }
}

/** Clamp font size to the valid range 12-24 */
export function clampFontSize(size: number): number {
  return Math.max(12, Math.min(24, size));
}

/** Clamp UI scale to the valid range 0.75-2.0 */
export function clampUiScale(scale: number): number {
  return Math.max(0.75, Math.min(2.0, scale));
}

/** Validate and clamp all fields in an AccessibilityConfig */
export function applyConfig(config: AccessibilityConfig): AccessibilityConfig {
  return {
    colorBlindMode: config.colorBlindMode,
    uiScale: clampUiScale(config.uiScale),
    highContrast: config.highContrast,
    reducedMotion: config.reducedMotion,
    keyboardNavigation: config.keyboardNavigation,
    fontSize: clampFontSize(config.fontSize),
  };
}

/**
 * AccessibilityManager -- manages accessibility configuration with
 * change notifications so that the UI can react to settings changes.
 */
export class AccessibilityManager {
  private config: AccessibilityConfig;
  private listeners: Array<(config: AccessibilityConfig) => void>;

  constructor(config?: Partial<AccessibilityConfig>) {
    this.config = applyConfig({ ...DEFAULT_ACCESSIBILITY, ...config });
    this.listeners = [];
  }

  /** Get a copy of the current config */
  getConfig(): AccessibilityConfig {
    return { ...this.config };
  }

  /** Set the color-blind simulation mode */
  setColorBlindMode(mode: ColorBlindMode): void {
    this.config.colorBlindMode = mode;
    this.notify();
  }

  /** Set the UI scale (clamped to 0.75-2.0) */
  setUiScale(scale: number): void {
    this.config.uiScale = clampUiScale(scale);
    this.notify();
  }

  /** Enable or disable high contrast mode */
  setHighContrast(enabled: boolean): void {
    this.config.highContrast = enabled;
    this.notify();
  }

  /** Enable or disable reduced motion */
  setReducedMotion(enabled: boolean): void {
    this.config.reducedMotion = enabled;
    this.notify();
  }

  /** Enable or disable keyboard navigation */
  setKeyboardNavigation(enabled: boolean): void {
    this.config.keyboardNavigation = enabled;
    this.notify();
  }

  /** Set the base font size in px (clamped to 12-24) */
  setFontSize(size: number): void {
    this.config.fontSize = clampFontSize(size);
    this.notify();
  }

  /** Get the color palette for the current color-blind mode */
  getPalette(): ColorPalette {
    return getPaletteForMode(this.config.colorBlindMode);
  }

  /** Register a listener for config changes. Returns an unsubscribe function. */
  onChange(listener: (config: AccessibilityConfig) => void): () => void {
    this.listeners.push(listener);
    return () => {
      const idx = this.listeners.indexOf(listener);
      if (idx >= 0) {
        this.listeners.splice(idx, 1);
      }
    };
  }

  /** Reset config to defaults */
  reset(): void {
    this.config = { ...DEFAULT_ACCESSIBILITY };
    this.notify();
  }

  private notify(): void {
    const snapshot = this.getConfig();
    for (const listener of this.listeners) {
      listener(snapshot);
    }
  }
}
