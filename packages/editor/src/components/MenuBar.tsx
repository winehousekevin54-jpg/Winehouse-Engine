import { useState, useEffect, useRef, useCallback } from 'react';

// ── Types ─────────────────────────────────────────────────────────────────────

interface MenuItemDef {
  label: string;
  shortcut?: string;
  disabled?: boolean;
  separator?: false;
  onClick?: () => void;
}
interface SeparatorDef { separator: true }
type MenuEntry = MenuItemDef | SeparatorDef;

interface MenuDef {
  label: string;
  entries: MenuEntry[];
}

interface Props {
  menus: MenuDef[];
  statusLabel: string;
  statusColor: string;
}

// ── Styles ────────────────────────────────────────────────────────────────────

const BAR: React.CSSProperties = {
  height: 28,
  flexShrink: 0,
  background: 'rgba(24, 24, 34, 0.92)',
  backdropFilter: 'blur(24px) saturate(160%)',
  WebkitBackdropFilter: 'blur(24px) saturate(160%)',
  borderBottom: '1px solid rgba(255,255,255,0.06)',
  display: 'flex',
  alignItems: 'center',
  paddingLeft: 6,
  paddingRight: 12,
  fontFamily: '-apple-system, "SF Pro Text", "Segoe UI", system-ui, sans-serif',
  fontSize: 13,
  userSelect: 'none',
  WebkitUserSelect: 'none',
  position: 'relative',
  zIndex: 1000,
};

const MENU_ITEM_BASE: React.CSSProperties = {
  padding: '1px 9px',
  borderRadius: 5,
  cursor: 'default',
  color: '#d8d8e8',
  fontWeight: 400,
  fontSize: 13,
  lineHeight: '26px',
  whiteSpace: 'nowrap',
};

const DROPDOWN: React.CSSProperties = {
  position: 'absolute',
  top: 'calc(100% + 2px)',
  background: 'rgba(36, 36, 48, 0.97)',
  backdropFilter: 'blur(32px) saturate(180%)',
  WebkitBackdropFilter: 'blur(32px) saturate(180%)',
  borderRadius: 8,
  border: '1px solid rgba(255,255,255,0.10)',
  boxShadow: '0 8px 32px rgba(0,0,0,0.7), 0 2px 8px rgba(0,0,0,0.4)',
  minWidth: 200,
  padding: '4px 0',
  zIndex: 9999,
};

const DROP_ITEM_BASE: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '0 14px',
  height: 22,
  fontSize: 13,
  cursor: 'default',
  borderRadius: 4,
  margin: '0 4px',
};

// ── Dropdown menu ─────────────────────────────────────────────────────────────

