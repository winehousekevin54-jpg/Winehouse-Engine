use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::mesh::{vertex_buffer_layout, ExtractedTexture, GpuMesh, Vertex};

// ── GPU uniform structs ───────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniforms {
    view_proj:             [[f32; 4]; 4],
    unjittered_view_proj:  [[f32; 4]; 4],
    prev_view_proj:        [[f32; 4]; 4],
    camera_pos:            [f32; 3],
    _pad0:                 f32,
    light_dir:             [f32; 3],
    _pad1:                 f32,
    light_color:           [f32; 3],
    _pad2:                 f32,
    ambient_color:         [f32; 3],
    _pad3:                 f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectUniforms {
    model:      [[f32; 4]; 4],
    prev_model: [[f32; 4]; 4],
    albedo:     [f32; 4],
    metallic:   f32,
    roughness:  f32,
    _pad:       [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ShadowUniforms {
    light_view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightingUniforms {
    inv_view_proj:   [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    view_proj:       [[f32; 4]; 4],
    viewport:        [f32; 2],
    near:            f32,
    far:             f32,
    cascade_vp:      [[[f32; 4]; 4]; 4],
    cascade_splits:  [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomUniforms {
    direction:  [f32; 2],
    texel_size: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct TaaUniforms {
    viewport:     [f32; 2],
    jitter:       [f32; 2],
    blend_factor: f32,
    _pad:         [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CasUniforms {
    sharpness: f32,
    _pad:      [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SsrUniforms {
    view_proj:     [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    viewport:      [f32; 2],
    near:          f32,
    far:           f32,
    camera_pos:    [f32; 3],
    max_distance:  f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct VolumetricUniforms {
    scattering:   [f32; 3],
    density:      f32,
    absorption:   [f32; 3],
    g_factor:     f32,
    max_distance: f32,
    steps:        f32,
    _pad:         [f32; 2],
}

// ── Scene object ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct SceneObjectInfo {
    pub id:       u64,
    pub name:     String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale:    [f32; 3],
    pub albedo:   [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
}

pub struct SceneObject {
    pub info:              SceneObjectInfo,
    pub mesh:              GpuMesh,
    pub object_buffer:     wgpu::Buffer,
    pub object_bind_group: wgpu::BindGroup,
    pub prev_model_matrix: Mat4,
}

impl SceneObject {
    pub fn model_matrix(&self) -> Mat4 {
        let t = Vec3::from(self.info.position);
        let r = Quat::from_array(self.info.rotation);
        let s = Vec3::from(self.info.scale);
        Mat4::from_scale_rotation_translation(s, r, t)
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    pub device:         wgpu::Device,
    pub queue:          wgpu::Queue,
    pub surface:        wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub camera:         Camera,
    pub objects:        Vec<SceneObject>,
    pub next_id:        u64,

    // Light
    light_dir: Vec3,

    // TAA state
    frame_index:    u32,
    prev_view_proj: Mat4,
    taa_valid:      bool,

    // Uniform buffers (updated each frame)
    scene_buffer:    wgpu::Buffer,
    shadow_buffers:  [wgpu::Buffer; 4],
    lighting_buffer: wgpu::Buffer,

    // Bloom uniform buffers (fixed direction)
    bloom_h_buffer:  wgpu::Buffer,
    bloom_v_buffer:  wgpu::Buffer,

    // TAA uniform buffer
    taa_buffer:      wgpu::Buffer,

    // CAS uniform buffer
    cas_buffer:      wgpu::Buffer,

    // SSR uniform buffer
    ssr_buffer:      wgpu::Buffer,

    // Volumetric uniform buffer
    vol_buffer:      wgpu::Buffer,

    // Bind group layouts
    object_bgl:           wgpu::BindGroupLayout,
    lighting_uniforms_bgl: wgpu::BindGroupLayout,
    lighting_inputs_bgl:  wgpu::BindGroupLayout,
    ssao_inputs_bgl:      wgpu::BindGroupLayout,
    ssao_blur_bgl:        wgpu::BindGroupLayout,
    bloom_threshold_bgl:  wgpu::BindGroupLayout,
    bloom_blur_bgl:       wgpu::BindGroupLayout,
    tonemap_bgl:          wgpu::BindGroupLayout,
    cas_bgl:              wgpu::BindGroupLayout,
    taa_bgl:              wgpu::BindGroupLayout,
    ssr_bgl:              wgpu::BindGroupLayout,
    ssr_composite_bgl:    wgpu::BindGroupLayout,
    vol_bgl:              wgpu::BindGroupLayout,
    vol_composite_bgl:    wgpu::BindGroupLayout,

    // Scene uniform bind group (shared across G-Buffer + shadow)
    scene_bg:          wgpu::BindGroup,
    shadow_pass_bgs:   [wgpu::BindGroup; 4],

    // Size-dependent bind groups (recreated on resize)
    lighting_uniforms_bg: wgpu::BindGroup,
    lighting_inputs_bg:   wgpu::BindGroup,
    ssao_bg:              wgpu::BindGroup,
    ssao_blur_bg:         wgpu::BindGroup,
    bloom_threshold_bg:   wgpu::BindGroup,
    bloom_blur_h_bg:      wgpu::BindGroup,
    bloom_blur_v_bg:      wgpu::BindGroup,
    tonemap_bg:           wgpu::BindGroup,
    cas_bg:               wgpu::BindGroup,
    taa_bg:               wgpu::BindGroup,
    ssr_bg:               wgpu::BindGroup,
    ssr_composite_bg:     wgpu::BindGroup,
    vol_bg:               wgpu::BindGroup,
    vol_composite_bg:     wgpu::BindGroup,

    // Size-dependent textures
    gbuffer_albedo_view:    wgpu::TextureView,
    gbuffer_normal_view:    wgpu::TextureView,
    gbuffer_depth_view:     wgpu::TextureView,
    velocity_view:          wgpu::TextureView,
    hdr_view:               wgpu::TextureView,
    ssao_view:              wgpu::TextureView,
    ssao_blur_view:         wgpu::TextureView,
    bloom_ping_view:        wgpu::TextureView,
    bloom_pong_view:        wgpu::TextureView,
    ldr_view:               wgpu::TextureView,
    taa_history_view:       wgpu::TextureView,
    taa_output_view:        wgpu::TextureView,
    taa_history_tex:        wgpu::Texture,
    taa_output_tex:         wgpu::Texture,
    ssr_view:               wgpu::TextureView,
    ssr_hdr_view:           wgpu::TextureView,
    vol_view:               wgpu::TextureView,

    // Fixed textures
    shadow_array_view:     wgpu::TextureView,
    shadow_cascade_views:  [wgpu::TextureView; 4],
    noise_view:            wgpu::TextureView,

    // Default PBR textures (1×1 fallbacks)
    default_albedo_view:   wgpu::TextureView,
    default_normal_view:   wgpu::TextureView,
    default_mr_view:       wgpu::TextureView,

    // Samplers
    linear_sampler:   wgpu::Sampler,
    repeat_sampler:   wgpu::Sampler,
    shadow_sampler:   wgpu::Sampler,
    point_sampler:    wgpu::Sampler,
    material_sampler: wgpu::Sampler,

    // Pipelines
    gbuffer_pipeline:          wgpu::RenderPipeline,
    shadow_pipeline:           wgpu::RenderPipeline,
    lighting_pipeline:         wgpu::RenderPipeline,
    ssao_pipeline:             wgpu::RenderPipeline,
    ssao_blur_pipeline:        wgpu::RenderPipeline,
    bloom_threshold_pipeline:  wgpu::RenderPipeline,
    bloom_blur_pipeline:       wgpu::RenderPipeline,
    tonemap_pipeline:          wgpu::RenderPipeline,
    cas_pipeline:              wgpu::RenderPipeline,
    taa_pipeline:              wgpu::RenderPipeline,
    ssr_pipeline:              wgpu::RenderPipeline,
    ssr_composite_pipeline:    wgpu::RenderPipeline,
    vol_pipeline:              wgpu::RenderPipeline,
    vol_composite_pipeline:    wgpu::RenderPipeline,
}

impl Renderer {
    pub async fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        let width  = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        // ── wgpu init ──────────────────────────────────────────────────────────
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

        let caps   = surface.get_capabilities(&adapter);
        // Prefer non-sRGB so we can apply our own gamma in the tonemap shader
        let format = caps.formats.iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage:                          wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width:                          width.max(1),
            height:                         height.max(1),
            present_mode:                   wgpu::PresentMode::Fifo,
            alpha_mode:                     caps.alpha_modes[0],
            view_formats:                   vec![],
            desired_maximum_frame_latency:  2,
        };
        surface.configure(&device, &surface_config);

        // ── Light setup ────────────────────────────────────────────────────────
        let light_dir = Vec3::new(-0.5_f32, -1.0, -0.3).normalize();

        // ── Shaders ────────────────────────────────────────────────────────────
        let sh_gbuffer  = shader(&device, include_str!("../../../shaders/gbuffer.wgsl"),  "G-Buffer");
        let sh_shadow   = shader(&device, include_str!("../../../shaders/shadow.wgsl"),   "Shadow");
        let sh_lighting = shader(&device, include_str!("../../../shaders/lighting.wgsl"), "Lighting");
        let sh_ssao     = shader(&device, include_str!("../../../shaders/ssao.wgsl"),     "SSAO");
        let sh_ssao_blur= shader(&device, include_str!("../../../shaders/ssao_blur.wgsl"),"SSAO Blur");
        let sh_bloom_th = shader(&device, include_str!("../../../shaders/bloom_threshold.wgsl"), "Bloom Threshold");
        let sh_bloom_bl = shader(&device, include_str!("../../../shaders/bloom_blur.wgsl"),      "Bloom Blur");
        let sh_tonemap  = shader(&device, include_str!("../../../shaders/tonemap.wgsl"),  "Tonemap");
        let sh_cas      = shader(&device, include_str!("../../../shaders/cas.wgsl"),      "CAS");
        let sh_taa      = shader(&device, include_str!("../../../shaders/taa.wgsl"),      "TAA");
        let sh_ssr      = shader(&device, include_str!("../../../shaders/ssr.wgsl"),      "SSR");
        let sh_ssr_comp = shader(&device, include_str!("../../../shaders/ssr_composite.wgsl"), "SSR Composite");
        let sh_vol      = shader(&device, include_str!("../../../shaders/volumetric.wgsl"), "Volumetric");
        let sh_vol_comp = shader(&device, include_str!("../../../shaders/volumetric_composite.wgsl"), "Volumetric Composite");

        // ── Samplers ───────────────────────────────────────────────────────────
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let repeat_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:           Some("Repeat"),
            address_mode_u:  wgpu::AddressMode::Repeat,
            address_mode_v:  wgpu::AddressMode::Repeat,
            mag_filter:      wgpu::FilterMode::Nearest,
            min_filter:      wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:           Some("Shadow"),
            compare:         Some(wgpu::CompareFunction::LessEqual),
            mag_filter:      wgpu::FilterMode::Linear,
            min_filter:      wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let point_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:       Some("Point"),
            mag_filter:  wgpu::FilterMode::Nearest,
            min_filter:  wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ── Uniform buffers ────────────────────────────────────────────────────
        let scene_buffer    = uniform_buf(&device, std::mem::size_of::<SceneUniforms>(),    "Scene");
        let shadow_buffers: [wgpu::Buffer; 4] = std::array::from_fn(|i| {
            uniform_buf(&device, std::mem::size_of::<ShadowUniforms>(), &format!("Shadow Cascade {i}"))
        });
        let lighting_buffer = uniform_buf(&device, std::mem::size_of::<LightingUniforms>(), "Lighting");
        let taa_buffer      = uniform_buf(&device, std::mem::size_of::<TaaUniforms>(),      "TAA");
        let cas_buffer      = uniform_buf(&device, std::mem::size_of::<CasUniforms>(),      "CAS");
        let ssr_buffer      = uniform_buf(&device, std::mem::size_of::<SsrUniforms>(),      "SSR");
        let vol_buffer      = uniform_buf(&device, std::mem::size_of::<VolumetricUniforms>(), "Volumetric");

        let w2 = width.max(2) / 2;
        let h2 = height.max(2) / 2;
        let bloom_h_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Bloom H"),
            contents: bytemuck::bytes_of(&BloomUniforms {
                direction:  [1.0, 0.0],
                texel_size: [1.0 / w2 as f32, 1.0 / h2 as f32],
            }),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bloom_v_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("Bloom V"),
            contents: bytemuck::bytes_of(&BloomUniforms {
                direction:  [0.0, 1.0],
                texel_size: [1.0 / w2 as f32, 1.0 / h2 as f32],
            }),
            usage:    wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Fixed textures ─────────────────────────────────────────────────────
        let (_shadow_map_tex, shadow_array_view, shadow_cascade_views) = create_shadow_map(&device);
        let (_noise_tex, noise_view) = create_noise_texture(&device, &queue);

        // ── Default PBR textures (1×1 fallbacks) ──────────────────────────────
        let default_albedo_view = create_1x1_texture(&device, &queue, &[255, 255, 255, 255], wgpu::TextureFormat::Rgba8UnormSrgb, "Default Albedo");
        let default_normal_view = create_1x1_texture(&device, &queue, &[128, 128, 255, 255], wgpu::TextureFormat::Rgba8Unorm, "Default Normal");
        let default_mr_view     = create_1x1_texture(&device, &queue, &[0, 128, 0, 255],     wgpu::TextureFormat::Rgba8Unorm, "Default MR");
        let material_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label:           Some("Material"),
            address_mode_u:  wgpu::AddressMode::Repeat,
            address_mode_v:  wgpu::AddressMode::Repeat,
            mag_filter:      wgpu::FilterMode::Linear,
            min_filter:      wgpu::FilterMode::Linear,
            mipmap_filter:   wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ── Bind group layouts ─────────────────────────────────────────────────
        let scene_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Scene BGL"),
            entries: &[bgl_uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let object_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Object BGL"),
            entries: &[
                bgl_uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),
                bgl_texture_2d(1),   // albedo map
                bgl_texture_2d(2),   // normal map
                bgl_texture_2d(3),   // metallic-roughness map
                bgl_sampler(4),      // material sampler
            ],
        });
        let shadow_pass_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Shadow Pass BGL"),
            entries: &[bgl_uniform(0, wgpu::ShaderStages::VERTEX)],
        });
        let lighting_uniforms_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Lighting Uniforms BGL"),
            entries: &[
                bgl_uniform(0, wgpu::ShaderStages::FRAGMENT),
                bgl_uniform(1, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let lighting_inputs_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Lighting Inputs BGL"),
            entries: &[
                bgl_texture_2d(0),
                bgl_texture_2d(1),
                bgl_depth_texture(2),
                bgl_depth_texture_array(3),
                bgl_comparison_sampler(4),
                bgl_texture_2d(5),
                bgl_sampler(6),
            ],
        });
        let ssao_inputs_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("SSAO Inputs BGL"),
            entries: &[
                bgl_texture_2d(0),
                bgl_depth_texture(1),
                bgl_texture_2d(2),
                bgl_sampler(3),
            ],
        });
        let ssao_blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("SSAO Blur BGL"),
            entries: &[bgl_texture_2d(0), bgl_sampler(1)],
        });
        let bloom_threshold_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Bloom Threshold BGL"),
            entries: &[bgl_texture_2d(0), bgl_sampler(1)],
        });
        let bloom_blur_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Bloom Blur BGL"),
            entries: &[bgl_uniform(0, wgpu::ShaderStages::FRAGMENT), bgl_texture_2d(1), bgl_sampler(2)],
        });
        let tonemap_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Tonemap BGL"),
            entries: &[bgl_texture_2d(0), bgl_texture_2d(1), bgl_sampler(2)],
        });
        let cas_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("CAS BGL"),
            entries: &[bgl_texture_2d(0), bgl_sampler(1), bgl_uniform(2, wgpu::ShaderStages::FRAGMENT)],
        });
        let taa_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("TAA BGL"),
            entries: &[
                bgl_uniform(0, wgpu::ShaderStages::FRAGMENT),
                bgl_texture_2d(1),
                bgl_texture_2d(2),
                bgl_texture_2d(3),
                bgl_depth_texture(4),
                bgl_sampler(5),
            ],
        });

        let ssr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("SSR BGL"),
            entries: &[
                bgl_uniform(0, wgpu::ShaderStages::FRAGMENT),  // SsrUniforms
                bgl_texture_2d(1),      // HDR (lit scene)
                bgl_depth_texture(2),   // G-Buffer depth
                bgl_texture_2d(3),      // G-Buffer normal
                bgl_texture_2d(4),      // G-Buffer albedo (for roughness/metallic)
                bgl_sampler(5),         // linear sampler
                bgl_sampler(6),         // point sampler
            ],
        });
        let ssr_composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("SSR Composite BGL"),
            entries: &[
                bgl_texture_2d(0),      // HDR
                bgl_texture_2d(1),      // SSR result
                bgl_sampler(2),         // linear sampler
            ],
        });

        let vol_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Volumetric BGL"),
            entries: &[
                bgl_uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT),  // SceneUniforms
                bgl_uniform(1, wgpu::ShaderStages::FRAGMENT),         // LightingUniforms
                bgl_depth_texture(2),                                  // G-Buffer depth
                bgl_depth_texture_array(3),                            // Shadow cascade array
                bgl_comparison_sampler(4),                             // Shadow sampler
                bgl_texture_2d(5),                                     // Noise texture
                bgl_sampler(6),                                        // Repeat sampler
                bgl_uniform(7, wgpu::ShaderStages::FRAGMENT),         // VolumetricUniforms
            ],
        });
        let vol_composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("Volumetric Composite BGL"),
            entries: &[
                bgl_texture_2d(0),   // HDR (ssr_hdr)
                bgl_texture_2d(1),   // Volumetric result
                bgl_sampler(2),      // Linear sampler
            ],
        });

        // ── Fixed bind groups ──────────────────────────────────────────────────
        let scene_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Scene BG"),
            layout:  &scene_bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: scene_buffer.as_entire_binding() }],
        });
        let shadow_pass_bgs: [wgpu::BindGroup; 4] = std::array::from_fn(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label:   Some("Shadow Pass BG"),
                layout:  &shadow_pass_bgl,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: shadow_buffers[i].as_entire_binding() }],
            })
        });

        // ── Pipelines ──────────────────────────────────────────────────────────
        let gbuffer_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label:                Some("G-Buffer Layout"),
                bind_group_layouts:   &[&scene_bgl, &object_bgl],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label:  Some("G-Buffer Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module:              &sh_gbuffer,
                    entry_point:         Some("vs_main"),
                    buffers:             &[vertex_buffer_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module:      &sh_gbuffer,
                    entry_point: Some("fs_main"),
                    targets:     &[
                        Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm,   blend: None, write_mask: wgpu::ColorWrites::ALL }),
                        Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float,  blend: None, write_mask: wgpu::ColorWrites::ALL }),
                        Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float,  blend: None, write_mask: wgpu::ColorWrites::ALL }),
                    ],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology:   wgpu::PrimitiveTopology::TriangleList,
                    cull_mode:  Some(wgpu::Face::Back),
                    front_face: wgpu::FrontFace::Ccw,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format:              wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare:       wgpu::CompareFunction::Less,
                    stencil:             Default::default(),
                    bias:                Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview:   None,
                cache:       None,
            })
        };

        let shadow_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label:                Some("Shadow Layout"),
                bind_group_layouts:   &[&shadow_pass_bgl, &object_bgl],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label:  Some("Shadow Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module:              &sh_shadow,
                    entry_point:         Some("vs_main"),
                    buffers:             &[vertex_buffer_layout()],
                    compilation_options: Default::default(),
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology:    wgpu::PrimitiveTopology::TriangleList,
                    cull_mode:   Some(wgpu::Face::Front), // front-face culling reduces peter-panning
                    front_face:  wgpu::FrontFace::Ccw,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format:              wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare:       wgpu::CompareFunction::Less,
                    stencil:             Default::default(),
                    bias:                wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview:   None,
                cache:       None,
            })
        };

        let lighting_pipeline = fullscreen_pipeline(
            &device, &sh_lighting, "Lighting",
            &[&lighting_uniforms_bgl, &lighting_inputs_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let ssao_pipeline = fullscreen_pipeline(
            &device, &sh_ssao, "SSAO",
            &[&lighting_uniforms_bgl, &ssao_inputs_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let ssao_blur_pipeline = fullscreen_pipeline(
            &device, &sh_ssao_blur, "SSAO Blur",
            &[&ssao_blur_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let bloom_threshold_pipeline = fullscreen_pipeline(
            &device, &sh_bloom_th, "Bloom Threshold",
            &[&bloom_threshold_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let bloom_blur_pipeline = fullscreen_pipeline(
            &device, &sh_bloom_bl, "Bloom Blur",
            &[&bloom_blur_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        // Tonemap → LDR intermediate (Rgba8Unorm); CAS reads this and writes to swapchain
        let tonemap_pipeline = fullscreen_pipeline(
            &device, &sh_tonemap, "Tonemap",
            &[&tonemap_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let cas_pipeline = fullscreen_pipeline(
            &device, &sh_cas, "CAS",
            &[&cas_bgl],
            &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let taa_pipeline = fullscreen_pipeline(
            &device, &sh_taa, "TAA",
            &[&taa_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let ssr_pipeline = fullscreen_pipeline(
            &device, &sh_ssr, "SSR",
            &[&ssr_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let ssr_composite_pipeline = fullscreen_pipeline(
            &device, &sh_ssr_comp, "SSR Composite",
            &[&ssr_composite_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let vol_pipeline = fullscreen_pipeline(
            &device, &sh_vol, "Volumetric",
            &[&vol_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );
        let vol_composite_pipeline = fullscreen_pipeline(
            &device, &sh_vol_comp, "Volumetric Composite",
            &[&vol_composite_bgl],
            &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        );

        let mut camera = Camera::new();
        camera.set_aspect(surface_config.width, surface_config.height);

        // ── Size-dependent resources ───────────────────────────────────────────
        let w = surface_config.width;
        let h = surface_config.height;
        let size_deps = SizeDependentResources::new(
            &device, w, h, &linear_sampler, &repeat_sampler,
            &shadow_sampler, &point_sampler,
            &shadow_array_view, &noise_view,
            &scene_buffer, &lighting_buffer, &taa_buffer, &cas_buffer, &ssr_buffer, &vol_buffer,
            &lighting_uniforms_bgl, &lighting_inputs_bgl,
            &ssao_inputs_bgl, &ssao_blur_bgl,
            &bloom_threshold_bgl, &bloom_blur_bgl,
            &tonemap_bgl, &cas_bgl, &taa_bgl,
            &ssr_bgl, &ssr_composite_bgl,
            &vol_bgl, &vol_composite_bgl,
            &bloom_h_buf, &bloom_v_buf,
        );

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            camera,
            objects: Vec::new(),
            next_id: 1,
            light_dir,
            frame_index: 0,
            prev_view_proj: Mat4::IDENTITY,
            taa_valid: false,
            scene_buffer,
            shadow_buffers,
            lighting_buffer,
            bloom_h_buffer: bloom_h_buf,
            bloom_v_buffer: bloom_v_buf,
            taa_buffer,
            cas_buffer,
            ssr_buffer,
            vol_buffer,
            object_bgl,
            lighting_uniforms_bgl,
            lighting_inputs_bgl,
            ssao_inputs_bgl,
            ssao_blur_bgl,
            bloom_threshold_bgl,
            bloom_blur_bgl,
            tonemap_bgl,
            cas_bgl,
            taa_bgl,
            ssr_bgl,
            ssr_composite_bgl,
            vol_bgl,
            vol_composite_bgl,
            scene_bg,
            shadow_pass_bgs,
            lighting_uniforms_bg:  size_deps.lighting_uniforms_bg,
            lighting_inputs_bg:    size_deps.lighting_inputs_bg,
            ssao_bg:               size_deps.ssao_bg,
            ssao_blur_bg:          size_deps.ssao_blur_bg,
            bloom_threshold_bg:    size_deps.bloom_threshold_bg,
            bloom_blur_h_bg:       size_deps.bloom_blur_h_bg,
            bloom_blur_v_bg:       size_deps.bloom_blur_v_bg,
            tonemap_bg:            size_deps.tonemap_bg,
            cas_bg:                size_deps.cas_bg,
            taa_bg:                size_deps.taa_bg,
            ssr_bg:                size_deps.ssr_bg,
            ssr_composite_bg:      size_deps.ssr_composite_bg,
            vol_bg:                size_deps.vol_bg,
            vol_composite_bg:      size_deps.vol_composite_bg,
            gbuffer_albedo_view:   size_deps.gbuffer_albedo_view,
            gbuffer_normal_view:   size_deps.gbuffer_normal_view,
            gbuffer_depth_view:    size_deps.gbuffer_depth_view,
            velocity_view:         size_deps.velocity_view,
            hdr_view:              size_deps.hdr_view,
            ssao_view:             size_deps.ssao_view,
            ssao_blur_view:        size_deps.ssao_blur_view,
            bloom_ping_view:       size_deps.bloom_ping_view,
            bloom_pong_view:       size_deps.bloom_pong_view,
            ldr_view:              size_deps.ldr_view,
            taa_history_view:      size_deps.taa_history_view,
            taa_output_view:       size_deps.taa_output_view,
            taa_history_tex:       size_deps.taa_history_tex,
            taa_output_tex:        size_deps.taa_output_tex,
            ssr_view:              size_deps.ssr_view,
            ssr_hdr_view:          size_deps.ssr_hdr_view,
            vol_view:              size_deps.vol_view,
            shadow_array_view,
            shadow_cascade_views,
            noise_view,
            default_albedo_view,
            default_normal_view,
            default_mr_view,
            linear_sampler,
            repeat_sampler,
            shadow_sampler,
            point_sampler,
            material_sampler,
            gbuffer_pipeline,
            shadow_pipeline,
            lighting_pipeline,
            ssao_pipeline,
            ssao_blur_pipeline,
            bloom_threshold_pipeline,
            bloom_blur_pipeline,
            tonemap_pipeline,
            cas_pipeline,
            taa_pipeline,
            ssr_pipeline,
            ssr_composite_pipeline,
            vol_pipeline,
            vol_composite_pipeline,
        })
    }

    // ── Resize ────────────────────────────────────────────────────────────────

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.surface_config.width  = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.camera.set_aspect(width, height);

        // Update bloom uniform buffers for new size
        let w2 = width.max(2) / 2;
        let h2 = height.max(2) / 2;
        self.queue.write_buffer(&self.bloom_h_buffer, 0, bytemuck::bytes_of(&BloomUniforms {
            direction: [1.0, 0.0], texel_size: [1.0 / w2 as f32, 1.0 / h2 as f32],
        }));
        self.queue.write_buffer(&self.bloom_v_buffer, 0, bytemuck::bytes_of(&BloomUniforms {
            direction: [0.0, 1.0], texel_size: [1.0 / w2 as f32, 1.0 / h2 as f32],
        }));

        let sd = SizeDependentResources::new(
            &self.device, width, height,
            &self.linear_sampler, &self.repeat_sampler,
            &self.shadow_sampler, &self.point_sampler,
            &self.shadow_array_view, &self.noise_view,
            &self.scene_buffer, &self.lighting_buffer, &self.taa_buffer, &self.cas_buffer, &self.ssr_buffer, &self.vol_buffer,
            &self.lighting_uniforms_bgl, &self.lighting_inputs_bgl,
            &self.ssao_inputs_bgl, &self.ssao_blur_bgl,
            &self.bloom_threshold_bgl, &self.bloom_blur_bgl,
            &self.tonemap_bgl, &self.cas_bgl, &self.taa_bgl,
            &self.ssr_bgl, &self.ssr_composite_bgl,
            &self.vol_bgl, &self.vol_composite_bgl,
            &self.bloom_h_buffer, &self.bloom_v_buffer,
        );
        self.lighting_uniforms_bg = sd.lighting_uniforms_bg;
        self.lighting_inputs_bg   = sd.lighting_inputs_bg;
        self.ssao_bg              = sd.ssao_bg;
        self.ssao_blur_bg         = sd.ssao_blur_bg;
        self.bloom_threshold_bg   = sd.bloom_threshold_bg;
        self.bloom_blur_h_bg      = sd.bloom_blur_h_bg;
        self.bloom_blur_v_bg      = sd.bloom_blur_v_bg;
        self.tonemap_bg           = sd.tonemap_bg;
        self.cas_bg               = sd.cas_bg;
        self.taa_bg               = sd.taa_bg;
        self.ssr_bg               = sd.ssr_bg;
        self.ssr_composite_bg     = sd.ssr_composite_bg;
        self.vol_bg               = sd.vol_bg;
        self.vol_composite_bg     = sd.vol_composite_bg;
        self.gbuffer_albedo_view  = sd.gbuffer_albedo_view;
        self.gbuffer_normal_view  = sd.gbuffer_normal_view;
        self.gbuffer_depth_view   = sd.gbuffer_depth_view;
        self.velocity_view        = sd.velocity_view;
        self.hdr_view             = sd.hdr_view;
        self.ssao_view            = sd.ssao_view;
        self.ssao_blur_view       = sd.ssao_blur_view;
        self.bloom_ping_view      = sd.bloom_ping_view;
        self.bloom_pong_view      = sd.bloom_pong_view;
        self.ldr_view             = sd.ldr_view;
        self.taa_history_view     = sd.taa_history_view;
        self.taa_output_view      = sd.taa_output_view;
        self.taa_history_tex      = sd.taa_history_tex;
        self.taa_output_tex       = sd.taa_output_tex;
        self.ssr_view             = sd.ssr_view;
        self.ssr_hdr_view         = sd.ssr_hdr_view;
        self.vol_view             = sd.vol_view;
        self.taa_valid            = false;
    }

    // ── Scene management ──────────────────────────────────────────────────────

    pub fn spawn_cube(&mut self, name: &str, position: [f32; 3], albedo: [f32; 3]) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let info = SceneObjectInfo {
            id, name: name.to_string(), position,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            albedo, metallic: 0.0, roughness: 0.5,
        };
        let albedo_view = self.default_albedo_view.clone();
        let normal_view = self.default_normal_view.clone();
        let mr_view     = self.default_mr_view.clone();
        self.push_object(info, GpuMesh::from_cube(&self.device), &albedo_view, &normal_view, &mr_view);
        id
    }

    pub fn load_gltf(&mut self, data: &[u8], name: &str) -> Result<u64, String> {
        let id   = self.next_id;
        self.next_id += 1;
        let result = GpuMesh::from_gltf_bytes(&self.device, data)?;

        let albedo_view = match &result.albedo_tex {
            Some(et) => upload_texture(&self.device, &self.queue, et, wgpu::TextureFormat::Rgba8UnormSrgb, "glTF Albedo"),
            None => self.default_albedo_view.clone(),
        };
        let normal_view = match &result.normal_tex {
            Some(et) => upload_texture(&self.device, &self.queue, et, wgpu::TextureFormat::Rgba8Unorm, "glTF Normal"),
            None => self.default_normal_view.clone(),
        };
        let mr_view = match &result.metallic_roughness_tex {
            Some(et) => upload_texture(&self.device, &self.queue, et, wgpu::TextureFormat::Rgba8Unorm, "glTF MR"),
            None => self.default_mr_view.clone(),
        };

        let info = SceneObjectInfo {
            id, name: name.to_string(),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale:    [1.0, 1.0, 1.0],
            albedo:   [1.0, 1.0, 1.0],
            metallic: 1.0, roughness: 1.0,
        };
        self.push_object(info, result.mesh, &albedo_view, &normal_view, &mr_view);
        Ok(id)
    }

    fn push_object(
        &mut self,
        info: SceneObjectInfo,
        mesh: GpuMesh,
        albedo_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        mr_view:     &wgpu::TextureView,
    ) {
        let model = {
            let t = Vec3::from(info.position);
            let r = Quat::from_array(info.rotation);
            let s = Vec3::from(info.scale);
            Mat4::from_scale_rotation_translation(s, r, t)
        };
        let uniforms  = make_object_uniforms(&info, model);
        let buffer    = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Uniform"), contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Object BG"),
            layout:  &self.object_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(albedo_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(normal_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(mr_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&self.material_sampler) },
            ],
        });
        self.objects.push(SceneObject {
            info, mesh, object_buffer: buffer, object_bind_group: bind_group,
            prev_model_matrix: model,
        });
    }

    pub fn set_transform(&mut self, id: u64, position: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) {
        if let Some(obj) = self.objects.iter_mut().find(|o| o.info.id == id) {
            // Save current model as prev before updating
            obj.prev_model_matrix = obj.model_matrix();
            obj.info.position = position;
            obj.info.rotation = rotation;
            obj.info.scale    = scale;
            let u = make_object_uniforms(&obj.info, obj.prev_model_matrix);
            self.queue.write_buffer(&obj.object_buffer, 0, bytemuck::bytes_of(&u));
        }
    }

    pub fn set_material(&mut self, id: u64, albedo: [f32; 3], metallic: f32, roughness: f32) {
        if let Some(obj) = self.objects.iter_mut().find(|o| o.info.id == id) {
            obj.info.albedo   = albedo;
            obj.info.metallic = metallic;
            obj.info.roughness= roughness;
            let u = make_object_uniforms(&obj.info, obj.prev_model_matrix);
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

    // ── Render ────────────────────────────────────────────────────────────────

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        use crate::camera::halton;

        let w = self.surface_config.width  as f32;
        let h = self.surface_config.height as f32;

        // ── Jitter ────────────────────────────────────────────────────────
        let ji = self.frame_index % 16;
        let jx = (halton(ji + 1, 2) - 0.5) * 2.0 / w;
        let jy = (halton(ji + 1, 3) - 0.5) * 2.0 / h;
        let jittered_vp  = self.camera.jittered_view_proj(jx, jy);
        let unjittered_vp = self.camera.view_proj();
        let inv_vp       = unjittered_vp.inverse();
        let cam_pos      = self.camera.position();

        // Update scene uniforms (jittered VP for G-Buffer, unjittered for lighting/post)
        self.queue.write_buffer(&self.scene_buffer, 0, bytemuck::bytes_of(&SceneUniforms {
            view_proj:            jittered_vp.to_cols_array_2d(),
            unjittered_view_proj: unjittered_vp.to_cols_array_2d(),
            prev_view_proj:       self.prev_view_proj.to_cols_array_2d(),
            camera_pos:           cam_pos.to_array(),
            _pad0:                0.0,
            light_dir:            self.light_dir.to_array(),
            _pad1:                0.0,
            light_color:          [1.2, 1.1, 1.0],
            _pad2:                0.0,
            ambient_color:        [0.06, 0.06, 0.09],
            _pad3:                0.0,
        }));

        // Compute cascade shadow maps
        let cascade_splits = compute_cascade_splits(self.camera.near, self.camera.far);
        let mut cascade_vps = [Mat4::IDENTITY; 4];
        let mut prev_split = self.camera.near;
        for i in 0..4 {
            cascade_vps[i] = cascade_view_proj(&self.camera, self.light_dir, prev_split, cascade_splits[i]);
            prev_split = cascade_splits[i];
        }

        // Update shadow uniforms (one buffer per cascade)
        for i in 0..4 {
            self.queue.write_buffer(&self.shadow_buffers[i], 0, bytemuck::bytes_of(&ShadowUniforms {
                light_view_proj: cascade_vps[i].to_cols_array_2d(),
            }));
        }

        // Update lighting uniforms (use unjittered VP)
        self.queue.write_buffer(&self.lighting_buffer, 0, bytemuck::bytes_of(&LightingUniforms {
            inv_view_proj:   inv_vp.to_cols_array_2d(),
            light_view_proj: cascade_vps[0].to_cols_array_2d(),
            view_proj:       unjittered_vp.to_cols_array_2d(),
            viewport:        [w, h],
            near:            self.camera.near,
            far:             self.camera.far,
            cascade_vp: [
                cascade_vps[0].to_cols_array_2d(),
                cascade_vps[1].to_cols_array_2d(),
                cascade_vps[2].to_cols_array_2d(),
                cascade_vps[3].to_cols_array_2d(),
            ],
            cascade_splits,
        }));

        // Update TAA uniforms
        let blend = if self.taa_valid { 0.1_f32 } else { 1.0 };
        self.queue.write_buffer(&self.taa_buffer, 0, bytemuck::bytes_of(&TaaUniforms {
            viewport:     [w, h],
            jitter:       [jx, jy],
            blend_factor: blend,
            _pad:         [0.0; 3],
        }));

        // Update CAS uniforms
        self.queue.write_buffer(&self.cas_buffer, 0, bytemuck::bytes_of(&CasUniforms {
            sharpness: 0.5,
            _pad:      [0.0; 3],
        }));

        // Update SSR uniforms
        self.queue.write_buffer(&self.ssr_buffer, 0, bytemuck::bytes_of(&SsrUniforms {
            view_proj:     unjittered_vp.to_cols_array_2d(),
            inv_view_proj: inv_vp.to_cols_array_2d(),
            viewport:      [w, h],
            near:          self.camera.near,
            far:           self.camera.far,
            camera_pos:    cam_pos.to_array(),
            max_distance:  15.0,
        }));

        // Update volumetric uniforms
        self.queue.write_buffer(&self.vol_buffer, 0, bytemuck::bytes_of(&VolumetricUniforms {
            scattering:   [0.5, 0.5, 0.5],
            density:      0.02,
            absorption:   [0.0, 0.0, 0.0],
            g_factor:     0.76,
            max_distance: 50.0,
            steps:        32.0,
            _pad:         [0.0; 2],
        }));

        // Update per-object uniforms with current prev_model, then advance prev_model
        for obj in &mut self.objects {
            let u = make_object_uniforms(&obj.info, obj.prev_model_matrix);
            self.queue.write_buffer(&obj.object_buffer, 0, bytemuck::bytes_of(&u));
            // Advance: current model becomes next frame's prev_model
            let t = Vec3::from(obj.info.position);
            let r = Quat::from_array(obj.info.rotation);
            let s = Vec3::from(obj.info.scale);
            obj.prev_model_matrix = Mat4::from_scale_rotation_translation(s, r, t);
        }

        let output  = self.surface.get_current_texture()?;
        let out_view = output.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        // ── 1. Shadow passes (4 cascades) ─────────────────────────────────────
        for cascade in 0..4usize {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shadow Cascade"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_cascade_views[cascade],
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_pass_bgs[cascade], &[]);
            for obj in &self.objects {
                pass.set_bind_group(1, &obj.object_bind_group, &[]);
                pass.set_vertex_buffer(0, obj.mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(obj.mesh.index_buffer.slice(..), obj.mesh.index_format);
                pass.draw_indexed(0..obj.mesh.index_count, 0, 0..1);
            }
        }

        // ── 2. G-Buffer pass ──────────────────────────────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("G-Buffer Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffer_albedo_view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.gbuffer_normal_view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.velocity_view, resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gbuffer_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.gbuffer_pipeline);
            pass.set_bind_group(0, &self.scene_bg, &[]);
            for obj in &self.objects {
                pass.set_bind_group(1, &obj.object_bind_group, &[]);
                pass.set_vertex_buffer(0, obj.mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(obj.mesh.index_buffer.slice(..), obj.mesh.index_format);
                pass.draw_indexed(0..obj.mesh.index_count, 0, 0..1);
            }
        }

        // ── 3. SSAO pass ──────────────────────────────────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ssao_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.ssao_pipeline);
            pass.set_bind_group(0, &self.lighting_uniforms_bg, &[]);
            pass.set_bind_group(1, &self.ssao_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 4. SSAO blur ──────────────────────────────────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO Blur Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ssao_blur_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.ssao_blur_pipeline);
            pass.set_bind_group(0, &self.ssao_blur_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 5. Lighting pass (→ HDR) ──────────────────────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lighting Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.lighting_pipeline);
            pass.set_bind_group(0, &self.lighting_uniforms_bg, &[]);
            pass.set_bind_group(1, &self.lighting_inputs_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 5b. SSR pass (HDR + G-Buffer → ssr_view) ─────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSR Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ssr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.ssr_pipeline);
            pass.set_bind_group(0, &self.ssr_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 5c. SSR Composite (HDR + SSR → ssr_hdr) ──────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSR Composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ssr_hdr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.ssr_composite_pipeline);
            pass.set_bind_group(0, &self.ssr_composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 5d. Volumetric (ray march → vol_view half-res) ───────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volumetric Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.vol_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.vol_pipeline);
            pass.set_bind_group(0, &self.vol_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 5e. Volumetric Composite (ssr_hdr + vol → hdr) ───────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volumetric Composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.hdr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.vol_composite_pipeline);
            pass.set_bind_group(0, &self.vol_composite_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 6. TAA resolve (hdr + history → taa_output) ────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("TAA Resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.taa_output_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.taa_pipeline);
            pass.set_bind_group(0, &self.taa_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // Copy taa_output → taa_history for next frame
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture:   &self.taa_output_tex,
                mip_level: 0,
                origin:    wgpu::Origin3d::ZERO,
                aspect:    wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture:   &self.taa_history_tex,
                mip_level: 0,
                origin:    wgpu::Origin3d::ZERO,
                aspect:    wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width:  self.surface_config.width.max(1),
                height: self.surface_config.height.max(1),
                depth_or_array_layers: 1,
            },
        );

        // ── 7. Bloom threshold (TAA output → bloom_ping) ─────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Threshold"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_ping_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.bloom_threshold_pipeline);
            pass.set_bind_group(0, &self.bloom_threshold_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 8. Bloom blur H (bloom_ping → bloom_pong) ────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Blur H"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_pong_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.bloom_blur_pipeline);
            pass.set_bind_group(0, &self.bloom_blur_h_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 9. Bloom blur V (bloom_pong → bloom_ping) ────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Blur V"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_ping_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.bloom_blur_pipeline);
            pass.set_bind_group(0, &self.bloom_blur_v_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 10. Tonemap (TAA output + bloom_ping → LDR Rgba8Unorm) ────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tonemap Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.ldr_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.tonemap_pipeline);
            pass.set_bind_group(0, &self.tonemap_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // ── 11. CAS Sharpening (LDR → swapchain) ────────────────────────────────
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("CAS Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &out_view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.cas_pipeline);
            pass.set_bind_group(0, &self.cas_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(enc.finish()));
        output.present();

        // ── End-of-frame state updates ────────────────────────────────────
        self.prev_view_proj = unjittered_vp;
        self.frame_index += 1;
        self.taa_valid = true;

        Ok(())
    }
}

// ── Size-dependent resources (recreated on resize) ────────────────────────────

struct SizeDependentResources {
    gbuffer_albedo_view:  wgpu::TextureView,
    gbuffer_normal_view:  wgpu::TextureView,
    gbuffer_depth_view:   wgpu::TextureView,
    velocity_view:        wgpu::TextureView,
    hdr_view:             wgpu::TextureView,
    ssao_view:            wgpu::TextureView,
    ssao_blur_view:       wgpu::TextureView,
    bloom_ping_view:      wgpu::TextureView,
    bloom_pong_view:      wgpu::TextureView,
    ldr_view:             wgpu::TextureView,
    taa_history_view:     wgpu::TextureView,
    taa_output_view:      wgpu::TextureView,
    taa_history_tex:      wgpu::Texture,
    taa_output_tex:       wgpu::Texture,
    ssr_view:             wgpu::TextureView,
    ssr_hdr_view:         wgpu::TextureView,
    vol_view:             wgpu::TextureView,
    lighting_uniforms_bg: wgpu::BindGroup,
    lighting_inputs_bg:   wgpu::BindGroup,
    ssao_bg:              wgpu::BindGroup,
    ssao_blur_bg:         wgpu::BindGroup,
    bloom_threshold_bg:   wgpu::BindGroup,
    bloom_blur_h_bg:      wgpu::BindGroup,
    bloom_blur_v_bg:      wgpu::BindGroup,
    tonemap_bg:           wgpu::BindGroup,
    cas_bg:               wgpu::BindGroup,
    taa_bg:               wgpu::BindGroup,
    ssr_bg:               wgpu::BindGroup,
    ssr_composite_bg:     wgpu::BindGroup,
    vol_bg:               wgpu::BindGroup,
    vol_composite_bg:     wgpu::BindGroup,
}

#[allow(clippy::too_many_arguments)]
impl SizeDependentResources {
    fn new(
        device:               &wgpu::Device,
        width:                u32,
        height:               u32,
        linear_sampler:       &wgpu::Sampler,
        repeat_sampler:       &wgpu::Sampler,
        shadow_sampler:       &wgpu::Sampler,
        point_sampler:        &wgpu::Sampler,
        shadow_array_view:    &wgpu::TextureView,
        noise_view:           &wgpu::TextureView,
        scene_buffer:         &wgpu::Buffer,
        lighting_buffer:      &wgpu::Buffer,
        taa_buffer:           &wgpu::Buffer,
        cas_buffer:           &wgpu::Buffer,
        ssr_buffer:           &wgpu::Buffer,
        vol_buffer:           &wgpu::Buffer,
        lighting_uniforms_bgl:&wgpu::BindGroupLayout,
        lighting_inputs_bgl:  &wgpu::BindGroupLayout,
        ssao_inputs_bgl:      &wgpu::BindGroupLayout,
        ssao_blur_bgl:        &wgpu::BindGroupLayout,
        bloom_threshold_bgl:  &wgpu::BindGroupLayout,
        bloom_blur_bgl:       &wgpu::BindGroupLayout,
        tonemap_bgl:          &wgpu::BindGroupLayout,
        cas_bgl:              &wgpu::BindGroupLayout,
        taa_bgl:              &wgpu::BindGroupLayout,
        ssr_bgl:              &wgpu::BindGroupLayout,
        ssr_composite_bgl:    &wgpu::BindGroupLayout,
        vol_bgl:              &wgpu::BindGroupLayout,
        vol_composite_bgl:    &wgpu::BindGroupLayout,
        bloom_h_buf:          &wgpu::Buffer,
        bloom_v_buf:          &wgpu::Buffer,
    ) -> Self {
        let w  = width.max(1);
        let h  = height.max(1);
        let w2 = (w / 2).max(1);
        let h2 = (h / 2).max(1);

        let gbuffer_albedo = tex2d(device, w, h, wgpu::TextureFormat::Rgba8Unorm,  "GB Albedo");
        let gbuffer_normal = tex2d(device, w, h, wgpu::TextureFormat::Rgba16Float, "GB Normal");
        let gbuffer_depth  = depth_tex(device, w, h, "GB Depth");
        let velocity       = tex2d(device, w, h, wgpu::TextureFormat::Rgba16Float, "Velocity");
        let hdr            = tex2d(device, w, h, wgpu::TextureFormat::Rgba16Float, "HDR");
        let ssao           = tex2d_r8(device, w, h, "SSAO");
        let ssao_blur      = tex2d_r8(device, w, h, "SSAO Blur");
        let bloom_ping     = tex2d(device, w2, h2, wgpu::TextureFormat::Rgba16Float, "Bloom Ping");
        let bloom_pong     = tex2d(device, w2, h2, wgpu::TextureFormat::Rgba16Float, "Bloom Pong");
        // LDR intermediate: tonemap writes here, CAS reads from here
        let ldr            = tex2d(device, w, h, wgpu::TextureFormat::Rgba8Unorm, "LDR");

        // TAA history & output need COPY_SRC/COPY_DST for end-of-frame blit
        let taa_tex_usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST;
        let taa_history_tex = device.create_texture(&wgpu::TextureDescriptor {
            label:  Some("TAA History"),
            size:   wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format:    wgpu::TextureFormat::Rgba16Float,
            usage:     taa_tex_usage,
            view_formats: &[],
        });
        let taa_output_tex = device.create_texture(&wgpu::TextureDescriptor {
            label:  Some("TAA Output"),
            size:   wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format:    wgpu::TextureFormat::Rgba16Float,
            usage:     taa_tex_usage,
            view_formats: &[],
        });

        let gbuffer_albedo_view = gbuffer_albedo.create_view(&Default::default());
        let gbuffer_normal_view = gbuffer_normal.create_view(&Default::default());
        // Chrome WebGPU requires aspect:DepthOnly for texture_depth_2d bindings.
        // Depth32Float has no stencil, so DepthOnly works for render attachments too.
        let gbuffer_depth_view  = gbuffer_depth.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        let velocity_view       = velocity.create_view(&Default::default());
        let hdr_view            = hdr.create_view(&Default::default());
        let ssao_view           = ssao.create_view(&Default::default());
        let ssao_blur_view      = ssao_blur.create_view(&Default::default());
        let bloom_ping_view     = bloom_ping.create_view(&Default::default());
        let bloom_pong_view     = bloom_pong.create_view(&Default::default());
        let ldr_view            = ldr.create_view(&Default::default());
        let taa_history_view    = taa_history_tex.create_view(&Default::default());
        let taa_output_view     = taa_output_tex.create_view(&Default::default());

        let lighting_uniforms_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Lighting Uniforms BG"),
            layout:  lighting_uniforms_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: scene_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: lighting_buffer.as_entire_binding() },
            ],
        });

        let lighting_inputs_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Lighting Inputs BG"),
            layout:  lighting_inputs_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&gbuffer_albedo_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&gbuffer_normal_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&gbuffer_depth_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(shadow_array_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(shadow_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&ssao_blur_view) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        let ssao_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("SSAO BG"),
            layout:  ssao_inputs_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&gbuffer_normal_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&gbuffer_depth_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(noise_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(repeat_sampler) },
            ],
        });

        let ssao_blur_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("SSAO Blur BG"),
            layout:  ssao_blur_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ssao_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(point_sampler) },
            ],
        });

        let bloom_threshold_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Bloom Threshold BG"),
            layout:  bloom_threshold_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&taa_output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        let bloom_blur_h_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Bloom Blur H BG"),
            layout:  bloom_blur_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bloom_h_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&bloom_ping_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        let bloom_blur_v_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Bloom Blur V BG"),
            layout:  bloom_blur_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: bloom_v_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&bloom_pong_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        let tonemap_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Tonemap BG"),
            layout:  tonemap_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&taa_output_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&bloom_ping_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        let cas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("CAS BG"),
            layout:  cas_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ldr_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(linear_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: cas_buffer.as_entire_binding() },
            ],
        });

        // SSR textures
        let ssr_tex  = tex2d(device, w, h, wgpu::TextureFormat::Rgba16Float, "SSR");
        let ssr_hdr  = tex2d(device, w, h, wgpu::TextureFormat::Rgba16Float, "SSR HDR");
        let ssr_view     = ssr_tex.create_view(&Default::default());
        let ssr_hdr_view = ssr_hdr.create_view(&Default::default());

        let ssr_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("SSR BG"),
            layout:  ssr_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: ssr_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&gbuffer_depth_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&gbuffer_normal_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&gbuffer_albedo_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(linear_sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(point_sampler) },
            ],
        });

        let ssr_composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("SSR Composite BG"),
            layout:  ssr_composite_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&hdr_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&ssr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        // Volumetric textures (half-resolution)
        let vol_tex  = tex2d(device, w2, h2, wgpu::TextureFormat::Rgba16Float, "Volumetric");
        let vol_view = vol_tex.create_view(&Default::default());

        let vol_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Volumetric BG"),
            layout:  vol_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: scene_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: lighting_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&gbuffer_depth_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(shadow_array_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(shadow_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(noise_view) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(repeat_sampler) },
                wgpu::BindGroupEntry { binding: 7, resource: vol_buffer.as_entire_binding() },
            ],
        });

        let vol_composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("Volumetric Composite BG"),
            layout:  vol_composite_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ssr_hdr_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&vol_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        // TAA reads from hdr_view (which has been written by vol composite or ssr composite)
        let taa_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("TAA BG"),
            layout:  taa_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: taa_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&hdr_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&taa_history_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&velocity_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&gbuffer_depth_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(linear_sampler) },
            ],
        });

        Self {
            gbuffer_albedo_view, gbuffer_normal_view, gbuffer_depth_view, velocity_view,
            hdr_view, ssao_view, ssao_blur_view, bloom_ping_view, bloom_pong_view, ldr_view,
            taa_history_view, taa_output_view, taa_history_tex, taa_output_tex,
            ssr_view, ssr_hdr_view, vol_view,
            lighting_uniforms_bg, lighting_inputs_bg, ssao_bg, ssao_blur_bg,
            bloom_threshold_bg, bloom_blur_h_bg, bloom_blur_v_bg, tonemap_bg, cas_bg, taa_bg,
            ssr_bg, ssr_composite_bg, vol_bg, vol_composite_bg,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn shader(device: &wgpu::Device, src: &str, label: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label:  Some(label),
        source: wgpu::ShaderSource::Wgsl(src.into()),
    })
}

