import { useEffect, useRef } from 'react';
import { useEditorStore } from '../store/editor';

export function Console() {
  const logs = useEditorStore((s) => s.logs);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs]);

  return (
    <div style={{
      width: '100%', height: '100%',
      background: '#0f0f18',
      display: 'flex', flexDirection: 'column',
      overflow: 'hidden', fontFamily: 'monospace',
    }}>
      <div style={{
        height: 28, background: '#16161e',
        borderBottom: '1px solid #2a2a3a',
        display: 'flex', alignItems: 'center',
        paddingLeft: 10, fontSize: 11,
        fontWeight: 600, color: '#606070',
        letterSpacing: '0.08em', flexShrink: 0,
      }}>
        CONSOLE
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '4px 8px' }}>
        {logs.map((log, i) => (
          <div key={i} style={{ fontSize: 11, color: '#7a7a9a', lineHeight: 1.6, whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
            {log}
          </div>
        ))}
        {logs.length === 0 && (
          <div style={{ fontSize: 11, color: '#303040', padding: 4 }}>
            No output.
          </div>
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