function Dropdown({ entries, onClose }: { entries: MenuEntry[]; onClose: () => void }) {
  const [hovered, setHovered] = useState<number | null>(null);

  return (
    <div style={DROPDOWN} onMouseLeave={() => setHovered(null)}>
      {entries.map((entry, i) => {
        if ('separator' in entry && entry.separator) {
          return (
            <div key={i} style={{
              height: 1,
              background: 'rgba(255,255,255,0.08)',
              margin: '3px 0',
            }} />
          );
        }
        const item = entry as MenuItemDef;
        const isHovered = hovered === i;
        const isDisabled = item.disabled ?? false;

        return (
          <div
            key={i}
            style={{
              ...DROP_ITEM_BASE,
              background: isHovered && !isDisabled ? '#0058d1' : 'transparent',
              color: isDisabled ? 'rgba(255,255,255,0.25)' : isHovered ? '#fff' : '#d8d8e8',
              cursor: isDisabled ? 'default' : 'pointer',
            }}
            onMouseEnter={() => setHovered(i)}
            onClick={() => {
              if (!isDisabled && item.onClick) {
                item.onClick();
                onClose();
              }
            }}
          >
            <span>{item.label}</span>
            {item.shortcut && (
              <span style={{
                fontSize: 12,
                color: isHovered && !isDisabled ? 'rgba(255,255,255,0.75)' : 'rgba(255,255,255,0.3)',
                marginLeft: 24,
              }}>
                {item.shortcut}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ── Single top-level menu ──────────────────────────────────────────────────────

function TopMenu({ label, entries, isOpen, onOpen, onClose }: {
  label: string;
  entries: MenuEntry[];
  isOpen: boolean;
  onOpen: () => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  return (
    <div ref={ref} style={{ position: 'relative' }}>
      <div
        style={{
          ...MENU_ITEM_BASE,
          background: isOpen ? 'rgba(255,255,255,0.12)' : 'transparent',
          color: isOpen ? '#fff' : '#d8d8e8',
        }}
        onMouseEnter={onOpen}
        onClick={isOpen ? onClose : onOpen}
      >
        {label}
      </div>
      {isOpen && <Dropdown entries={entries} onClose={onClose} />}
    </div>
  );
}

// ── MenuBar ────────────────────────────────────────────────────────────────────

export function MenuBar({ menus, statusLabel, statusColor }: Props) {
  const [openMenu, setOpenMenu] = useState<number | null>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const closeAll = useCallback(() => setOpenMenu(null), []);

  // Close on outside click
  useEffect(() => {
    if (openMenu === null) return;
    function handle(e: MouseEvent) {
      if (barRef.current && !barRef.current.contains(e.target as Node)) {
        closeAll();
      }
    }
    document.addEventListener('mousedown', handle);
    return () => document.removeEventListener('mousedown', handle);
  }, [openMenu, closeAll]);

  return (
    <div ref={barRef} style={BAR}>
      {/* App icon + name */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 7, paddingRight: 6 }}>
        <div style={{
          width: 15, height: 15, borderRadius: 4, flexShrink: 0,
          background: 'linear-gradient(135deg, #7c5cfc 0%, #4f8eff 100%)',
        }} />
        <span style={{ fontWeight: 700, fontSize: 13, color: '#e8e8f4', letterSpacing: '-0.01em' }}>
          Winehouse
        </span>
      </div>

      {/* Menu items */}
      {menus.map((menu, i) => (
        <TopMenu
          key={menu.label}
          label={menu.label}
          entries={menu.entries}
          isOpen={openMenu === i}
          onOpen={() => setOpenMenu(i)}
          onClose={closeAll}
        />
      ))}

      {/* Right-side status — icon only, dropdown on click (macOS menu-bar-extra style) */}
      <div style={{ marginLeft: 'auto', position: 'relative' }}>
        <div
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            width: 28, height: 26,
            borderRadius: 5,
            cursor: 'default',
            background: openMenu === -1 ? 'rgba(255,255,255,0.12)' : 'transparent',
          }}
          onClick={() => setOpenMenu(openMenu === -1 ? null : -1)}
        >
          <div style={{
            width: 8, height: 8, borderRadius: '50%',
            background: statusColor,
            boxShadow: `0 0 6px ${statusColor}`,
          }} />
        </div>
        {openMenu === -1 && (
          <div style={{ ...DROPDOWN, right: 0, left: 'auto', minWidth: 180 }}>
            <div style={{
              ...DROP_ITEM_BASE,
              margin: '0 4px',
              color: '#a0a0b0',
              fontSize: 11,
              height: 'auto',
              padding: '6px 14px 4px',
              display: 'block',
              cursor: 'default',
            }}>
              RENDERER STATUS
            </div>
            <div style={{ height: 1, background: 'rgba(255,255,255,0.08)', margin: '3px 0' }} />
            <div style={{ ...DROP_ITEM_BASE, margin: '0 4px', color: statusColor, cursor: 'default' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div style={{ width: 6, height: 6, borderRadius: '50%', background: statusColor, boxShadow: `0 0 5px ${statusColor}`, flexShrink: 0 }} />
                {statusLabel}
              </span>
            </div>
            <div style={{ ...DROP_ITEM_BASE, margin: '0 4px', color: 'rgba(255,255,255,0.3)', fontSize: 11, cursor: 'default' }}>
              Backend: WebGPU
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
