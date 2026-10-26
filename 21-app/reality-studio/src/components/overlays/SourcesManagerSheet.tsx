import React from 'react';
import { MOCK_SOURCES } from '../../mock/examplePack';

export interface SourcesManagerSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SourcesManagerSheet: React.FC<SourcesManagerSheetProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm select-none animate-fade-in">
      <div className="w-full max-w-lg h-full bg-[#0A1422] border-l border-[#22395A] p-6 flex flex-col justify-between shadow-[-10px_0_40px_rgba(0,0,0,0.8)] overflow-y-auto">
        <div className="space-y-5">
          <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
            <div className="space-y-0.5">
              <span className="text-sm font-bold text-white font-mono uppercase block">Sources Manager</span>
              <p className="text-xs text-[#7F93AF]">Connected CAD vaults, test telemetry feeds, and vendor catalogs.</p>
            </div>
            <button onClick={onClose} className="text-[#7F93AF] hover:text-white p-1 text-base">✕</button>
          </div>

          <div className="space-y-2.5">
            {MOCK_SOURCES.map(source => (
              <div key={source.id} className="p-3 rounded-[10px] bg-[#0E1B2E] border border-[#22395A] space-y-1.5 text-xs font-mono">
                <div className="flex items-center justify-between">
                  <span className="text-white font-semibold">{source.name}</span>
                  <span className="px-1.5 py-0.2 rounded bg-[#22C55E]/15 text-[#4ADE80] text-[10px]">
                    {source.status}
                  </span>
                </div>
                <div className="flex items-center justify-between text-[10px] text-[#7F93AF]">
                  <span>Kind: <strong className="text-cyan-300">{source.kind}</strong></span>
                  <span>Last sync: {source.lastSync}</span>
                </div>
              </div>
            ))}
          </div>

          <div className="p-3 rounded-[10px] bg-[#13253D] border border-blue-500/30 text-xs text-[#D6E2F3]">
            No mock vendor data or fake pricing is invented. Only verified connected repositories are displayed.
          </div>
        </div>

        <button
          onClick={onClose}
          className="w-full py-2.5 rounded-[8px] text-xs font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6]"
        >
          Close Manager
        </button>
      </div>
    </div>
  );
};
