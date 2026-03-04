/** Camera state (matches renderer CameraState) */
export interface CameraState {
  x: number;         // center X in tile coords
  y: number;         // center Y in tile coords
  zoom: number;      // zoom level (0.5 to 4.0)
  viewportW: number; // viewport width in pixels
  viewportH: number; // viewport height in pixels
}

/** Input binding configuration */
export interface KeyBindings {
  panUp: string[];     // default ['w', 'ArrowUp']
  panDown: string[];   // default ['s', 'ArrowDown']
  panLeft: string[];   // default ['a', 'ArrowLeft']
  panRight: string[];  // default ['d', 'ArrowRight']
  zoomIn: string[];    // default ['+', '=']
  zoomOut: string[];   // default ['-']
  rotateLeft: string[];  // default ['q']
  rotateRight: string[]; // default ['e']
  pause: string[];     // default [' '] (space)
  speed1: string[];    // default ['1']
  speed2: string[];    // default ['2']
  speed3: string[];    // default ['3']
}

/** Camera limits */
export interface CameraLimits {
  minZoom: number;   // default 0.25
  maxZoom: number;   // default 4.0
  minX: number;      // default 0
  maxX: number;      // map width in tiles
  minY: number;      // default 0
  maxY: number;      // map height in tiles
  panSpeed: number;  // tiles per second at zoom 1.0
  zoomSpeed: number; // zoom change per wheel tick
  edgeScrollSize: number; // pixels from edge to trigger scroll
  edgeScrollSpeed: number; // tiles per second for edge scroll
}

/** Camera event types */
export type CameraEventType = 'cameraChanged' | 'speedChange' | 'click' | 'rightClick';
export type CameraEventHandler = (type: CameraEventType, data: any) => void;

export const DEFAULT_KEY_BINDINGS: KeyBindings = {
  panUp: ['w', 'ArrowUp'],
  panDown: ['s', 'ArrowDown'],
  panLeft: ['a', 'ArrowLeft'],
  panRight: ['d', 'ArrowRight'],
  zoomIn: ['+', '='],
  zoomOut: ['-'],
  rotateLeft: ['q'],
  rotateRight: ['e'],
  pause: [' '],
  speed1: ['1'],
  speed2: ['2'],
  speed3: ['3'],
};

export const DEFAULT_CAMERA_LIMITS: CameraLimits = {
  minZoom: 0.25,
  maxZoom: 4.0,
  minX: 0,
  maxX: 256,
  minY: 0,
  maxY: 256,
  panSpeed: 20,
  zoomSpeed: 0.15,
  edgeScrollSize: 20,
  edgeScrollSpeed: 15,
};

/**
 * CameraController — handles all camera input (keyboard, mouse, touch).
 */
export class CameraController {
  private camera: CameraState;
  private limits: CameraLimits;
  private bindings: KeyBindings;
  private keysDown: Set<string>;
  private isDragging: boolean;
  private dragStartX: number;
  private dragStartY: number;
  private dragCameraStartX: number;
  private dragCameraStartY: number;
  private eventHandlers: CameraEventHandler[];
  private edgeScrollEnabled: boolean;

  constructor(
    viewport: { width: number; height: number },
    mapSize?: { width: number; height: number },
    limits?: Partial<CameraLimits>,
    bindings?: Partial<KeyBindings>,
  ) {
    this.camera = {
      x: (mapSize?.width ?? 256) / 2,
      y: (mapSize?.height ?? 256) / 2,
      zoom: 1.0,
      viewportW: viewport.width,
      viewportH: viewport.height,
    };
    this.limits = {
      ...DEFAULT_CAMERA_LIMITS,
      maxX: mapSize?.width ?? 256,
      maxY: mapSize?.height ?? 256,
      ...limits,
    };
    this.bindings = { ...DEFAULT_KEY_BINDINGS, ...bindings };
    this.keysDown = new Set();
    this.isDragging = false;
    this.dragStartX = 0;
    this.dragStartY = 0;
    this.dragCameraStartX = 0;
    this.dragCameraStartY = 0;
    this.eventHandlers = [];
    this.edgeScrollEnabled = true;
  }

  // --- Getters ---
  getCamera(): CameraState { return { ...this.camera }; }
  getLimits(): CameraLimits { return { ...this.limits }; }
  isDraggingCamera(): boolean { return this.isDragging; }

  // --- Direct Camera Control ---
  setCamera(camera: Partial<CameraState>): void {
    Object.assign(this.camera, camera);
    this.clampCamera();
    this.emit('cameraChanged', this.getCamera());
  }

  centerOn(tileX: number, tileY: number): void {
    this.camera.x = tileX;
    this.camera.y = tileY;
    this.clampCamera();
    this.emit('cameraChanged', this.getCamera());
  }

  setZoom(zoom: number): void {
    this.camera.zoom = Math.max(this.limits.minZoom, Math.min(this.limits.maxZoom, zoom));
    this.emit('cameraChanged', this.getCamera());
  }

  // --- Keyboard Input ---
  onKeyDown(key: string): void {
    this.keysDown.add(key);
  }

  onKeyUp(key: string): void {
    this.keysDown.delete(key);
  }

  isKeyDown(key: string): boolean {
    return this.keysDown.has(key);
  }

