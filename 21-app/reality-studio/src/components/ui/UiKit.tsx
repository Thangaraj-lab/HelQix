import React from 'react';
import { Value as ValueContract } from '../../contracts';

// --- TAGS (Rule 2 & 10: MockTag, PreviewTag, PlaceholderTag, ExampleTag) ---

export const MockTag: React.FC<{ label?: string }> = ({ label = 'MOCK' }) => (
  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-red-500/15 border border-red-500/40 text-red-400">
    {label}
  </span>
);

export const PreviewTag: React.FC<{ label?: string }> = ({ label = 'PREVIEW' }) => (
  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-amber-500/15 border border-amber-500/40 text-amber-300">
    {label}
  </span>
);

export const PlaceholderTag: React.FC<{ label?: string }> = ({ label = 'PLACEHOLDER MODEL' }) => (
  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-purple-500/15 border border-purple-500/40 text-purple-300">
    {label}
  </span>
);

export const ExampleTag: React.FC<{ label?: string }> = ({ label = 'Example' }) => (
  <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-blue-500/15 border border-blue-500/35 text-cyan-300">
    {label}
  </span>
);

// --- VALUE COMPONENT (Rule 6: Every engineering number is {value, unit, origin, status}) ---

export interface ValueViewProps {
  label?: string;
  value: ValueContract;
  className?: string;
  isFailingRequirement?: boolean;
}

export const ValueView: React.FC<ValueViewProps> = ({
  label,
  value,
  className = '',
  isFailingRequirement = false
}) => {
  // Rule 2 & 6: Failing requirement can never render verified status
  const resolvedStatus = isFailingRequirement ? 'unverified' : value.status;

  return (
    <div className={`flex flex-col gap-1 p-2.5 rounded-[10px] bg-[#0A1422]/70 border border-[#22395A] ${className}`}>
      <div className="flex items-center justify-between text-[11px] text-[#7F93AF]">
        {label && <span className="font-medium truncate">{label}</span>}
        <div className="flex items-center gap-1.5 ml-auto">
          {value.mock && <MockTag />}
          <span className="px-1.5 py-0.2 rounded text-[9px] font-mono uppercase bg-[#8B5CF6]/15 border border-[#8B5CF6]/30 text-[#C4B5FD]">
            {value.origin}
          </span>
          <span
            className={`px-1.5 py-0.2 rounded text-[9px] font-mono uppercase border ${
              resolvedStatus === 'verified'
                ? 'bg-[#22C55E]/15 border-[#22C55E]/40 text-[#4ADE80]'
                : 'bg-slate-700/20 border-slate-600/40 text-slate-400'
            }`}
          >
            {resolvedStatus}
          </span>
        </div>
      </div>
      <div className="flex items-baseline gap-1 mt-0.5">
        <span className="text-base font-bold text-[#F2F6FB] font-mono tabular-nums num">{value.v}</span>
        <span className="text-xs font-medium text-[#22D3EE] font-mono">{value.unit}</span>
      </div>
    </div>
  );
};

// --- PANEL (Section 3: Radii 14, 1px border, 3 elevations, blur on overlays, solid elsewhere) ---

export interface PanelProps {
  elevation?: 1 | 2 | 3;
  overlay?: boolean;
  className?: string;
  children: React.ReactNode;
}

export const Panel: React.FC<PanelProps> = ({
  elevation = 1,
  overlay = false,
  className = '',
  children
}) => {
  const bgClass = overlay
    ? 'bg-[#0E1B2E]/80 backdrop-blur-md'
    : elevation === 3
    ? 'bg-[#13253D]'
    : elevation === 2
    ? 'bg-[#0E1B2E]'
    : 'bg-[#0A1422]';

  const shadowClass =
    elevation === 3
      ? 'shadow-[0_16px_48px_rgba(0,0,0,0.7)]'
      : elevation === 2
      ? 'shadow-[0_8px_32px_rgba(0,0,0,0.5)]'
      : 'shadow-[0_4px_16px_rgba(0,0,0,0.3)]';

  return (
    <div className={`rounded-[14px] border border-[#22395A] ${bgClass} ${shadowClass} ${className}`}>
      {children}
    </div>
  );
};

// --- TABS (Section 3) ---

export interface TabsProps {
  tabs: { id: string; label: string; badge?: string }[];
  activeTab: string;
  onChange: (id: string) => void;
  className?: string;
}

