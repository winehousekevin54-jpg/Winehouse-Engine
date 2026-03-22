import { useState } from 'react';
import { useEditorStore } from '../store/editor';
import type { AssetEntry } from '../store/editor';
import { LoadGltfCommand, syncScene } from '../commands';

const PANEL: React.CSSProperties = {
  width: '100%',
  height: '100%',
  background: '#13131a',
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
};

const HEADER: React.CSSProperties = {
  height: 28,
  background: '#16161e',
  borderBottom: '1px solid #2a2a3a',
  display: 'flex',
  alignItems: 'center',
  paddingLeft: 10,
  fontSize: 11,
  fontWeight: 600,
  color: '#606070',
  letterSpacing: '0.08em',
  flexShrink: 0,
};

// ── 3D model SVG icon ─────────────────────────────────────────────────────────

function ModelIcon({ color }: { color: string }) {
  return (
    <svg width="32" height="32" viewBox="0 0 32 32" fill="none">
      {/* cube outline */}
      <polygon points="16,4 28,10 28,22 16,28 4,22 4,10" stroke={color} strokeWidth="1.5" fill="none" strokeLinejoin="round"/>
      <line x1="16" y1="4" x2="16" y2="28" stroke={color} strokeWidth="1" strokeDasharray="2 2" opacity="0.5"/>
      <line x1="4" y1="10" x2="28" y2="10" stroke={color} strokeWidth="1" strokeDasharray="2 2" opacity="0.5"/>
      <line x1="4" y1="22" x2="28" y2="10" stroke={color} strokeWidth="1" opacity="0.3"/>
      <line x1="28" y1="22" x2="4" y2="10" stroke={color} strokeWidth="1" opacity="0.3"/>
    </svg>
  );
}

// ── Asset tile ────────────────────────────────────────────────────────────────

function AssetTile({ asset, onSpawn }: { asset: AssetEntry; onSpawn: (asset: AssetEntry) => void }) {
  const [hovered, setHovered] = useState(false);

  return (
    <div
      title={`${asset.name}\n${asset.sizeKb} KB — double-click to add to scene`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onDoubleClick={() => onSpawn(asset)}
      style={{
        width: 80,
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 5,
        padding: '8px 4px 6px',
        borderRadius: 6,
        background: hovered ? 'rgba(255,255,255,0.06)' : 'transparent',
        border: `1px solid ${hovered ? 'rgba(255,255,255,0.10)' : 'transparent'}`,
        cursor: 'pointer',
        userSelect: 'none',
        WebkitUserSelect: 'none',
      }}
    >
      <ModelIcon color={hovered ? '#7c5cfc' : '#4a4a6a'} />
      <span style={{
        fontSize: 10,
        color: hovered ? '#c0c0e0' : '#606070',
        textAlign: 'center',
        lineHeight: 1.3,
        wordBreak: 'break-all',
        maxWidth: '100%',
        overflow: 'hidden',
        display: '-webkit-box',
        WebkitLineClamp: 2,
        WebkitBoxOrient: 'vertical',
      }}>
        {asset.name}
      </span>
      <span style={{ fontSize: 9, color: '#404050' }}>{asset.sizeKb} KB</span>
    </div>
  );
}

// ── ProjectBrowser ────────────────────────────────────────────────────────────

export function ProjectBrowser() {
  const assets = useEditorStore((s) => s.assets);
  const pushCommand = useEditorStore((s) => s.pushCommand);
  const setEntities = useEditorStore((s) => s.setEntities);

  async function handleSpawn(asset: AssetEntry) {
    const cmd = new LoadGltfCommand(asset.data, asset.name);
    await cmd.executeAsync();
    pushCommand(cmd);
    syncScene(setEntities);
  }

  return (
    <div style={PANEL}>
      <div style={HEADER}>
        PROJECT
        <span style={{ marginLeft: 6, color: '#404050', fontWeight: 400, fontSize: 10 }}>
          {assets.length} asset{assets.length !== 1 ? 's' : ''}
        </span>
      </div>
      <div style={{
        flex: 1,
        overflowY: 'auto',
        padding: 8,
        display: 'flex',
        flexWrap: 'wrap',
        alignContent: 'flex-start',
        gap: 4,
      }}>
        {assets.length === 0 ? (
          <div style={{
            width: '100%',
            paddingTop: 24,
            fontSize: 11,
            color: '#404050',
            textAlign: 'center',
          }}>
            Import a .glb file via the Hierarchy panel.
            <br />
            Double-click a tile to add it to the scene.
          </div>
        ) : (
          assets.map((a) => (
            <AssetTile key={a.id} asset={a} onSpawn={handleSpawn} />
          ))
        )}
      </div>
    </div>
  );
}
