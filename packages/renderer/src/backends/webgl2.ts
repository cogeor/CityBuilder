// @townbuilder/renderer — WebGL2 instanced sprite rendering backend

// ─── Shader Sources ────────────────────────────────────────────────────────

/** Instanced sprite vertex shader: transforms instance position + UV coords. */
export const VERTEX_SHADER_SRC = `#version 300 es
precision highp float;

// Per-vertex attributes (quad)
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;

// Per-instance attributes
layout(location = 2) in vec2 a_offset;     // screen-space offset (px)
layout(location = 3) in vec2 a_size;        // sprite size (px)
layout(location = 4) in vec4 a_uvRect;      // atlas UV rect: x, y, w, h
layout(location = 5) in vec4 a_tint;        // RGBA tint color

uniform vec2 u_resolution;

out vec2 v_texcoord;
out vec4 v_tint;

void main() {
    // Scale quad to sprite size and offset to screen position
    vec2 pos = a_position * a_size + a_offset;

    // Convert pixel coords to clip space [-1, 1]
    vec2 clipPos = (pos / u_resolution) * 2.0 - 1.0;
    clipPos.y = -clipPos.y; // flip Y for screen coords

    gl_Position = vec4(clipPos, 0.0, 1.0);

    // Map quad texcoord [0,1] to atlas UV rect
    v_texcoord = a_uvRect.xy + a_texcoord * a_uvRect.zw;
    v_tint = a_tint;
}
`;

/** Fragment shader: samples atlas texture and applies tint color. */
export const FRAGMENT_SHADER_SRC = `#version 300 es
precision highp float;

in vec2 v_texcoord;
in vec4 v_tint;

uniform sampler2D u_atlas;

out vec4 fragColor;

void main() {
    vec4 texel = texture(u_atlas, v_texcoord);
    fragColor = texel * v_tint;
}
`;

// ─── Interfaces ────────────────────────────────────────────────────────────

/** Rendering statistics for the current frame. */
export interface RenderStats {
  drawCalls: number;
  instancesDrawn: number;
  texturesUploaded: number;
  frameTime: number;
}

/** Abstract render backend interface for GPU-accelerated sprite rendering. */
export interface IRenderBackend {
  init(canvas: any): boolean;
  resize(width: number, height: number): void;
  beginFrame(): void;
  endFrame(): void;
  uploadInstances(buffer: ArrayBuffer, count: number): void;
  drawInstances(atlasId: number, offset: number, count: number): void;
  uploadTexture(atlasId: number, image: any): void;
  destroy(): void;
  isInitialized(): boolean;
  getStats(): RenderStats;
}

// ─── Mock Classes (for testing) ────────────────────────────────────────────

/** Tracks calls made to the mock GL context for test verification. */
export interface MockGLCallLog {
  createProgram: number;
  createShader: number;
  createBuffer: number;
  createTexture: number;
  bufferData: number;
  drawArraysInstanced: number;
  deleteBuffer: number;
  deleteTexture: number;
  deleteProgram: number;
  viewport: number;
  clear: number;
  useProgram: number;
  texImage2D: number;
  shaderSource: number;
  compileShader: number;
  attachShader: number;
  linkProgram: number;
}

/** Mock WebGL2RenderingContext for unit tests (no real GPU required). */
export class MockGL {
  public calls: MockGLCallLog = {
    createProgram: 0,
    createShader: 0,
    createBuffer: 0,
    createTexture: 0,
    bufferData: 0,
    drawArraysInstanced: 0,
    deleteBuffer: 0,
    deleteTexture: 0,
    deleteProgram: 0,
    viewport: 0,
    clear: 0,
    useProgram: 0,
    texImage2D: 0,
    shaderSource: 0,
    compileShader: 0,
    attachShader: 0,
    linkProgram: 0,
  };

