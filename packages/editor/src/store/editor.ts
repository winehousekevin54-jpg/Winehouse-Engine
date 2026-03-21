import { create } from 'zustand';
import type { Command } from '../commands';
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

  // Undo / Redo
  undoStack: Command[];
  redoStack: Command[];

  // Actions
  setEntities: (entities: SceneObjectInfo[]) => void;
  selectEntity: (id: number | null) => void;
  setEngineStatus: (status: 'loading' | 'running' | 'error', error?: string) => void;
  addLog: (message: string) => void;

  executeCommand: (cmd: Command, setEntities: (e: SceneObjectInfo[]) => void) => void;
  undo: (setEntities: (e: SceneObjectInfo[]) => void) => void;
  redo: (setEntities: (e: SceneObjectInfo[]) => void) => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  entities: [],
  selectedId: null,
  engineStatus: 'loading',
  engineError: null,
  logs: [],
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

  executeCommand: (cmd, setEntities) => {
    cmd.execute();
    setEntities(get().entities); // will be overridden by caller
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
