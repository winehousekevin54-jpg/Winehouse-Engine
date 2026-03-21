/// Vertex: position (xyz) + normal (xyz)
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

/// Unit cube with per-face normals (24 verts, 36 indices)
pub fn cube_vertices() -> Vec<Vertex> {
    [
        // Front +Z
        Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 0.0, 1.0] },
        // Back -Z
        Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0] },
        Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 0.0, -1.0] },
        // Left -X
        Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-0.5, -0.5, 0.5], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-0.5, 0.5, 0.5], normal: [-1.0, 0.0, 0.0] },
        Vertex { position: [-0.5, 0.5, -0.5], normal: [-1.0, 0.0, 0.0] },
        // Right +X
        Vertex { position: [0.5, -0.5, 0.5], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [0.5, 0.5, -0.5], normal: [1.0, 0.0, 0.0] },
        Vertex { position: [0.5, 0.5, 0.5], normal: [1.0, 0.0, 0.0] },
        // Top +Y
        Vertex { position: [-0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [0.5, 0.5, 0.5], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0] },
        Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0] },
        // Bottom -Y
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0] },
        Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, -1.0, 0.0] },
    ]
    .to_vec()
}

pub fn cube_indices() -> Vec<u16> {
    vec![
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Left
        12, 13, 14, 12, 14, 15, // Right
        16, 17, 18, 16, 18, 19, // Top
        20, 21, 22, 20, 22, 23, // Bottom
    ]
}

pub fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: 12,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
        ],
    }
}
