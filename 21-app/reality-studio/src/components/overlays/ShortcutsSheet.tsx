import React from 'react';
import { Drawer } from '../ui/UiKit';

const ROWS: [string, string][] = [
  ['Ctrl or Cmd + K', 'Open the command palette'],
  ['Space', 'Play or pause the simulation'],
  ['F', 'Reset the 3D view'],
  ['Esc', 'Close the open panel'],
  ['?', 'Show this list'],
  ['Drag', 'Orbit the 3D view'],
  ['Scroll', 'Zoom the 3D view'],
  ['Click a part', 'Select it everywhere']
];

export const ShortcutsSheet: React.FC<{ isOpen: boolean; onClose: () => void }> = ({ isOpen, onClose }) => (
  <Drawer isOpen={isOpen} onClose={onClose} title="Keyboard and mouse">
    <dl className="space-y-3">
      {ROWS.map(([k, v]) => (
        <div key={k} className="flex items-center justify-between gap-4 text-[13px]">
          <dd className="text-ink">{v}</dd>
          <dt><kbd className="num text-[11px] text-strong border border-line rounded-[6px] px-2 py-1 bg-white/[0.04]">{k}</kbd></dt>
        </div>
      ))}
    </dl>
  </Drawer>
);
