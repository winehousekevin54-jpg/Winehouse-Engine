/// Placeholder for the WebGPU renderer.
/// The actual rendering logic is in the wasm-bridge crate for Phase 0 (Hello Triangle).
pub struct Renderer;

impl Renderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
