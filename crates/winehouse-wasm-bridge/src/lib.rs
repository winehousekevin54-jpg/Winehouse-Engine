use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("Winehouse Engine initialized");
}

#[wasm_bindgen]
pub async fn start_renderer(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("canvas not found")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    // Create wgpu instance with WebGPU backend
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {e}")))?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or("No suitable GPU adapter found")?;

    log::info!("GPU Adapter: {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("Winehouse Device"),
            ..Default::default()
        }, None)
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create device: {e}")))?;

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Triangle vertex data (position + color)
    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 3],
    }

    let vertices = [
        Vertex { position: [0.0, 0.5], color: [1.0, 0.0, 0.0] },     // top - red
        Vertex { position: [-0.5, -0.5], color: [0.0, 1.0, 0.0] },   // bottom left - green
        Vertex { position: [0.5, -0.5], color: [0.0, 0.0, 1.0] },    // bottom right - blue
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Triangle Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../../shaders/triangle.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Triangle Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    },
                    wgpu::VertexAttribute {
                        offset: 8,
                        shader_location: 1,
                        format: wgpu::VertexFormat::Float32x3,
                    },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    // Render loop using requestAnimationFrame
    let render = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let render_clone = render.clone();

    let surface = Rc::new(surface);
    let device = Rc::new(device);
    let queue = Rc::new(queue);
    let render_pipeline = Rc::new(render_pipeline);
    let vertex_buffer = Rc::new(vertex_buffer);

    *render_clone.borrow_mut() = Some(Closure::new({
        let render = render.clone();
        let surface = surface.clone();
        let device = device.clone();
        let queue = queue.clone();
        let render_pipeline = render_pipeline.clone();
        let vertex_buffer = vertex_buffer.clone();

        move || {
            let output = match surface.get_current_texture() {
                Ok(t) => t,
                Err(_) => return,
            };
            let view = output.texture.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.05,
                                g: 0.05,
                                b: 0.08,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(&render_pipeline);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..3, 0..1);
            }

            queue.submit(std::iter::once(encoder.finish()));
            output.present();

            // Request next frame
            let window = web_sys::window().unwrap();
            let r = render.borrow();
            let cb = r.as_ref().unwrap();
            window.request_animation_frame(cb.as_ref().unchecked_ref()).ok();
        }
    }));

    // Start the render loop
    let window = web_sys::window().unwrap();
    let r = render_clone.borrow();
    let cb = r.as_ref().unwrap();
    window.request_animation_frame(cb.as_ref().unchecked_ref())
        .map_err(|e| JsValue::from_str(&format!("Failed to start render loop: {e:?}")))?;

    log::info!("Render loop started");
    Ok(())
}

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
