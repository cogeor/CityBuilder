//! Core wgpu renderer — window, device, pipeline, and render loop.

use crate::instance::{GpuInstance, Uniforms};
use crate::projection;
use crate::tile_visuals::{GpuPattern, TileVisualRegistry, MAX_PATTERNS};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::*;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Camera state for panning around the map.
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
    pub zoom: f32,
    pub keys_held: [bool; 4], // W, A, S, D
}

impl Camera {
    fn new(center_x: f32, center_y: f32) -> Self {
        Self {
            x: center_x,
            y: center_y,
            speed: 400.0,
            zoom: 1.0,
            keys_held: [false; 4],
        }
    }

    fn update(&mut self, dt: f32) {
        if self.keys_held[0] { self.y -= self.speed * dt; } // W
        if self.keys_held[1] { self.x -= self.speed * dt; } // A
        if self.keys_held[2] { self.y += self.speed * dt; } // S
        if self.keys_held[3] { self.x += self.speed * dt; } // D
    }
}

/// Build GPU instances from a flat slice of tile data.
///
/// Each element is `(x, y, pattern_id)`. `max_dim` is the maximum of map width
/// and height, used for z-ordering.
pub fn build_terrain_instances(tiles: &[(i16, i16, u32)], max_dim: u16) -> Vec<GpuInstance> {
    let mut instances = Vec::with_capacity(tiles.len());

    for &(x, y, pattern_id) in tiles {
        let (sx, sy) = projection::tile_to_screen(x, y);
        let z = projection::tile_z_order(x, y, max_dim);
        instances.push(GpuInstance::new(sx, sy, z, pattern_id));
    }

    instances
}

/// All GPU state.
struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_buffer_capacity: usize,
    uniform_buffer: wgpu::Buffer,
    pattern_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    num_indices: u32,
    instance_count: u32,
    size: PhysicalSize<u32>,
}

/// The isometric diamond quad — 4 vertices forming a diamond shape.
fn diamond_vertices() -> [f32; 8] {
    [
        0.0, -0.5,  // top
        0.5,  0.0,  // right
        0.0,  0.5,  // bottom
       -0.5,  0.0,  // left
    ]
}

fn diamond_indices() -> [u16; 6] {
    [0, 1, 2, 0, 2, 3] // two triangles
}

fn create_render_state(
    window: Arc<Window>,
    instances: &[GpuInstance],
    visuals: &TileVisualRegistry,
) -> RenderState {
    let size = window.inner_size();

    let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = wgpu_instance.create_surface(window).unwrap();

    let adapter = pollster::block_on(
        wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }),
    ).expect("Failed to find GPU adapter");

    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("city_render"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }, None),
    ).expect("Failed to create GPU device");

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats.iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("terrain_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain.wgsl").into()),
    });

    // Vertex buffer (diamond quad)
    let vertices = diamond_vertices();
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Index buffer
    let indices = diamond_indices();
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index_buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Instance buffer (writable for live updates)
    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("instance_buffer"),
        contents: bytemuck::cast_slice(instances),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    // Uniform buffer
    let uniforms = Uniforms::ortho(size.width as f32, size.height as f32, 0.0, 0.0);
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Pattern uniform buffer
    let pattern_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("pattern_buffer"),
        contents: bytemuck::cast_slice(visuals.as_gpu_array()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Bind group layout: binding 0 = uniforms (vertex), binding 1 = patterns (fragment)
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind_group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: pattern_buffer.as_entire_binding(),
            },
        ],
    });

    // Pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline_layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Vertex buffer layouts
    let vertex_layout = wgpu::VertexBufferLayout {
        array_stride: 8, // 2 * f32
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        }],
    };

    let instance_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GpuInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            // screen_pos: vec2<f32>
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            // z_order: f32
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32,
            },
            // pattern_id: u32
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 3,
                format: wgpu::VertexFormat::Uint32,
            },
        ],
    };

    // Render pipeline
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terrain_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout, instance_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    RenderState {
        surface,
        device,
        queue,
        config,
        pipeline,
        vertex_buffer,
        index_buffer,
        instance_buffer,
        instance_buffer_capacity: instances.len(),
        uniform_buffer,
        pattern_buffer,
        bind_group,
        bind_group_layout,
        num_indices: indices.len() as u32,
        instance_count: instances.len() as u32,
        size,
    }
}

