import React, { useEffect, useMemo, useRef, useState } from 'react';

export interface Series {
  id: string;
  label: string;
  points: [number, number][];
  color: string;
  dashed?: boolean;
  /** 0..1, for the previous result shown dimmed under a Stale chip */
  opacity?: number;
}

export interface LineChartProps {
  series: Series[];
  xDomain: [number, number];
  yDomain?: [number, number];
  /** Horizontal reference line, e.g. a requirement limit */
  threshold?: { value: number; label: string };
  /** Vertical cursor at this x; the readout lists each series' value there */
  cursor?: number;
  xUnit: string;
  yUnit: string;
  onScrub?: (x: number) => void;
  height?: number;
  className?: string;
}

const PAD = { l: 38, r: 10, t: 10, b: 22 };

function niceDomain(series: Series[], given?: [number, number]): [number, number] {
  if (given) return given;
  const ys = series.flatMap(s => s.points.map(p => p[1]));
  const lo = Math.min(...ys);
  const hi = Math.max(...ys);
  const span = hi - lo || 1;
  return [lo - span * 0.08, hi + span * 0.08];
}

/** Valueat x by linear interpolation; points are sorted by x. */
export function valueAt(points: [number, number][], x: number): number | null {
  if (points.length === 0) return null;
  if (x <= points[0][0]) return points[0][1];
  for (let i = 1; i < points.length; i++) {
    if (x <= points[i][0]) {
      const [x0, y0] = points[i - 1];
      const [x1, y1] = points[i];
      return x1 === x0 ? y1 : y0 + ((y1 - y0) * (x - x0)) / (x1 - x0);
    }
  }
  return points[points.length - 1][1];
}

export const LineChart: React.FC<LineChartProps> = ({ series, xDomain, yDomain, threshold, cursor, xUnit, yUnit, onScrub, height = 150, className = '' }) => {
  // Drawn at the real pixel width so text stays the same size whatever the card width is.
  const box = useRef<HTMLDivElement>(null);
  const [W, setW] = useState(480);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setW(Math.max(200, Math.round(e.contentRect.width))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  const H = height;
  const [x0, x1] = xDomain;
  const [y0, y1] = useMemo(() => niceDomain(threshold ? [...series, { id: 't', label: '', color: '', points: [[x0, threshold.value]] }] : series, yDomain), [series, yDomain, threshold, x0]);
  const sx = (x: number) => PAD.l + ((x - x0) / (x1 - x0 || 1)) * (W - PAD.l - PAD.r);
  const sy = (y: number) => H - PAD.b - ((y - y0) / (y1 - y0 || 1)) * (H - PAD.t - PAD.b);
  const path = (pts: [number, number][]) => pts.map((p, i) => `${i ? 'L' : 'M'}${sx(p[0]).toFixed(1)},${sy(p[1]).toFixed(1)}`).join(' ');

  const yTicks = [0, 0.5, 1].map(f => y0 + (y1 - y0) * f);
  const xTicks = [0, 0.25, 0.5, 0.75, 1].map(f => x0 + (x1 - x0) * f);

  const scrub = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!onScrub || (e.type === 'pointermove' && e.buttons !== 1)) return;
    const r = e.currentTarget.getBoundingClientRect();
    const px = ((e.clientX - r.left) / r.width) * W;
    onScrub(Math.max(x0, Math.min(x1, x0 + ((px - PAD.l) / (W - PAD.l - PAD.r)) * (x1 - x0))));
  };

  return (
    <div ref={box} className="w-full">
    <svg
      viewBox={`0 0 ${W} ${H}`}
      width={W}
      height={H}
      className={`block select-none ${onScrub ? 'cursor-col-resize' : ''} ${className}`}
      onPointerDown={scrub}
      onPointerMove={scrub}
      role="img"
      aria-label={series.map(s => s.label).join(', ') + ` (${yUnit} over ${xUnit})`}
    >
      {yTicks.map((t, i) => (
        <g key={i}>
          <line x1={PAD.l} x2={W - PAD.r} y1={sy(t)} y2={sy(t)} stroke="var(--border-line)" />
          <text x={PAD.l - 6} y={sy(t) + 3} textAnchor="end" fontSize="9" fill="var(--text-muted)" className="num">{t.toFixed(Math.abs(y1 - y0) < 10 ? 1 : 0)}</text>
        </g>
      ))}
      {xTicks.map((t, i) => (
        <text key={i} x={sx(t)} y={H - 6} textAnchor="middle" fontSize="9" fill="var(--text-muted)" className="num">{t.toFixed(t % 1 ? 1 : 0)}</text>
      ))}
      <text x={W - PAD.r} y={H - 6} textAnchor="end" fontSize="9" fill="var(--text-muted)">{xUnit}</text>

      {threshold && (
        <g>
          <line x1={PAD.l} x2={W - PAD.r} y1={sy(threshold.value)} y2={sy(threshold.value)} stroke="var(--status-critical)" strokeDasharray="4 3" strokeOpacity="0.7" />
          <text x={W - PAD.r - 2} y={sy(threshold.value) - 4} textAnchor="end" fontSize="9" fill="var(--status-critical)">{threshold.label}</text>
        </g>
      )}

      {series.map(s => (
        <path key={s.id} d={path(s.points)} fill="none" stroke={s.color} strokeWidth="1.6" strokeDasharray={s.dashed ? '4 3' : undefined} strokeOpacity={s.opacity ?? 1} strokeLinejoin="round" />
      ))}

      {cursor !== undefined && cursor >= x0 && cursor <= x1 && (
        <g>
          <line x1={sx(cursor)} x2={sx(cursor)} y1={PAD.t} y2={H - PAD.b} stroke="var(--text-authoritative)" strokeOpacity="0.5" />
          {series.map(s => {
            const v = valueAt(s.points, cursor);
            return v === null ? null : <circle key={s.id} cx={sx(cursor)} cy={sy(v)} r="3" fill={s.color} stroke="var(--bg-panel)" strokeWidth="1.5" />;
          })}
        </g>
      )}
    </svg>
    </div>
  );
};

/** Compact trend line for telemetry tiles. */
export const Sparkline: React.FC<{ points: [number, number][]; color: string; cursor?: number; className?: string }> = ({ points, color, cursor, className = '' }) => {
  if (points.length < 2) return null;
  const xs = points.map(p => p[0]);
  const ys = points.map(p => p[1]);
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  const [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  const W = 100;
  const H = 24;
  const sx = (x: number) => ((x - x0) / (x1 - x0 || 1)) * W;
  const sy = (y: number) => H - 2 - ((y - y0) / (y1 - y0 || 1)) * (H - 4);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className={`w-full h-6 ${className}`} aria-hidden>
      <path d={points.map((p, i) => `${i ? 'L' : 'M'}${sx(p[0]).toFixed(1)},${sy(p[1]).toFixed(1)}`).join(' ')} fill="none" stroke={color} strokeWidth="1.3" vectorEffect="non-scaling-stroke" />
      {cursor !== undefined && <line x1={sx(cursor)} x2={sx(cursor)} y1="0" y2={H} stroke="var(--text-authoritative)" strokeOpacity="0.5" vectorEffect="non-scaling-stroke" />}
    </svg>
  );
};
