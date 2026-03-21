pub mod camera;
pub mod mesh;

#[cfg(target_arch = "wasm32")]
pub mod renderer;

pub use camera::Camera;
pub use mesh::{cube_indices, cube_vertices, vertex_buffer_layout, Vertex};

#[cfg(target_arch = "wasm32")]
pub use renderer::{Renderer, SceneObjectInfo};