  /** Check if any key in a binding group is pressed */
  isBindingActive(keys: string[]): boolean {
    return keys.some(k => this.keysDown.has(k));
  }

  // --- Mouse Input ---
  onMouseDown(screenX: number, screenY: number, button: number): void {
    if (button === 1 || button === 2) {
      // Middle or right mouse button → start camera drag
      this.isDragging = true;
      this.dragStartX = screenX;
      this.dragStartY = screenY;
      this.dragCameraStartX = this.camera.x;
      this.dragCameraStartY = this.camera.y;
    } else if (button === 0) {
      this.emit('click', { screenX, screenY });
    }
  }

  onMouseMove(screenX: number, screenY: number): void {
    if (this.isDragging) {
      const dx = (screenX - this.dragStartX) / (64 * this.camera.zoom);
      const dy = (screenY - this.dragStartY) / (32 * this.camera.zoom);
      this.camera.x = this.dragCameraStartX - dx;
      this.camera.y = this.dragCameraStartY - dy;
      this.clampCamera();
      this.emit('cameraChanged', this.getCamera());
    }
  }

  onMouseUp(button: number): void {
    if (button === 1 || button === 2) {
      this.isDragging = false;
    }
  }

  onRightClick(screenX: number, screenY: number): void {
    this.emit('rightClick', { screenX, screenY });
  }

  // --- Mouse Wheel ---
  onWheel(deltaY: number, screenX: number, screenY: number): void {
    const zoomDir = deltaY > 0 ? -1 : 1;
    const newZoom = this.camera.zoom + zoomDir * this.limits.zoomSpeed;
    this.camera.zoom = Math.max(this.limits.minZoom, Math.min(this.limits.maxZoom, newZoom));
    this.emit('cameraChanged', this.getCamera());
  }

  // --- Touch Input ---
  onPinch(scaleFactor: number): void {
    const newZoom = this.camera.zoom * scaleFactor;
    this.camera.zoom = Math.max(this.limits.minZoom, Math.min(this.limits.maxZoom, newZoom));
    this.emit('cameraChanged', this.getCamera());
  }

  // --- Edge Scrolling ---
  setEdgeScrollEnabled(enabled: boolean): void {
    this.edgeScrollEnabled = enabled;
  }

  /** Check mouse position for edge scrolling. Returns pan delta. */
  computeEdgeScroll(mouseX: number, mouseY: number): { dx: number; dy: number } {
    if (!this.edgeScrollEnabled) return { dx: 0, dy: 0 };

    let dx = 0;
    let dy = 0;
    const edge = this.limits.edgeScrollSize;

    if (mouseX < edge) dx = -1;
    else if (mouseX > this.camera.viewportW - edge) dx = 1;

    if (mouseY < edge) dy = -1;
    else if (mouseY > this.camera.viewportH - edge) dy = 1;

    return { dx, dy };
  }

  // --- Frame Update ---
  /**
   * Update camera based on current input state.
   * Call once per frame with delta time in seconds.
   */
  update(deltaSeconds: number): boolean {
    let changed = false;
    const speed = this.limits.panSpeed * deltaSeconds / this.camera.zoom;

    if (this.isBindingActive(this.bindings.panUp)) {
      this.camera.y -= speed;
      changed = true;
    }
    if (this.isBindingActive(this.bindings.panDown)) {
      this.camera.y += speed;
      changed = true;
    }
    if (this.isBindingActive(this.bindings.panLeft)) {
      this.camera.x -= speed;
      changed = true;
    }
    if (this.isBindingActive(this.bindings.panRight)) {
      this.camera.x += speed;
      changed = true;
    }

    // Keyboard zoom
    if (this.isBindingActive(this.bindings.zoomIn)) {
      this.camera.zoom = Math.min(this.limits.maxZoom, this.camera.zoom + this.limits.zoomSpeed * deltaSeconds * 3);
      changed = true;
    }
    if (this.isBindingActive(this.bindings.zoomOut)) {
      this.camera.zoom = Math.max(this.limits.minZoom, this.camera.zoom - this.limits.zoomSpeed * deltaSeconds * 3);
      changed = true;
    }

    if (changed) {
      this.clampCamera();
      this.emit('cameraChanged', this.getCamera());
    }

    return changed;
  }

  // --- Viewport ---
  setViewport(width: number, height: number): void {
    this.camera.viewportW = width;
    this.camera.viewportH = height;
    this.emit('cameraChanged', this.getCamera());
  }

  // --- Clamping ---
  private clampCamera(): void {
    this.camera.x = Math.max(this.limits.minX, Math.min(this.limits.maxX, this.camera.x));
    this.camera.y = Math.max(this.limits.minY, Math.min(this.limits.maxY, this.camera.y));
    this.camera.zoom = Math.max(this.limits.minZoom, Math.min(this.limits.maxZoom, this.camera.zoom));
  }

  // --- Events ---
  addEventListener(handler: CameraEventHandler): void {
    this.eventHandlers.push(handler);
  }

  removeEventListener(handler: CameraEventHandler): void {
    const idx = this.eventHandlers.indexOf(handler);
    if (idx >= 0) this.eventHandlers.splice(idx, 1);
  }

  private emit(type: CameraEventType, data: any): void {
    for (const handler of this.eventHandlers) handler(type, data);
  }
}
