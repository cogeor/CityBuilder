//! Core wgpu renderer — two-pass: terrain diamonds + building sprites.

use crate::instance::{GpuInstance, SpriteInstance, Uniforms};
use crate::projection;
use crate::sprites;
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
    pub keys_held: [bool; 4],
}

impl Camera {
    fn new(center_x: f32, center_y: f32) -> Self {
        Self { x: center_x, y: center_y, speed: 400.0, zoom: 1.0, keys_held: [false; 4] }
    }

    fn update(&mut self, dt: f32) {
        if self.keys_held[0] { self.y -= self.speed * dt; }
        if self.keys_held[1] { self.x -= self.speed * dt; }
        if self.keys_held[2] { self.y += self.speed * dt; }
        if self.keys_held[3] { self.x += self.speed * dt; }
    }
}

/// Build GPU terrain instances from tile data.
pub fn build_terrain_instances(tiles: &[(i16, i16, u32)], max_dim: u16) -> Vec<GpuInstance> {
    tiles.iter().map(|&(x, y, color_id)| {
        let (sx, sy) = projection::tile_to_screen(x, y);
        let z = projection::tile_z_order(x, y, max_dim);
        GpuInstance::new(sx, sy, z, color_id)
    }).collect()
}

/// Build sprite instances from building data.
///
/// Each element: `(tile_x, tile_y, sprite_id)`.
/// sprite_id: 1=house, 2=shop, 3=factory, 4=civic
pub fn build_sprite_instances(buildings: &[(i16, i16, u32)], max_dim: u16) -> Vec<SpriteInstance> {
    buildings.iter().map(|&(x, y, sprite_id)| {
        let (sx, sy) = projection::tile_to_screen(x, y);
        let z = projection::tile_z_order(x, y, max_dim);
        let uv = sprites::sprite_uvs(sprite_id);
        SpriteInstance::new(sx, sy, z, uv, sprites::SPRITE_W as f32, sprites::SPRITE_H as f32)
    }).collect()
}

// ─── Render state ────────────────────────────────────────────────────────────

/// The isometric diamond quad.
fn diamond_vertices() -> [f32; 8] {
    [0.0, -0.5, 0.5, 0.0, 0.0, 0.5, -0.5, 0.0]
}

fn diamond_indices() -> [u16; 6] {
    [0, 1, 2, 0, 2, 3]
}