export const Tabs: React.FC<TabsProps> = ({ tabs, activeTab, onChange, className = '' }) => {
  return (
    <div className={`flex items-center gap-1 p-1 rounded-[10px] bg-[#0A1422] border border-[#22395A] ${className}`}>
      {tabs.map(tab => {
        const isActive = activeTab === tab.id;
        return (
          <button
            key={tab.id}
            onClick={() => onChange(tab.id)}
            className={`px-3 py-1.5 rounded-[8px] text-xs font-medium transition-all flex items-center gap-1.5 ${
              isActive
                ? 'bg-[#2F7BFF]/25 border border-[#2F7BFF]/60 text-[#22D3EE] font-semibold shadow-[0_0_12px_rgba(47,123,255,0.3)]'
                : 'text-[#7F93AF] hover:text-[#D6E2F3] hover:bg-white/5 border border-transparent'
            }`}
          >
            <span>{tab.label}</span>
            {tab.badge && (
              <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-blue-900/50 text-cyan-300">
                {tab.badge}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};

// --- SLIDER WITH UNIT (Section 3 & 6) ---

export interface SliderWithUnitProps {
  label: string;
  value: number;
  unit: string;
  min: number;
  max: number;
  step?: number;
  onChange: (val: number) => void;
  className?: string;
}

export const SliderWithUnit: React.FC<SliderWithUnitProps> = ({
  label,
  value,
  unit,
  min,
  max,
  step = 1,
  onChange,
  className = ''
}) => {
  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      <div className="flex items-center justify-between text-xs">
        <span className="text-[#7F93AF]">{label}</span>
        <div className="flex items-baseline gap-1 font-mono text-[#F2F6FB] font-semibold tabular-nums">
          <span>{value}</span>
          <span className="text-[#22D3EE] text-[11px] font-normal">{unit}</span>
        </div>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={e => onChange(parseFloat(e.target.value))}
        className="w-full accent-[#22D3EE]"
      />
    </div>
  );
};

// --- DRAWER (Overlay Sheet from Right or Bottom) ---

export interface DrawerProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  side?: 'right' | 'bottom';
  children: React.ReactNode;
}

export const Drawer: React.FC<DrawerProps> = ({
  isOpen,
  onClose,
  title,
  side = 'right',
  children
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm animate-fade-in">
      <div
        className={`bg-[#0E1B2E] border-[#22395A] p-6 shadow-2xl flex flex-col justify-between overflow-y-auto ${
          side === 'right'
            ? 'w-full max-w-xl h-full border-l'
            : 'w-full h-80 bottom-0 border-t'
        }`}
      >
        <div className="flex items-center justify-between border-b border-[#22395A] pb-4 mb-4">
          <h2 className="text-base font-bold text-[#F2F6FB]">{title}</h2>
          <button
            onClick={onClose}
            className="text-[#7F93AF] hover:text-white p-1 rounded-[8px] hover:bg-white/5"
          >
            ✕
          </button>
        </div>
        <div className="flex-1 overflow-y-auto">{children}</div>
      </div>
    </div>
  );
};

// --- SKELETON (Technical wireframe skeleton, never a spinner - Section 7) ---

export const Skeleton: React.FC<{ className?: string }> = ({ className = 'h-6 w-full' }) => (
  <div className={`rounded-[8px] bg-gradient-to-r from-[#0E1B2E] via-[#13253D] to-[#0E1B2E] bg-[length:200%_100%] animate-shimmer border border-[#22395A]/50 ${className}`} />
);

// --- EMPTY STATE (Section 7) ---

export const EmptyState: React.FC<{
  title: string;
  description: string;
  actionLabel?: string;
  onAction?: () => void;
}> = ({ title, description, actionLabel, onAction }) => (
  <div className="flex flex-col items-center justify-center p-8 text-center rounded-[14px] border border-dashed border-[#22395A] bg-[#0A1422]/50">
    <div className="w-10 h-10 rounded-full bg-[#13253D] flex items-center justify-center text-[#7F93AF] mb-3">
      ⌱
    </div>
    <div className="text-sm font-bold text-[#F2F6FB] mb-1">{title}</div>
    <div className="text-xs text-[#7F93AF] max-w-xs mb-4">{description}</div>
    {actionLabel && onAction && (
      <button
        onClick={onAction}
        className="px-4 py-2 rounded-[8px] text-xs font-medium text-white bg-[#2F7BFF] hover:bg-[#3B82F6] transition-colors"
      >
        {actionLabel}
      </button>
    )}
  </div>
);

// --- ERROR STATE (Section 7) ---

export const ErrorState: React.FC<{
  error: string;
  reason?: string;
  onRetry?: () => void;
}> = ({ error, reason, onRetry }) => (
  <div className="flex flex-col p-4 rounded-[14px] border border-red-500/40 bg-red-950/20 text-xs text-red-200 gap-2">
    <div className="flex items-center gap-2 font-bold text-red-400">
      <span>✕</span>
      <span>{error}</span>
    </div>
    {reason && <div className="text-[#7F93AF]">{reason}</div>}
    {onRetry && (
      <div className="flex items-center gap-2 pt-1">
        <button
          onClick={onRetry}
          className="px-3 py-1 rounded-[6px] bg-red-500/20 border border-red-500/40 text-red-300 hover:bg-red-500/30 font-medium"
        >
          Retry
        </button>
      </div>
    )}
  </div>
);
