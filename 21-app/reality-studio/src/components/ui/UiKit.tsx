import React from 'react';
import { Check, Inbox, TriangleAlert, X } from 'lucide-react';
import { Value as ValueContract } from '../../contracts';
import { engineMode } from '../../engine';

// The honest-labelling tags (file 37 rules 2 and 10) and the shared primitives. Calm by design:
// solid surfaces, hairline borders, one accent. Tags are the only small-caps text in the product.

const tagBase =
  'inline-flex items-center h-[18px] px-1.5 rounded-[5px] text-[10px] font-medium tracking-[0.04em] uppercase border whitespace-nowrap';

export const MockTag: React.FC<{ label?: string }> = ({ label = 'Mock' }) => engineMode === 'real' ? null : (
  <span className={`${tagBase} text-bad border-bad/30 bg-bad/10`} title="Demonstration data from src/mock, not an engine result">{label}</span>
);

export const PreviewTag: React.FC<{ label?: string }> = ({ label = 'Preview' }) => (
  <span className={`${tagBase} text-warn border-warn/30 bg-warn/10`} title="Not available in the engine yet">{label}</span>
);

export const PlaceholderTag: React.FC<{ label?: string }> = ({ label = 'Placeholder model' }) => (
  <span className={`${tagBase} text-assistant border-assistant/30 bg-assistant/10`}>{label}</span>
);

export const ExampleTag: React.FC<{ label?: string }> = ({ label = 'Example' }) => (
  <span className={`${tagBase} text-trace border-trace/30 bg-trace/10`}>{label}</span>
);

/** The Example tag, only while the mock engine supplies the content. */
export const ExampleIfMock: React.FC = () => (engineMode === 'mock' ? <ExampleTag /> : null);

// --- Buttons ---------------------------------------------------------------

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'primary' | 'secondary' | 'ghost' | 'assistant' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  icon?: React.ReactNode;
};

const buttonVariants: Record<NonNullable<ButtonProps['variant']>, string> = {
  primary: 'bg-accent text-white hover:brightness-110 shadow-[0_1px_0_rgba(255,255,255,0.18)_inset,0_6px_18px_-6px_rgba(47,123,255,0.7)]',
  secondary: 'bg-raised text-strong border border-line hover:border-line-strong',
  ghost: 'text-muted hover:text-strong hover:bg-white/5',
  assistant: 'bg-assistant/15 text-assistant border border-assistant/30 hover:bg-assistant/25',
  danger: 'bg-bad/10 text-bad border border-bad/30 hover:bg-bad/20'
};
const buttonSizes = { sm: 'h-7 px-2.5 text-xs gap-1.5', md: 'h-8 px-3.5 text-[13px] gap-2', lg: 'h-10 px-5 text-sm gap-2' };

export const Button: React.FC<ButtonProps> = ({ variant = 'secondary', size = 'md', icon, className = '', children, ...rest }) => (
  <button
    {...rest}
    className={`inline-flex items-center justify-center whitespace-nowrap rounded-[8px] font-medium transition-[background,border-color,filter,color] duration-150 disabled:opacity-40 disabled:pointer-events-none select-none ${buttonSizes[size]} ${buttonVariants[variant]} ${className}`}
  >
    {icon}
    {children}
  </button>
);

export const IconButton: React.FC<React.ButtonHTMLAttributes<HTMLButtonElement> & { label: string; active?: boolean }> = ({
  label,
  active = false,
  className = '',
  children,
  ...rest
}) => (
  <button
    {...rest}
    aria-label={label}
    title={label}
    className={`h-8 w-8 inline-flex items-center justify-center rounded-[8px] transition-colors ${
      active ? 'bg-accent/15 text-strong' : 'text-muted hover:text-strong hover:bg-white/5'
    } ${className}`}
  >
    {children}
  </button>
);

// --- Text ------------------------------------------------------------------

/** Section label: sentence case, muted. No all-caps eyebrow labels (file 37 section 3). */
export const SectionLabel: React.FC<{ children: React.ReactNode; aside?: React.ReactNode; className?: string; /** heading level; 2 directly under a page title, 3 (default) inside panels */ level?: 2 | 3 }> = ({ children, aside, className = '', level = 3 }) => {
  const Tag = level === 2 ? 'h2' : 'h3';
  return (
  <div className={`flex items-center justify-between gap-2 ${className}`}>
    <Tag className="text-xs font-medium text-muted">{children}</Tag>
    {aside}
  </div>
  );
};

export const Chip: React.FC<{ children: React.ReactNode; tone?: 'neutral' | 'ok' | 'warn' | 'bad' | 'accent'; className?: string }> = ({ children, tone = 'neutral', className = '' }) => {
  const tones = {
    neutral: 'text-ink border-line bg-white/[0.03]',
    ok: 'text-ok border-ok/30 bg-ok/10',
    warn: 'text-warn border-warn/30 bg-warn/10',
    bad: 'text-bad border-bad/30 bg-bad/10',
    accent: 'text-accent border-accent/30 bg-accent/10'
  };
  return <span className={`inline-flex items-center gap-1.5 h-6 px-2 rounded-full border text-[11px] whitespace-nowrap ${tones[tone]} ${className}`}>{children}</span>;
};