fn uniform_buf(device: &wgpu::Device, size: usize, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label:              Some(label),
        size:               size as u64,
        usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn tex2d(device: &wgpu::Device, w: u32, h: u32, format: wgpu::TextureFormat, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label:                Some(label),
        size:                 wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format,
        usage:                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats:         &[],
    })
}

// R8Unorm is NOT a required RENDER_ATTACHMENT format in WebGPU (Chrome rejects it).
// Use Rgba8Unorm instead — the occlusion value lives in the R channel.
fn tex2d_r8(device: &wgpu::Device, w: u32, h: u32, label: &str) -> wgpu::Texture {
    tex2d(device, w, h, wgpu::TextureFormat::Rgba8Unorm, label)
}

fn depth_tex(device: &wgpu::Device, w: u32, h: u32, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label:                Some(label),
        size:                 wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format:               wgpu::TextureFormat::Depth32Float,
        usage:                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats:         &[],
    })
}

fn create_shadow_map(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView, [wgpu::TextureView; 4]) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label:                Some("Shadow Map"),
        size:                 wgpu::Extent3d { width: 2048, height: 2048, depth_or_array_layers: 4 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format:               wgpu::TextureFormat::Depth32Float,
        usage:                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats:         &[],
    });
    // Full array view for sampling in the lighting shader
    let array_view = tex.create_view(&wgpu::TextureViewDescriptor {
        dimension:         Some(wgpu::TextureViewDimension::D2Array),
        aspect:            wgpu::TextureAspect::DepthOnly,
        base_array_layer:  0,
        array_layer_count: Some(4),
        ..Default::default()
    });
    // Per-layer views for rendering each cascade
    let cascade_views = std::array::from_fn(|i| {
        tex.create_view(&wgpu::TextureViewDescriptor {
            dimension:         Some(wgpu::TextureViewDimension::D2),
            aspect:            wgpu::TextureAspect::DepthOnly,
            base_array_layer:  i as u32,
            array_layer_count: Some(1),
            ..Default::default()
        })
    });
    (tex, array_view, cascade_views)
}

