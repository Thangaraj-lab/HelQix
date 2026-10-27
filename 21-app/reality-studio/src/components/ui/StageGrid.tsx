import React from 'react';

/**
 * Layout shared by every Studio stage: a left panel, a right panel, and a centre that stays open so
 * the persistent canvas shows through. Panels float with an even gutter (file 33 section 4).
 * `top` and `bottom` sit over the canvas; everything is click-through except the panels themselves.
 */
export const StageGrid: React.FC<{
  left?: React.ReactNode;
  right?: React.ReactNode;
  top?: React.ReactNode;
  bottom?: React.ReactNode;
  /** Full-width row under all columns, for wide content such as graphs */
  footer?: React.ReactNode;
  leftWidth?: number;
  rightWidth?: number;
}> = ({ left, right, top, bottom, footer, leftWidth = 288, rightWidth = 300 }) => (
  <div
    className="relative z-10 flex-1 min-h-0 grid gap-3 p-3 pointer-events-none"
    style={{ gridTemplateRows: footer ? 'minmax(0,1fr) auto' : 'minmax(0,1fr)', gridTemplateColumns: `${left ? `minmax(220px,${leftWidth}px)` : ''} minmax(0,1fr) ${right ? `minmax(220px,${rightWidth}px)` : ''}`.trim() }}
  >
    {left && <div className="flex flex-col gap-3 min-h-0 overflow-y-auto pointer-events-auto">{left}</div>}
    <div className="flex flex-col justify-between gap-3 min-h-0 min-w-0">
      <div className="pointer-events-auto flex justify-center">{top}</div>
      <div className="pointer-events-auto">{bottom}</div>
    </div>
    {right && <div className="flex flex-col gap-3 min-h-0 overflow-y-auto pointer-events-auto">{right}</div>}
    {footer && <div className="pointer-events-auto" style={{ gridColumn: '1 / -1' }}>{footer}</div>}
  </div>
);

/** A titled panel for StageGrid columns. */
export const PanelCard: React.FC<{ title?: string; aside?: React.ReactNode; className?: string; children: React.ReactNode }> = ({ title, aside, className = '', children }) => (
  <section className={`surface p-4 space-y-3 ${className}`} aria-label={title}>
    {title && (
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-xs font-medium text-muted">{title}</h3>
        {aside}
      </div>
    )}
    {children}
  </section>
);
