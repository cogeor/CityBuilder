import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  OverlayPanel,
  UIOverlayType,
  OVERLAY_BUTTONS,
  OVERLAY_LEGENDS,
  type OverlayPanelEventType,
} from '../overlay_panel.js';

describe('OverlayPanel', () => {
  let panel: OverlayPanel;

  beforeEach(() => {
    panel = new OverlayPanel();
  });

  // --- Constructor ---

  it('constructor starts with None overlay', () => {
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.None);
  });

  it('default minimap config values', () => {
    const config = panel.getMinimapConfig();
    expect(config.width).toBe(200);
    expect(config.height).toBe(200);
    expect(config.mapWidth).toBe(256);
    expect(config.mapHeight).toBe(256);
  });

  // --- Overlay ---

  it('setOverlay changes active overlay', () => {
    panel.setOverlay(UIOverlayType.Traffic);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.Traffic);
  });

  it('toggleOverlay enables overlay', () => {
    panel.toggleOverlay(UIOverlayType.Power);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.Power);
  });

  it('toggleOverlay disables if same', () => {
    panel.setOverlay(UIOverlayType.Power);
    panel.toggleOverlay(UIOverlayType.Power);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.None);
  });

  // --- Buttons ---

  it('getButtons returns 9 buttons', () => {
    const buttons = panel.getButtons();
    expect(buttons).toHaveLength(9);
  });

  it('getButtons returns copies', () => {
    const buttons = panel.getButtons();
    buttons[0].label = 'MUTATED';
    expect(panel.getButtons()[0].label).toBe('Traffic');
  });

  it('setButtonEnabled disables button', () => {
    panel.setButtonEnabled(UIOverlayType.Traffic, false);
    const buttons = panel.getButtons();
    const trafficBtn = buttons.find(b => b.type === UIOverlayType.Traffic);
    expect(trafficBtn!.enabled).toBe(false);
  });

  // --- Legend ---

  it('getActiveLegend returns null for None', () => {
    expect(panel.getActiveLegend()).toBeNull();
  });

  it('getActiveLegend returns legend for Traffic', () => {
    panel.setOverlay(UIOverlayType.Traffic);
    const legend = panel.getActiveLegend();
    expect(legend).not.toBeNull();
    expect(legend!.title).toBe('Traffic Density');
    expect(legend!.entries).toHaveLength(3);
  });

  it('getActiveLegend returns legend for Zoning', () => {
    panel.setOverlay(UIOverlayType.Zoning);
    const legend = panel.getActiveLegend();
    expect(legend).not.toBeNull();
    expect(legend!.title).toBe('Zone Types');
    expect(legend!.entries).toHaveLength(3);
  });

  it('getActiveLegend returns null for overlay without legend', () => {
    panel.setOverlay(UIOverlayType.Crime);
    expect(panel.getActiveLegend()).toBeNull();
  });

  // --- Shortcuts ---

  it('handleShortcut toggles overlay for matching key', () => {
    const result = panel.handleShortcut('T');
    expect(result).toBe(true);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.Traffic);
  });

  it('handleShortcut returns false for unknown key', () => {
    const result = panel.handleShortcut('X');
    expect(result).toBe(false);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.None);
  });

  it('handleShortcut ignores disabled buttons', () => {
    panel.setButtonEnabled(UIOverlayType.Traffic, false);
    const result = panel.handleShortcut('T');
    expect(result).toBe(false);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.None);
  });

  it('handleShortcut is case insensitive', () => {
    const result = panel.handleShortcut('t');
    expect(result).toBe(true);
    expect(panel.getActiveOverlay()).toBe(UIOverlayType.Traffic);
  });

  // --- Minimap ---

  it('minimapToTile converts correctly', () => {
    // default: 200px -> 256 tiles, so 100px -> tile 128
    const result = panel.minimapToTile(100, 100);
    expect(result.tileX).toBe(128);
    expect(result.tileY).toBe(128);
  });

  it('minimapToTile handles edges', () => {
    const topLeft = panel.minimapToTile(0, 0);
    expect(topLeft.tileX).toBe(0);
    expect(topLeft.tileY).toBe(0);

    // At pixel 199 of 200 -> floor((199/200)*256) = floor(254.72) = 254
    const bottomRight = panel.minimapToTile(199, 199);
    expect(bottomRight.tileX).toBe(254);
    expect(bottomRight.tileY).toBe(254);
  });

  it('onMinimapClick emits event with tile coords', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.onMinimapClick(100, 100);
    expect(handler).toHaveBeenCalledWith('minimapClick', { tileX: 128, tileY: 128 });
  });

  it('getMinimapConfig returns copy', () => {
    const config = panel.getMinimapConfig();
    config.width = 999;
    expect(panel.getMinimapConfig().width).toBe(200);
  });

  it('updateMinimapViewport updates fields', () => {
    panel.updateMinimapViewport({ x: 50, y: 60 });
    const vp = panel.getMinimapViewport();
    expect(vp.x).toBe(50);
    expect(vp.y).toBe(60);
    // unchanged fields preserved
    expect(vp.viewW).toBe(30);
    expect(vp.viewH).toBe(20);
  });

  it('getViewportRect computes pixel coordinates', () => {
    // default: viewport at (128, 128), viewW=30, viewH=20
    // scaleX = 200/256 = 0.78125, scaleY = 200/256 = 0.78125
    // x = (128 - 15) * 0.78125 = 113 * 0.78125 = 88.28125
    // y = (128 - 10) * 0.78125 = 118 * 0.78125 = 92.1875
    // w = 30 * 0.78125 = 23.4375
    // h = 20 * 0.78125 = 15.625
    const rect = panel.getViewportRect();
    expect(rect.x).toBeCloseTo(88.28125, 4);
    expect(rect.y).toBeCloseTo(92.1875, 4);
    expect(rect.w).toBeCloseTo(23.4375, 4);
    expect(rect.h).toBeCloseTo(15.625, 4);
  });

  // --- Events ---

  it('addEventListener receives events', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.setOverlay(UIOverlayType.Water);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith('overlayChanged', { overlay: UIOverlayType.Water });
  });

  it('removeEventListener stops receiving', () => {
    const handler = vi.fn();
    panel.addEventListener(handler);
    panel.removeEventListener(handler);
    panel.setOverlay(UIOverlayType.Water);
    expect(handler).not.toHaveBeenCalled();
  });

  // --- Static Data ---

  it('OVERLAY_BUTTONS has 9 entries', () => {
    expect(OVERLAY_BUTTONS).toHaveLength(9);
  });

  it('OVERLAY_LEGENDS has Traffic, Power, Pollution, Zoning', () => {
    expect(OVERLAY_LEGENDS[UIOverlayType.Traffic]).toBeDefined();
    expect(OVERLAY_LEGENDS[UIOverlayType.Power]).toBeDefined();
    expect(OVERLAY_LEGENDS[UIOverlayType.Pollution]).toBeDefined();
    expect(OVERLAY_LEGENDS[UIOverlayType.Zoning]).toBeDefined();
  });

  it('constructor accepts custom minimap config', () => {
    const custom = new OverlayPanel({ width: 300, height: 150 });
    const config = custom.getMinimapConfig();
    expect(config.width).toBe(300);
    expect(config.height).toBe(150);
    // defaults preserved for unset fields
    expect(config.mapWidth).toBe(256);
    expect(config.mapHeight).toBe(256);
  });
});
