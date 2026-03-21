import { useEffect, useRef, useState } from 'react';

export function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState<'loading' | 'running' | 'error'>('loading');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function init() {
      try {
        const wasm = await import('../../../crates/winehouse-wasm-bridge/pkg/winehouse_wasm_bridge');
        // Initialize the WASM module (loads the .wasm file)
        await wasm.default();
        if (cancelled) return;
        await wasm.start_renderer('viewport');
        if (cancelled) return;
        setStatus('running');
      } catch (e) {
        if (cancelled) return;
        const msg = e instanceof Error ? e.message : String(e);
        console.error('Engine init failed:', msg);
        setError(msg);
        setStatus('error');
      }
    }

    init();
    return () => { cancelled = true; };
  }, []);

  return (
    <div style={{
      width: '100%',
      height: '100%',
      display: 'flex',
      flexDirection: 'column',
      background: '#0d0d12',
    }}>
      {/* Title Bar */}
      <div style={{
        height: 36,
        background: '#16161e',
        borderBottom: '1px solid #2a2a3a',
        display: 'flex',
        alignItems: 'center',
        paddingLeft: 16,
        fontSize: 13,
        fontWeight: 600,
        color: '#a0a0b0',
        letterSpacing: '0.05em',
        flexShrink: 0,
      }}>
        WINEHOUSE ENGINE
        <span style={{ marginLeft: 12, fontSize: 11, color: '#505068', fontWeight: 400 }}>
          v0.1.0
        </span>
        <span style={{
          marginLeft: 'auto',
          marginRight: 16,
          fontSize: 11,
          color: status === 'running' ? '#4ade80' : status === 'error' ? '#f87171' : '#fbbf24',
        }}>
          {status === 'loading' ? 'Initializing WebGPU...' :
           status === 'running' ? 'WebGPU Active' :
           'Error'}
        </span>
      </div>

      {/* Viewport */}
      <div style={{ flex: 1, position: 'relative' }}>
        <canvas
          id="viewport"
          ref={canvasRef}
          style={{
            width: '100%',
            height: '100%',
            display: 'block',
          }}
        />
        {status === 'error' && (
          <div style={{
            position: 'absolute',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            background: '#1a1a2e',
            border: '1px solid #f87171',
            borderRadius: 8,
            padding: '24px 32px',
            maxWidth: 500,
            textAlign: 'center',
          }}>
            <div style={{ color: '#f87171', fontSize: 16, fontWeight: 600, marginBottom: 8 }}>
              WebGPU Initialization Failed
            </div>
            <div style={{ color: '#a0a0b0', fontSize: 13 }}>
              {error}
            </div>
            <div style={{ color: '#707080', fontSize: 11, marginTop: 12 }}>
              Make sure you're using Chrome 113+ or Edge 113+ with WebGPU enabled.
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
