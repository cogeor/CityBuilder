// @townbuilder/runtime — Render worker: OffscreenCanvas render loop
// Manages GPU context detection, camera state, instance buffers,
// and the requestAnimationFrame render loop in a Web Worker.

import { MessageType } from "../messaging/types.js";

// ---- RenderWorkerState Enum ----

/** Render worker lifecycle states. */
export enum RenderWorkerState {
  Uninitialized = "uninitialized",
  Initializing = "initializing",
  Ready = "ready",
  Running = "running",
  Paused = "paused",
  Error = "error",
}

// ---- GPUBackendType Enum ----

/** GPU backend type detected at runtime. */
export enum GPUBackendType {
  WebGPU = "webgpu",
  WebGL2 = "webgl2",
  None = "none",
}

// ---- Interfaces ----

/** Camera state for rendering. */
export interface RenderCamera {
  x: number;
  y: number;
  zoom: number;
  viewportW: number;
  viewportH: number;
}

/** Render worker configuration. */
export interface RenderWorkerConfig {
  preferredBackend: GPUBackendType;
  targetFps: number;
  enableVSync: boolean;
  maxChunkRebuildsPerFrame: number;
}

/** Pick request — returns entity at screen coordinate. */
export interface PickRequest {
  id: number;
  screenX: number;
  screenY: number;
}

/** Pick result. */
export interface PickResult {
  requestId: number;
  tileX: number;
  tileY: number;
  entityId: number; // 0 if no entity
}

/** Render frame stats. */
export interface RenderFrameStats {
  fps: number;
  frameTimeMs: number;
  drawCalls: number;
  instanceCount: number;
  backend: GPUBackendType;
}

// ---- Constants ----

/** Default render worker configuration. */
export const DEFAULT_RENDER_CONFIG: RenderWorkerConfig = {
  preferredBackend: GPUBackendType.WebGPU,
  targetFps: 60,
  enableVSync: true,
  maxChunkRebuildsPerFrame: 4,
};

// ---- RenderWorker Class ----

/**
 * RenderWorker manages the render loop on an OffscreenCanvas.
 *
 * Lifecycle:
 * 1. Receive OffscreenCanvas via transferControlToOffscreen
 * 2. Detect GPU backend (WebGPU -> WebGL2 fallback)
 * 3. Load atlas textures and metadata
 * 4. Enter render loop (requestAnimationFrame)
 * 5. Each frame: consume buffers -> update camera -> draw
 *
 * The class is decoupled from the global `self` / `postMessage` so it can be
 * tested without a real Worker environment.
 */
export class RenderWorker {
  private state: RenderWorkerState;
  private config: RenderWorkerConfig;
  private canvas: OffscreenCanvas | null;
  private backend: GPUBackendType;
  private camera: RenderCamera;
  private frameCount: number;
  private lastFrameTime: number;
  private currentFps: number;
  private animFrameId: number;
  private pendingPicks: PickRequest[];
  private instanceBuffer: Float32Array | null;
  private instanceCount: number;
  private onMessage: ((msg: any) => void) | null;

  constructor(config?: Partial<RenderWorkerConfig>) {
    this.state = RenderWorkerState.Uninitialized;
    this.config = { ...DEFAULT_RENDER_CONFIG, ...config };
    this.canvas = null;
    this.backend = GPUBackendType.None;
    this.camera = { x: 0, y: 0, zoom: 1, viewportW: 800, viewportH: 600 };
    this.frameCount = 0;
    this.lastFrameTime = 0;
    this.currentFps = 0;
    this.animFrameId = 0;
    this.pendingPicks = [];
    this.instanceBuffer = null;
    this.instanceCount = 0;
    this.onMessage = null;
  }

  // ---- Getters ----

  /** Get current state. */
  getState(): RenderWorkerState {
    return this.state;
  }

  /** Get detected backend. */
  getBackend(): GPUBackendType {
    return this.backend;
  }

  /** Get current camera (returns a copy). */
  getCamera(): RenderCamera {
    return { ...this.camera };
  }

  /** Get frame count. */
  getFrameCount(): number {
    return this.frameCount;
  }

  /** Get current FPS. */
  getFps(): number {
    return this.currentFps;
  }

  // ---- Initialization ----

  /**
   * Initialize with an OffscreenCanvas.
   * Detects available GPU backend.
   */
  async initialize(canvas: OffscreenCanvas): Promise<GPUBackendType> {
    this.state = RenderWorkerState.Initializing;
    this.canvas = canvas;

    // Detect backend
    this.backend = await this.detectBackend();

    if (this.backend === GPUBackendType.None) {
      this.state = RenderWorkerState.Error;
      throw new Error("No suitable GPU backend available");
    }

    this.state = RenderWorkerState.Ready;
    return this.backend;
  }

  /**
   * Detect available GPU backend.
   * Tries WebGPU first if preferred, falls back to WebGL2.
   */
  private async detectBackend(): Promise<GPUBackendType> {
    if (this.config.preferredBackend === GPUBackendType.WebGPU) {
      // Check for WebGPU support (navigator.gpu)
      if (typeof navigator !== "undefined" && "gpu" in navigator) {
        try {
          const adapter = await (navigator as any).gpu.requestAdapter();
          if (adapter) return GPUBackendType.WebGPU;
        } catch {
          /* fall through */
        }
      }
    }

    // Try WebGL2
    if (this.canvas) {
      try {
        const ctx = this.canvas.getContext("webgl2");
        if (ctx) return GPUBackendType.WebGL2;
      } catch {
        /* fall through */
      }
    }

    return GPUBackendType.None;
  }

