import { useEffect, useCallback } from 'react';
import { initEngine, getSceneObjects } from './bridge/EngineAPI';
import { useEditorStore } from './store/editor';
import { syncScene } from './commands';
import { MenuBar } from './components/MenuBar';
import { SceneView } from './panels/SceneView';
import { Hierarchy } from './panels/Hierarchy';
import { Inspector } from './panels/Inspector';
import { Console } from './panels/Console';

export function App() {
  const engineStatus = useEditorStore((s) => s.engineStatus);
  const engineError = useEditorStore((s) => s.engineError);
  const setEngineStatus = useEditorStore((s) => s.setEngineStatus);
  const setEntities = useEditorStore((s) => s.setEntities);
  const addLog = useEditorStore((s) => s.addLog);
  const undo = useEditorStore((s) => s.undo);
  const redo = useEditorStore((s) => s.redo);
  const undoStack = useEditorStore((s) => s.undoStack);
  const redoStack = useEditorStore((s) => s.redoStack);

  // Engine init
  useEffect(() => {
    let cancelled = false;
    async function init() {
      try {
        addLog('Initializing engine…');
        await initEngine('viewport');
        if (cancelled) return;
        addLog('Engine initialized — WebGPU ready.');
        setEntities(getSceneObjects());
        setEngineStatus('running');
      } catch (e) {
        if (cancelled) return;
        const msg = e instanceof Error ? e.message : String(e);
        addLog(`ERROR: ${msg}`);
        setEngineStatus('error', msg);
      }
    }
    init();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Ctrl+Z / Ctrl+Y keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement) return; // don't intercept input fields
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key === 'z' && !e.shiftKey) {
      e.preventDefault();
      undo(() => syncScene(setEntities));
      syncScene(setEntities);
    }
    if (mod && (e.key === 'y' || (e.key === 'z' && e.shiftKey))) {
      e.preventDefault();
      redo(() => syncScene(setEntities));
      syncScene(setEntities);
    }
  }, [undo, redo, setEntities]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  const statusColor =
    engineStatus === 'running' ? '#4ade80' :
    engineStatus === 'error'   ? '#f87171' :
                                  '#fbbf24';

  const canUndo = undoStack.length > 0;
  const canRedo = redoStack.length > 0;

  const menus = [
    {
      label: 'Edit',
      entries: [
        {
          label: 'Undo',
          shortcut: '⌘Z',
          disabled: !canUndo,
          onClick: () => { undo(() => syncScene(setEntities)); syncScene(setEntities); },
        },
        {
          label: 'Redo',
          shortcut: '⌘⇧Z',
          disabled: !canRedo,
          onClick: () => { redo(() => syncScene(setEntities)); syncScene(setEntities); },
        },
      ],
    },
    {
      label: 'Help',
      entries: [
        { label: 'Winehouse Engine', disabled: true },
        { separator: true as const },
        { label: 'Version 0.1.0', disabled: true },
        { label: 'Phase 1 — PBR Renderer', disabled: true },
      ],
    },
  ];

  const statusLabel =
    engineStatus === 'loading' ? 'Initializing…' :
    engineStatus === 'running' ? 'WebGPU Active' : 'Error';

  return (
    <div style={{
      width: '100vw', height: '100vh',
      display: 'flex', flexDirection: 'column',
      background: '#0d0d12', overflow: 'hidden',
      fontFamily: '"Inter", "Segoe UI", system-ui, sans-serif',
    }}>
      <MenuBar menus={menus} statusLabel={statusLabel} statusColor={statusColor} />

      {/* ── Main workspace ── */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>

          {/* Hierarchy */}
          <div style={{ width: 200, flexShrink: 0, borderRight: '1px solid #2a2a3a' }}>
            <Hierarchy />
          </div>

          {/* Scene view */}
          <div style={{ flex: 1, position: 'relative' }}>
            <SceneView />
            {engineStatus === 'error' && (
              <div style={{
                position: 'absolute', top: '50%', left: '50%',
                transform: 'translate(-50%, -50%)',
                background: '#1a1a2e', border: '1px solid #f87171',
                borderRadius: 8, padding: '24px 32px',
                maxWidth: 480, textAlign: 'center',
              }}>
                <div style={{ color: '#f87171', fontSize: 16, fontWeight: 600, marginBottom: 8 }}>
                  WebGPU Initialization Failed
                </div>
                <div style={{ color: '#a0a0b0', fontSize: 13 }}>{engineError}</div>
                <div style={{ color: '#707080', fontSize: 11, marginTop: 12 }}>
                  Use Chrome 113+, Edge 113+, or Firefox Nightly with WebGPU enabled.
                </div>
              </div>
            )}
          </div>

          {/* Inspector */}
          <div style={{ width: 220, flexShrink: 0, borderLeft: '1px solid #2a2a3a' }}>
            <Inspector />
          </div>
        </div>

        {/* Console */}
        <div style={{ height: 140, flexShrink: 0, borderTop: '1px solid #2a2a3a' }}>
          <Console />
        </div>
      </div>
    </div>
  );
}
