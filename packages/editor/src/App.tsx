import { useEffect, useCallback, useState } from 'react';
import { initEngine, getSceneObjects, resizeViewport } from './bridge/EngineAPI';
import { useEditorStore } from './store/editor';
import { syncScene } from './commands';
import { MenuBar } from './components/MenuBar';
import { SceneView } from './panels/SceneView';
import { Hierarchy } from './panels/Hierarchy';
import { Inspector } from './panels/Inspector';
import { Console } from './panels/Console';
import { ProjectBrowser } from './panels/ProjectBrowser';

export function App() {
  const [bottomTab, setBottomTab] = useState<'console' | 'project'>('console');
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
        // Force resize to the real canvas size after init — the engine
        // may have been initialized before the layout was finalized.
        const canvas = document.getElementById('viewport') as HTMLCanvasElement | null;
        if (canvas) {
          const dpr = window.devicePixelRatio || 1;
          const w = Math.round(canvas.clientWidth * dpr);
          const h = Math.round(canvas.clientHeight * dpr);
          if (w > 0 && h > 0) {
            canvas.width = w;
            canvas.height = h;
            resizeViewport(w, h);
          }
        }
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

        {/* Bottom panel with tabs */}
        <div style={{ height: 160, flexShrink: 0, borderTop: '1px solid #2a2a3a', display: 'flex', flexDirection: 'column' }}>
          {/* Tab bar */}
          <div style={{
            height: 28, flexShrink: 0,
            background: '#16161e',
            borderBottom: '1px solid #2a2a3a',
            display: 'flex', alignItems: 'flex-end',
            paddingLeft: 8, gap: 2,
          }}>
            {(['console', 'project'] as const).map((tab) => (
              <div
                key={tab}
                onClick={() => setBottomTab(tab)}
                style={{
                  padding: '4px 12px',
                  fontSize: 11, fontWeight: 600,
                  letterSpacing: '0.06em',
                  color: bottomTab === tab ? '#c0c0e0' : '#505060',
                  background: bottomTab === tab ? '#13131a' : 'transparent',
                  borderTop: bottomTab === tab ? '1px solid #2a2a3a' : '1px solid transparent',
                  borderLeft: bottomTab === tab ? '1px solid #2a2a3a' : '1px solid transparent',
                  borderRight: bottomTab === tab ? '1px solid #2a2a3a' : '1px solid transparent',
                  borderRadius: '4px 4px 0 0',
                  cursor: 'pointer',
                  userSelect: 'none',
                  marginBottom: bottomTab === tab ? -1 : 0,
                }}
              >
                {tab.toUpperCase()}
              </div>
            ))}
          </div>
          {/* Tab content */}
          <div style={{ flex: 1, overflow: 'hidden' }}>
            {bottomTab === 'console' ? <Console /> : <ProjectBrowser />}
          </div>
        </div>
      </div>
    </div>
  );
}
