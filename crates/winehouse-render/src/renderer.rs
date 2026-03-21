use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::mesh::{cube_indices, cube_vertices, vertex_buffer_layout, Vertex};

// ── GPU uniform structs ──────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _pad0: f32,
    light_dir: [f32; 3],
    _pad1: f32,
    light_color: [f32; 3],
    _pad2: f32,
    ambient_color: [f32; 3],
    _pad3: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectUniforms {
    model: [[f32; 4]; 4],
    albedo: [f32; 4],
    metallic: f32,
    roughness: f32,
    _pad: [f32; 2],
}

// ── Scene object ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct SceneObjectInfo {
    pub id: u64,
    pub name: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4], // xyzw quaternion
    pub scale: [f32; 3],
    pub albedo: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
}

pub struct SceneObject {
    pub info: SceneObjectInfo,
    pub object_buffer: wgpu::Buffer,
    pub object_bind_group: wgpu::BindGroup,
}

impl SceneObject {
    pub fn model_matrix(&self) -> Mat4 {
        let t = Vec3::from(self.info.position);
        let r = Quat::from_array(self.info.rotation);
        let s = Vec3::from(self.info.scale);
        Mat4::from_scale_rotation_translation(s, r, t)
    }
}

// ── Main renderer ─────────────────────────────────────────────────────────────

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,

    pub camera: Camera,

    // GPU resources
    scene_buffer: wgpu::Buffer,
    scene_bind_group: wgpu::BindGroup,
    object_bgl: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,

    // Static geometry (cube shared for all objects for now)
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    cube_index_count: u32,

    pub objects: Vec<SceneObject>,
    pub next_id: u64,
}

impl Renderer {
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| format!("Surface error: {e}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("No WebGPU adapter")?;

        let (device, queue) = adapter
            .request_device(&Default::default(), None)
            .await
            .map_err(|e| format!("Device error: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Depth texture
        let (depth_texture, depth_view) =
            create_depth_texture(&device, surface_config.width, surface_config.height);

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("PBR Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/pbr.wgsl").into()),
        });

        // Scene uniform buffer + bind group layout
        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scene BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Scene Uniform"),
            size: std::mem::size_of::<SceneUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene BG"),
            layout: &scene_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene_buffer.as_entire_binding(),
            }],
        });

        // Object uniform bind group layout
        let object_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Object BGL"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&scene_bgl, &object_bgl],
            push_constant_ranges: &[],
        });

        // Render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("PBR Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
            cache: None,
        });

        // Cube geometry
        let verts = cube_vertices();
        let idxs = cube_indices();
        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube VB"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube IB"),
            contents: bytemuck::cast_slice(&idxs),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut camera = Camera::new();
        camera.set_aspect(surface_config.width, surface_config.height);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            camera,
            scene_buffer,
            scene_bind_group,
            object_bgl,
            render_pipeline,
            depth_texture,
            depth_view,
            cube_vertex_buffer,
            cube_index_buffer,
            cube_index_count: idxs.len() as u32,
            objects: Vec::new(),
            next_id: 1,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera.set_aspect(width, height);
        let (dt, dv) = create_depth_texture(&self.device, width, height);
        self.depth_texture = dt;
        self.depth_view = dv;
    }

    pub fn spawn_cube(&mut self, name: &str, position: [f32; 3], albedo: [f32; 3]) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let info = SceneObjectInfo {
            id,
            name: name.to_string(),
            position,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            albedo,
            metallic: 0.0,
            roughness: 0.5,
        };

        let obj_uniforms = object_uniforms_from_info(&info);
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Uniform"),
            contents: bytemuck::bytes_of(&obj_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Object BG"),
            layout: &self.object_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });

        self.objects.push(SceneObject { info, object_buffer: buffer, object_bind_group: bind_group });
        id
    }

    pub fn set_transform(
        &mut self, id: u64,
        position: [f32; 3], rotation: [f32; 4], scale: [f32; 3],
    ) {
        if let Some(obj) = self.objects.iter_mut().find(|o| o.info.id == id) {
            obj.info.position = position;
            obj.info.rotation = rotation;
            obj.info.scale = scale;
            let u = object_uniforms_from_info(&obj.info);
            self.queue.write_buffer(&obj.object_buffer, 0, bytemuck::bytes_of(&u));
        }
    }

    pub fn set_material(
        &mut self, id: u64, albedo: [f32; 3], metallic: f32, roughness: f32,
    ) {
        if let Some(obj) = self.objects.iter_mut().find(|o| o.info.id == id) {
            obj.info.albedo = albedo;
            obj.info.metallic = metallic;
            obj.info.roughness = roughness;
            let u = object_uniforms_from_info(&obj.info);
            self.queue.write_buffer(&obj.object_buffer, 0, bytemuck::bytes_of(&u));
        }
    }

    pub fn despawn(&mut self, id: u64) {
        self.objects.retain(|o| o.info.id != id);
    }

    pub fn get_scene_json(&self) -> String {
        let infos: Vec<&SceneObjectInfo> = self.objects.iter().map(|o| &o.info).collect();
        serde_json::to_string(&infos).unwrap_or_default()
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Update scene uniforms
        let view_proj = self.camera.view_proj();
        let cam_pos = self.camera.position();
        let scene_uni = SceneUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: cam_pos.to_array(),
            _pad0: 0.0,
            light_dir: [-0.5_f32, -1.0, -0.3].map(|v| {
                let l = (0.5_f32 * 0.5 + 1.0 * 1.0 + 0.3 * 0.3).sqrt();
                v / l
            }),
            _pad1: 0.0,
            light_color: [1.0, 0.95, 0.9],
            _pad2: 0.0,
            ambient_color: [0.08, 0.08, 0.12],
            _pad3: 0.0,
        };
        self.queue.write_buffer(&self.scene_buffer, 0, bytemuck::bytes_of(&scene_uni));

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PBR Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.07, g: 0.07, b: 0.10, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            pass.set_index_buffer(self.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            for obj in &self.objects {
                pass.set_bind_group(1, &obj.object_bind_group, &[]);
                pass.draw_indexed(0..self.cube_index_count, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn create_depth_texture(
    device: &wgpu::Device, width: u32, height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    (tex, view)
}

fn object_uniforms_from_info(info: &SceneObjectInfo) -> ObjectUniforms {
    let model = {
        let t = Vec3::from(info.position);
        let r = Quat::from_array(info.rotation);
        let s = Vec3::from(info.scale);
        Mat4::from_scale_rotation_translation(s, r, t)
    };
    ObjectUniforms {
        model: model.to_cols_array_2d(),
        albedo: [info.albedo[0], info.albedo[1], info.albedo[2], 1.0],
        metallic: info.metallic,
        roughness: info.roughness,
        _pad: [0.0; 2],
    }
}

// Suppress unused import warning for Vertex (used in mesh.rs only via bytemuck)
const _: fn() = || { let _ = std::mem::size_of::<Vertex>(); };
