import React from 'react';
import { useStudio } from '../../stores';

export interface SettingsSheetProps {
  isOpen: boolean;
  onClose: () => void;
  onToggleTheme: () => void;
  currentTheme: 'dark' | 'light';
}

export const SettingsSheet: React.FC<SettingsSheetProps> = ({
  isOpen,
  onClose,
  onToggleTheme,
  currentTheme
}) => {
  const { session } = useStudio();

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm select-none animate-fade-in">
      <div className="w-full max-w-md h-full bg-[#0A1422] border-l border-[#22395A] p-6 flex flex-col justify-between shadow-[-10px_0_40px_rgba(0,0,0,0.8)] overflow-y-auto">
        <div className="space-y-6">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-[#22395A] pb-4">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-blue-600 to-cyan-400 p-[1px]">
                <div className="w-full h-full rounded-full bg-[#050B14] flex items-center justify-center text-xs font-bold text-white">
                  HQ
                </div>
              </div>
              <div>
                <span className="text-sm font-bold text-white font-mono block">Engineer Settings</span>
                <span className="text-[10px] text-[#7F93AF]">Workspace Preferences</span>
              </div>
            </div>
            <button onClick={onClose} className="text-[#7F93AF] hover:text-white p-1 text-base">✕</button>
          </div>

          {/* Theme Option */}
          <div className="space-y-2">
            <label className="text-xs font-mono uppercase text-[#7F93AF] font-bold">Studio Theme</label>
            <div className="grid grid-cols-2 gap-2 text-xs font-mono">
              <button
                onClick={() => { if (currentTheme !== 'dark') onToggleTheme(); }}
                className={`p-2.5 rounded-[8px] border text-center transition-all ${
                  currentTheme === 'dark'
                    ? 'bg-[#2F7BFF]/25 border-[#2F7BFF] text-white font-bold'
                    : 'bg-[#0E1B2E] border-[#22395A] text-[#7F93AF]'
                }`}
              >
                Dark (Ultra-navy)
              </button>
              <button
                onClick={() => { if (currentTheme !== 'light') onToggleTheme(); }}
                className={`p-2.5 rounded-[8px] border text-center transition-all ${
                  currentTheme === 'light'
                    ? 'bg-[#2F7BFF]/25 border-[#2F7BFF] text-white font-bold'
                    : 'bg-[#0E1B2E] border-[#22395A] text-[#7F93AF]'
                }`}
              >
                Light
              </button>
            </div>
          </div>

          {/* Quality Tier Option */}
          <div className="space-y-2">
            <label className="text-xs font-mono uppercase text-[#7F93AF] font-bold">Rendering Quality Tier</label>
            <div className="grid grid-cols-3 gap-2 text-xs font-mono">
              {(['Auto', 'High', 'Low'] as const).map(tier => (
                <button
                  key={tier}
                  onClick={() => session.setQualityTier(tier)}
                  className={`p-2 rounded-[8px] border text-center transition-all ${
                    session.qualityTier === tier
                      ? 'bg-[#2F7BFF]/25 border-[#2F7BFF] text-white font-bold'
                      : 'bg-[#0E1B2E] border-[#22395A] text-[#7F93AF]'
                  }`}
                >
                  {tier}
                </button>
              ))}
            </div>
          </div>

          {/* Engine Determinism Mode */}
          <div className="p-3.5 rounded-[12px] bg-[#0E1B2E] border border-[#22395A] space-y-1.5 text-xs font-mono">
            <div className="flex items-center justify-between text-white font-semibold">
              <span>Solver Engine</span>
              <span className="text-[#4ADE80]">Bitwise Deterministic</span>
            </div>
            <div className="text-[11px] text-[#7F93AF]">
              Ensures frame hashes match exactly across re-executions and audits.
            </div>
          </div>
        </div>

        <div className="pt-4 border-t border-[#22395A]">
          <button
            onClick={onClose}
            className="w-full py-2 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6]"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
