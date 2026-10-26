import React, { useState, useEffect } from 'react';

export interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectAction: (actionId: string) => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  isOpen,
  onClose,
  onSelectAction
}) => {
  const [query, setQuery] = useState('');

  // Keyboard shortcut listener for Esc
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const actions = [
    { id: 'create_project', label: 'Create new project', category: 'Project', icon: '✦', shortcut: 'Ctrl+N' },
    { id: 'run_simulation', label: 'Run simulation', category: 'Execution', icon: '▶', shortcut: 'Space' },
    { id: 'pause_simulation', label: 'Pause simulation', category: 'Execution', icon: '⏸', shortcut: 'Space' },
    { id: 'freeze_state', label: 'Freeze state checkpoint', category: 'Analysis', icon: '❄', shortcut: 'Ctrl+F' },
    { id: 'branch_intervention', label: 'Create intervention branch', category: 'Analysis', icon: '⑂', shortcut: 'Ctrl+B' },
    { id: 'compare_branches', label: 'Compare scenario branches', category: 'Comparison', icon: '⚖', shortcut: '' },
    { id: 'verify_evidence', label: 'Run credibility verification', category: 'Verification', icon: '✓', shortcut: '' },
    { id: 'open_evidence', label: 'Open evidence package sheet', category: 'Export', icon: '📄', shortcut: '' },
    { id: 'open_sources', label: 'Open connected sources manager', category: 'Data', icon: '🗄', shortcut: '' },
    { id: 'add_part', label: 'Define a part from scratch', category: 'Model', icon: '+', shortcut: 'A' }
  ];

  const filtered = actions.filter(
    a => a.label.toLowerCase().includes(query.toLowerCase()) || a.category.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/60 backdrop-blur-sm select-none p-4 animate-fade-in">
      <div className="w-full max-w-xl rounded-[14px] bg-[#0A1422] border border-[#2F7BFF]/50 shadow-[0_0_50px_rgba(47,123,255,0.35)] overflow-hidden flex flex-col">
        {/* Search Input Bar */}
        <div className="p-3.5 border-b border-[#22395A] flex items-center gap-3">
          <span className="text-cyan-400 text-sm font-mono">⌘</span>
          <input
            type="text"
            autoFocus
            placeholder="Type a command or search action (e.g. Run, Branch, Evidence)..."
            value={query}
            onChange={e => setQuery(e.target.value)}
            className="flex-1 bg-transparent text-sm text-white focus:outline-none placeholder:text-slate-500 font-mono"
          />
          <span className="text-[10px] font-mono text-[#7F93AF] bg-[#13253D] px-1.5 py-0.5 rounded border border-[#22395A]">
            ESC to close
          </span>
        </div>

        {/* Action List */}
        <div className="max-h-80 overflow-y-auto p-2 space-y-1">
          {filtered.length === 0 ? (
            <div className="p-6 text-center text-xs text-slate-500 italic">No matching commands found.</div>
          ) : (
            filtered.map((action, idx) => (
              <div
                key={action.id}
                onClick={() => {
                  onSelectAction(action.id);
                  onClose();
                }}
                className="p-2.5 rounded-[8px] flex items-center justify-between text-xs hover:bg-[#2F7BFF]/20 hover:border-[#2F7BFF]/40 border border-transparent cursor-pointer transition-colors"
              >
                <div className="flex items-center gap-2.5">
                  <span className="w-5 h-5 rounded bg-[#13253D] flex items-center justify-center text-cyan-300 font-mono text-xs">
                    {action.icon}
                  </span>
                  <div className="space-y-0.5">
                    <span className="text-white font-medium">{action.label}</span>
                    <span className="text-[10px] font-mono text-[#7F93AF] block">{action.category}</span>
                  </div>
                </div>

                {action.shortcut && (
                  <span className="text-[10px] font-mono text-slate-400 bg-[#13253D] px-1.5 py-0.5 rounded border border-[#22395A]">
                    {action.shortcut}
                  </span>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};
