import React from 'react';
import { DeterminismLevel, OriginBadge, StatusBadge } from '../../engineering/stateStore';

export interface OriginStatusBadgeProps {
  origin?: OriginBadge;
  status?: StatusBadge;
  isFailingRequirement?: boolean;
  className?: string;
}

export const OriginStatusBadges: React.FC<OriginStatusBadgeProps> = ({
  origin = 'Simulated',
  status = 'Verified',
  isFailingRequirement = false,
  className = ''
}) => {
  // HONESTY RULE: Never show a green Verified badge on a failing requirement.
  const resolvedStatus: StatusBadge = isFailingRequirement ? 'Unverified' : status;

  return (
    <div className={`inline-flex items-center gap-1.5 ${className}`}>
      <span className="ui-badge-origin" title={`Data Origin: ${origin}`}>
        {origin}
      </span>
      <span
        className={
          resolvedStatus === 'Verified'
            ? 'ui-badge-status-verified'
            : 'ui-badge-status-unverified'
        }
        title={isFailingRequirement ? 'Requirement Failing: Cannot be Verified' : `Status: ${resolvedStatus}`}
      >
        {resolvedStatus}
      </span>
    </div>
  );
};

export const DeterminismChip: React.FC<{
  level?: DeterminismLevel;
  className?: string;
}> = ({ level = 'Bitwise', className = '' }) => {
  return (
    <div className={`ui-badge-determinism ${className}`} title={`Determinism Level: ${level}`}>
      <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse"></span>
      <span className="tracking-wide">{level} Determinism</span>
    </div>
  );
};

export const PreviewTag: React.FC<{ label?: string }> = ({ label = 'PREVIEW' }) => (
  <span className="ui-badge-preview" title="Feature not finalized in engine crate yet">
    {label}
  </span>
);

export const MockTag: React.FC<{ label?: string }> = ({ label = 'MOCK' }) => (
  <span className="ui-badge-mock" title="Demonstration data layer (src/mock)">
    {label}
  </span>
);

export interface MetricValueProps {
  label: string;
  value: string | number;
  unit: string; // Every number shows a unit
  origin?: OriginBadge;
  status?: StatusBadge;
  delta?: string;
  deltaType?: 'positive' | 'negative' | 'neutral';
  isFailingRequirement?: boolean;
  isMock?: boolean;
}

export const MetricValue: React.FC<MetricValueProps> = ({
  label,
  value,
  unit,
  origin = 'Simulated',
  status = 'Verified',
  delta,
  deltaType = 'neutral',
  isFailingRequirement = false,
  isMock = false
}) => {
  return (
    <div className="flex flex-col gap-1 p-2.5 rounded-lg bg-slate-900/40 border border-slate-800/80 hover:border-blue-500/30 transition-colors">
      <div className="flex items-center justify-between gap-2 text-xs text-slate-400">
        <span className="font-medium truncate">{label}</span>
        <div className="flex items-center gap-1">
          {isMock && <MockTag />}
          <OriginStatusBadges origin={origin} status={status} isFailingRequirement={isFailingRequirement} />
        </div>
      </div>
      <div className="flex items-baseline justify-between mt-0.5">
        <div className="flex items-baseline gap-1">
          <span className="text-lg font-semibold text-white num">{value}</span>
          <span className="text-xs font-medium text-cyan-400/90">{unit}</span>
        </div>
        {delta && (
          <span
            className={`text-xs font-mono font-medium ${
              deltaType === 'positive'
                ? 'text-emerald-400'
                : deltaType === 'negative'
                ? 'text-rose-400'
                : 'text-slate-400'
            }`}
          >
            {delta}
          </span>
        )}
      </div>
    </div>
  );
};