  // ---- Render Loop Control ----

  /** Start the render loop. */
  start(): void {
    if (
      this.state !== RenderWorkerState.Ready &&
      this.state !== RenderWorkerState.Paused
    ) {
      throw new Error(`Cannot start from state: ${this.state}`);
    }
    this.state = RenderWorkerState.Running;
    this.scheduleFrame();
  }

  /** Pause the render loop. */
  pause(): void {
    if (this.state !== RenderWorkerState.Running) return;
    this.state = RenderWorkerState.Paused;
    if (this.animFrameId) {
      cancelAnimationFrame(this.animFrameId);
      this.animFrameId = 0;
    }
  }

  /** Resume after pause. */
  resume(): void {
    if (this.state !== RenderWorkerState.Paused) return;
    this.state = RenderWorkerState.Running;
    this.scheduleFrame();
  }

  // ---- State Updates ----

  /** Update camera state. */
  updateCamera(camera: Partial<RenderCamera>): void {
    Object.assign(this.camera, camera);
  }

  /** Submit new instance buffer from sim. */
  updateInstances(buffer: Float32Array, count: number): void {
    this.instanceBuffer = buffer;
    this.instanceCount = count;
  }

  /** Submit a pick request. */
  submitPick(request: PickRequest): void {
    this.pendingPicks.push(request);
  }

  /** Set message callback for outgoing messages. */
  setMessageHandler(handler: (msg: any) => void): void {
    this.onMessage = handler;
  }

  // ---- Message Router ----

  /**
   * Handle incoming message from main thread.
   */
  handleMessage(msg: any): void {
    if (!msg || !msg.type) return;

    switch (msg.type) {
      case "init":
        // Canvas initialization handled separately
        break;
      case "camera":
        if (msg.camera) this.updateCamera(msg.camera);
        break;
      case "instances":
        if (msg.buffer && typeof msg.count === "number") {
          this.updateInstances(msg.buffer, msg.count);
        }
        break;
      case "pick":
        if (msg.request) this.submitPick(msg.request);
        break;
      case "pause":
        this.pause();
        break;
      case "resume":
        this.resume();
        break;
      case "config":
        if (msg.config) Object.assign(this.config, msg.config);
        break;
      default:
        break;
    }
  }

  // ---- Frame Scheduling ----

  /** Schedule next animation frame. */
  private scheduleFrame(): void {
    if (this.state !== RenderWorkerState.Running) return;
    // In test environments, requestAnimationFrame may not exist
    if (typeof requestAnimationFrame === "function") {
      this.animFrameId = requestAnimationFrame(() => this.renderFrame());
    }
  }

  /** Execute one render frame. */
  private renderFrame(): void {
    if (this.state !== RenderWorkerState.Running) return;

    const now = performance.now();
    const deltaMs = now - this.lastFrameTime;
    this.lastFrameTime = now;
    this.frameCount++;

    // Update FPS
    if (deltaMs > 0) {
      this.currentFps = Math.round(1000 / deltaMs);
    }

    // Process pick requests
    this.processPicks();

    // Send stats periodically (every 60 frames)
    if (this.frameCount % 60 === 0 && this.onMessage) {
      this.onMessage({
        type: "stats",
        stats: this.getStats(),
      });
    }

    this.scheduleFrame();
  }

  // ---- Pick Processing ----

  /** Process pending pick requests. */
  private processPicks(): void {
    for (const pick of this.pendingPicks) {
      // Simple screen-to-tile conversion (placeholder)
      const result: PickResult = {
        requestId: pick.id,
        tileX: Math.floor((pick.screenX + this.camera.x) / 128),
        tileY: Math.floor((pick.screenY + this.camera.y) / 64),
        entityId: 0,
      };
      if (this.onMessage) {
        this.onMessage({ type: "pickResult", result });
      }
    }
    this.pendingPicks = [];
  }

  // ---- Stats ----

  /** Get current frame stats. */
  getStats(): RenderFrameStats {
    return {
      fps: this.currentFps,
      frameTimeMs:
        this.lastFrameTime > 0 ? performance.now() - this.lastFrameTime : 0,
      drawCalls: 0,
      instanceCount: this.instanceCount,
      backend: this.backend,
    };
  }

  // ---- Shutdown ----

  /** Shutdown and clean up. */
  shutdown(): void {
    this.pause();
    this.state = RenderWorkerState.Uninitialized;
    this.canvas = null;
    this.instanceBuffer = null;
    this.instanceCount = 0;
    this.pendingPicks = [];
  }
}

// ---- Worker Bootstrap ----

// When running in a real Web Worker context, wire up the message handler.
// This block is guarded so it doesn't execute during unit tests.
declare const self: any;

if (
  typeof self !== "undefined" &&
  typeof self.postMessage === "function" &&
  typeof self.addEventListener === "function"
) {
  const worker = new RenderWorker();
  worker.setMessageHandler((msg: any) => self.postMessage(msg));

  self.addEventListener("message", (event: MessageEvent) => {
    const data = event.data;

    // Special handling for canvas transfer
    if (data && data.type === "init" && data.canvas instanceof OffscreenCanvas) {
      worker
        .initialize(data.canvas)
        .then((backend) => {
          self.postMessage({ type: "initialized", backend });
          worker.start();
        })
        .catch((err: Error) => {
          self.postMessage({ type: "error", error: err.message });
        });
      return;
    }

    worker.handleMessage(data);
  });
}
