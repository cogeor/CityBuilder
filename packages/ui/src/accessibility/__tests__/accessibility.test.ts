import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  ColorBlindMode,
  type AccessibilityConfig,
  DEFAULT_ACCESSIBILITY,
  STANDARD_PALETTE,
  PROTANOPIA_PALETTE,
  DEUTERANOPIA_PALETTE,
  TRITANOPIA_PALETTE,
  getPaletteForMode,
  clampFontSize,
  clampUiScale,
  applyConfig,
  AccessibilityManager,
} from '../accessibility.js';

describe('Accessibility', () => {
  // --- DEFAULT_ACCESSIBILITY ---

  it('DEFAULT_ACCESSIBILITY has expected values', () => {
    expect(DEFAULT_ACCESSIBILITY.colorBlindMode).toBe(ColorBlindMode.None);
    expect(DEFAULT_ACCESSIBILITY.uiScale).toBe(1.0);
    expect(DEFAULT_ACCESSIBILITY.highContrast).toBe(false);
    expect(DEFAULT_ACCESSIBILITY.reducedMotion).toBe(false);
    expect(DEFAULT_ACCESSIBILITY.keyboardNavigation).toBe(false);
    expect(DEFAULT_ACCESSIBILITY.fontSize).toBe(16);
  });

  // --- getPaletteForMode ---

  it('getPaletteForMode returns standard palette for None', () => {
    expect(getPaletteForMode(ColorBlindMode.None)).toEqual(STANDARD_PALETTE);
  });

  it('getPaletteForMode returns protanopia palette', () => {
    expect(getPaletteForMode(ColorBlindMode.Protanopia)).toEqual(PROTANOPIA_PALETTE);
  });

  it('getPaletteForMode returns deuteranopia palette', () => {
    expect(getPaletteForMode(ColorBlindMode.Deuteranopia)).toEqual(DEUTERANOPIA_PALETTE);
  });

  it('getPaletteForMode returns tritanopia palette', () => {
    expect(getPaletteForMode(ColorBlindMode.Tritanopia)).toEqual(TRITANOPIA_PALETTE);
  });

  // --- clampFontSize ---

  it('clampFontSize clamps below 12 to 12', () => {
    expect(clampFontSize(8)).toBe(12);
    expect(clampFontSize(0)).toBe(12);
    expect(clampFontSize(-5)).toBe(12);
  });

  it('clampFontSize clamps above 24 to 24', () => {
    expect(clampFontSize(30)).toBe(24);
    expect(clampFontSize(100)).toBe(24);
  });

  it('clampFontSize passes through valid values', () => {
    expect(clampFontSize(12)).toBe(12);
    expect(clampFontSize(18)).toBe(18);
    expect(clampFontSize(24)).toBe(24);
  });

  // --- clampUiScale ---

  it('clampUiScale clamps below 0.75 to 0.75', () => {
    expect(clampUiScale(0.5)).toBe(0.75);
    expect(clampUiScale(0)).toBe(0.75);
    expect(clampUiScale(-1)).toBe(0.75);
  });

  it('clampUiScale clamps above 2.0 to 2.0', () => {
    expect(clampUiScale(3.0)).toBe(2.0);
    expect(clampUiScale(10)).toBe(2.0);
  });

  it('clampUiScale passes through valid values', () => {
    expect(clampUiScale(0.75)).toBe(0.75);
    expect(clampUiScale(1.0)).toBe(1.0);
    expect(clampUiScale(2.0)).toBe(2.0);
  });

  // --- applyConfig ---

  it('applyConfig clamps out-of-range values', () => {
    const raw: AccessibilityConfig = {
      colorBlindMode: ColorBlindMode.Protanopia,
      uiScale: 5.0,
      highContrast: true,
      reducedMotion: false,
      keyboardNavigation: true,
      fontSize: 2,
    };
    const result = applyConfig(raw);
    expect(result.uiScale).toBe(2.0);
    expect(result.fontSize).toBe(12);
    expect(result.colorBlindMode).toBe(ColorBlindMode.Protanopia);
    expect(result.highContrast).toBe(true);
  });
});

