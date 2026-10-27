import { Binding } from '../contracts';
import { Series } from './types';

// Motion is data (file 37 rule 4): an object moves only because a stored variable is bound to it.
// These functions turn a binding and a time into a visual value. They are pure, so scrubbing to the
// same time always gives the same picture, forwards or backwards.

/** Rotation is drawn slower than the data (3000 rpm is unwatchable); the UI shows this ratio. */
export const VISUAL_SPEED_SCALE = 50;

export const SUPPORTED_CHANNELS: Binding['channel'][] = ['rotation', 'color', 'emissive', 'vibration', 'translation', 'visibility'];

export function valueAtTime(points: [number, number][] | undefined, t: number): number | null {
  if (!points || points.length === 0) return null;
  if (t <= points[0][0]) return points[0][1];
  for (let i = 1; i < points.length; i++) {
    if (t <= points[i][0]) {
      const [t0, v0] = points[i - 1];
      const [t1, v1] = points[i];
      return t1 === t0 ? v1 : v0 + ((v1 - v0) * (t - t0)) / (t1 - t0);
    }
  }
  return points[points.length - 1][1];
}

const cumulative = new WeakMap<[number, number][], number[]>();

/** Integral of the variable from the start to t (trapezoid rule over the stored samples). */
export function integralAtTime(points: [number, number][] | undefined, t: number): number {
  if (!points || points.length < 2) return 0;
  let cum = cumulative.get(points);
  if (!cum) {
    cum = [0];
    for (let i = 1; i < points.length; i++) cum.push(cum[i - 1] + ((points[i][1] + points[i - 1][1]) / 2) * (points[i][0] - points[i - 1][0]));
    cumulative.set(points, cum);
  }
  if (t <= points[0][0]) return 0;
  for (let i = 1; i < points.length; i++) {
    if (t <= points[i][0]) {
      const dt = t - points[i - 1][0];
      const v = valueAtTime(points, t) ?? 0;
      return cum[i - 1] + ((points[i - 1][1] + v) / 2) * dt;
    }
  }
  return cum[cum.length - 1];
}

/** Position of the value inside the binding's range, 0 to 1 (not clamped above 1, so overshoot shows). */
export function normalised(b: Binding, v: number): number {
  const [lo, hi] = b.range ?? [0, 1];
  return hi === lo ? 0 : Math.max(0, (v - lo) / (hi - lo));
}

export interface PartVisualState {
  /** radians about the bound axis, for rotation bindings */
  rotation: { axis: 'x' | 'y' | 'z'; angle: number } | null;
  /** 0..1+ along the cool to hot colour ramp */
  heat: number | null;
  /** 0..1+ glow */
  glow: number | null;
  /** 0..1+ shake amplitude */
  shake: number | null;
  /** 0..1 along the translation axis */
  /** amount: 0..1 along the range. metres: the real displacement from the first sample, when the unit is m. */
  slide: { axis: 'x' | 'y' | 'z'; amount: number; metres: number | null } | null;
  visible: boolean;
}

export function stateFor(partId: string, bindings: Binding[], series: Series, t: number): PartVisualState {
  const out: PartVisualState = { rotation: null, heat: null, glow: null, shake: null, slide: null, visible: true };
  for (const b of bindings) {
    if (b.node !== partId) continue;
    const pts = series[b.variable];
    const v = valueAtTime(pts, t);
    if (v === null) continue;
    switch (b.channel) {
      case 'rotation': out.rotation = { axis: b.axis ?? 'y', angle: integralAtTime(pts, t) / VISUAL_SPEED_SCALE }; break;
      case 'color': out.heat = normalised(b, v); break;
      case 'emissive': out.glow = normalised(b, v); break;
      case 'vibration': out.shake = normalised(b, v); break;
      case 'translation': out.slide = { axis: b.axis ?? 'x', amount: normalised(b, v), metres: b.units === 'm' && pts ? v - pts[0][1] : null }; break;
      case 'visibility': out.visible = normalised(b, v) > 0.5; break;
      default: break;
    }
  }
  return out;
}

export function unsupportedChannels(bindings: Binding[]): Binding['channel'][] {
  return Array.from(new Set(bindings.map(b => b.channel).filter(c => !SUPPORTED_CHANNELS.includes(c))));
}