  // WebGL constants
  readonly VERTEX_SHADER = 0x8B31;
  readonly FRAGMENT_SHADER = 0x8B30;
  readonly COMPILE_STATUS = 0x8B81;
  readonly LINK_STATUS = 0x8B82;
  readonly ARRAY_BUFFER = 0x8892;
  readonly DYNAMIC_DRAW = 0x88E8;
  readonly TRIANGLES = 0x0004;
  readonly COLOR_BUFFER_BIT = 0x4000;
  readonly DEPTH_BUFFER_BIT = 0x0100;
  readonly TEXTURE_2D = 0x0DE1;
  readonly TEXTURE0 = 0x84C0;
  readonly RGBA = 0x1908;
  readonly UNSIGNED_BYTE = 0x1401;
  readonly FLOAT = 0x1406;
  readonly TEXTURE_WRAP_S = 0x2802;
  readonly TEXTURE_WRAP_T = 0x2803;
  readonly TEXTURE_MIN_FILTER = 0x2801;
  readonly TEXTURE_MAG_FILTER = 0x2800;
  readonly CLAMP_TO_EDGE = 0x812F;
  readonly LINEAR = 0x2601;
  readonly BLEND = 0x0BE2;
  readonly SRC_ALPHA = 0x0302;
  readonly ONE_MINUS_SRC_ALPHA = 0x0303;

  createProgram(): object {
    this.calls.createProgram++;
    return { __mock: "program" };
  }

  createShader(_type: number): object {
    this.calls.createShader++;
    return { __mock: "shader" };
  }

  createBuffer(): object {
    this.calls.createBuffer++;
    return { __mock: "buffer" };
  }

  createTexture(): object {
    this.calls.createTexture++;
    return { __mock: "texture" };
  }

  shaderSource(_shader: any, _source: string): void {
    this.calls.shaderSource++;
  }

  compileShader(_shader: any): void {
    this.calls.compileShader++;
  }

  getShaderParameter(_shader: any, _pname: number): boolean {
    return true;
  }

  attachShader(_program: any, _shader: any): void {
    this.calls.attachShader++;
  }

  linkProgram(_program: any): void {
    this.calls.linkProgram++;
  }

  getProgramParameter(_program: any, _pname: number): boolean {
    return true;
  }

  useProgram(_program: any): void {
    this.calls.useProgram++;
  }

  getUniformLocation(_program: any, _name: string): object {
    return { __mock: "uniformLocation" };
  }

  getAttribLocation(_program: any, _name: string): number {
    return 0;
  }

  uniform2f(_location: any, _x: number, _y: number): void {}

  uniform1i(_location: any, _v: number): void {}

  bindBuffer(_target: number, _buffer: any): void {}

  bufferData(_target: number, _data: any, _usage: number): void {
    this.calls.bufferData++;
  }

  enableVertexAttribArray(_index: number): void {}

  vertexAttribPointer(
    _index: number,
    _size: number,
    _type: number,
    _normalized: boolean,
    _stride: number,
    _offset: number,
  ): void {}

  vertexAttribDivisor(_index: number, _divisor: number): void {}

  drawArraysInstanced(
    _mode: number,
    _first: number,
    _count: number,
    _instanceCount: number,
  ): void {
    this.calls.drawArraysInstanced++;
  }

  viewport(_x: number, _y: number, _width: number, _height: number): void {
    this.calls.viewport++;
  }

  clearColor(_r: number, _g: number, _b: number, _a: number): void {}

  clear(_mask: number): void {
    this.calls.clear++;
  }

  enable(_cap: number): void {}

  blendFunc(_sfactor: number, _dfactor: number): void {}

  activeTexture(_unit: number): void {}

  bindTexture(_target: number, _texture: any): void {}

  texImage2D(
    _target: number,
    _level: number,
    _internalformat: number,
    _format: number,
    _type: number,
    _source: any,
  ): void {
    this.calls.texImage2D++;
  }

  texParameteri(_target: number, _pname: number, _param: number): void {}

