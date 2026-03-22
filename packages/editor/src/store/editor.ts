import { create } from 'zustand';
import type { Command } from '../commands';
import type { SceneObjectInfo } from '../bridge/EngineAPI';

export interface AssetEntry {
  id: string;       // unique asset id (uuid-ish)
  name: string;
  type: 'gltf';
  data: Uint8Array; // raw bytes — used to re-spawn on drag/double-click
  sizeKb: number;
}

interface EditorState {
  // Scene
  entities: SceneObjectInfo[];
  selectedId: number | null;

  // Engine status
  engineStatus: 'loading' | 'running' | 'error';
  engineError: string | null;

  // Console
  logs: string[];

  // Asset library
  assets: AssetEntry[];
  // maps assetId → list of scene entity ids spawned from that asset
  assetSpawns: Record<string, number[]>;

  // Undo / Redo
  undoStack: Command[];
  redoStack: Command[];

  // Actions
  setEntities: (entities: SceneObjectInfo[]) => void;
  selectEntity: (id: number | null) => void;
  setEngineStatus: (status: 'loading' | 'running' | 'error', error?: string) => void;
  addLog: (message: string) => void;
  addAsset: (asset: AssetEntry) => void;
  trackSpawn: (assetId: string, entityId: number) => void;
  updateAsset: (assetId: string, data: Uint8Array, sizeKb: number) => void;
  clearSpawns: (assetId: string) => void;

  executeCommand: (cmd: Command, setEntities: (e: SceneObjectInfo[]) => void) => void;
  pushCommand: (cmd: Command) => void;
  undo: (setEntities: (e: SceneObjectInfo[]) => void) => void;
  redo: (setEntities: (e: SceneObjectInfo[]) => void) => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  entities: [],
  selectedId: null,
  engineStatus: 'loading',
  engineError: null,
  logs: [],
  assets: [],
  assetSpawns: {},
  undoStack: [],
  redoStack: [],

  setEntities: (entities) => set({ entities }),
  selectEntity: (id) => set({ selectedId: id }),
  setEngineStatus: (status, error) =>
    set({ engineStatus: status, engineError: error ?? null }),
  addLog: (message) =>
    set((state) => ({
      logs: [...state.logs.slice(-499), `[${new Date().toLocaleTimeString()}] ${message}`],
    })),
  addAsset: (asset) =>
    set((state) => ({ assets: [...state.assets, asset] })),
  trackSpawn: (assetId, entityId) =>
    set((state) => ({
      assetSpawns: {
        ...state.assetSpawns,
        [assetId]: [...(state.assetSpawns[assetId] ?? []), entityId],
      },
    })),
  updateAsset: (assetId, data, sizeKb) =>
    set((state) => ({
      assets: state.assets.map((a) =>
        a.id === assetId ? { ...a, data, sizeKb } : a
      ),
    })),
  clearSpawns: (assetId) =>
    set((state) => ({
      assetSpawns: { ...state.assetSpawns, [assetId]: [] },
    })),

  executeCommand: (cmd, setEntities) => {
    cmd.execute();
    setEntities(get().entities); // will be overridden by caller
    set((state) => ({
      undoStack: [...state.undoStack, cmd],
      redoStack: [],
    }));
  },

  // Add to history without executing (use when action already applied live)
  pushCommand: (cmd) => {
    set((state) => ({
      undoStack: [...state.undoStack, cmd],
      redoStack: [],
    }));
  },

  undo: (setEntities) => {
    const { undoStack, redoStack } = get();
    if (undoStack.length === 0) return;
    const cmd = undoStack[undoStack.length - 1];
    cmd.undo();
    setEntities(get().entities); // caller syncs
    set({
      undoStack: undoStack.slice(0, -1),
      redoStack: [...redoStack, cmd],
    });
  },

  redo: (setEntities) => {
    const { undoStack, redoStack } = get();
    if (redoStack.length === 0) return;
    const cmd = redoStack[redoStack.length - 1];
    cmd.execute();
    setEntities(get().entities); // caller syncs
    set({
      undoStack: [...undoStack, cmd],
      redoStack: redoStack.slice(0, -1),
    });
  },
}));
