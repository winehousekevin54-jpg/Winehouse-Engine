import { useState, useEffect, useRef } from 'react';
import { useEditorStore } from '../store/editor';
import { SetTransformCommand, SetMaterialCommand, syncScene } from '../commands';
import { setMaterial } from '../bridge/EngineAPI';
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
      onBlur={() => { const n = parseFloat(local); if (!isNaN(n)) onChange(n); }}
      onKeyDown={(e) => { if (e.key === 'Enter') { const n = parseFloat(local); if (!isNaN(n)) onChange(n); } }}
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
          <span style={{ fontSize: 10, color: axis === 'x' ? '#f87171' : axis === 'y' ? '#4ade80' : '#60a5fa', width: 8, flexShrink: 0 }}>
            {axis.toUpperCase()}
          </span>
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
  const executeCommand = useEditorStore((s) => s.executeCommand);
  const pushCommand = useEditorStore((s) => s.pushCommand);

  // Track albedo at the moment the color picker opens, for single-commit undo
  const colorPickerStart = useRef<[number, number, number] | null>(null);

  const entity = entities.find((e) => e.id === selectedId) ?? null;

  function applyTransform(partial: Partial<Pick<SceneObjectInfo, 'position' | 'rotation' | 'scale'>>) {
    if (!entity) return;
    const before = { position: entity.position, rotation: entity.rotation, scale: entity.scale };
    const after = {
      position: partial.position ?? entity.position,
      rotation: partial.rotation ?? entity.rotation,
      scale: partial.scale ?? entity.scale,
    };
    const cmd = new SetTransformCommand(entity.id, before, after);
    executeCommand(cmd, () => syncScene(setEntities));
    syncScene(setEntities);
  }

  function applyMaterial(partial: Partial<Pick<SceneObjectInfo, 'albedo' | 'metallic' | 'roughness'>>) {
    if (!entity) return;
    const before = { albedo: entity.albedo, metallic: entity.metallic, roughness: entity.roughness };
    const after = {
      albedo: partial.albedo ?? entity.albedo,
      metallic: partial.metallic ?? entity.metallic,
      roughness: partial.roughness ?? entity.roughness,
    };
    const cmd = new SetMaterialCommand(entity.id, before, after);
    executeCommand(cmd, () => syncScene(setEntities));
    syncScene(setEntities);
  }

  if (!entity) {
    return (
      <div style={PANEL}>
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
        <Vec3Row label="Position" values={entity.position} onChange={(v) => applyTransform({ position: v })} />
        <Vec3Row label="Scale" values={entity.scale} onChange={(v) => applyTransform({ scale: v })} />

        <div style={SECTION_LABEL}>MATERIAL</div>
        <Vec3Row label="Albedo" values={entity.albedo} onChange={(v) => applyMaterial({ albedo: v })} />
        <div style={{ display: 'flex', alignItems: 'center', padding: '3px 10px', gap: 4 }}>
          <span style={{ width: 60, fontSize: 11, color: '#606070', flexShrink: 0 }}>Metallic</span>
          <div style={{ flex: 1 }}>
            <NumberInput value={entity.metallic} step={0.05}
              onChange={(v) => applyMaterial({ metallic: Math.max(0, Math.min(1, v)) })} />
          </div>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', padding: '3px 10px', gap: 4 }}>
          <span style={{ width: 60, fontSize: 11, color: '#606070', flexShrink: 0 }}>Roughness</span>
          <div style={{ flex: 1 }}>
            <NumberInput value={entity.roughness} step={0.05}
              onChange={(v) => applyMaterial({ roughness: Math.max(0, Math.min(1, v)) })} />
          </div>
        </div>

        {/* Clickable color picker swatch — live preview, single undo entry */}
        <div style={{ padding: '6px 10px' }}>
          <label style={{ display: 'block', cursor: 'pointer', position: 'relative' }} title="Click to open color picker">
            <div
              style={{
                height: 22, borderRadius: 4,
                background: `rgb(${Math.round(entity.albedo[0] * 255)}, ${Math.round(entity.albedo[1] * 255)}, ${Math.round(entity.albedo[2] * 255)})`,
                border: '2px solid #3a3a5a',
                boxShadow: '0 0 0 1px #1a1a2a',
                transition: 'border-color 0.15s',
              }}
              onMouseEnter={e => (e.currentTarget.style.borderColor = '#6060cc')}
              onMouseLeave={e => (e.currentTarget.style.borderColor = '#3a3a5a')}
            />
            <input
              type="color"
              value={`#${Math.round(entity.albedo[0] * 255).toString(16).padStart(2, '0')}${Math.round(entity.albedo[1] * 255).toString(16).padStart(2, '0')}${Math.round(entity.albedo[2] * 255).toString(16).padStart(2, '0')}`}
              onFocus={() => {
                // Snapshot the color at the moment the picker opens
                colorPickerStart.current = entity.albedo;
              }}
              onChange={(e) => {
                // Live preview: apply directly to WASM, no command yet
                const hex = e.target.value;
                const r = parseInt(hex.slice(1, 3), 16) / 255;
                const g = parseInt(hex.slice(3, 5), 16) / 255;
                const b = parseInt(hex.slice(5, 7), 16) / 255;
                setMaterial(entity.id, r, g, b, entity.metallic, entity.roughness);
                syncScene(setEntities);
              }}
              onBlur={(e) => {
                // Picker closed: commit ONE command covering the full change
                if (!colorPickerStart.current) return;
                const hex = e.target.value;
                const r = parseInt(hex.slice(1, 3), 16) / 255;
                const g = parseInt(hex.slice(3, 5), 16) / 255;
                const b = parseInt(hex.slice(5, 7), 16) / 255;
                const finalAlbedo: [number, number, number] = [r, g, b];
                const before = colorPickerStart.current;
                colorPickerStart.current = null;
                if (before.join() === finalAlbedo.join()) return; // no change
                pushCommand(new SetMaterialCommand(
                  entity.id,
                  { albedo: before,       metallic: entity.metallic, roughness: entity.roughness },
                  { albedo: finalAlbedo,  metallic: entity.metallic, roughness: entity.roughness },
                ));
              }}
              style={{ position: 'absolute', opacity: 0, width: 0, height: 0, pointerEvents: 'none' }}
            />
          </label>
        </div>
      </div>
    </div>
  );
}
