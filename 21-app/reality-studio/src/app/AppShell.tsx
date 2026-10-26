import React, { useState } from 'react';
import { useAppState, ScreenId, QualityTier, DeterminismLevel } from '../engineering/stateStore';
import { DeterminismChip } from '../components/ui/HonestyBadges';

export const AppShell: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const {
    currentScreen,
    setCurrentScreen,
    qualityTier,
    setQualityTier,
    determinism,
    setDeterminism,
    toast
  } = useAppState();

  const [searchQuery, setSearchQuery] = useState('');
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [showScreenDrawer, setShowScreenDrawer] = useState(false);

  // Sidebar primary navigation items
  const sidebarItems: { id: ScreenId; label: string; icon: React.ReactNode; preview?: boolean }[] = [
    {
      id: 'home',
      label: 'Overview',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
      )
    },
    {
      id: 'model_editor',
      label: 'Model',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
          <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
          <line x1="12" y1="22.08" x2="12" y2="12" />
        </svg>
      )
    },
    {
      id: 'workspace',
      label: 'Simulate',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="10" />
          <polygon points="10 8 16 12 10 16 10 8" />
        </svg>
      )
    },
    {
      id: 'failure_causality',
      label: 'Understand',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <line x1="18" y1="20" x2="18" y2="10" />
          <line x1="12" y1="20" x2="12" y2="4" />
          <line x1="6" y1="20" x2="6" y2="14" />
        </svg>
      )
    },
    {
      id: 'compare',
      label: 'Improve',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
          <polyline points="17 6 23 6 23 12" />
        </svg>
      )
    },
    {
      id: 'build_optimization',
      label: 'Build',
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
        </svg>
      )
    },
    {
      id: 'compute_resources',
      label: 'Compute',
      preview: true,
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <rect x="4" y="4" width="16" height="16" rx="2" />
          <rect x="9" y="9" width="6" height="6" />
          <line x1="9" y1="1" x2="9" y2="4" />
          <line x1="15" y1="1" x2="15" y2="4" />
          <line x1="9" y1="20" x2="9" y2="23" />
          <line x1="15" y1="20" x2="15" y2="23" />
        </svg>
      )
    },
    {
      id: 'verify_validation',
      label: 'Verify',
      preview: true,
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
          <polyline points="9 12 11 14 15 10" />
        </svg>
      )
    },
    {
      id: 'edge_monitoring',
      label: 'Edge',
      preview: true,
      icon: (
        <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M5 12.55a11 11 0 0 1 14.08 0" />
          <path d="M1.42 9a16 16 0 0 1 21.16 0" />
          <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
          <line x1="12" y1="20" x2="12.01" y2="20" />
        </svg>
      )
    }
  ];

  // Full 16-screens inventory for quick navigation & review
  const allScreens: { id: ScreenId; num: string; title: string; category: string }[] = [
    { id: 'landing', num: '01', title: 'Landing / Entry Portal', category: 'Entry' },
    { id: 'home', num: '02', title: 'Home / Overview & Trust Summary', category: 'Overview' },
    { id: 'workspace', num: '03', title: 'Engineering Workspace / Reality Canvas', category: 'Workspace' },
    { id: 'model_editor', num: '04', title: 'Model Editor & Component Library', category: 'Model' },
    { id: 'simulation_setup', num: '05', title: 'Simulation Setup & Solvers', category: 'Simulate' },
    { id: 'simulation_run', num: '06', title: 'Simulation Run (Live 3D & Hotspot)', category: 'Simulate' },
    { id: 'simulation_results', num: '07', title: 'Simulation Results & Field Slices', category: 'Simulate' },
    { id: 'failure_causality', num: '08', title: 'Failure Investigation / Causality', category: 'Understand' },
    { id: 'compare', num: '09', title: 'Scenario Comparison & Deltas', category: 'Improve' },
    { id: 'scenario_builder', num: '10', title: 'Scenario Builder & Node Graph', category: 'Simulate' },
    { id: 'timeline_replay', num: '11', title: 'Timeline & Deterministic Replay', category: 'Verify' },
    { id: 'build_optimization', num: '12', title: 'Build / Automated Optimization', category: 'Build' },
    { id: 'compute_resources', num: '13', title: 'Compute & Hybrid Cluster Nodes', category: 'Compute' },
    { id: 'verify_validation', num: '14', title: 'Verify & ASME Validation Report', category: 'Verify' },
    { id: 'deploy_export', num: '15', title: 'Deploy / Export & Evidence Package', category: 'Deploy' },
    { id: 'edge_monitoring', num: '16', title: 'Edge & Real-time Telemetry Ingest', category: 'Edge' }
  ];

  const getBreadcrumb = () => {
    const current = allScreens.find(s => s.id === currentScreen);
    return `Project > Workspace > ${current?.title || 'Studio'}`;
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#050B18] text-slate-100 select-none">
      {/* Toast Alert */}
      {toast && (
        <div className="fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-xl bg-slate-900/95 border border-cyan-500/50 shadow-[0_0_25px_rgba(34,211,238,0.35)] backdrop-blur-md animate-fade-in">
          <span className="w-2 h-2 rounded-full bg-cyan-400 animate-ping"></span>
          <span className="text-xs font-mono text-cyan-200">{toast.message}</span>
        </div>
      )}

      {/* Screen Selection Modal / Drawer */}
      {showScreenDrawer && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-6">
          <div className="glass-panel-elevated w-full max-w-4xl p-6 flex flex-col gap-5 border border-blue-500/40 shadow-[0_0_50px_rgba(47,123,255,0.35)]">
            <div className="flex items-center justify-between border-b border-slate-800 pb-3">
              <div className="flex items-center gap-3">
                <span className="text-cyan-400 font-mono text-sm font-semibold tracking-wider">HELQIX // 16 SCREEN MATRIX</span>
                <span className="px-2 py-0.5 rounded text-[10px] font-semibold bg-blue-500/20 text-blue-300 border border-blue-500/30">
                  ALL INTERACTIVE
                </span>
              </div>
              <button
                onClick={() => setShowScreenDrawer(false)}
                className="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800"
              >
                <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 max-h-[60vh] overflow-y-auto pr-1">
              {allScreens.map(scr => (
                <button
                  key={scr.id}
                  onClick={() => {
                    setCurrentScreen(scr.id);
                    setShowScreenDrawer(false);
                  }}
                  className={`p-3 rounded-xl text-left border transition-all flex flex-col justify-between h-24 ${
                    currentScreen === scr.id
                      ? 'bg-blue-600/30 border-cyan-400 shadow-[0_0_15px_rgba(34,211,238,0.3)]'
                      : 'bg-slate-900/60 border-slate-800 hover:border-blue-500/50 hover:bg-slate-800/60'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-xs font-bold text-cyan-400">{scr.num}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800/80 text-slate-400 border border-slate-700">
                      {scr.category}
                    </span>
                  </div>
                  <div className="text-xs font-medium text-slate-200 line-clamp-2">{scr.title}</div>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* LEFT SIDEBAR (matches reference Image 2, 3, 4) */}
      <aside className="w-52 h-full flex flex-col justify-between border-r border-slate-800/80 bg-[#050B18]/90 backdrop-blur-xl relative z-30 shrink-0">
        <div>
          {/* HELQIX Brand Logo with Glowing Q */}
          <div
            onClick={() => setCurrentScreen('landing')}
            className="h-14 px-5 flex items-center gap-2 cursor-pointer border-b border-slate-800/60 hover:bg-blue-950/20 transition-colors"
          >
            <div className="text-lg font-bold tracking-[0.25em] text-white flex items-center">
              <span>HEL</span>
              <span className="text-cyan-400 relative">
                Q
                <span className="absolute inset-0 text-cyan-400 blur-[6px] opacity-75">Q</span>
              </span>
              <span>IX</span>
            </div>
          </div>

          {/* Navigation Items */}
          <nav className="p-2 space-y-1">
            {sidebarItems.map(item => {
              const isActive = currentScreen === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setCurrentScreen(item.id)}
                  className={`w-full relative flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
                    isActive
                      ? 'bg-blue-600/20 text-white font-semibold shadow-[0_0_15px_rgba(47,123,255,0.25)] border border-blue-500/40'
                      : 'text-slate-400 hover:text-slate-100 hover:bg-slate-900/60'
                  }`}
                >
                  {isActive && <div className="active-glow-bar" />}
                  <span className={isActive ? 'text-cyan-400' : 'text-slate-400'}>{item.icon}</span>
                  <span className="truncate">{item.label}</span>
                  {item.preview && (
                    <span className="ml-auto text-[9px] px-1 py-0.2 rounded font-mono text-amber-300 bg-amber-500/15 border border-amber-500/30">
                      PREVIEW
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Bottom Section: Screen Matrix Launcher & Glowing Planet-Arc Card */}
        <div className="p-3 space-y-2.5 border-t border-slate-800/60">
          <button
            onClick={() => setShowScreenDrawer(true)}
            className="w-full py-1.5 px-2 rounded-lg text-[11px] font-mono font-medium text-cyan-300 bg-blue-950/40 border border-blue-500/30 hover:border-cyan-400 hover:bg-blue-900/40 transition-all flex items-center justify-between"
          >
            <span className="flex items-center gap-1.5">
              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="3" width="7" height="7" />
                <rect x="14" y="3" width="7" height="7" />
                <rect x="14" y="14" width="7" height="7" />
                <rect x="3" y="14" width="7" height="7" />
              </svg>
              <span>16-Screen View</span>
            </span>
            <span className="px-1.5 rounded bg-blue-500/30 text-[10px]">16</span>
          </button>

          {/* Glowing Planet-Arc Glass Card (exact match to Image 2, 3, 4) */}
          <div className="relative p-3 rounded-xl bg-gradient-to-b from-blue-950/60 to-slate-950/80 border border-blue-500/30 shadow-[0_0_20px_rgba(47,123,255,0.2)] overflow-hidden group">
            <div className="absolute -right-3 -top-3 w-16 h-16 rounded-full border border-cyan-400/50 shadow-[0_0_20px_#22D3EE] opacity-60 pointer-events-none group-hover:scale-110 transition-transform" />
            <div className="relative z-10 flex flex-col gap-1">
              <div className="text-[10px] tracking-wider font-bold text-cyan-400 uppercase">HELQIX</div>
              <div className="text-[10px] font-medium text-slate-300 leading-tight">
                ENGINEERING INTELLIGENCE
              </div>
              <div className="mt-1 flex items-center gap-1.5 text-[9px] text-slate-400 font-mono">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                <span>v1.0.4 • Bitwise</span>
              </div>
            </div>
          </div>
        </div>
      </aside>

      {/* MAIN VIEWPORT */}
      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        {/* TOP BAR (matches reference Image 2, 3, 4) */}
        <header className="h-14 px-6 flex items-center justify-between border-b border-slate-800/80 bg-[#050B18]/70 backdrop-blur-md shrink-0 z-20">
          {/* Breadcrumb Navigation */}
          <div className="flex items-center gap-2 text-xs text-slate-400">
            <button
              onClick={() => setCurrentScreen('home')}
              className="hover:text-cyan-400 transition-colors flex items-center gap-1.5"
            >
              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
              </svg>
            </button>
            <span>/</span>
            <span className="text-slate-300 font-medium truncate max-w-md">{getBreadcrumb()}</span>
          </div>

          {/* Right Controls: Search, Quality Tier, Determinism Chip, Notifications, Avatar */}
          <div className="flex items-center gap-3">
            {/* Quick Search */}
            <div className="relative">
              <button
                onClick={() => setIsSearchOpen(!isSearchOpen)}
                className="ui-btn-icon"
                title="Search parameters, components, runs"
              >
                <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <circle cx="11" cy="11" r="8" />
                  <line x1="21" y1="21" x2="16.65" y2="16.65" />
                </svg>
              </button>
              {isSearchOpen && (
                <div className="absolute right-0 top-10 w-72 p-2 rounded-xl glass-panel-elevated z-50">
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={e => setSearchQuery(e.target.value)}
                    placeholder="Search model, telemetry, causality..."
                    className="w-full px-3 py-1.5 rounded-lg bg-slate-900/90 border border-slate-700 text-xs text-white focus:border-cyan-400 outline-none"
                    autoFocus
                  />
                </div>
              )}
            </div>

            {/* Quality Tier Selector */}
            <div className="flex items-center gap-1 p-0.5 rounded-lg bg-slate-900/80 border border-slate-800 text-[11px] font-mono">
              {(['Auto', 'High', 'Low'] as QualityTier[]).map(tier => (
                <button
                  key={tier}
                  onClick={() => setQualityTier(tier)}
                  className={`px-2 py-0.5 rounded transition-colors ${
                    qualityTier === tier
                      ? 'bg-blue-600/40 text-cyan-300 font-semibold border border-blue-500/40'
                      : 'text-slate-400 hover:text-slate-200'
                  }`}
                  title={`Graphics Quality Tier: ${tier}`}
                >
                  {tier}
                </button>
              ))}
            </div>

            {/* Determinism Level Selector */}
            <DeterminismChip level={determinism} />

            {/* Theme Toggle (Engineering Dark baseline) */}
            <button className="ui-btn-icon" title="Theme toggled (Engineering Dark optimal)">
              <svg className="w-4 h-4 text-amber-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
            </button>

            {/* Notification Bell with Dot */}
            <button className="ui-btn-icon relative" title="System Notifications">
              <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" />
                <path d="M13.73 21a2 2 0 0 1-3.46 0" />
              </svg>
              <span className="absolute top-1.5 right-1.5 w-2 h-2 rounded-full bg-cyan-400 animate-pulse shadow-[0_0_8px_#22D3EE]"></span>
            </button>

            {/* User Avatar Ring */}
            <div className="w-8 h-8 rounded-full border border-cyan-400/60 p-0.5 shadow-[0_0_10px_rgba(34,211,238,0.3)]">
              <div className="w-full h-full rounded-full bg-gradient-to-tr from-blue-600 to-cyan-400 flex items-center justify-center text-[11px] font-bold text-white">
                HQ
              </div>
            </div>
          </div>
        </header>

        {/* CONTENT AREA */}
        <main className="flex-1 overflow-auto relative">
          {children}
        </main>
      </div>
    </div>
  );
};
