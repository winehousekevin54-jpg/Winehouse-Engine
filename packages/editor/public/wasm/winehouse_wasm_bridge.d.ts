/* tslint:disable */
/* eslint-disable */

/**
 * Orbit the camera around its target.
 */
export function camera_orbit(delta_azimuth: number, delta_elevation: number): void;

/**
 * Zoom the camera in/out (factor > 0 zooms in, < 0 zooms out).
 */
export function camera_zoom(factor: number): void;

/**
 * Remove an entity from the scene.
 */
export function despawn(id: bigint): void;

/**
 * Initialize the engine and attach it to a canvas. Must be called once before anything else.
 */
export function engine_init(canvas_id: string): Promise<void>;

/**
 * Submit one rendered frame. Call this every requestAnimationFrame.
 */
export function engine_render(): void;

/**
 * Notify the renderer that the canvas has been resized.
 */
export function engine_resize(width: number, height: number): void;

/**
 * Return all scene objects as a JSON array string.
 */
export function get_scene_json(): string;

/**
 * Load a glTF/GLB file from raw bytes and spawn it as a scene object. Returns the entity id.
 */
export function load_gltf_bytes(data: Uint8Array, name: string): bigint;

export function main(): void;

/**
 * Set the PBR material properties of an entity.
 */
export function set_material(id: bigint, r: number, g: number, b: number, metallic: number, roughness: number): void;

/**
 * Set the transform (position / rotation / scale) of an entity.
 */
export function set_transform(id: bigint, px: number, py: number, pz: number, rx: number, ry: number, rz: number, rw: number, sx: number, sy: number, sz: number): void;

/**
 * Spawn a cube and return its entity id.
 */
export function spawn_cube(name: string, x: number, y: number, z: number, r: number, g: number, b: number): bigint;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly camera_orbit: (a: number, b: number) => void;
    readonly camera_zoom: (a: number) => void;
    readonly despawn: (a: bigint) => void;
    readonly engine_init: (a: number, b: number) => any;
    readonly engine_render: () => void;
    readonly engine_resize: (a: number, b: number) => void;
    readonly get_scene_json: () => [number, number];
    readonly load_gltf_bytes: (a: number, b: number, c: number, d: number) => [bigint, number, number];
    readonly main: () => void;
    readonly set_material: (a: bigint, b: number, c: number, d: number, e: number, f: number) => void;
    readonly set_transform: (a: bigint, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number) => void;
    readonly spawn_cube: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => bigint;
    readonly wasm_bindgen_4d206b8a667c0594___closure__destroy___dyn_core_e0615fd90a40850c___ops__function__FnMut__wasm_bindgen_4d206b8a667c0594___JsValue____Output___core_e0615fd90a40850c___result__Result_____wasm_bindgen_4d206b8a667c0594___JsError___: (a: number, b: number) => void;
    readonly wasm_bindgen_4d206b8a667c0594___convert__closures_____invoke___wasm_bindgen_4d206b8a667c0594___JsValue__core_e0615fd90a40850c___result__Result_____wasm_bindgen_4d206b8a667c0594___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_4d206b8a667c0594___convert__closures_____invoke___js_sys_a4f91c42b15b0bac___Function_fn_wasm_bindgen_4d206b8a667c0594___JsValue_____wasm_bindgen_4d206b8a667c0594___sys__Undefined___js_sys_a4f91c42b15b0bac___Function_fn_wasm_bindgen_4d206b8a667c0594___JsValue_____wasm_bindgen_4d206b8a667c0594___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