  deleteBuffer(_buffer: any): void {
    this.calls.deleteBuffer++;
  }

  deleteTexture(_texture: any): void {
    this.calls.deleteTexture++;
  }

  deleteProgram(_program: any): void {
    this.calls.deleteProgram++;
  }

  deleteShader(_shader: any): void {}
}

/** Mock canvas element for testing without a DOM. */
export class MockCanvas {
  public width: number;
  public height: number;
  private readonly mockGL: MockGL;
  private readonly supportWebGL2: boolean;

  constructor(
    width: number = 800,
    height: number = 600,
    supportWebGL2: boolean = true,
  ) {
    this.width = width;
    this.height = height;
    this.mockGL = new MockGL();
    this.supportWebGL2 = supportWebGL2;
  }

  getContext(type: string): MockGL | null {
    if (type === "webgl2" && this.supportWebGL2) {
      return this.mockGL;
    }
    return null;
  }
}

// ─── WebGL2Backend ─────────────────────────────────────────────────────────

/** Creates a fresh zeroed stats object. */
function createStats(): RenderStats {
  return {
    drawCalls: 0,
    instancesDrawn: 0,
    texturesUploaded: 0,
    frameTime: 0,
  };
}

/**
 * WebGL2 render backend for instanced sprite rendering.
 *
 * Uses instanced drawing (drawArraysInstanced) to efficiently render
 * large numbers of sprites in as few draw calls as possible. Each
 * draw call corresponds to one texture atlas batch.
 */
export class WebGL2Backend implements IRenderBackend {
  public gl: any = null;
  public program: any = null;
  public instanceBuffer: any = null;
  public textures: Map<number, any> = new Map();
  public stats: RenderStats = createStats();
  public initialized: boolean = false;

  private resolutionLocation: any = null;
  private frameStartTime: number = 0;
  private instanceCount: number = 0;

  /**
   * Initialize the WebGL2 context from a canvas element.
   * Creates the shader program and instance buffer.
   * Returns false if WebGL2 is not available.
   */
  init(canvas: any): boolean {
    const gl = canvas.getContext("webgl2");
    if (!gl) {
      return false;
    }

    this.gl = gl;

    // Create shader program
    this.program = this.createShaderProgram(gl);
    if (!this.program) {
      return false;
    }

    // Create instance buffer
    this.instanceBuffer = gl.createBuffer();

    // Get uniform locations
    gl.useProgram(this.program);
    this.resolutionLocation = gl.getUniformLocation(
      this.program,
      "u_resolution",
    );

    // Enable alpha blending
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    this.initialized = true;
    return true;
  }

  /**
   * Resize the viewport to match the canvas dimensions.
   */
  resize(width: number, height: number): void {
    if (!this.gl) return;
    this.gl.viewport(0, 0, width, height);
    if (this.resolutionLocation) {
      this.gl.useProgram(this.program);
      this.gl.uniform2f(this.resolutionLocation, width, height);
    }
  }

  /**
   * Begin a new frame: clear the screen and reset per-frame stats.
   */
  beginFrame(): void {
    if (!this.gl) return;
    this.frameStartTime = performance.now();
    this.stats = createStats();

    this.gl.clearColor(0.1, 0.1, 0.15, 1.0);
    this.gl.clear(this.gl.COLOR_BUFFER_BIT | this.gl.DEPTH_BUFFER_BIT);
  }

  /**
   * End the current frame: record frame timing.
   */
  endFrame(): void {
    this.stats.frameTime = performance.now() - this.frameStartTime;
  }

