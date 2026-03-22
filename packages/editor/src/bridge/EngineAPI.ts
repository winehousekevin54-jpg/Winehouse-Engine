export interface SceneObjectInfo {
  id: number;
  name: string;
  position: [number, number, number];
  rotation: [number, number, number, number]; // xyzw quaternion
  scale: [number, number, number];
  albedo: [number, number, number];
  metallic: number;
  roughness: number;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
let wasm: any = null;

export async function initEngine(canvasId: string): Promise<void> {
  wasm = await import('../../../../crates/winehouse-wasm-bridge/pkg/winehouse_wasm_bridge');
  await wasm.default();
  await wasm.engine_init(canvasId);
}

export function renderFrame(): void {
  wasm?.engine_render();
}

export function resizeViewport(width: number, height: number): void {
  wasm?.engine_resize(width, height);
}

export function spawnCube(
  name: string,
  x: number, y: number, z: number,
  r: number, g: number, b: number,
): number {
  const id: bigint = wasm?.spawn_cube(name, x, y, z, r, g, b) ?? 0n;
  return Number(id);
}

export function despawn(id: number): void {
  wasm?.despawn(BigInt(id));
}

export function setTransform(
  id: number,
  px: number, py: number, pz: number,
  rx: number, ry: number, rz: number, rw: number,
  sx: number, sy: number, sz: number,
): void {
  wasm?.set_transform(BigInt(id), px, py, pz, rx, ry, rz, rw, sx, sy, sz);
}

export function setMaterial(
  id: number,
  r: number, g: number, b: number,
  metallic: number, roughness: number,
): void {
  wasm?.set_material(BigInt(id), r, g, b, metallic, roughness);
}

export async function loadGltfBytes(data: Uint8Array, name: string): Promise<number> {
  const id: bigint = await wasm?.load_gltf_bytes(data, name) ?? 0n;
  return Number(id);
}

export function cameraOrbit(deltaAzimuth: number, deltaElevation: number): void {
  wasm?.camera_orbit(deltaAzimuth, deltaElevation);
}

export function cameraZoom(factor: number): void {
  wasm?.camera_zoom(factor);
}

export function getSceneObjects(): SceneObjectInfo[] {
  if (!wasm) return [];
  try {
    const json: string = wasm.get_scene_json();
    return JSON.parse(json) as SceneObjectInfo[];
  } catch {
    return [];
  }
}
