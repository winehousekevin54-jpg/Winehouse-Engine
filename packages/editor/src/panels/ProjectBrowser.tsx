import { useState, useRef } from 'react';
import { useEditorStore } from '../store/editor';
import type { AssetEntry } from '../store/editor';
import { LoadGltfCommand, syncScene } from '../commands';
import { despawn } from '../bridge/EngineAPI';

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
      <polygon points="16,4 28,10 28,22 16,28 4,22 4,10" stroke={color} strokeWidth="1.5" fill="none" strokeLinejoin="round"/>
      <line x1="16" y1="4" x2="16" y2="28" stroke={color} strokeWidth="1" strokeDasharray="2 2" opacity="0.5"/>
      <line x1="4" y1="10" x2="28" y2="10" stroke={color} strokeWidth="1" strokeDasharray="2 2" opacity="0.5"/>
      <line x1="4" y1="22" x2="28" y2="10" stroke={color} strokeWidth="1" opacity="0.3"/>
      <line x1="28" y1="22" x2="4" y2="10" stroke={color} strokeWidth="1" opacity="0.3"/>
    </svg>
  );
}

// ── Reload icon ───────────────────────────────────────────────────────────────

function ReloadIcon() {
  return (
    <svg width="11" height="11" viewBox="0 0 16 16" fill="none">
      <path d="M13.5 8A5.5 5.5 0 1 1 8 2.5c1.8 0 3.4.87 4.4 2.2L11 6h4V2l-1.5 1.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
    </svg>
  );
}

// ── Asset tile ────────────────────────────────────────────────────────────────

function AssetTile({
  asset,
  onSpawn,
  onReload,
}: {
  asset: AssetEntry;
  onSpawn: (asset: AssetEntry) => void;
  onReload: (asset: AssetEntry, data: Uint8Array, sizeKb: number) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const reloadInputRef = useRef<HTMLInputElement>(null);

  function handleDragStart(e: React.DragEvent) {
    e.dataTransfer.setData('winehouse/assetId', asset.id);
    e.dataTransfer.effectAllowed = 'copy';
  }

  async function handleReloadFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    e.target.value = '';
    const buffer = await file.arrayBuffer();
    const data = new Uint8Array(buffer);
    onReload(asset, data, Math.round(data.byteLength / 1024));
  }

  return (
    <div
      draggable
      title={`${asset.name} · ${asset.sizeKb} KB\nDouble-click to add · Drag into viewport`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onDragStart={handleDragStart}
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
        cursor: 'grab',
        userSelect: 'none',
        WebkitUserSelect: 'none',
        position: 'relative',
      }}
    >
      <ModelIcon color={hovered ? '#7c5cfc' : '#4a4a6a'} />

      {/* Reload button — top-right corner, visible on hover */}
      {hovered && (
        <div
          title="Reload asset from file"
          onClick={(e) => { e.stopPropagation(); reloadInputRef.current?.click(); }}
          style={{
            position: 'absolute', top: 4, right: 4,
            color: '#606080', cursor: 'pointer',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: 16, height: 16, borderRadius: 3,
            background: 'rgba(255,255,255,0.06)',
          }}
        >
          <ReloadIcon />
        </div>
      )}
      <input
        ref={reloadInputRef}
        type="file"
        accept=".glb,.gltf"
        style={{ display: 'none' }}
        onChange={handleReloadFile}
      />

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
  const assetSpawns = useEditorStore((s) => s.assetSpawns);
  const pushCommand = useEditorStore((s) => s.pushCommand);
  const trackSpawn = useEditorStore((s) => s.trackSpawn);
  const updateAsset = useEditorStore((s) => s.updateAsset);
  const clearSpawns = useEditorStore((s) => s.clearSpawns);
  const setEntities = useEditorStore((s) => s.setEntities);

  async function spawnAsset(asset: AssetEntry): Promise<number> {
    const cmd = new LoadGltfCommand(asset.data, asset.name);
    await cmd.executeAsync();
    pushCommand(cmd);
    syncScene(setEntities);
    trackSpawn(asset.id, cmd.spawnedId);
    return cmd.spawnedId;
  }

  async function handleReload(asset: AssetEntry, newData: Uint8Array, newSizeKb: number) {
    // Despawn all existing instances of this asset
    const spawnedIds = assetSpawns[asset.id] ?? [];
    spawnedIds.forEach((id) => despawn(id));
    clearSpawns(asset.id);

    // Update asset data in store
    updateAsset(asset.id, newData, newSizeKb);

    // Re-spawn one fresh instance with the new geometry
    const updatedAsset: AssetEntry = { ...asset, data: newData, sizeKb: newSizeKb };
    const cmd = new LoadGltfCommand(updatedAsset.data, updatedAsset.name);
    await cmd.executeAsync();
    pushCommand(cmd);
    syncScene(setEntities);
    trackSpawn(asset.id, cmd.spawnedId);
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
            Drag a tile into the viewport or double-click to add.
          </div>
        ) : (
          assets.map((a) => (
            <AssetTile
              key={a.id}
              asset={a}
              onSpawn={spawnAsset}
              onReload={handleReload}
            />
          ))
        )}
      </div>
    </div>
  );
}