fn create_noise_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::Texture, wgpu::TextureView) {
    // 4×4 noise texture: random 2D vectors in tangent plane (z=0), stored as RGBA8Unorm
    // Encodes vectors from [-1,1] to [0,255] in RG channels
    let angles: [f32; 16] = [
        0.0, 0.7854, 1.5708, 2.3562, 3.1416, 3.9270, 4.7124, 5.4978,
        0.3927, 1.1781, 1.9635, 2.7489, 3.5343, 4.3197, 5.1051, 5.8905,
    ];
    let mut data = [0u8; 16 * 4];
    for (i, &angle) in angles.iter().enumerate() {
        let (s, c) = angle.sin_cos();
        data[i * 4 + 0] = ((c * 0.5 + 0.5) * 255.0) as u8;
        data[i * 4 + 1] = ((s * 0.5 + 0.5) * 255.0) as u8;
        data[i * 4 + 2] = 128;
        data[i * 4 + 3] = 255;
    }
    let tex = device.create_texture_with_data(queue, &wgpu::TextureDescriptor {
        label:                Some("SSAO Noise"),
        size:                 wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format:               wgpu::TextureFormat::Rgba8Unorm,
        usage:                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats:         &[],
    }, wgpu::util::TextureDataOrder::LayerMajor, &data);
    let view = tex.create_view(&Default::default());
    (tex, view)
}