export const StatusDot: React.FC<{ tone: 'ok' | 'warn' | 'bad' | 'muted' | 'accent' }> = ({ tone }) => {
  const c = { ok: 'bg-ok', warn: 'bg-warn', bad: 'bg-bad', muted: 'bg-muted', accent: 'bg-accent' }[tone];
  return <span className={`inline-block w-1.5 h-1.5 rounded-full ${c}`} aria-hidden />;
};

/** Brand lockup: hex-arrow mark, HELQIX, "Engineering studio" (file 33 section 3). */
export const Logo: React.FC<{ size?: 'sm' | 'md'; withSub?: boolean; markOnly?: boolean }> = ({ size = 'md', withSub = true, markOnly = false }) => (
  <span className="inline-flex items-center gap-2.5">
    <svg width={size === 'sm' ? 22 : 26} height={size === 'sm' ? 22 : 26} viewBox="0 0 28 28" fill="none" aria-hidden>
      <path d="M14 2.5 24 8.25v11.5L14 25.5 4 19.75V8.25L14 2.5Z" stroke="url(#lg)" strokeWidth="1.6" strokeLinejoin="round" />
      <path d="M10 17.5 14 9l4 8.5" stroke="#F3F7FC" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
      <defs>
        <linearGradient id="lg" x1="4" y1="2" x2="24" y2="26" gradientUnits="userSpaceOnUse">
          <stop stopColor="#22D3EE" />
          <stop offset="1" stopColor="#2F7BFF" />
        </linearGradient>
      </defs>
    </svg>
    {!markOnly && <span className="flex flex-col leading-none">
      <span className="font-display font-light tracking-[0.28em] text-strong text-[15px]">HELQIX</span>
      {withSub && <span className="text-[10px] text-muted mt-1 tracking-[0.04em]">Engineering studio</span>}
    </span>}
  </span>
);

// --- Values (rule 6: every engineering number is {value, unit, origin, status}) ---

export interface ValueViewProps {
  label?: string;
  value: ValueContract;
  className?: string;
  isFailingRequirement?: boolean;
}

export const ValueView: React.FC<ValueViewProps> = ({ label, value, className = '', isFailingRequirement = false }) => {
  // A failing requirement can never render verified.
  const status = isFailingRequirement ? 'unverified' : value.status;
  return (
    <div className={`flex flex-col gap-1.5 p-3 rounded-[10px] bg-white/[0.025] border border-line ${className}`}>
      <div className="flex items-center justify-between gap-2 text-[11px] text-muted">
        {label && <span className="truncate">{label}</span>}
        <span className="flex items-center gap-1 ml-auto shrink-0">
          {value.mock && <MockTag />}
          <span className="capitalize">{value.origin.replace('_', ' ')}</span>
          <span aria-hidden>·</span>
          <span className={status === 'verified' ? 'text-ok' : ''}>{status === 'verified' ? 'Verified' : 'Unverified'}</span>
        </span>
      </div>
      <div className="flex items-baseline gap-1.5">
        <span className="num text-[18px] font-medium text-strong">{value.v}</span>
        <span className="num text-xs text-muted">{value.unit}</span>
      </div>
    </div>
  );
};

/** Compact reading for HUDs and tiles: value, unit, and the Origin and Status words. */
export const Reading: React.FC<{ label: string; value: number | string; unit: string; origin?: string; status?: string; tone?: 'default' | 'bad' | 'warn'; mock?: boolean }> = ({
  label,
  value,
  unit,
  origin = 'Simulated',
  status = 'Unverified',
  tone = 'default',
  mock
}) => (
  <div className="min-w-0">
    <div className="flex items-center gap-1.5 text-[11px] text-muted">
      <span className="truncate">{label}</span>
      {mock && <MockTag />}
    </div>
    <div className="flex items-baseline gap-1 mt-0.5">
      <span className={`num text-[19px] font-medium ${tone === 'bad' ? 'text-bad' : tone === 'warn' ? 'text-warn' : 'text-strong'}`}>{value}</span>
      <span className="num text-xs text-muted">{unit}</span>
    </div>
    <div className="text-[10px] text-muted/80 mt-0.5">{origin} · {status}</div>
  </div>
);

// --- Surfaces --------------------------------------------------------------

export interface PanelProps {
  elevation?: 1 | 2 | 3;
  overlay?: boolean;
  className?: string;
  children: React.ReactNode;
}

export const Panel: React.FC<PanelProps> = ({ elevation = 1, overlay = false, className = '', children }) => (
  <div className={`${overlay ? 'surface-glass' : elevation >= 3 ? 'surface-raised shadow-[var(--shadow-2)]' : 'surface'} ${className}`}>{children}</div>
);

