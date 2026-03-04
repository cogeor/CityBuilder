import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  CameraController,
  DEFAULT_KEY_BINDINGS,
  DEFAULT_CAMERA_LIMITS,
  type CameraEventType,
} from '../camera_controller.js';

describe('CameraController', () => {
  let ctrl: CameraController;

  beforeEach(() => {
    ctrl = new CameraController(
      { width: 800, height: 600 },
      { width: 128, height: 128 },
    );
  });

  // --- Constructor ---

  it('centers camera on map', () => {
    const cam = ctrl.getCamera();
    expect(cam.x).toBe(64);
    expect(cam.y).toBe(64);
    expect(cam.zoom).toBe(1.0);
  });

  it('applies custom viewport', () => {
    const cam = ctrl.getCamera();
    expect(cam.viewportW).toBe(800);
    expect(cam.viewportH).toBe(600);
  });

  // --- Getters ---

  it('getCamera returns copy', () => {
    const a = ctrl.getCamera();
    const b = ctrl.getCamera();
    expect(a).toEqual(b);
    a.x = 999;
    expect(ctrl.getCamera().x).toBe(64);
  });

  // --- Direct Camera Control ---

  it('setCamera updates fields', () => {
    ctrl.setCamera({ x: 10, y: 20, zoom: 2.0 });
    const cam = ctrl.getCamera();
    expect(cam.x).toBe(10);
    expect(cam.y).toBe(20);
    expect(cam.zoom).toBe(2.0);
  });

  it('centerOn moves camera', () => {
    ctrl.centerOn(30, 40);
    const cam = ctrl.getCamera();
    expect(cam.x).toBe(30);
    expect(cam.y).toBe(40);
  });

  it('setZoom clamps to limits', () => {
    ctrl.setZoom(100);
    expect(ctrl.getCamera().zoom).toBe(DEFAULT_CAMERA_LIMITS.maxZoom);

    ctrl.setZoom(-5);
    expect(ctrl.getCamera().zoom).toBe(DEFAULT_CAMERA_LIMITS.minZoom);

    ctrl.setZoom(2.0);
    expect(ctrl.getCamera().zoom).toBe(2.0);
  });

  // --- Keyboard Input ---

  it('onKeyDown tracks key state', () => {
    ctrl.onKeyDown('w');
    expect(ctrl.isKeyDown('w')).toBe(true);
    expect(ctrl.isKeyDown('s')).toBe(false);
  });

  it('onKeyUp removes key state', () => {
    ctrl.onKeyDown('w');
    ctrl.onKeyUp('w');
    expect(ctrl.isKeyDown('w')).toBe(false);
  });

  it('isBindingActive checks multiple keys', () => {
    ctrl.onKeyDown('ArrowUp');
    expect(ctrl.isBindingActive(DEFAULT_KEY_BINDINGS.panUp)).toBe(true);

    ctrl.onKeyUp('ArrowUp');
    expect(ctrl.isBindingActive(DEFAULT_KEY_BINDINGS.panUp)).toBe(false);
  });

  // --- Update (keyboard panning) ---

  it('update pans up with W key', () => {
    ctrl.onKeyDown('w');
    const before = ctrl.getCamera().y;
    ctrl.update(0.1);
    expect(ctrl.getCamera().y).toBeLessThan(before);
  });

  it('update pans down with S key', () => {
    ctrl.onKeyDown('s');
    const before = ctrl.getCamera().y;
    ctrl.update(0.1);
    expect(ctrl.getCamera().y).toBeGreaterThan(before);
  });

  it('update pans left with A key', () => {
    ctrl.onKeyDown('a');
    const before = ctrl.getCamera().x;
    ctrl.update(0.1);
    expect(ctrl.getCamera().x).toBeLessThan(before);
  });

  it('update pans right with D key', () => {
    ctrl.onKeyDown('d');
    const before = ctrl.getCamera().x;
    ctrl.update(0.1);
    expect(ctrl.getCamera().x).toBeGreaterThan(before);
  });

  it('update pans with arrow keys', () => {
    ctrl.onKeyDown('ArrowUp');
    const beforeY = ctrl.getCamera().y;
    ctrl.update(0.1);
    expect(ctrl.getCamera().y).toBeLessThan(beforeY);

    ctrl.onKeyUp('ArrowUp');
    ctrl.onKeyDown('ArrowRight');
    const beforeX = ctrl.getCamera().x;
    ctrl.update(0.1);
    expect(ctrl.getCamera().x).toBeGreaterThan(beforeX);
  });

  it('update zooms with + key', () => {
    ctrl.onKeyDown('+');
    const before = ctrl.getCamera().zoom;
    ctrl.update(0.1);
    expect(ctrl.getCamera().zoom).toBeGreaterThan(before);
  });

  it('update clamps position to limits', () => {
    // Push camera far out of bounds
    ctrl.setCamera({ x: -100, y: -100 });
    const cam = ctrl.getCamera();
    expect(cam.x).toBe(0);
    expect(cam.y).toBe(0);

    ctrl.setCamera({ x: 9999, y: 9999 });
    const cam2 = ctrl.getCamera();
    expect(cam2.x).toBe(128);
    expect(cam2.y).toBe(128);
  });

  it('update returns false when no keys pressed', () => {
    const changed = ctrl.update(0.1);
    expect(changed).toBe(false);
  });

  // --- Mouse Input ---

  it('onMouseDown starts drag with middle button', () => {
    expect(ctrl.isDraggingCamera()).toBe(false);
    ctrl.onMouseDown(400, 300, 1);
    expect(ctrl.isDraggingCamera()).toBe(true);
  });

  it('onMouseMove moves camera during drag', () => {
    ctrl.onMouseDown(400, 300, 1); // middle button drag start
    const before = ctrl.getCamera();
    ctrl.onMouseMove(500, 400);
    const after = ctrl.getCamera();
    expect(after.x).not.toBe(before.x);
    expect(after.y).not.toBe(before.y);
  });

  it('onMouseUp stops drag', () => {
    ctrl.onMouseDown(400, 300, 1);
    expect(ctrl.isDraggingCamera()).toBe(true);
    ctrl.onMouseUp(1);
    expect(ctrl.isDraggingCamera()).toBe(false);
  });

  // --- Mouse Wheel ---

  it('onWheel zooms in/out', () => {
    const before = ctrl.getCamera().zoom;
    ctrl.onWheel(-100, 400, 300); // scroll up → zoom in
    expect(ctrl.getCamera().zoom).toBeGreaterThan(before);

    const mid = ctrl.getCamera().zoom;
    ctrl.onWheel(100, 400, 300); // scroll down → zoom out
    expect(ctrl.getCamera().zoom).toBeLessThan(mid);
  });

  it('onWheel clamps zoom', () => {
    // Zoom in a lot
    for (let i = 0; i < 100; i++) ctrl.onWheel(-100, 400, 300);
    expect(ctrl.getCamera().zoom).toBe(DEFAULT_CAMERA_LIMITS.maxZoom);

    // Zoom out a lot
    for (let i = 0; i < 100; i++) ctrl.onWheel(100, 400, 300);
    expect(ctrl.getCamera().zoom).toBe(DEFAULT_CAMERA_LIMITS.minZoom);
  });

  // --- Pinch ---

  it('onPinch adjusts zoom', () => {
    const before = ctrl.getCamera().zoom;
    ctrl.onPinch(1.5);
    expect(ctrl.getCamera().zoom).toBe(Math.min(DEFAULT_CAMERA_LIMITS.maxZoom, before * 1.5));
  });

  // --- Edge Scroll ---

  it('computeEdgeScroll detects left edge', () => {
    const result = ctrl.computeEdgeScroll(5, 300);
    expect(result.dx).toBe(-1);
    expect(result.dy).toBe(0);
  });

  it('computeEdgeScroll detects right edge', () => {
    const result = ctrl.computeEdgeScroll(795, 300);
    expect(result.dx).toBe(1);
    expect(result.dy).toBe(0);
  });

  it('computeEdgeScroll returns zero when disabled', () => {
    ctrl.setEdgeScrollEnabled(false);
    const result = ctrl.computeEdgeScroll(5, 5);
    expect(result.dx).toBe(0);
    expect(result.dy).toBe(0);
  });

  // --- Viewport ---

  it('setViewport updates dimensions', () => {
    ctrl.setViewport(1920, 1080);
    const cam = ctrl.getCamera();
    expect(cam.viewportW).toBe(1920);
    expect(cam.viewportH).toBe(1080);
  });

  // --- Events ---

  it('addEventListener receives events', () => {
    const events: Array<{ type: CameraEventType; data: any }> = [];
    ctrl.addEventListener((type, data) => events.push({ type, data }));

    ctrl.centerOn(50, 50);
    expect(events.length).toBe(1);
    expect(events[0].type).toBe('cameraChanged');
    expect(events[0].data.x).toBe(50);
  });

  it('onMouseDown left button emits click', () => {
    const events: Array<{ type: CameraEventType; data: any }> = [];
    ctrl.addEventListener((type, data) => events.push({ type, data }));

    ctrl.onMouseDown(100, 200, 0);
    expect(events.length).toBe(1);
    expect(events[0].type).toBe('click');
    expect(events[0].data).toEqual({ screenX: 100, screenY: 200 });
  });
});