  /**
   * Upload instance data to the GPU instance buffer.
   */
  uploadInstances(buffer: ArrayBuffer, count: number): void {
    if (!this.gl || !this.instanceBuffer) return;

    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.instanceBuffer);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, buffer, this.gl.DYNAMIC_DRAW);
    this.instanceCount = count;
  }

  /**
   * Draw a batch of instanced sprites for a given atlas texture.
   */
  drawInstances(atlasId: number, offset: number, count: number): void {
    if (!this.gl || !this.program) return;

    // Bind atlas texture
    const texture = this.textures.get(atlasId);
    if (texture) {
      this.gl.activeTexture(this.gl.TEXTURE0);
      this.gl.bindTexture(this.gl.TEXTURE_2D, texture);
    }

    this.gl.useProgram(this.program);
    this.gl.drawArraysInstanced(this.gl.TRIANGLES, offset, 6, count);

    this.stats.drawCalls++;
    this.stats.instancesDrawn += count;
  }

  /**
   * Upload a texture atlas image to the GPU.
   */
  uploadTexture(atlasId: number, image: any): void {
    if (!this.gl) return;

    // Delete existing texture for this atlas if any
    const existing = this.textures.get(atlasId);
    if (existing) {
      this.gl.deleteTexture(existing);
    }

    const texture = this.gl.createTexture();
    this.gl.activeTexture(this.gl.TEXTURE0);
    this.gl.bindTexture(this.gl.TEXTURE_2D, texture);

    this.gl.texImage2D(
      this.gl.TEXTURE_2D,
      0,
      this.gl.RGBA,
      this.gl.RGBA,
      this.gl.UNSIGNED_BYTE,
      image,
    );

    // Set texture parameters for pixel art
    this.gl.texParameteri(
      this.gl.TEXTURE_2D,
      this.gl.TEXTURE_WRAP_S,
      this.gl.CLAMP_TO_EDGE,
    );
    this.gl.texParameteri(
      this.gl.TEXTURE_2D,
      this.gl.TEXTURE_WRAP_T,
      this.gl.CLAMP_TO_EDGE,
    );
    this.gl.texParameteri(
      this.gl.TEXTURE_2D,
      this.gl.TEXTURE_MIN_FILTER,
      this.gl.LINEAR,
    );
    this.gl.texParameteri(
      this.gl.TEXTURE_2D,
      this.gl.TEXTURE_MAG_FILTER,
      this.gl.LINEAR,
    );

    this.textures.set(atlasId, texture);
    this.stats.texturesUploaded++;
  }

  /**
   * Clean up all GPU resources: buffers, textures, and shader program.
   */
  destroy(): void {
    if (!this.gl) return;

    if (this.instanceBuffer) {
      this.gl.deleteBuffer(this.instanceBuffer);
      this.instanceBuffer = null;
    }

    for (const [, texture] of this.textures) {
      this.gl.deleteTexture(texture);
    }
    this.textures.clear();

    if (this.program) {
      this.gl.deleteProgram(this.program);
      this.program = null;
    }

    this.initialized = false;
    this.gl = null;
  }

  /**
   * Returns whether the backend has been successfully initialized.
   */
  isInitialized(): boolean {
    return this.initialized;
  }

  /**
   * Returns a snapshot of the current rendering statistics.
   */
  getStats(): RenderStats {
    return { ...this.stats };
  }

  // ─── Private Helpers ───────────────────────────────────────────────────

  private createShaderProgram(gl: any): any {
    const vertexShader = gl.createShader(gl.VERTEX_SHADER);
    gl.shaderSource(vertexShader, VERTEX_SHADER_SRC);
    gl.compileShader(vertexShader);

    if (!gl.getShaderParameter(vertexShader, gl.COMPILE_STATUS)) {
      return null;
    }

    const fragmentShader = gl.createShader(gl.FRAGMENT_SHADER);
    gl.shaderSource(fragmentShader, FRAGMENT_SHADER_SRC);
    gl.compileShader(fragmentShader);

    if (!gl.getShaderParameter(fragmentShader, gl.COMPILE_STATUS)) {
      return null;
    }

    const program = gl.createProgram();
    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      return null;
    }

    // Shaders can be deleted after linking
    gl.deleteShader(vertexShader);
    gl.deleteShader(fragmentShader);

    return program;
  }
}
