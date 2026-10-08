/**
 * Orthographic projection + hit-testing for the Reality view.
 * --------------------------------------------------------------------------
 * This module performs NO physics and mutates NO state. It maps authoritative
 * engineering values (positions in meters) to screen pixels, and back. Visual
 * transforms never write into engineering state — selection is the only
 * interaction that leaves this component, and it changes UI context only.
 */

export interface ViewCamera {
  yaw: number;   // radians around vertical axis
  pitch: number; // radians
  zoom: number;  // pixels per meter
  panX: number;  // px
  panY: number;  // px
}

export const defaultCamera = (): ViewCamera => ({ yaw: 0.6, pitch: 0.5, zoom: 26, panX: 0, panY: 0 });

export interface ProjectedPoint { x: number; y: number; depth: number }

/** Project an engineering-space point (x,y,z in meters; y up) to screen px. */
export function project(p: readonly [number, number, number], cam: ViewCamera, w: number, h: number): ProjectedPoint {
  const cy = Math.cos(cam.yaw), sy = Math.sin(cam.yaw);
  const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
  // rotate around Y then X
  const x1 = p[0] * cy - p[2] * sy;
  const z1 = p[0] * sy + p[2] * cy;
  const y2 = p[1] * cp - z1 * sp;
  const z2 = p[1] * sp + z1 * cp;
  return { x: w / 2 + cam.panX + x1 * cam.zoom, y: h / 2 + cam.panY - y2 * cam.zoom, depth: z2 };
}

export function hitTest(
  mx: number, my: number,
  objects: ReadonlyArray<{ id: string; center: ProjectedPoint; radiusPx: number }>,
): string | null {
  let best: string | null = null;
  let bestDepth = Number.POSITIVE_INFINITY;
  for (const o of objects) {
    const dx = mx - o.center.x;
    const dy = my - o.center.y;
    if (dx * dx + dy * dy <= o.radiusPx * o.radiusPx && o.center.depth < bestDepth) {
      best = o.id;
      bestDepth = o.center.depth;
    }
  }
  return best;
}