fn create_1x1_texture(
    device: &wgpu::Device,
    queue:  &wgpu::Queue,
    pixel:  &[u8; 4],
    format: wgpu::TextureFormat,
    label:  &str,
) -> wgpu::TextureView {
    let tex = device.create_texture_with_data(queue, &wgpu::TextureDescriptor {
        label:                Some(label),
        size:                 wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format,
        usage:                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats:         &[],
    }, wgpu::util::TextureDataOrder::LayerMajor, pixel);
    tex.create_view(&Default::default())
}

/// Upload an `ExtractedTexture` (from glTF) to the GPU.
pub fn upload_texture(
    device: &wgpu::Device,
    queue:  &wgpu::Queue,
    et:     &ExtractedTexture,
    format: wgpu::TextureFormat,
    label:  &str,
) -> wgpu::TextureView {
    let tex = device.create_texture_with_data(queue, &wgpu::TextureDescriptor {
        label:                Some(label),
        size:                 wgpu::Extent3d { width: et.width, height: et.height, depth_or_array_layers: 1 },
        mip_level_count:      1,
        sample_count:         1,
        dimension:            wgpu::TextureDimension::D2,
        format,
        usage:                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats:         &[],
    }, wgpu::util::TextureDataOrder::LayerMajor, &et.rgba);
    tex.create_view(&Default::default())
}

