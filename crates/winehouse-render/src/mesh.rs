use glam::Vec3;
use wgpu::util::DeviceExt;

/// Vertex: position (xyz) + normal (xyz) + uv (xy) + tangent (xyzw)
/// 48 bytes total
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal:   [f32; 3],
    pub uv:       [f32; 2],
    pub tangent:  [f32; 4],
}

/// Owned GPU buffers for a single mesh.
pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
    pub index_format: wgpu::IndexFormat,
}

/// Raw image bytes extracted from a glTF material.
pub struct ExtractedTexture {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Result of loading a glTF file: mesh + optional PBR textures.
pub struct GltfLoadResult {
    pub mesh: GpuMesh,
    pub albedo_tex: Option<ExtractedTexture>,
    pub normal_tex: Option<ExtractedTexture>,
    pub metallic_roughness_tex: Option<ExtractedTexture>,
}

impl GpuMesh {
    /// Unit cube (24 verts / 36 u16 indices, per-face normals).
    pub fn from_cube(device: &wgpu::Device) -> Self {
        let verts = cube_vertices();
        let idxs = cube_indices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube VB"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube IB"),
            contents: bytemuck::cast_slice(&idxs),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: idxs.len() as u32,
            index_format: wgpu::IndexFormat::Uint16,
        }
    }

    /// Parse a GLB/glTF binary blob, extract the first primitive.
    /// Flat normals are generated if the file has none.
    pub fn from_gltf_bytes(device: &wgpu::Device, data: &[u8]) -> Result<GltfLoadResult, String> {
        let (document, buffers, images) =
            gltf::import_slice(data).map_err(|e| format!("glTF parse error: {e}"))?;

        let gltf_mesh = document
            .meshes()
            .next()
            .ok_or_else(|| "No mesh found in glTF".to_string())?;

        let primitive = gltf_mesh
            .primitives()
            .next()
            .ok_or_else(|| "No primitive found in mesh".to_string())?;

        let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .ok_or_else(|| "Missing vertex positions".to_string())?
            .collect();

        let indices: Vec<u32> = match reader.read_indices() {
            Some(iter) => iter.into_u32().collect(),
            None => (0..positions.len() as u32).collect(),
        };

        let normals: Vec<[f32; 3]> = match reader.read_normals() {
            Some(iter) => iter.collect(),
            None => generate_flat_normals(&positions, &indices),
        };

        let uvs: Vec<[f32; 2]> = match reader.read_tex_coords(0) {
            Some(iter) => iter.into_f32().collect(),
            None => vec![[0.0, 0.0]; positions.len()],
        };

        let tangents: Vec<[f32; 4]> = match reader.read_tangents() {
            Some(iter) => iter.collect(),
            None => normals.iter().map(|n| tangent_from_normal(*n)).collect(),
        };

        let vertices: Vec<Vertex> = positions
            .iter()
            .zip(normals.iter())
            .zip(uvs.iter())
            .zip(tangents.iter())
            .map(|(((p, n), uv), t)| Vertex {
                position: *p,
                normal:   *n,
                uv:       *uv,
                tangent:  *t,
            })
            .collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glTF VB"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("glTF IB"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Extract PBR textures from the material (if any)
        let material = primitive.material();
        let pbr = material.pbr_metallic_roughness();

        let extract_image = |tex_info_idx: Option<usize>| -> Option<ExtractedTexture> {
            let idx = tex_info_idx?;
            let img = images.get(idx)?;
            let (w, h) = (img.width, img.height);
            let rgba = match img.format {
                gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
                gltf::image::Format::R8G8B8 => {
                    let mut out = Vec::with_capacity((w * h * 4) as usize);
                    for chunk in img.pixels.chunks(3) {
                        out.extend_from_slice(chunk);
                        out.push(255);
                    }
                    out
                }
                _ => return None,
            };
            Some(ExtractedTexture { rgba, width: w, height: h })
        };

        let albedo_tex = extract_image(
            pbr.base_color_texture().map(|t| t.texture().source().index()),
        );
        let normal_tex = extract_image(
            material.normal_texture().map(|t| t.texture().source().index()),
        );
        let metallic_roughness_tex = extract_image(
            pbr.metallic_roughness_texture().map(|t| t.texture().source().index()),
        );

        let mesh = GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            index_format: wgpu::IndexFormat::Uint32,
        };

        Ok(GltfLoadResult {
            mesh,
            albedo_tex,
            normal_tex,
            metallic_roughness_tex,
        })
    }
}

