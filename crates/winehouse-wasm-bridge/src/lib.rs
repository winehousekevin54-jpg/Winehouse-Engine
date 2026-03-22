use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use winehouse_render::Renderer;

thread_local! {
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("Winehouse Engine v0.1.0");
}

fn get_canvas(canvas_id: &str) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
    document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("element is not a canvas"))
}

/// Initialize the engine and attach it to a canvas. Must be called once before anything else.
#[wasm_bindgen]
pub async fn engine_init(canvas_id: &str) -> Result<(), JsValue> {
    let canvas = get_canvas(canvas_id)?;
    let renderer = Renderer::new(canvas)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    RENDERER.with(|r| *r.borrow_mut() = Some(renderer));

    // Spawn a default cube so the viewport isn't empty
    with_renderer(|r| {
        r.spawn_cube("Cube", [0.0, 0.0, 0.0], [0.8, 0.3, 0.15]);
    });

    log::info!("Engine initialized");
    Ok(())
}

/// Submit one rendered frame. Call this every requestAnimationFrame.
#[wasm_bindgen]
pub fn engine_render() {
    with_renderer(|r| {
        let _ = r.render();
    });
}

/// Notify the renderer that the canvas has been resized.
#[wasm_bindgen]
pub fn engine_resize(width: u32, height: u32) {
    with_renderer(|r| r.resize(width, height));
}

/// Spawn a cube and return its entity id.
#[wasm_bindgen]
pub fn spawn_cube(name: &str, x: f32, y: f32, z: f32, r: f32, g: f32, b: f32) -> u64 {
    with_renderer_ret(|renderer| renderer.spawn_cube(name, [x, y, z], [r, g, b]), 0)
}

/// Remove an entity from the scene.
#[wasm_bindgen]
pub fn despawn(id: u64) {
    with_renderer(|r| r.despawn(id));
}

/// Set the transform (position / rotation / scale) of an entity.
#[wasm_bindgen]
pub fn set_transform(
    id: u64,
    px: f32, py: f32, pz: f32,
    rx: f32, ry: f32, rz: f32, rw: f32,
    sx: f32, sy: f32, sz: f32,
) {
    with_renderer(|r| r.set_transform(id, [px, py, pz], [rx, ry, rz, rw], [sx, sy, sz]));
}

/// Set the PBR material properties of an entity.
#[wasm_bindgen]
pub fn set_material(id: u64, r: f32, g: f32, b: f32, metallic: f32, roughness: f32) {
    with_renderer(|renderer| renderer.set_material(id, [r, g, b], metallic, roughness));
}

/// Orbit the camera around its target.
#[wasm_bindgen]
pub fn camera_orbit(delta_azimuth: f32, delta_elevation: f32) {
    with_renderer(|r| r.camera.orbit(delta_azimuth, delta_elevation));
}

/// Zoom the camera in/out (factor > 0 zooms in, < 0 zooms out).
#[wasm_bindgen]
pub fn camera_zoom(factor: f32) {
    with_renderer(|r| r.camera.zoom(factor));
}

/// Load a glTF/GLB file from raw bytes and spawn it as a scene object. Returns the entity id.
#[wasm_bindgen]
pub fn load_gltf_bytes(data: &[u8], name: &str) -> Result<u64, JsValue> {
    with_renderer_ret(
        |r| r.load_gltf(data, name).map_err(|e| JsValue::from_str(&e)),
        Err(JsValue::from_str("Renderer not initialized")),
    )
}

/// Return all scene objects as a JSON array string.
#[wasm_bindgen]
pub fn get_scene_json() -> String {
    with_renderer_ret(|r| r.get_scene_json(), String::new())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn with_renderer<F: FnOnce(&mut Renderer)>(f: F) {
    RENDERER.with(|r| {
        if let Some(renderer) = r.borrow_mut().as_mut() {
            f(renderer);
        }
    });
}

fn with_renderer_ret<T, F: FnOnce(&mut Renderer) -> T>(f: F, default: T) -> T {
    RENDERER.with(|r| {
        if let Some(renderer) = r.borrow_mut().as_mut() {
            f(renderer)
        } else {
            default
        }
    })
}