fn fullscreen_pipeline(
    device:  &wgpu::Device,
    shader:  &wgpu::ShaderModule,
    label:   &str,
    bgls:    &[&wgpu::BindGroupLayout],
    targets: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label:                Some(label),
        bind_group_layouts:   bgls,
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label:  Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module:              shader,
            entry_point:         Some("vs_main"),
            buffers:             &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module:              shader,
            entry_point:         Some("fs_main"),
            targets,
            compilation_options: Default::default(),
        }),
        primitive:     wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample:   wgpu::MultisampleState::default(),
        multiview:     None,
        cache:         None,
    })
}

// ── Bind group layout helpers ─────────────────────────────────────────────────

fn bgl_uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty:                wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size:  None,
        },
        count: None,
    }
}

fn bgl_texture_2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type:    wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled:   false,
        },
        count: None,
    }
}

fn bgl_depth_texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type:    wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled:   false,
        },
        count: None,
    }
}

fn bgl_depth_texture_array(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type:    wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2Array,
            multisampled:   false,
        },
        count: None,
    }
}

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn bgl_comparison_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
        count: None,
    }
}

fn make_object_uniforms(info: &SceneObjectInfo, prev_model: Mat4) -> ObjectUniforms {
    let t = Vec3::from(info.position);
    let r = Quat::from_array(info.rotation);
    let s = Vec3::from(info.scale);
    let model = Mat4::from_scale_rotation_translation(s, r, t);
    ObjectUniforms {
        model:      model.to_cols_array_2d(),
        prev_model: prev_model.to_cols_array_2d(),
        albedo:     [info.albedo[0], info.albedo[1], info.albedo[2], 1.0],
        metallic:   info.metallic,
        roughness:  info.roughness,
        _pad:       [0.0; 2],
    }
}