export interface TabsProps {
  tabs: { id: string; label: string; badge?: string }[];
  activeTab: string;
  onChange: (id: string) => void;
  className?: string;
}

/** Underline-free segmented control: quiet, one selected state. */
export const Tabs: React.FC<TabsProps> = ({ tabs, activeTab, onChange, className = '' }) => (
  <div role="tablist" className={`inline-flex items-center gap-0.5 p-0.5 rounded-[9px] bg-white/[0.04] border border-line ${className}`}>
    {tabs.map(tab => {
      const on = activeTab === tab.id;
      return (
        <button
          key={tab.id}
          role="tab"
          aria-selected={on}
          onClick={() => onChange(tab.id)}
          className={`h-7 px-3 rounded-[7px] text-xs font-medium transition-colors flex items-center gap-1.5 ${on ? 'bg-raised text-strong shadow-[var(--shadow-1)]' : 'text-muted hover:text-ink'}`}
        >
          {tab.label}
          {tab.badge && <span className="num text-[10px] text-muted">{tab.badge}</span>}
        </button>
      );
    })}
  </div>
);

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

export const SliderWithUnit: React.FC<SliderWithUnitProps> = ({ label, value, unit, min, max, step = 1, onChange, className = '' }) => (
  <label className={`flex flex-col gap-2 ${className}`}>
    <span className="flex items-center justify-between text-xs">
      <span className="text-muted">{label}</span>
      <span className="num text-strong">
        {value} <span className="text-muted">{unit}</span>
      </span>
    </span>
    <input type="range" min={min} max={max} step={step} value={value} onChange={e => onChange(parseFloat(e.target.value))} />
  </label>
);

export interface DrawerProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  side?: 'right' | 'bottom';
  children: React.ReactNode;
}

export const Drawer: React.FC<DrawerProps> = ({ isOpen, onClose, title, side = 'right', children }) => {
  if (!isOpen) return null;
  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/55 animate-fade-in" onMouseDown={e => e.target === e.currentTarget && onClose()}>
      <div
        role="dialog"
        aria-label={title}
        className={`bg-panel border-line shadow-[var(--shadow-3)] flex flex-col ${side === 'right' ? 'w-full max-w-[460px] h-full border-l' : 'w-full h-80 self-end border-t'}`}
      >
        <div className="flex items-center justify-between px-5 h-14 border-b border-line shrink-0">
          <h2 className="text-sm font-medium text-strong">{title}</h2>
          <IconButton label="Close" onClick={onClose}><X size={16} /></IconButton>
        </div>
        <div className="flex-1 overflow-y-auto p-5">{children}</div>
      </div>
    </div>
  );
};

export const Skeleton: React.FC<{ className?: string }> = ({ className = 'h-6 w-full' }) => (
  <div className={`rounded-[8px] bg-gradient-to-r from-white/[0.03] via-white/[0.07] to-white/[0.03] bg-[length:200%_100%] animate-shimmer ${className}`} />
);

export const EmptyState: React.FC<{ title: string; description: string; actionLabel?: string; onAction?: () => void }> = ({ title, description, actionLabel, onAction }) => (
  <div className="flex flex-col items-center justify-center py-10 px-6 text-center rounded-[14px] border border-dashed border-line">
    <div className="w-9 h-9 rounded-full bg-white/[0.04] flex items-center justify-center text-muted mb-3"><Inbox size={16} /></div>
    <div className="text-sm font-medium text-strong mb-1">{title}</div>
    <div className="text-xs text-muted max-w-xs mb-4">{description}</div>
    {actionLabel && onAction && <Button variant="primary" onClick={onAction}>{actionLabel}</Button>}
  </div>
);

export const ErrorState: React.FC<{ error: string; reason?: string; onRetry?: () => void }> = ({ error, reason, onRetry }) => (
  <div role="alert" className="flex flex-col p-4 rounded-[12px] border border-bad/30 bg-bad/[0.06] text-xs gap-2">
    <div className="flex items-center gap-2 font-medium text-bad"><TriangleAlert size={14} />{error}</div>
    {reason && <div className="text-muted">{reason}</div>}
    {onRetry && <div><Button size="sm" variant="danger" onClick={onRetry}>Retry</Button></div>}
  </div>
);

/** Tiny confirmation tick used after an applied action ("Apply", then "Applied"). */
export const Done: React.FC<{ label: string }> = ({ label }) => (
  <span className="inline-flex items-center gap-1 text-xs text-ok"><Check size={13} />{label}</span>
);

// --- Determinism chip (was in HonestyBadges) ---------------------------------

export const DeterminismChip: React.FC<{ level?: string; className?: string }> = ({ level = 'Bitwise', className = '' }) => (
  <Chip tone="neutral" className={className}>
    <span className="w-1.5 h-1.5 rounded-full bg-trace" aria-hidden />
    {level} determinism
  </Chip>
);
