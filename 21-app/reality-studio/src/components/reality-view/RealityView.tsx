/**
 * Reality view surface.
 * --------------------------------------------------------------------------
 * Renders a representation DERIVED from authoritative engineering state
 * (positions read from Layer-04 state values via props). It never modifies
 * engineering state; camera transforms are pure presentation. When the Layer-14
 * Visual Runtime (Rust/WASM) is wired into this bundle, it will mount into the
 * same container through `runtimeMountId` — this canvas renderer is the
 * fallback surface so the studio remains usable pre-WASM integration.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import type { AuthoritativeState, ModelComponent } from "../../api/types";
import { defaultCamera, hitTest, project, type ViewCamera } from "./projection";
import "./reality.css";

interface Props {
  components: ReadonlyArray<ModelComponent>;
  liveState: AuthoritativeState | null;
  selectedId: string | null;
  onSelect(id: string): void;
}

/** Extract [x,y,z] meters for a component from authoritative state values. */
function positionOf(state: AuthoritativeState | null, componentId: string, index: number): readonly [number, number, number] {
  if (state) {
    const vals = state.values.filter((v) => v.componentId === componentId);
    const get = (name: string) => vals.find((v) => v.property.toLowerCase() === name)?.quantity.value;
    const x = get("position.x") ?? get("x");
    const y = get("position.y") ?? get("y");
    const z = get("position.z") ?? get("z");
    if (x !== undefined || y !== undefined || z !== undefined) {
      return [x ?? 0, y ?? 0, z ?? 0];
    }
  }
  // No positional data in state → deterministic layout slot (presentation only).
  const cols = 5;
  return [(index % cols) * 2 - 4, 0, Math.floor(index / cols) * 2 - 2];
}

export function RealityView({ components, liveState, selectedId, onSelect }: Props) {
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [camera, setCamera] = useState<ViewCamera>(defaultCamera);
  const dragRef = useRef<{ mode: "orbit" | "pan"; lastX: number; lastY: number } | null>(null);
  const objectsRef = useRef<ReadonlyArray<{ id: string; center: ReturnType<typeof project>; radiusPx: number }>>([]);

  const positions = useMemo(
    () => components.map((c, i) => ({ component: c, pos: positionOf(liveState, c.id, i) })),
    [components, liveState],
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) return;
    const dpr = window.devicePixelRatio ?? 1;
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // ground grid (presentation)
    ctx.strokeStyle = "rgba(42,51,64,0.55)";
    ctx.lineWidth = 1;
    for (let g = -6; g <= 6; g += 1) {
      const a = project([g, 0, -6], camera, w, h);
      const b = project([g, 0, 6], camera, w, h);
      const c2 = project([-6, 0, g], camera, w, h);
      const d2 = project([6, 0, g], camera, w, h);
      ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
      ctx.beginPath(); ctx.moveTo(c2.x, c2.y); ctx.lineTo(d2.x, d2.y); ctx.stroke();
    }

    const objects: { id: string; center: ReturnType<typeof project>; radiusPx: number }[] = [];
    const sorted = [...positions]
      .map((p) => ({ ...p, projected: project(p.pos, camera, w, h) }))
      .sort((a, b) => b.projected.depth - a.projected.depth);

    for (const item of sorted) {
      const r = Math.max(8, camera.zoom * 0.45);
      objects.push({ id: item.component.id, center: item.projected, radiusPx: r });
      const selected = item.component.id === selectedId;
      const grad = ctx.createRadialGradient(item.projected.x - r / 3, item.projected.y - r / 3, r / 6, item.projected.x, item.projected.y, r);
      grad.addColorStop(0, selected ? "#4fd3c2" : "#2f3b49");
      grad.addColorStop(1, selected ? "#1e5f57" : "#171f29");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(item.projected.x, item.projected.y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = selected ? "rgba(79,211,194,.9)" : "rgba(58,70,86,.9)";
      ctx.lineWidth = selected ? 2 : 1;
      ctx.stroke();
      ctx.fillStyle = selected ? "#d7dde6" : "#8b96a5";
      ctx.font = "11px Inter, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText(item.component.name, item.projected.x, item.projected.y + r + 13);
    }
    objectsRef.current = objects;
  }, [positions, camera, selectedId]);

  return (
    <div className="reality-view" ref={wrapRef}>
      <div className="reality-view__controls" role="toolbar" aria-label="View controls">
        <button className="hq-button hq-button--sm" onClick={() => setCamera((c) => ({ ...c, pitch: Math.min(1.4, c.pitch + 0.15) }))} title="Orbit up">Orbit ↑</button>
        <button className="hq-button hq-button--sm" onClick={() => setCamera((c) => ({ ...c, pitch: Math.max(-1.4, c.pitch - 0.15) }))} title="Orbit down">Orbit ↓</button>
        <button className="hq-button hq-button--sm" onClick={() => setCamera((c) => ({ ...c, panY: c.panY + 30 }))} title="Pan">Pan</button>
        <button className="hq-button hq-button--sm" onClick={() => setCamera((c) => ({ ...c, zoom: Math.min(120, c.zoom * 1.25) }))} title="Zoom in">＋</button>
        <button className="hq-button hq-button--sm" onClick={() => setCamera((c) => ({ ...c, zoom: Math.max(6, c.zoom / 1.25) }))} title="Zoom out">－</button>
        <button className="hq-button hq-button--sm" onClick={() => setCamera(defaultCamera())} title="Reset view">Fit ⟲</button>
      </div>
      <canvas
        ref={canvasRef}
        role="application"
        aria-label="3D reality view driven by authoritative engineering state"
        tabIndex={0}
        onMouseDown={(e) => { dragRef.current = { mode: e.shiftKey ? "pan" : "orbit", lastX: e.clientX, lastY: e.clientY }; }}
        onMouseMove={(e) => {
          const drag = dragRef.current;
          if (!drag) return;
          const dx = e.clientX - drag.lastX;
          const dy = e.clientY - drag.lastY;
          drag.lastX = e.clientX; drag.lastY = e.clientY;
          setCamera((c) =>
            drag.mode === "orbit"
              ? { ...c, yaw: c.yaw + dx * 0.01, pitch: Math.max(-1.4, Math.min(1.4, c.pitch + dy * 0.008)) }
              : { ...c, panX: c.panX + dx, panY: c.panY + dy },
          );
        }}
        onMouseUp={() => { dragRef.current = null; }}
        onMouseLeave={() => { dragRef.current = null; }}
        onClick={(e) => {
          const rect = (e.target as HTMLCanvasElement).getBoundingClientRect();
          const id = hitTest(e.clientX - rect.left, e.clientY - rect.top, objectsRef.current);
          if (id) onSelect(id);
        }}
        onWheel={(e) => setCamera((c) => ({ ...c, zoom: Math.max(6, Math.min(120, c.zoom * (e.deltaY < 0 ? 1.1 : 0.91))) }))}
      />
      <div className="reality-view__legend">
        Rendering derived from engineering state · drag to orbit · shift+drag to pan · click object to select component
      </div>
    </div>
  );
}