impl RenderState {
    /// Update the instance buffer with new data. Recreates if capacity changed.
    fn update_instances(&mut self, instances: &[GpuInstance]) {
        self.instance_count = instances.len() as u32;
        if instances.len() <= self.instance_buffer_capacity {
            self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
        } else {
            self.instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.instance_buffer_capacity = instances.len();
        }
    }
}

/// Application state for winit event loop.
pub struct IsometricApp {
    state: Option<RenderState>,
    window: Option<Arc<Window>>,
    instances: Vec<GpuInstance>,
    visuals: TileVisualRegistry,
    camera: Camera,
    last_frame: std::time::Instant,
}

impl IsometricApp {
    pub fn new(instances: Vec<GpuInstance>, cam_x: f32, cam_y: f32, cam_speed: f32, zoom: f32) -> Self {
        let mut camera = Camera::new(cam_x, cam_y);
        camera.speed = cam_speed;
        camera.zoom = zoom;
        Self {
            state: None,
            window: None,
            instances,
            visuals: TileVisualRegistry::new(),
            camera,
            last_frame: std::time::Instant::now(),
        }
    }

    /// Update the instance buffer with new tile data.
    pub fn update_instances(&mut self, instances: Vec<GpuInstance>) {
        self.instances = instances;
        if let Some(state) = self.state.as_mut() {
            state.update_instances(&self.instances);
        }
    }

    fn render(&mut self) {
        let state = self.state.as_ref().unwrap();

        // Update camera
        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.camera.update(dt);

        // Update uniforms
        let uniforms = Uniforms::ortho_zoom(
            state.size.width as f32,
            state.size.height as f32,
            self.camera.x,
            self.camera.y,
            self.camera.zoom,
        );
        state.queue.write_buffer(&state.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let output = match state.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost) => { return; }
            Err(wgpu::SurfaceError::OutOfMemory) => { panic!("GPU out of memory"); }
            Err(_) => { return; }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("render_encoder") },
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terrain_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08, g: 0.08, b: 0.12, a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&state.pipeline);
            pass.set_bind_group(0, &state.bind_group, &[]);
            pass.set_vertex_buffer(0, state.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
            pass.set_index_buffer(state.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..state.num_indices, 0, 0..state.instance_count);
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Request next frame
        self.window.as_ref().unwrap().request_redraw();
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let Some(state) = self.state.as_mut() {
                state.size = new_size;
                state.config.width = new_size.width;
                state.config.height = new_size.height;
                state.surface.configure(&state.device, &state.config);
            }
        }
    }
}

impl ApplicationHandler for IsometricApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("City Builder — Isometric Terrain")
                .with_inner_size(PhysicalSize::new(1280u32, 720u32));
            let window = Arc::new(event_loop.create_window(attrs).unwrap());
            let state = create_render_state(window.clone(), &self.instances, &self.visuals);
            self.state = Some(state);
            self.window = Some(window);
            self.last_frame = std::time::Instant::now();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: key_state,
                    ..
                },
                ..
            } => {
                let pressed = key_state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW | KeyCode::ArrowUp => self.camera.keys_held[0] = pressed,
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.camera.keys_held[1] = pressed,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.camera.keys_held[2] = pressed,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.camera.keys_held[3] = pressed,
                    KeyCode::Escape => event_loop.exit(),
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
                };
                let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                self.camera.zoom = (self.camera.zoom * factor).clamp(1.0, 200.0);
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }
}

/// Run the isometric renderer. Blocks until window is closed.
pub fn run(instances: Vec<GpuInstance>, cam_x: f32, cam_y: f32) {
    run_with_options(instances, cam_x, cam_y, 400.0, 1.0);
}

/// Run the isometric renderer with custom camera speed and zoom.
pub fn run_with_options(instances: Vec<GpuInstance>, cam_x: f32, cam_y: f32, cam_speed: f32, zoom: f32) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = IsometricApp::new(instances, cam_x, cam_y, cam_speed, zoom);
    event_loop.run_app(&mut app).unwrap();
}
