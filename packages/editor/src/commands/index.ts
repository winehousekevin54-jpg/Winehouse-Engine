import {
  spawnCube,
  despawn,
  loadGltfBytes,
  setTransform,
  setMaterial,
  getSceneObjects,
} from '../bridge/EngineAPI';
import type { SceneObjectInfo } from '../bridge/EngineAPI';

// ── Command interface ─────────────────────────────────────────────────────────

export interface Command {
  readonly description: string;
  execute(): void;
  undo(): void;
}

// ── Concrete commands ─────────────────────────────────────────────────────────

export class SpawnCubeCommand implements Command {
  readonly description: string;
  private spawnedId: number = 0;

  constructor(
    private name: string,
    private position: [number, number, number],
    private albedo: [number, number, number],
  ) {
    this.description = `Spawn "${name}"`;
  }

  execute() {
    this.spawnedId = spawnCube(
      this.name,
      this.position[0], this.position[1], this.position[2],
      this.albedo[0], this.albedo[1], this.albedo[2],
    );
  }

  undo() {
    if (this.spawnedId !== 0) despawn(this.spawnedId);
  }
}

export class DespawnCommand implements Command {
  readonly description: string;
  // id may change after undo (new spawn), so we track it
  private currentId: number;

  constructor(private entity: SceneObjectInfo) {
    this.description = `Delete "${entity.name}"`;
    this.currentId = entity.id;
  }

  execute() {
    despawn(this.currentId);
  }

  undo() {
    // Re-spawn with the same visual properties; captures new id for future redo
    const newId = spawnCube(
      this.entity.name,
      this.entity.position[0], this.entity.position[1], this.entity.position[2],
      this.entity.albedo[0], this.entity.albedo[1], this.entity.albedo[2],
    );
    setTransform(
      newId,
      this.entity.position[0], this.entity.position[1], this.entity.position[2],
      this.entity.rotation[0], this.entity.rotation[1], this.entity.rotation[2], this.entity.rotation[3],
      this.entity.scale[0], this.entity.scale[1], this.entity.scale[2],
    );
    setMaterial(
      newId,
      this.entity.albedo[0], this.entity.albedo[1], this.entity.albedo[2],
      this.entity.metallic,
      this.entity.roughness,
    );
    this.currentId = newId;
  }
}

export class SetTransformCommand implements Command {
  readonly description = 'Set Transform';

  constructor(
    private id: number,
    private before: { position: [number,number,number]; rotation: [number,number,number,number]; scale: [number,number,number] },
    private after:  { position: [number,number,number]; rotation: [number,number,number,number]; scale: [number,number,number] },
  ) {}

  execute() {
    const a = this.after;
    setTransform(this.id, a.position[0], a.position[1], a.position[2], a.rotation[0], a.rotation[1], a.rotation[2], a.rotation[3], a.scale[0], a.scale[1], a.scale[2]);
  }

  undo() {
    const b = this.before;
    setTransform(this.id, b.position[0], b.position[1], b.position[2], b.rotation[0], b.rotation[1], b.rotation[2], b.rotation[3], b.scale[0], b.scale[1], b.scale[2]);
  }
}

export class SetMaterialCommand implements Command {
  readonly description = 'Set Material';

  constructor(
    private id: number,
    private before: { albedo: [number,number,number]; metallic: number; roughness: number },
    private after:  { albedo: [number,number,number]; metallic: number; roughness: number },
  ) {}

  execute() {
    const a = this.after;
    setMaterial(this.id, a.albedo[0], a.albedo[1], a.albedo[2], a.metallic, a.roughness);
  }

  undo() {
    const b = this.before;
    setMaterial(this.id, b.albedo[0], b.albedo[1], b.albedo[2], b.metallic, b.roughness);
  }
}

export class LoadGltfCommand implements Command {
  readonly description: string;
  private spawnedId: number = 0;

  constructor(private data: Uint8Array, private name: string) {
    this.description = `Import "${name}"`;
  }

  async executeAsync(): Promise<void> {
    this.spawnedId = await loadGltfBytes(this.data, this.name);
  }

  execute() { /* async – use executeAsync */ }

  undo() {
    if (this.spawnedId !== 0) despawn(this.spawnedId);
  }
}

// ── Shared helper: sync scene state after any command ────────────────────────

export function syncScene(setEntities: (e: SceneObjectInfo[]) => void) {
  setEntities(getSceneObjects());
}
