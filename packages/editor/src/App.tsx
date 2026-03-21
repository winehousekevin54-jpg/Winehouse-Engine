import { useEffect } from 'react';
import { initEngine } from './bridge/EngineAPI';
import { useEditorStore } from './store/editor';
import { SceneView } from './panels/SceneView';
import { Hierarchy } from './panels/Hierarchy';
import { Inspector } from './panels/Inspector';
import { Console } from './panels/Console';

export function App() {
  const engineStatus = useEditorStore((s) => s.engineStatus);
  const engineError = useEditorStore((s) => s.engineError);
  const setEngineStatus = useEditorStore((s) => s.setEngineStatus);
  const addLog = useEditorStore((s) => s.addLog);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        addLog('Initializing engine…');
        await initEngine('viewport');
        if (cancelled) return;
        addLog('Engine initialized — WebGPU ready.');
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

  const statusColor =
    engineStatus === 'running' ? '#4ade80' :
    engineStatus === 'error'   ? '#f87171' :
                                  '#fbbf24';

  return (
    <div style={{
      width: '100vw', height: '100vh',
      display: 'flex', flexDirection: 'column',
      background: '#0d0d12', overflow: 'hidden',
      fontFamily: '"Inter", "Segoe UI", system-ui, sans-serif',
    }}>
      {/* ── Title bar ── */}
      <div style={{
        height: 36, flexShrink: 0,
        background: '#16161e',
        borderBottom: '1px solid #2a2a3a',
        display: 'flex', alignItems: 'center',
        paddingLeft: 16, gap: 8,
        fontSize: 13, fontWeight: 700,
        color: '#a0a0b0', letterSpacing: '0.05em',
      }}>
        WINEHOUSE ENGINE
        <span style={{ fontSize: 11, color: '#404050', fontWeight: 400 }}>v0.1.0</span>
        <span style={{ marginLeft: 'auto', marginRight: 16, fontSize: 11, color: statusColor }}>
          {engineStatus === 'loading' ? '● Initializing WebGPU…' :
           engineStatus === 'running' ? '● WebGPU Active' :
           '● Error'}
        </span>
      </div>

      {/* ── Main workspace ── */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        {/* Top row: Hierarchy | Scene | Inspector */}
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>

          {/* Hierarchy */}
          <div style={{
            width: 200, flexShrink: 0,
            borderRight: '1px solid #2a2a3a',
          }}>
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
          <div style={{
            width: 220, flexShrink: 0,
            borderLeft: '1px solid #2a2a3a',
          }}>
            <Inspector />
          </div>
        </div>

        {/* Console */}
        <div style={{
          height: 140, flexShrink: 0,
          borderTop: '1px solid #2a2a3a',
        }}>
          <Console />
        </div>
      </div>
    </div>
  );
}
