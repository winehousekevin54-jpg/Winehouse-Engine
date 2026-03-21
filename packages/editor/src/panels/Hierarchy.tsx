import { useEditorStore } from '../store/editor';
import { spawnCube, despawn, getSceneObjects } from '../bridge/EngineAPI';

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

  function handleAdd() {
    if (engineStatus !== 'running') return;
    const count = entities.length;
    spawnCube(
      `Cube ${count + 1}`,
      (Math.random() - 0.5) * 4,
      (Math.random() - 0.5) * 4,
      (Math.random() - 0.5) * 4,
      Math.random(), Math.random(), Math.random(),
    );
    setEntities(getSceneObjects());
  }

  function handleDelete(id: number) {
    despawn(id);
    setEntities(getSceneObjects());
    if (selectedId === id) selectEntity(null);
  }

  return (
    <div style={PANEL}>
      <div style={HEADER}>
        HIERARCHY
        <button
          onClick={handleAdd}
          title="Add Cube"
          style={{
            marginLeft: 'auto', marginRight: 6,
            background: '#2a2a4a', border: '1px solid #3a3a5a',
            color: '#a0a0c0', borderRadius: 3, cursor: 'pointer',
            fontSize: 14, lineHeight: 1, padding: '1px 6px',
          }}
        >+</button>
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
