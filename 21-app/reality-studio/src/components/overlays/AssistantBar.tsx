import React, { useState } from 'react';
import { PreviewTag } from '../ui/UiKit';

export interface AssistantBarProps {
  isOpen?: boolean;
  onClose?: () => void;
  onApply: () => void;
  onSimulate: () => void;
  onBranch: () => void;
}

export const AssistantBar: React.FC<AssistantBarProps> = ({
  isOpen = true,
  onClose,
  onApply,
  onSimulate,
  onBranch
}) => {
  const [isExpanded, setIsExpanded] = useState<boolean>(true);

  if (!isOpen) return null;

  return (
    <aside
      aria-label="Engineering Assistant Panel"
      className="fixed bottom-6 right-6 z-40 max-w-sm w-full pointer-events-auto select-none animate-fade-in shadow-2xl"
    >
      <div className="p-3.5 rounded-[12px] bg-[#0E1B2E]/95 backdrop-blur-md border border-[#8B5CF6]/50 shadow-[0_0_30px_rgba(139,92,246,0.25)] space-y-2.5">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
          <div className="flex items-center gap-1.5">
            <span className="text-[#8B5CF6] text-xs">✦</span>
            <span className="text-xs font-bold text-white font-mono">Rule-Based Assistant</span>
            <PreviewTag label="PREVIEW" />
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setIsExpanded(p => !p)}
              className="text-[#7F93AF] hover:text-white text-xs px-1.5 py-0.5 rounded hover:bg-white/5"
              title={isExpanded ? 'Collapse' : 'Expand'}
            >
              {isExpanded ? '▲' : '▼'}
            </button>
            {onClose && (
              <button
                onClick={onClose}
                className="text-[#7F93AF] hover:text-white text-xs px-1.5 py-0.5 rounded hover:bg-white/5"
                title="Close Assistant"
              >
                ✕
              </button>
            )}
          </div>
        </div>

        {isExpanded && (
          <div className="space-y-2 text-xs">
            {/* Intent & Proposal (Section 6.17) */}
            <div>
              <div className="text-[10px] font-mono text-[#7F93AF]">INTENT</div>
              <div className="text-white font-medium">Prevent Stator Thermal Failure Trip</div>
            </div>

            <div>
              <div className="text-[10px] font-mono text-[#7F93AF]">PROPOSAL</div>
              <div className="text-[#22D3EE] font-semibold">Enable Liquid Cooling Bypass (+20% Convection)</div>
            </div>

            {/* Reasoning & Expected Effect (Section 6.17) */}
            <div className="p-2 rounded bg-[#13253D] border border-[#22395A] space-y-1 font-mono text-[10px]">
              <div>
                <span className="text-[#7F93AF]">Reasoning: </span>
                <span className="text-slate-300">Joule heating exceeds heat sink convective boundary at t=8.42s</span>
              </div>
              <div>
                <span className="text-[#7F93AF]">Expected Effect: </span>
                <span className="text-[#4ADE80] font-bold">Peak temperature drops by 18.8 °C (remains below 120 °C)</span>
              </div>
            </div>

            {/* Action Buttons: Apply, Simulate, Create branch, Reject (Section 6.17) */}
            <div className="flex items-center justify-end gap-1.5 pt-1">
              <button
                onClick={onClose}
                className="px-2 py-1 rounded text-[10px] font-medium text-[#7F93AF] hover:text-white"
              >
                Dismiss
              </button>
              <button
                onClick={onApply}
                className="px-2.5 py-1 rounded text-[10px] font-semibold text-white bg-[#2F7BFF] hover:bg-[#3B82F6]"
              >
                Apply
              </button>
              <button
                onClick={onBranch}
                className="px-2.5 py-1 rounded text-[10px] font-semibold text-white bg-[#8B5CF6] hover:bg-[#7C3AED]"
              >
                Create Branch
              </button>
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};
