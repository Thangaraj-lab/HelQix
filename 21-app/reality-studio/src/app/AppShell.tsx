import React from 'react';
import { FolderOpen, Plus } from 'lucide-react';
import { navigate } from './router';
import { engineMode } from '../engine';
import { Logo, MockTag, StatusDot } from '../components/ui/UiKit';

// Frame for Home (file 37 section 6.2). Stages live inside the Studio (/p/:id), not in this sidebar.
export const AppShell: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-void text-ink">
      <aside className="w-[200px] h-full flex flex-col justify-between border-r border-line bg-deck shrink-0">
        <div>
          <button onClick={() => navigate('/')} className="h-16 w-full px-5 flex items-center border-b border-line text-left" aria-label="HELQIX start page">
            <Logo size="sm" />
          </button>
          <nav className="p-3 space-y-1" aria-label="Home">
            <button aria-current="page" className="w-full h-9 flex items-center gap-2.5 px-3 rounded-[8px] text-[13px] font-medium bg-white/[0.06] text-strong">
              <FolderOpen size={15} className="text-accent" /> Projects
            </button>
            <button onClick={() => navigate('/new')} className="w-full h-9 flex items-center gap-2.5 px-3 rounded-[8px] text-[13px] text-muted hover:text-strong hover:bg-white/5">
              <Plus size={15} /> New project
            </button>
          </nav>
        </div>
        <div className="p-4 border-t border-line text-[11px] text-muted flex items-center gap-2">
          <StatusDot tone={engineMode === 'mock' ? 'warn' : 'ok'} />
          <span>Engine: {engineMode === 'mock' ? 'Mock' : 'HELQIX API'}</span>
          {engineMode === 'mock' && <MockTag />}
        </div>
      </aside>

      <main className="flex-1 h-full overflow-auto">{children}</main>
    </div>
  );
};