/// Unit quad for sprites: (0,0) to (1,1).
fn quad_vertices() -> [f32; 8] {
    [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
}

fn quad_indices() -> [u16; 6] {
    [0, 1, 2, 0, 2, 3]
}

struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    // Shared
    uniform_buffer: wgpu::Buffer,
    depth_texture: wgpu::TextureView,

    // Terrain pass
    terrain_pipeline: wgpu::RenderPipeline,
    terrain_vertex_buffer: wgpu::Buffer,
    terrain_index_buffer: wgpu::Buffer,
    terrain_instance_buffer: wgpu::Buffer,
    terrain_instance_capacity: usize,
    terrain_instance_count: u32,
    terrain_bind_group: wgpu::BindGroup,

    // Sprite pass
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_vertex_buffer: wgpu::Buffer,
    sprite_index_buffer: wgpu::Buffer,
    sprite_instance_buffer: wgpu::Buffer,
    sprite_instance_capacity: usize,
    sprite_instance_count: u32,
    sprite_bind_group: wgpu::BindGroup,
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn create_render_state(
    window: Arc<Window>,
    terrain_instances: &[GpuInstance],
    sprite_instances: &[SpriteInstance],
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
    ).expect("No GPU adapter");

    let (device, queue) = pollster::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("city_render"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }, None),
    ).expect("No GPU device");

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps.formats.iter()
        .find(|f| f.is_srgb()).copied()
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

    // Uniform buffer (shared)
    let uniforms = Uniforms::ortho(size.width as f32, size.height as f32, 0.0, 0.0);
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Depth texture
    let depth_texture = create_depth_texture(&device, size.width, size.height);

    // ── Terrain pipeline ─────────────────────────────────────────────────
    let terrain_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("terrain_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/terrain.wgsl").into()),
    });

    let terrain_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("terrain_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let terrain_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("terrain_bg"),
        layout: &terrain_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let terrain_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("terrain_pl"),
        bind_group_layouts: &[&terrain_bind_group_layout],
        push_constant_ranges: &[],
    });

    let terrain_vertex_layout = wgpu::VertexBufferLayout {
        array_stride: 8,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 }],
    };

    let terrain_instance_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<GpuInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
            wgpu::VertexAttribute { offset: 8, shader_location: 2, format: wgpu::VertexFormat::Float32 },
            wgpu::VertexAttribute { offset: 12, shader_location: 3, format: wgpu::VertexFormat::Uint32 },
        ],
    };

    let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("terrain_pipeline"),
        layout: Some(&terrain_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &terrain_shader,
            entry_point: Some("vs_main"),
            buffers: &[terrain_vertex_layout, terrain_instance_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &terrain_shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false, // terrain is flat, don't block sprites
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let verts = diamond_vertices();
    let terrain_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("terrain_vb"), contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let idxs = diamond_indices();
    let terrain_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("terrain_ib"), contents: bytemuck::cast_slice(&idxs),
        usage: wgpu::BufferUsages::INDEX,
    });
    let terrain_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("terrain_inst"), contents: bytemuck::cast_slice(terrain_instances),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    // ── Sprite pipeline ──────────────────────────────────────────────────
    let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sprite_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
    });

    // Generate and upload sprite atlas
    let atlas_data = sprites::generate_atlas();
    let atlas_texture = device.create_texture_with_data(
        &queue,
        &wgpu::TextureDescriptor {
            label: Some("sprite_atlas"),
            size: wgpu::Extent3d {
                width: sprites::ATLAS_W,
                height: sprites::ATLAS_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &atlas_data,
    );
    let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("sprite_sampler"),
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let sprite_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sprite_bgl"),
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
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sprite_bg"),
        layout: &sprite_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas_view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&atlas_sampler) },
        ],
    });

    let sprite_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sprite_pl"),
        bind_group_layouts: &[&sprite_bind_group_layout],
        push_constant_ranges: &[],
    });

    let sprite_vertex_layout = wgpu::VertexBufferLayout {
        array_stride: 8,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 }],
    };

    let sprite_instance_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteInstance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute { offset: 0, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },  // screen_pos
            wgpu::VertexAttribute { offset: 8, shader_location: 2, format: wgpu::VertexFormat::Float32 },    // z_order
            wgpu::VertexAttribute { offset: 16, shader_location: 3, format: wgpu::VertexFormat::Float32x4 }, // uv_rect
            wgpu::VertexAttribute { offset: 32, shader_location: 4, format: wgpu::VertexFormat::Float32x2 }, // size
        ],
    };

    let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite_pipeline"),
        layout: Some(&sprite_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &sprite_shader,
            entry_point: Some("vs_main"),
            buffers: &[sprite_vertex_layout, sprite_instance_layout],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &sprite_shader,
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
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(depth_stencil_state()),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    let qverts = quad_vertices();
    let sprite_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite_vb"), contents: bytemuck::cast_slice(&qverts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let qidxs = quad_indices();
    let sprite_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite_ib"), contents: bytemuck::cast_slice(&qidxs),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Sprite instance buffer — start with dummy if empty
    let sprite_buf_data: &[u8] = if sprite_instances.is_empty() {
        &[0u8; std::mem::size_of::<SpriteInstance>()]
    } else {
        bytemuck::cast_slice(sprite_instances)
    };
    let sprite_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite_inst"), contents: sprite_buf_data,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    RenderState {
        surface, device, queue, config, size,
        uniform_buffer, depth_texture,
        terrain_pipeline, terrain_vertex_buffer, terrain_index_buffer,
        terrain_instance_buffer,
        terrain_instance_capacity: terrain_instances.len(),
        terrain_instance_count: terrain_instances.len() as u32,
        terrain_bind_group,
        sprite_pipeline, sprite_vertex_buffer, sprite_index_buffer,
        sprite_instance_buffer,
        sprite_instance_capacity: sprite_instances.len().max(1),
        sprite_instance_count: sprite_instances.len() as u32,
        sprite_bind_group,
    }
}

impl RenderState {
    fn update_terrain(&mut self, instances: &[GpuInstance]) {
        self.terrain_instance_count = instances.len() as u32;
        if instances.len() <= self.terrain_instance_capacity {
            self.queue.write_buffer(&self.terrain_instance_buffer, 0, bytemuck::cast_slice(instances));
        } else {
            self.terrain_instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("terrain_inst"), contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.terrain_instance_capacity = instances.len();
        }
    }