// ── Cube geometry ─────────────────────────────────────────────────────────────

pub fn cube_vertices() -> Vec<Vertex> {
    [
        // Front +Z  (tangent = +X)
        Vertex { position: [-0.5, -0.5, 0.5], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, -0.5, 0.5],  normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, 0.5, 0.5],   normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, 0.5, 0.5],  normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        // Back -Z  (tangent = -X)
        Vertex { position: [0.5, -0.5, -0.5],  normal: [0.0, 0.0, -1.0], uv: [0.0, 1.0], tangent: [-1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, 0.0, -1.0], uv: [1.0, 1.0], tangent: [-1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, 0.5, -0.5],  normal: [0.0, 0.0, -1.0], uv: [1.0, 0.0], tangent: [-1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, 0.5, -0.5],   normal: [0.0, 0.0, -1.0], uv: [0.0, 0.0], tangent: [-1.0, 0.0, 0.0, 1.0] },
        // Left -X  (tangent = +Z)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [-1.0, 0.0, 0.0], uv: [0.0, 1.0], tangent: [0.0, 0.0, 1.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5],  normal: [-1.0, 0.0, 0.0], uv: [1.0, 1.0], tangent: [0.0, 0.0, 1.0, 1.0] },
        Vertex { position: [-0.5, 0.5, 0.5],   normal: [-1.0, 0.0, 0.0], uv: [1.0, 0.0], tangent: [0.0, 0.0, 1.0, 1.0] },
        Vertex { position: [-0.5, 0.5, -0.5],  normal: [-1.0, 0.0, 0.0], uv: [0.0, 0.0], tangent: [0.0, 0.0, 1.0, 1.0] },
        // Right +X  (tangent = -Z)
        Vertex { position: [0.5, -0.5, 0.5],  normal: [1.0, 0.0, 0.0], uv: [0.0, 1.0], tangent: [0.0, 0.0, -1.0, 1.0] },
        Vertex { position: [0.5, -0.5, -0.5], normal: [1.0, 0.0, 0.0], uv: [1.0, 1.0], tangent: [0.0, 0.0, -1.0, 1.0] },
        Vertex { position: [0.5, 0.5, -0.5],  normal: [1.0, 0.0, 0.0], uv: [1.0, 0.0], tangent: [0.0, 0.0, -1.0, 1.0] },
        Vertex { position: [0.5, 0.5, 0.5],   normal: [1.0, 0.0, 0.0], uv: [0.0, 0.0], tangent: [0.0, 0.0, -1.0, 1.0] },
        // Top +Y  (tangent = +X)
        Vertex { position: [-0.5, 0.5, 0.5],  normal: [0.0, 1.0, 0.0], uv: [0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, 0.5, 0.5],   normal: [0.0, 1.0, 0.0], uv: [1.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, 0.5, -0.5],  normal: [0.0, 1.0, 0.0], uv: [1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, 0.5, -0.5], normal: [0.0, 1.0, 0.0], uv: [0.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        // Bottom -Y  (tangent = +X)
        Vertex { position: [-0.5, -0.5, -0.5], normal: [0.0, -1.0, 0.0], uv: [0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, -0.5, -0.5],  normal: [0.0, -1.0, 0.0], uv: [1.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [0.5, -0.5, 0.5],   normal: [0.0, -1.0, 0.0], uv: [1.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
        Vertex { position: [-0.5, -0.5, 0.5],  normal: [0.0, -1.0, 0.0], uv: [0.0, 0.0], tangent: [1.0, 0.0, 0.0, 1.0] },
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
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 32,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x4,
            },
        ],
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn generate_flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        let p0 = Vec3::from(positions[tri[0] as usize]);
        let p1 = Vec3::from(positions[tri[1] as usize]);
        let p2 = Vec3::from(positions[tri[2] as usize]);
        let n = (p1 - p0).cross(p2 - p0).normalize_or_zero().to_array();
        normals[tri[0] as usize] = n;
        normals[tri[1] as usize] = n;
        normals[tri[2] as usize] = n;
    }
    normals
}
/// Compute a plausible tangent from a normal using a cross-product fallback.
fn tangent_from_normal(n: [f32; 3]) -> [f32; 4] {
    let nv = Vec3::from(n);
    let up = if nv.y.abs() < 0.999 { Vec3::Y } else { Vec3::X };
    let t = nv.cross(up).normalize_or_zero();
    [t.x, t.y, t.z, 1.0]
}