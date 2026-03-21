import { create } from 'zustand';
import type { SceneObjectInfo } from '../bridge/EngineAPI';

interface EditorState {
  // Scene
  entities: SceneObjectInfo[];
  selectedId: number | null;

  // Engine status
  engineStatus: 'loading' | 'running' | 'error';
  engineError: string | null;

  // Console
  logs: string[];

  // Actions
  setEntities: (entities: SceneObjectInfo[]) => void;
  selectEntity: (id: number | null) => void;
  setEngineStatus: (status: 'loading' | 'running' | 'error', error?: string) => void;
  addLog: (message: string) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  entities: [],
  selectedId: null,
  engineStatus: 'loading',
  engineError: null,
  logs: [],

  setEntities: (entities) => set({ entities }),
  selectEntity: (id) => set({ selectedId: id }),
  setEngineStatus: (status, error) =>
    set({ engineStatus: status, engineError: error ?? null }),
  addLog: (message) =>
    set((state) => ({
      logs: [...state.logs.slice(-499), `[${new Date().toLocaleTimeString()}] ${message}`],
    })),
}));
