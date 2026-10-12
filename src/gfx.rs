//! WebGPU renderer (wgpu 24).
//!
//! Two passes per frame:
//!   1. SCENE: draws the static + dynamic meshes into a small offscreen texture (~320 px tall)
//!      with toon lighting and fog. This is what gives the chunky "A Short Hike" pixel look.
//!   2. POST:  integer-scales that texture to the canvas with no filtering, then layers weather
//!      (rain, fog banks) and a vignette on the same low-res pixel grid.

use crate::mesh::FLOATS_PER_VERTEX;
use crate::scene::{GLOBALS_FLOATS, POST_FLOATS};
use wgpu::util::DeviceExt;

/// Target height of the low-res scene. The actual scale is the nearest whole number of screen
/// pixels per game pixel, so every game pixel is a perfect square.
const PIXEL_TARGET_H: f32 = 320.0;
const MAX_DYN_VERTS: usize = 150_000;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

const VERTEX_ATTRS: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32];

struct LowRes {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    w: u32,
    h: u32,
    scale: u32,
}

pub struct Renderer {
    canvas: web_sys::HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    scene_pipeline: wgpu::RenderPipeline,
    post_pipeline: wgpu::RenderPipeline,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    post_buf: wgpu::Buffer,
    post_bgl: wgpu::BindGroupLayout,
    post_bg: wgpu::BindGroup,

    static_vb: wgpu::Buffer,
    static_count: u32,
    dyn_vb: wgpu::Buffer,

    low: LowRes,
}

fn canvas_pixel_size(canvas: &web_sys::HtmlCanvasElement) -> (u32, u32) {
    let dpr = web_sys::window().map(|w| w.device_pixel_ratio()).unwrap_or(1.0);
    let w = (canvas.client_width() as f64 * dpr).round().max(1.0) as u32;
    let h = (canvas.client_height() as f64 * dpr).round().max(1.0) as u32;
    (w, h)
}

fn make_low_res(device: &wgpu::Device, w: u32, h: u32) -> LowRes {
    let scale = (h as f32 / PIXEL_TARGET_H).round().max(1.0) as u32;
    let lw = (w + scale - 1) / scale;
    let lh = (h + scale - 1) / scale;
    let size = wgpu::Extent3d { width: lw.max(1), height: lh.max(1), depth_or_array_layers: 1 };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene-color"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SCENE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene-depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    LowRes {
        color: color.create_view(&wgpu::TextureViewDescriptor::default()),
        depth: depth.create_view(&wgpu::TextureViewDescriptor::default()),
        w: size.width,
        h: size.height,
        scale,
    }
}

fn make_post_bg(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, low: &LowRes, buf: &wgpu::Buffer) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&low.color) },
            wgpu::BindGroupEntry { binding: 1, resource: buf.as_entire_binding() },
        ],
    })
}

impl Renderer {
    pub async fn new(canvas: web_sys::HtmlCanvasElement, static_data: &[f32]) -> Result<Renderer, String> {
        let (w, h) = canvas_pixel_size(&canvas);
        canvas.set_width(w);
        canvas.set_height(h);

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| format!("Couldn't create a WebGPU surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No WebGPU adapter found. Is hardware acceleration enabled in your browser?")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("spirit-collector"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("Couldn't open the GPU device: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        // Prefer a non-sRGB format: all our color math is done directly in display space.
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // ---- uniforms
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: (GLOBALS_FLOATS * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals-bg"),
            layout: &globals_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });

        // ---- scene pipeline
        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene-layout"),
            bind_group_layouts: &[&globals_bgl],
            push_constant_ranges: &[],
        });
        let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene-pipeline"),
            layout: Some(&scene_layout),
            vertex: wgpu::VertexState {
                module: &scene_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: (FLOATS_PER_VERTEX * 4) as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRS,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &scene_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            // No culling: the mesh builder flips normals instead of caring about winding.
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- post pipeline
        let post_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post"),
            size: (POST_FLOATS * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let post_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/post.wgsl").into()),
        });
        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post-layout"),
            bind_group_layouts: &[&post_bgl],
            push_constant_ranges: &[],
        });
        let post_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("post-pipeline"),
            layout: Some(&post_layout),
            vertex: wgpu::VertexState {
                module: &post_shader,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ---- geometry buffers
        let static_vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static-vb"),
            contents: bytemuck::cast_slice(static_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let dyn_vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic-vb"),
            size: (MAX_DYN_VERTS * FLOATS_PER_VERTEX * 4) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let low = make_low_res(&device, w, h);
        let post_bg = make_post_bg(&device, &post_bgl, &low, &post_buf);

        Ok(Renderer {
            canvas,
            surface,
            device,
            queue,
            config,
            scene_pipeline,
            post_pipeline,
            globals_buf,
            globals_bg,
            post_buf,
            post_bgl,
            post_bg,
            static_vb,
            static_count: (static_data.len() / FLOATS_PER_VERTEX) as u32,
            dyn_vb,
            low,
        })
    }

    /// Call once per frame before building uniforms: follows the canvas size / window resizes.
    pub fn resize_if_needed(&mut self) {
        let (w, h) = canvas_pixel_size(&self.canvas);
        let max = self.device.limits().max_texture_dimension_2d;
        let (w, h) = (w.min(max), h.min(max));
        if w != self.config.width || h != self.config.height {
            self.canvas.set_width(w);
            self.canvas.set_height(h);
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
            self.low = make_low_res(&self.device, w, h);
            self.post_bg = make_post_bg(&self.device, &self.post_bgl, &self.low, &self.post_buf);
        }
    }

    /// Replaces the static world geometry (used when you walk through a gate into another area).
    pub fn set_static(&mut self, static_data: &[f32]) {
        self.static_vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("static-vb"),
            contents: bytemuck::cast_slice(static_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.static_count = (static_data.len() / FLOATS_PER_VERTEX) as u32;
    }

    /// (low-res width, low-res height, integer pixel scale)
    pub fn low_res(&self) -> (u32, u32, u32) {
        (self.low.w, self.low.h, self.low.scale)
    }

    pub fn render(&mut self, globals: &[f32], post: &[f32], dynamic: &[f32], clear: [f32; 3]) {
        self.queue.write_buffer(&self.globals_buf, 0, bytemuck::cast_slice(globals));
        self.queue.write_buffer(&self.post_buf, 0, bytemuck::cast_slice(post));
        let dyn_verts = (dynamic.len() / FLOATS_PER_VERTEX).min(MAX_DYN_VERTS);
        if dyn_verts > 0 {
            self.queue
                .write_buffer(&self.dyn_vb, 0, bytemuck::cast_slice(&dynamic[..dyn_verts * FLOATS_PER_VERTEX]));
        }

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });

        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.low.color,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0] as f64,
                            g: clear[1] as f64,
                            b: clear[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.low.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.scene_pipeline);
            pass.set_bind_group(0, &self.globals_bg, &[]);
            if self.static_count > 0 {
                pass.set_vertex_buffer(0, self.static_vb.slice(..));
                pass.draw(0..self.static_count, 0..1);
            }
            if dyn_verts > 0 {
                pass.set_vertex_buffer(0, self.dyn_vb.slice(..));
                pass.draw(0..dyn_verts as u32, 0..1);
            }
        }

        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.post_pipeline);
            pass.set_bind_group(0, &self.post_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(enc.finish()));
        frame.present();
    }
}
