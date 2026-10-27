import React from 'react';
import { useAppState } from '../engineering/stateStore';
import { navigate } from './router';
import { engineMode } from '../engine';
import { MockTag } from '../components/ui/UiKit';

// Frame for Home (file 36 section 6.2). Stages live inside the Studio (/p/:id), not in this sidebar.
export const AppShell: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { toast } = useAppState();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#050B18] text-slate-100">
      {toast && (
        <div role="status" className="fixed bottom-6 right-6 z-50 px-4 py-3 rounded-xl bg-slate-900/95 border border-cyan-500/50 text-xs font-mono text-cyan-200">
          {toast.message}
        </div>
      )}

      <aside className="w-52 h-full flex flex-col justify-between border-r border-slate-800/80 bg-[#050B18]/90 shrink-0">
        <div>
          <button
            onClick={() => navigate('/')}
            className="h-14 w-full px-5 flex items-center border-b border-slate-800/60 hover:bg-blue-950/20 text-lg font-bold tracking-[0.25em] text-white"
            title="Back to the start page"
          >
            HEL<span className="text-cyan-400">Q</span>IX
          </button>
          <nav className="p-2 space-y-1">
            <button
              onClick={() => navigate('/home')}
              aria-current="page"
              className="w-full flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-semibold bg-blue-600/20 text-white border border-blue-500/40"
            >
              Home
            </button>
            <button
              onClick={() => navigate('/new')}
              className="w-full flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-900/60"
            >
              + New project
            </button>
          </nav>
        </div>
        <div className="p-3 border-t border-slate-800/60 text-[10px] font-mono text-slate-400 flex items-center gap-1.5">
          <span className={`w-1.5 h-1.5 rounded-full ${engineMode === 'mock' ? 'bg-amber-400' : 'bg-emerald-400'}`} />
          <span>Engine: {engineMode === 'mock' ? 'MOCK' : 'HELQIX API'}</span>
          {engineMode === 'mock' && <MockTag />}
        </div>
      </aside>

      <main className="flex-1 h-full overflow-auto">{children}</main>
    </div>
  );
};
