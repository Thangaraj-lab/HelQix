import React, { useEffect, useMemo, useState } from 'react';
import { CornerDownLeft, Search } from 'lucide-react';

export interface PaletteCommand {
  id: string;
  label: string;
  group: string;
  hint?: string;
  disabledReason?: string;
}

export interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  commands: PaletteCommand[];
  onSelect: (id: string) => void;
}

// Ctrl or Cmd + K (file 37 section 6.17). Lists only commands that do something; a command whose
// prerequisite is missing is shown dimmed with the reason, never hidden and never faked.
export const CommandPalette: React.FC<CommandPaletteProps> = ({ isOpen, onClose, commands, onSelect }) => {
  const [query, setQuery] = useState('');
  const [index, setIndex] = useState(0);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return commands.filter(c => !q || c.label.toLowerCase().includes(q) || c.group.toLowerCase().includes(q));
  }, [commands, query]);

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setIndex(0);
    }
  }, [isOpen]);
  useEffect(() => setIndex(0), [query]);

  if (!isOpen) return null;

  const run = (c: PaletteCommand | undefined) => {
    if (!c || c.disabledReason) return;
    onSelect(c.id);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[14vh] bg-black/55 animate-fade-in p-4" onMouseDown={e => e.target === e.currentTarget && onClose()}>
      <div role="dialog" aria-label="Command palette" className="w-full max-w-[560px] surface-glass !bg-panel overflow-hidden animate-rise">
        <div className="h-12 px-4 flex items-center gap-3 border-b border-line">
          <Search size={15} className="text-muted" />
          <input
            autoFocus
            value={query}
            onChange={e => setQuery(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'ArrowDown') { e.preventDefault(); setIndex(i => Math.min(i + 1, filtered.length - 1)); }
              else if (e.key === 'ArrowUp') { e.preventDefault(); setIndex(i => Math.max(i - 1, 0)); }
              else if (e.key === 'Enter') run(filtered[index]);
              else if (e.key === 'Escape') onClose();
            }}
            placeholder="Type a command"
            aria-label="Command"
            className="flex-1 bg-transparent text-sm placeholder:text-muted focus:outline-none"
          />
          <kbd className="text-[10px] text-muted border border-line rounded px-1.5 h-5 inline-flex items-center">Esc</kbd>
        </div>
        <ul className="max-h-[340px] overflow-y-auto p-1.5" role="listbox">
          {filtered.length === 0 && <li className="px-3 py-6 text-center text-xs text-muted">No matching command.</li>}
          {filtered.map((c, i) => (
            <li key={c.id} role="option" aria-selected={i === index} aria-disabled={!!c.disabledReason}>
              <button
                onClick={() => run(c)}
                onMouseEnter={() => setIndex(i)}
                className={`w-full text-left h-10 px-3 rounded-[8px] flex items-center justify-between gap-3 text-[13px] ${i === index ? 'bg-white/[0.06]' : ''} ${c.disabledReason ? 'opacity-50' : ''}`}
              >
                <span className="flex items-baseline gap-2 min-w-0">
                  <span className="text-strong truncate">{c.label}</span>
                  <span className="text-[11px] text-muted">{c.disabledReason ?? c.group}</span>
                </span>
                {i === index && !c.disabledReason ? <CornerDownLeft size={13} className="text-muted shrink-0" /> : c.hint && <kbd className="text-[10px] text-muted border border-line rounded px-1.5 h-5 inline-flex items-center">{c.hint}</kbd>}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
};