    fn update_sprites(&mut self, instances: &[SpriteInstance]) {
        self.sprite_instance_count = instances.len() as u32;
        if instances.is_empty() { return; }
        if instances.len() <= self.sprite_instance_capacity {
            self.queue.write_buffer(&self.sprite_instance_buffer, 0, bytemuck::cast_slice(instances));
        } else {
            self.sprite_instance_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite_inst"), contents: bytemuck::cast_slice(instances),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            self.sprite_instance_capacity = instances.len();
        }
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

/// Render data returned by the sim tick callback.
pub struct FrameData {
    pub terrain: Vec<GpuInstance>,
    pub sprites: Vec<SpriteInstance>,
}

pub struct IsometricApp {
    state: Option<RenderState>,
    window: Option<Arc<Window>>,
    terrain_instances: Vec<GpuInstance>,
    sprite_instances: Vec<SpriteInstance>,
    camera: Camera,
    last_frame: std::time::Instant,
    on_tick: Option<Box<dyn FnMut() -> FrameData>>,
    sim_accumulator: f32,
    pub sim_ticks_per_sec: f32,
    max_ticks_per_frame: u32,
}

impl IsometricApp {
    pub fn new(terrain: Vec<GpuInstance>, sprites: Vec<SpriteInstance>, cam_x: f32, cam_y: f32, cam_speed: f32, zoom: f32) -> Self {
        let mut camera = Camera::new(cam_x, cam_y);
        camera.speed = cam_speed;
        camera.zoom = zoom;
        Self {
            state: None, window: None,
            terrain_instances: terrain,
            sprite_instances: sprites,
            camera,
            last_frame: std::time::Instant::now(),
            on_tick: None,
            sim_accumulator: 0.0,
            sim_ticks_per_sec: 10.0,
            max_ticks_per_frame: 5,
        }
    }

    pub fn set_on_tick(&mut self, callback: impl FnMut() -> FrameData + 'static) {
        self.on_tick = Some(Box::new(callback));
    }

    fn render(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.camera.update(dt);

        // Fixed-timestep sim ticks
        if let Some(callback) = self.on_tick.as_mut() {
            self.sim_accumulator += dt;
            let tick_dt = 1.0 / self.sim_ticks_per_sec;
            let mut ticked = false;
            let mut n = 0u32;
            while self.sim_accumulator >= tick_dt && n < self.max_ticks_per_frame {
                self.sim_accumulator -= tick_dt;
                let data = callback();
                self.terrain_instances = data.terrain;
                self.sprite_instances = data.sprites;
                ticked = true;
                n += 1;
            }
            if self.sim_accumulator > tick_dt { self.sim_accumulator = tick_dt; }
            if ticked {
                if let Some(state) = self.state.as_mut() {
                    state.update_terrain(&self.terrain_instances);
                    state.update_sprites(&self.sprite_instances);
                }
            }
        }

        let state = self.state.as_ref().unwrap();

        // Update uniforms
        let uniforms = Uniforms::ortho_zoom(
            state.size.width as f32, state.size.height as f32,
            self.camera.x, self.camera.y, self.camera.zoom,
        );
        state.queue.write_buffer(&state.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let output = match state.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::OutOfMemory) => panic!("GPU OOM"),
            Err(_) => return,
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("render") },
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.08, b: 0.12, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &state.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Pass 1: Terrain
            pass.set_pipeline(&state.terrain_pipeline);
            pass.set_bind_group(0, &state.terrain_bind_group, &[]);
            pass.set_vertex_buffer(0, state.terrain_vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, state.terrain_instance_buffer.slice(..));
            pass.set_index_buffer(state.terrain_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..state.terrain_instance_count);

            // Pass 2: Sprites
            if state.sprite_instance_count > 0 {
                pass.set_pipeline(&state.sprite_pipeline);
                pass.set_bind_group(0, &state.sprite_bind_group, &[]);
                pass.set_vertex_buffer(0, state.sprite_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, state.sprite_instance_buffer.slice(..));
                pass.set_index_buffer(state.sprite_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..6, 0, 0..state.sprite_instance_count);
            }
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.window.as_ref().unwrap().request_redraw();
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            if let Some(state) = self.state.as_mut() {
                state.size = new_size;
                state.config.width = new_size.width;
                state.config.height = new_size.height;
                state.surface.configure(&state.device, &state.config);
                state.depth_texture = create_depth_texture(&state.device, new_size.width, new_size.height);
            }
        }
    }
}

impl ApplicationHandler for IsometricApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = Window::default_attributes()
                .with_title("City Builder")
                .with_inner_size(PhysicalSize::new(1280u32, 720u32));
            let window = Arc::new(event_loop.create_window(attrs).unwrap());
            let state = create_render_state(window.clone(), &self.terrain_instances, &self.sprite_instances);
            self.state = Some(state);
            self.window = Some(window);
            self.last_frame = std::time::Instant::now();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => self.resize(new_size),
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(key), state: key_state, .. }, ..
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
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
                };
                let factor = if scroll > 0.0 { 0.9 } else { 1.1 };
                self.camera.zoom = (self.camera.zoom * factor).clamp(0.5, 200.0);
            }
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub fn run_with_sim(
    terrain: Vec<GpuInstance>,
    sprites: Vec<SpriteInstance>,
    cam_x: f32, cam_y: f32,
    cam_speed: f32, zoom: f32,
    ticks_per_sec: f32,
    on_tick: impl FnMut() -> FrameData + 'static,
) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = IsometricApp::new(terrain, sprites, cam_x, cam_y, cam_speed, zoom);
    app.sim_ticks_per_sec = ticks_per_sec;
    app.set_on_tick(on_tick);
    event_loop.run_app(&mut app).unwrap();
}
