import { useRef } from 'react';
import { useEditorStore } from '../store/editor';
import { SpawnCubeCommand, DespawnCommand, LoadGltfCommand, syncScene } from '../commands';

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
  gap: 6,
};

export function Hierarchy() {
  const entities = useEditorStore((s) => s.entities);
  const selectedId = useEditorStore((s) => s.selectedId);
  const selectEntity = useEditorStore((s) => s.selectEntity);
  const setEntities = useEditorStore((s) => s.setEntities);
  const engineStatus = useEditorStore((s) => s.engineStatus);
  const executeCommand = useEditorStore((s) => s.executeCommand);
  const pushCommand = useEditorStore((s) => s.pushCommand);
  const addAsset = useEditorStore((s) => s.addAsset);
  const trackSpawn = useEditorStore((s) => s.trackSpawn);
  const fileInputRef = useRef<HTMLInputElement>(null);

  function handleAdd() {
    if (engineStatus !== 'running') return;
    const count = entities.length;
    const cmd = new SpawnCubeCommand(
      `Cube ${count + 1}`,
      [(Math.random() - 0.5) * 4, (Math.random() - 0.5) * 4, (Math.random() - 0.5) * 4],
      [Math.random(), Math.random(), Math.random()],
    );
    executeCommand(cmd, () => syncScene(setEntities));
    syncScene(setEntities);
  }

  async function handleGltfFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file || engineStatus !== 'running') return;
    e.target.value = '';
    const buffer = await file.arrayBuffer();
    const data = new Uint8Array(buffer);
    const name = file.name.replace(/\.(glb|gltf)$/i, '');
    const assetId = `${Date.now()}-${name}`;
    const cmd = new LoadGltfCommand(data, name);
    await cmd.executeAsync();
    pushCommand(cmd);
    syncScene(setEntities);
    addAsset({ id: assetId, name, type: 'gltf', data, sizeKb: Math.round(data.byteLength / 1024) });
    trackSpawn(assetId, cmd.spawnedId);
  }

  function handleDelete(id: number) {
    const entity = entities.find((e) => e.id === id);
    if (!entity) return;
    const cmd = new DespawnCommand(entity);
    executeCommand(cmd, () => syncScene(setEntities));
    syncScene(setEntities);
    if (selectedId === id) selectEntity(null);
  }

  return (
    <div style={PANEL}>
      <div style={HEADER}>
        HIERARCHY
        <div style={{ marginLeft: 'auto', display: 'flex', gap: 4, marginRight: 6 }}>
          <button
            onClick={handleAdd}
            title="Add Cube"
            style={{
              background: '#2a2a4a', border: '1px solid #3a3a5a',
              color: '#a0a0c0', borderRadius: 3, cursor: 'pointer',
              fontSize: 14, lineHeight: 1, padding: '1px 6px',
            }}
          >+</button>
          <button
            onClick={() => fileInputRef.current?.click()}
            title="Import .glb / .gltf"
            style={{
              background: '#2a2a4a', border: '1px solid #3a3a5a',
              color: '#a0a0c0', borderRadius: 3, cursor: 'pointer',
              fontSize: 10, lineHeight: 1, padding: '2px 5px',
            }}
          >GLB</button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".glb,.gltf"
            style={{ display: 'none' }}
            onChange={handleGltfFile}
          />
        </div>
      </div>
      <div style={{ flex: 1, overflowY: 'auto' }}>
        {entities.map((e) => (
          <div
            key={e.id}
            onClick={() => selectEntity(e.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              padding: '4px 10px',
              fontSize: 12,
              color: selectedId === e.id ? '#c8c8ff' : '#909090',
              background: selectedId === e.id ? '#1e1e32' : 'transparent',
              cursor: 'pointer',
              gap: 6,
              borderLeft: selectedId === e.id ? '2px solid #6060cc' : '2px solid transparent',
            }}
          >
            <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {e.name}
            </span>
            <button
              onClick={(ev) => { ev.stopPropagation(); handleDelete(e.id); }}
              title="Delete"
              style={{
                background: 'none', border: 'none', color: '#505060',
                cursor: 'pointer', fontSize: 12, padding: '0 2px',
                lineHeight: 1, opacity: 0.6,
              }}
            >✕</button>
          </div>
        ))}
        {entities.length === 0 && (
          <div style={{ padding: 16, fontSize: 11, color: '#404050', textAlign: 'center' }}>
            No entities. Click + to add a cube.
          </div>
        )}
      </div>
    </div>
  );
}