describe('AccessibilityManager', () => {
  let manager: AccessibilityManager;

  beforeEach(() => {
    manager = new AccessibilityManager();
  });

  it('starts with default config', () => {
    const config = manager.getConfig();
    expect(config.colorBlindMode).toBe(ColorBlindMode.None);
    expect(config.uiScale).toBe(1.0);
    expect(config.highContrast).toBe(false);
    expect(config.reducedMotion).toBe(false);
    expect(config.keyboardNavigation).toBe(false);
    expect(config.fontSize).toBe(16);
  });

  it('accepts partial config in constructor', () => {
    const m = new AccessibilityManager({ highContrast: true, fontSize: 20 });
    const config = m.getConfig();
    expect(config.highContrast).toBe(true);
    expect(config.fontSize).toBe(20);
    // defaults for unset fields
    expect(config.colorBlindMode).toBe(ColorBlindMode.None);
  });

  it('setColorBlindMode updates config and palette', () => {
    manager.setColorBlindMode(ColorBlindMode.Protanopia);
    expect(manager.getConfig().colorBlindMode).toBe(ColorBlindMode.Protanopia);
    expect(manager.getPalette()).toEqual(PROTANOPIA_PALETTE);
  });

  it('setUiScale clamps value', () => {
    manager.setUiScale(5.0);
    expect(manager.getConfig().uiScale).toBe(2.0);

    manager.setUiScale(0.1);
    expect(manager.getConfig().uiScale).toBe(0.75);

    manager.setUiScale(1.5);
    expect(manager.getConfig().uiScale).toBe(1.5);
  });

  it('setHighContrast updates config', () => {
    manager.setHighContrast(true);
    expect(manager.getConfig().highContrast).toBe(true);
  });

  it('setReducedMotion updates config', () => {
    manager.setReducedMotion(true);
    expect(manager.getConfig().reducedMotion).toBe(true);
  });

  it('setKeyboardNavigation updates config', () => {
    manager.setKeyboardNavigation(true);
    expect(manager.getConfig().keyboardNavigation).toBe(true);
  });

  it('setFontSize clamps value', () => {
    manager.setFontSize(8);
    expect(manager.getConfig().fontSize).toBe(12);

    manager.setFontSize(30);
    expect(manager.getConfig().fontSize).toBe(24);

    manager.setFontSize(18);
    expect(manager.getConfig().fontSize).toBe(18);
  });

  it('onChange fires on changes', () => {
    const listener = vi.fn();
    manager.onChange(listener);

    manager.setHighContrast(true);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(expect.objectContaining({ highContrast: true }));

    manager.setFontSize(20);
    expect(listener).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenCalledWith(expect.objectContaining({ fontSize: 20 }));
  });

  it('unsubscribe stops notifications', () => {
    const listener = vi.fn();
    const unsub = manager.onChange(listener);

    manager.setHighContrast(true);
    expect(listener).toHaveBeenCalledTimes(1);

    unsub();

    manager.setFontSize(20);
    expect(listener).toHaveBeenCalledTimes(1); // no additional call
  });

  it('reset returns to defaults', () => {
    manager.setColorBlindMode(ColorBlindMode.Tritanopia);
    manager.setUiScale(1.5);
    manager.setHighContrast(true);
    manager.setFontSize(20);

    manager.reset();
    const config = manager.getConfig();
    expect(config.colorBlindMode).toBe(ColorBlindMode.None);
    expect(config.uiScale).toBe(1.0);
    expect(config.highContrast).toBe(false);
    expect(config.fontSize).toBe(16);
  });

  it('reset fires onChange', () => {
    const listener = vi.fn();
    manager.onChange(listener);

    manager.reset();
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('getConfig returns a copy (not a reference)', () => {
    const config1 = manager.getConfig();
    config1.highContrast = true;
    config1.fontSize = 24;
    const config2 = manager.getConfig();
    expect(config2.highContrast).toBe(false);
    expect(config2.fontSize).toBe(16);
  });
});