// ── Cascade Shadow Map helpers ────────────────────────────────────────────────

const NUM_CASCADES: usize = 4;
const SHADOW_DISTANCE: f32 = 50.0;
const CASCADE_LAMBDA: f32 = 0.75;

fn compute_cascade_splits(near: f32, far: f32) -> [f32; 4] {
    let shadow_far = far.min(SHADOW_DISTANCE);
    let cn = near.max(0.1);
    let mut splits = [0.0_f32; NUM_CASCADES];
    for i in 0..NUM_CASCADES {
        let p = (i + 1) as f32 / NUM_CASCADES as f32;
        let log_split = cn * (shadow_far / cn).powf(p);
        let uniform_split = cn + (shadow_far - cn) * p;
        splits[i] = CASCADE_LAMBDA * log_split + (1.0 - CASCADE_LAMBDA) * uniform_split;
    }
    splits
}

fn cascade_view_proj(camera: &Camera, light_dir: Vec3, near: f32, far: f32) -> Mat4 {
    use glam::Vec4;

    let cam_vp  = Mat4::perspective_rh(camera.fov_y, camera.aspect, near, far) * camera.view_matrix();
    let inv_vp  = cam_vp.inverse();

    // 8 NDC corners (WebGPU: Z ∈ [0,1])
    let mut world = [Vec3::ZERO; 8];
    let mut idx = 0;
    for &z in &[0.0_f32, 1.0] {
        for &y in &[-1.0_f32, 1.0] {
            for &x in &[-1.0_f32, 1.0] {
                let w = inv_vp * Vec4::new(x, y, z, 1.0);
                world[idx] = w.truncate() / w.w;
                idx += 1;
            }
        }
    }

    let center = world.iter().copied().fold(Vec3::ZERO, |a, b| a + b) / 8.0;

    let up = if light_dir.cross(Vec3::Y).length_squared() < 0.001 { Vec3::Z } else { Vec3::Y };
    let light_view = Mat4::look_at_rh(center - light_dir * 100.0, center, up);

    // AABB in light-view space
    let mut ls_min = Vec3::splat(f32::MAX);
    let mut ls_max = Vec3::splat(f32::MIN);
    for c in &world {
        let ls = light_view.transform_point3(*c);
        ls_min = ls_min.min(ls);
        ls_max = ls_max.max(ls);
    }

    // orthographic_rh maps view-space Z in [-near, -far] to NDC [0,1].
    // Extend toward the light to capture shadow casters outside the camera frustum.
    let near_z = 0.1_f32;
    let far_z  = (-ls_min.z).max(1.0) + 50.0;

    Mat4::orthographic_rh(ls_min.x, ls_max.x, ls_min.y, ls_max.y, near_z, far_z) * light_view
}

const _: fn() = || { let _ = std::mem::size_of::<Vertex>(); };
