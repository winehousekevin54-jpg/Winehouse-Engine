import { useState, useEffect } from 'react';
import { useEditorStore } from '../store/editor';
import { setTransform, setMaterial, getSceneObjects } from '../bridge/EngineAPI';
import type { SceneObjectInfo } from '../bridge/EngineAPI';

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

const SECTION_LABEL: React.CSSProperties = {
  fontSize: 10,
  fontWeight: 700,
  color: '#505060',
  letterSpacing: '0.1em',
  padding: '8px 10px 4px',
  borderBottom: '1px solid #1e1e2a',
};

function NumberInput({
  value, onChange, step = 0.1,
}: { value: number; onChange: (v: number) => void; step?: number }) {
  const [local, setLocal] = useState(value.toFixed(3));
  useEffect(() => setLocal(value.toFixed(3)), [value]);

  return (
    <input
      type="number"
      value={local}
      step={step}
      onChange={(e) => setLocal(e.target.value)}
      onBlur={() => {
        const n = parseFloat(local);
        if (!isNaN(n)) onChange(n);
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          const n = parseFloat(local);
          if (!isNaN(n)) onChange(n);
        }
      }}
      style={{
        width: '100%', background: '#1a1a28', border: '1px solid #2a2a3a',
        color: '#c0c0d0', borderRadius: 3, padding: '2px 4px', fontSize: 11,
        outline: 'none', boxSizing: 'border-box',
      }}
    />
  );
}

function Vec3Row({ label, values, onChange }: {
  label: string;
  values: [number, number, number];
  onChange: (v: [number, number, number]) => void;
}) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', padding: '3px 10px', gap: 4 }}>
      <span style={{ width: 60, fontSize: 11, color: '#606070', flexShrink: 0 }}>{label}</span>
      {(['x', 'y', 'z'] as const).map((axis, i) => (
        <div key={axis} style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 2 }}>
          <span style={{ fontSize: 10, color: axis === 'x' ? '#f87171' : axis === 'y' ? '#4ade80' : '#60a5fa', width: 8, flexShrink: 0 }}>{axis.toUpperCase()}</span>
          <NumberInput
            value={values[i]}
            onChange={(v) => {
              const next = [...values] as [number, number, number];
              next[i] = v;
              onChange(next);
            }}
          />
        </div>
      ))}
    </div>
  );
}

export function Inspector() {
  const entities = useEditorStore((s) => s.entities);
  const selectedId = useEditorStore((s) => s.selectedId);
  const setEntities = useEditorStore((s) => s.setEntities);

  const entity = entities.find((e) => e.id === selectedId) ?? null;

  function refresh() {
    setEntities(getSceneObjects());
  }

  function updateTransform(partial: Partial<Pick<SceneObjectInfo, 'position' | 'rotation' | 'scale'>>) {
    if (!entity) return;
    const pos = partial.position ?? entity.position;
    const rot = partial.rotation ?? entity.rotation;
    const scl = partial.scale ?? entity.scale;
    setTransform(entity.id, pos[0], pos[1], pos[2], rot[0], rot[1], rot[2], rot[3], scl[0], scl[1], scl[2]);
    refresh();
  }

  function updateMaterial(partial: Partial<Pick<SceneObjectInfo, 'albedo' | 'metallic' | 'roughness'>>) {
    if (!entity) return;
    const albedo = partial.albedo ?? entity.albedo;
    const metallic = partial.metallic ?? entity.metallic;
    const roughness = partial.roughness ?? entity.roughness;
    setMaterial(entity.id, albedo[0], albedo[1], albedo[2], metallic, roughness);
    refresh();
  }

  if (!entity) {
    return (
      <div style={{ ...PANEL }}>
        <div style={HEADER}>INSPECTOR</div>
        <div style={{ padding: 16, fontSize: 11, color: '#404050', textAlign: 'center' }}>
          Select an entity to inspect.
        </div>
      </div>
    );
  }

  return (
    <div style={PANEL}>
      <div style={HEADER}>INSPECTOR — {entity.name}</div>
      <div style={{ flex: 1, overflowY: 'auto' }}>
        <div style={SECTION_LABEL}>TRANSFORM</div>
        <Vec3Row label="Position" values={entity.position} onChange={(v) => updateTransform({ position: v })} />
        <Vec3Row label="Scale" values={entity.scale} onChange={(v) => updateTransform({ scale: v })} />

        <div style={SECTION_LABEL}>MATERIAL</div>
        <Vec3Row label="Albedo" values={entity.albedo} onChange={(v) => updateMaterial({ albedo: v })} />
        <div style={{ display: 'flex', alignItems: 'center', padding: '3px 10px', gap: 4 }}>
          <span style={{ width: 60, fontSize: 11, color: '#606070', flexShrink: 0 }}>Metallic</span>
          <div style={{ flex: 1 }}>
            <NumberInput value={entity.metallic} step={0.05} onChange={(v) => updateMaterial({ metallic: Math.max(0, Math.min(1, v)) })} />
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', padding: '3px 10px', gap: 4 }}>
          <span style={{ width: 60, fontSize: 11, color: '#606070', flexShrink: 0 }}>Roughness</span>
          <div style={{ flex: 1 }}>
            <NumberInput value={entity.roughness} step={0.05} onChange={(v) => updateMaterial({ roughness: Math.max(0, Math.min(1, v)) })} />
          </div>
        </div>

        {/* Color preview swatch */}
        <div style={{ padding: '8px 10px' }}>
          <div style={{
            height: 20,
            borderRadius: 4,
            background: `rgb(${Math.round(entity.albedo[0] * 255)}, ${Math.round(entity.albedo[1] * 255)}, ${Math.round(entity.albedo[2] * 255)})`,
            border: '1px solid #2a2a3a',
          }} />
        </div>
      </div>
    </div>
  );
}
