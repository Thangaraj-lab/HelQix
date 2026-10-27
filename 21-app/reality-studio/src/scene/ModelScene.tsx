import React, { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { RGBELoader } from 'three/examples/jsm/loaders/RGBELoader.js';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { Binding } from '../contracts';
import { getAsset } from '../app/assets';
import { CameraPreset, Look, SceneConnection, ScenePart, Series } from './types';
import { PartVisualState, stateFor } from './bindings';

// The Reality Canvas for a real model: the project's own parts, ports and connections, driven only by
// stored data. Geometry is schematic (millimetre dimensions drawn at 1 unit per 100 mm, positions in
// metres), so the scene says so. Nothing moves unless a binding ties a stored variable to a part.

export interface ModelSceneProps {
  parts: ScenePart[];
  connections: SceneConnection[];
  bindings: Binding[];
  series: Series;
  t: number;
  /** A second copy beside the first, driven by other series (Improve shows original and branch) */
  compare?: { series: Series; label: string } | null;
  primaryLabel?: string;
  /** The part that breached a requirement, and when */
  failure?: { partId: string; t: number; text: string } | null;
  selectedId?: string | null;
  highlightIds?: string[];
  focusId?: string | null;
  onSelect?: (id: string | null) => void;
  look: Look;
  cameraPreset: CameraPreset;
  qualityTier: 'Auto' | 'High' | 'Low';
  /** Draft models are drawn as translucent ghosts until committed */
  draft?: boolean;
  labels?: 'auto' | 'always';
  /** Pixels covered by side panels, so the model is centred in the free middle */
  insets?: { left: number; right: number };
  /** Changes when the user asks to reset the view (key F) */
  fitSignal?: number;
  /** A user-supplied HDRI (asset id in the browser's asset store); null uses the built-in studio light */
  environmentAssetId?: string | null;
  className?: string;
}

const KIND_COLOR: Record<string, number> = {
  electrical_dc: 0x2f7bff,
  mechanical_torque: 0x22d3ee,
  telemetry_signal: 0x8b5cf6,
  fluid_port: 0x38bdf8,
  thermal: 0xf59e0b
};
const METALS = [0x93a4bd, 0x8196b3, 0xa3afc0, 0x7389a8, 0x9aa9bf];
const ACCENT = 0x2f7bff;
const TRACE = 0x22d3ee;

const HEAT_STOPS = [new THREE.Color(0xffffff), new THREE.Color(0xf59e0b), new THREE.Color(0xef4444)];
function heatColor(f: number, base: THREE.Color, out: THREE.Color): THREE.Color {
  if (f <= 0) return out.copy(base);
  if (f < 0.5) return out.copy(base).lerp(HEAT_STOPS[1], f / 0.5);
  return out.copy(HEAT_STOPS[1]).lerp(HEAT_STOPS[2], Math.min(1, (f - 0.5) / 0.5));
}

const hash01 = (n: number) => { const x = Math.sin(n * 127.1 + 311.7) * 43758.5453; return x - Math.floor(x); };

function dimsOf(p: ScenePart): [number, number, number] {
  const g = p.geometry;
  if (g.kind !== 'primitive') return [1, 1, 1];
  const d = (i: number) => (g.dims[i]?.v ?? 100) / 100;
  if (g.shape === 'cylinder') return [d(0), d(1), d(0)];
  if (g.shape === 'sphere') return [d(0), d(0), d(0)];
  if (g.shape === 'plate') return [d(0), Math.max(d(2), 0.04), d(1)];
  return [d(0), d(1), d(2)];
}

function geometryOf(p: ScenePart): THREE.BufferGeometry {
  const g = p.geometry;
  const [sx, sy, sz] = dimsOf(p);
  if (g.kind === 'primitive' && g.shape === 'cylinder') return new THREE.CylinderGeometry(sx / 2, sx / 2, sy, 40);
  if (g.kind === 'primitive' && g.shape === 'sphere') return new THREE.SphereGeometry(sx / 2, 32, 24);
  return new THREE.BoxGeometry(sx, sy, sz);
}

function labelSprite(text: string, tone: 'normal' | 'alert'): THREE.Sprite {
  const c = document.createElement('canvas');
  const ctx = c.getContext('2d')!;
  const font = '500 26px "IBM Plex Sans", system-ui, sans-serif';
  ctx.font = font;
  const w = Math.ceil(ctx.measureText(text).width) + 36;
  c.width = w;
  c.height = 48;
  ctx.font = font;
  ctx.fillStyle = tone === 'alert' ? 'rgba(239,68,68,0.92)' : 'rgba(8,16,30,0.82)';
  ctx.beginPath();
  ctx.roundRect(0, 0, w, 48, 12);
  ctx.fill();
  ctx.strokeStyle = tone === 'alert' ? 'rgba(255,255,255,0.35)' : 'rgba(143,173,220,0.35)';
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.fillStyle = '#F3F7FC';
  ctx.textBaseline = 'middle';
  ctx.fillText(text, 18, 25);
  const tex = new THREE.CanvasTexture(c);
  tex.colorSpace = THREE.SRGBColorSpace;
  const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, depthTest: false, depthWrite: false }));
  sp.userData.base = [(w / 48) * 0.26, 0.26];
  sp.scale.set((w / 48) * 0.26, 0.26, 1);
  sp.renderOrder = 10;
  return sp;
}

interface PartVisual {
  id: string;
  key: string;
  group: THREE.Group;
  body: THREE.Group;
  mesh: THREE.Mesh;
  /** Materials of an imported model, tinted by emissive since its own colours are kept */
  importMats: THREE.MeshStandardMaterial[];
  loadedSize: [number, number, number] | null;
  edges: THREE.LineSegments;
  marker: THREE.Mesh | null;
  label: THREE.Sprite;
  alert: THREE.Sprite | null;
  size: [number, number, number];
  target: THREE.Vector3;
  born: number;
  base: THREE.Color;
  placeholder: boolean;
}

interface Instance {
  root: THREE.Group;
  parts: Map<string, PartVisual>;
  connections: THREE.Group;
  connKey: string;
  title: THREE.Sprite | null;
  offsetX: number;
}

export const ModelScene: React.FC<ModelSceneProps> = props => {
  const mount = useRef<HTMLDivElement>(null);
  const P = useRef(props);
  P.current = props;
  const api = useRef<{ refit: () => void } | null>(null);

  useEffect(() => {
    const container = mount.current!;
    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false, powerPreference: 'high-performance' });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(renderer.domElement);
    renderer.domElement.style.display = 'block';

    const scene = new THREE.Scene();
    const bg = new THREE.Color(0x030814);
    scene.background = bg;
    const fog = new THREE.Fog(0x030814, 14, 40);
    scene.fog = fog;
    const pmrem = new THREE.PMREMGenerator(renderer);
    const envTex = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = envTex;
    let hdr: { env: THREE.Texture; sky: THREE.Texture } | null = null;
    let lastEnvId: string | null | undefined;
    const useHdri = (id: string | null) => {
      if (id === lastEnvId) return;
      lastEnvId = id;
      if (hdr) { hdr.env.dispose(); hdr.sky.dispose(); hdr = null; }
      scene.environment = envTex;
      scene.backgroundBlurriness = 0;
      if (!id) return;
      getAsset(id).then(buf => {
        if (!buf || lastEnvId !== id) return;
        const data = new RGBELoader().parse(buf);
        const sky = new THREE.DataTexture(data.data as never, data.width, data.height, THREE.RGBAFormat, data.type);
        sky.mapping = THREE.EquirectangularReflectionMapping;
        sky.colorSpace = THREE.LinearSRGBColorSpace;
        sky.minFilter = THREE.LinearFilter; sky.magFilter = THREE.LinearFilter; sky.generateMipmaps = false; sky.needsUpdate = true;
        hdr = { env: pmrem.fromEquirectangular(sky).texture, sky };
        scene.environment = hdr.env;
      }).catch(() => undefined);
    };

    const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 200);
    const key = new THREE.DirectionalLight(0xe8f1ff, 2.4);
    key.position.set(5, 9, 6);
    key.castShadow = true;
    key.shadow.mapSize.set(1024, 1024);
    key.shadow.camera.left = -9; key.shadow.camera.right = 9; key.shadow.camera.top = 9; key.shadow.camera.bottom = -9;
    key.shadow.bias = -0.0004;
    scene.add(key, new THREE.AmbientLight(0x1a2a4a, 0.9));
    const rim = new THREE.PointLight(0x22d3ee, 14, 18);
    rim.position.set(-5, 3, 3);
    scene.add(rim);

    // Floor: Realistic is a dark reflective disc with a reticle; Engineering is a neutral floor with a grid.
    const realisticFloor = new THREE.Group();
    const disc = new THREE.Mesh(new THREE.CylinderGeometry(4.6, 4.7, 0.1, 96), new THREE.MeshStandardMaterial({ color: 0x050d1c, metalness: 0.85, roughness: 0.28 }));
    disc.position.y = -0.05; disc.receiveShadow = true;
    realisticFloor.add(disc);
    [4.6, 3.4, 2.2].forEach((r, i) => {
      const ring = new THREE.Mesh(new THREE.RingGeometry(r - 0.012, r + 0.012, 96), new THREE.MeshBasicMaterial({ color: i === 0 ? TRACE : ACCENT, transparent: true, opacity: i === 0 ? 0.7 : 0.35, side: THREE.DoubleSide }));
      ring.rotation.x = -Math.PI / 2; ring.position.y = 0.002;
      realisticFloor.add(ring);
    });
    const engineeringFloor = new THREE.Group();
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), new THREE.MeshStandardMaterial({ color: 0x0e1b2e, roughness: 0.9, metalness: 0 }));
    plane.rotation.x = -Math.PI / 2; plane.receiveShadow = true;
    const grid = new THREE.GridHelper(40, 80, 0x2b4468, 0x182a44);
    (grid.material as THREE.Material).transparent = true; (grid.material as THREE.Material).opacity = 0.7;
    grid.position.y = 0.003;
    engineeringFloor.add(plane, grid);
    scene.add(realisticFloor, engineeringFloor);

    // Axis gizmo so orientation is never a guess
    const gizmo = new THREE.Group();
    [[1, 0, 0, 0xef4444], [0, 1, 0, 0x22c55e], [0, 0, 1, 0x38bdf8]].forEach(([x, y, z, c]) => {
      gizmo.add(new THREE.Line(new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(), new THREE.Vector3(x as number, y as number, z as number).multiplyScalar(0.5)]), new THREE.LineBasicMaterial({ color: c as number })));
    });
    gizmo.position.set(-3.2, 0.01, 2.6);
    scene.add(gizmo);

    // Heat shimmer for a part past its limit: deterministic in t, so scrubbing repeats it exactly
    const PN = 70;
    const heatGeo = new THREE.BufferGeometry();
    heatGeo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(PN * 3), 3));
    const heat = new THREE.Points(heatGeo, new THREE.PointsMaterial({ color: 0xff8a5c, size: 0.07, transparent: true, opacity: 0.85, depthWrite: false }));
    heat.visible = false;
    scene.add(heat);

    const instances: Instance[] = [];
    const raycaster = new THREE.Raycaster();
    const tmpColor = new THREE.Color();
    const cam = { target: new THREE.Vector3(0, 0.6, 0), radius: 7, theta: 0.8, phi: 1.05 };
    const goal = { target: new THREE.Vector3(0, 0.6, 0), radius: 7, theta: 0.8, phi: 1.05 };
    let userMoved = false;
    let lastFitKey = '';
    let lastFocus: string | null | undefined;
    let lastPreset: CameraPreset | undefined;
    let lastFitSignal = P.current.fitSignal;
    const clock = new THREE.Clock();
    let raf = 0;
    let hovered: string | null = null;

    // ---------- instance building ----------
    const makeInstance = (): Instance => {
      const root = new THREE.Group();
      const connections = new THREE.Group();
      root.add(connections);
      scene.add(root);
      return { root, parts: new Map(), connections, connKey: '', title: null, offsetX: 0 };
    };

    const gltf = new GLTFLoader();
    const modelCache = new Map<string, Promise<THREE.Object3D | null>>();
    const loadImported = (assetId: string): Promise<THREE.Object3D | null> => {
      let pr = modelCache.get(assetId);
      if (!pr) {
        pr = getAsset(assetId).then(buf => (buf ? new Promise<THREE.Object3D | null>(res => gltf.parse(buf, '', g => res(g.scene), () => res(null))) : null)).catch(() => null);
        modelCache.set(assetId, pr);
      }
      // each part gets its own copy so tinting one does not tint another
      return pr.then(o => (o ? o.clone(true) : null)).then(o => { o?.traverse(x => { const m = x as THREE.Mesh; if (m.isMesh && !Array.isArray(m.material)) m.material = m.material.clone(); }); return o; });
    };

    const buildPart = (inst: Instance, p: ScenePart, index: number): PartVisual => {
      const size = dimsOf(p);
      const geo = geometryOf(p);
      const base = new THREE.Color(METALS[index % METALS.length]);
      const mat = new THREE.MeshPhysicalMaterial({ color: base, metalness: 0.72, roughness: 0.34, clearcoat: 0.35, clearcoatRoughness: 0.3, envMapIntensity: 1.1 });
      const mesh = new THREE.Mesh(geo, mat);
      const imported = p.geometry.kind === 'import';
      mesh.castShadow = !imported; mesh.receiveShadow = !imported;
      if (imported) mat.visible = false; // an invisible proxy for picking until the model is loaded
      mesh.userData.partId = p.id;
      const edges = new THREE.LineSegments(new THREE.EdgesGeometry(geo, 25), new THREE.LineBasicMaterial({ color: ACCENT, transparent: true, opacity: 0.55 }));
      const body = new THREE.Group();
      body.add(mesh, edges);
      const group = new THREE.Group();
      group.add(body);
      const label = labelSprite(p.name.replace(/ \(Example\)$/, ''), 'normal');
      label.position.set(0, size[1] / 2 + 0.34, 0);
      group.add(label);
      // port dots
      group.userData.partId = p.id;
      const v: PartVisual = { id: p.id, key: '', group, body, mesh, edges, marker: null, label, alert: null, size, target: new THREE.Vector3(), born: clock.elapsedTime, base, placeholder: !!p.placeholder, importMats: [], loadedSize: null };
      inst.root.add(group);
      if (p.geometry.kind === 'import') {
        edges.visible = false;
        loadImported(p.geometry.assetId).then(model => {
          if (!model || !v.group.parent) return;
          const box = new THREE.Box3().setFromObject(model);
          const sz = box.getSize(new THREE.Vector3());
          const k = 1.2 / Math.max(sz.x, sz.y, sz.z, 1e-6); // schematic: largest side drawn at 1.2 units
          model.scale.multiplyScalar(k);
          const b2 = new THREE.Box3().setFromObject(model);
          const c = b2.getCenter(new THREE.Vector3());
          model.position.sub(c);
          const s2 = b2.getSize(new THREE.Vector3());
          v.loadedSize = [s2.x, s2.y, s2.z]; v.size = [s2.x, s2.y, s2.z];
          model.traverse(o => {
            const m = o as THREE.Mesh;
            if (m.isMesh) {
              m.castShadow = true; m.receiveShadow = true; m.userData.partId = p.id;
              (Array.isArray(m.material) ? m.material : [m.material]).forEach(mm => { if ((mm as THREE.MeshStandardMaterial).isMeshStandardMaterial) v.importMats.push(mm as THREE.MeshStandardMaterial); });
            }
          });
          // the proxy takes the model's size so clicks and connections line up
          v.mesh.geometry.dispose(); v.mesh.geometry = new THREE.BoxGeometry(s2.x, s2.y, s2.z);
          v.body.add(model);
          label.position.set(0, s2.y / 2 + 0.34, 0);
        });
      }
      return v;
    };

    const partKey = (p: ScenePart) => JSON.stringify([p.geometry, !!p.placeholder]);

    const syncParts = (inst: Instance, props: ModelSceneProps) => {
      const ids = new Set(props.parts.map(p => p.id));
      for (const [id, v] of inst.parts) {
        if (!ids.has(id)) { inst.root.remove(v.group); v.mesh.geometry.dispose(); inst.parts.delete(id); }
      }
      props.parts.forEach((p, i) => {
        const k = partKey(p);
        let v = inst.parts.get(p.id);
        if (v && v.key !== k && v.key !== '') { inst.root.remove(v.group); v.mesh.geometry.dispose(); inst.parts.delete(p.id); v = undefined; }
        if (!v) { v = buildPart(inst, p, i); inst.parts.set(p.id, v); }
        v.key = k;
        v.size = v.loadedSize ?? dimsOf(p);
        const [x, y, z] = p.pos ?? [0, 0.5 + i * 0.01, 0];
        v.target.set(x, y, z);
        if (v.group.position.lengthSq() === 0 && v.born === clock.elapsedTime) v.group.position.copy(v.target).add(new THREE.Vector3(0, 0.9, 0));
        v.placeholder = !!p.placeholder;
      });
      // connections: typed tubes between the facing surfaces of the two parts, with a dot at each port
      const ck = JSON.stringify([props.connections.map(c => [c.id, c.from, c.to, c.kind]), props.parts.map(p => [p.id, p.pos])]);
      if (ck !== inst.connKey) {
        inst.connKey = ck;
        while (inst.connections.children.length) {
          const c = inst.connections.children.pop()!;
          c.traverse(o => { const m = o as THREE.Mesh; if (m.geometry) m.geometry.dispose(); });
        }
        for (const c of props.connections) {
          const a = inst.parts.get(c.from.partId); const b = inst.parts.get(c.to.partId);
          if (!a || !b) continue;
          const pa = a.target.clone(); const pb = b.target.clone();
          const dir = pb.clone().sub(pa).normalize();
          const ra = Math.max(...a.size) * 0.5 * 0.92; const rb = Math.max(...b.size) * 0.5 * 0.92;
          const s = pa.clone().addScaledVector(dir, ra); const e = pb.clone().addScaledVector(dir, -rb);
          const mid = s.clone().add(e).multiplyScalar(0.5).add(new THREE.Vector3(0, 0.28, 0));
          const curve = new THREE.CatmullRomCurve3([s, mid, e]);
          const color = KIND_COLOR[c.kind] ?? 0x8196b3;
          const tube = new THREE.Mesh(new THREE.TubeGeometry(curve, 24, 0.028, 8), new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.5, roughness: 0.4 }));
          const g = new THREE.Group();
          g.add(tube);
          for (const pt of [s, e]) { const dot = new THREE.Mesh(new THREE.SphereGeometry(0.065, 16, 12), new THREE.MeshStandardMaterial({ color, emissive: color, emissiveIntensity: 0.8 })); dot.position.copy(pt); g.add(dot); }
          inst.connections.add(g);
        }
      }
    };

    // ---------- camera ----------
    const boundsOf = () => {
      const box = new THREE.Box3();
      for (const inst of instances) for (const v of inst.parts.values()) {
        const h = new THREE.Vector3(...v.size).multiplyScalar(0.5);
        box.expandByPoint(v.target.clone().add(h).add(new THREE.Vector3(inst.offsetX, 0, 0)));
        box.expandByPoint(v.target.clone().sub(h).add(new THREE.Vector3(inst.offsetX, 0, 0)));
      }
      return box;
    };
    const fit = () => {
      const box = boundsOf();
      if (box.isEmpty()) { goal.target.set(0, 0.6, 0); goal.radius = 7; return; }
      const size = box.getSize(new THREE.Vector3());
      goal.target.copy(box.getCenter(new THREE.Vector3()));
      const free = Math.max(0.45, 1 - (P.current.insets ? P.current.insets.left + P.current.insets.right : 0) / Math.max(1, container.clientWidth));
      goal.radius = (Math.max(3, Math.max(size.x, size.y * 1.6, size.z) * 1.0 + 1.3)) / free;
    };
    api.current = { refit: () => { userMoved = false; fit(); } };

    const applyPreset = (pr: CameraPreset) => {
      userMoved = false;
      if (pr === 'Top') { goal.theta = 0.0; goal.phi = 0.04; }
      else if (pr === 'Front') { goal.theta = 0; goal.phi = 1.42; }
      else if (pr === 'Right') { goal.theta = Math.PI / 2; goal.phi = 1.42; }
      else { goal.theta = 0.8; goal.phi = 1.05; }
      fit();
    };

    // ---------- interaction ----------
    let drag: { x: number; y: number; moved: number } | null = null;
    const dom = renderer.domElement;
    const ndc = (e: PointerEvent) => { const r = dom.getBoundingClientRect(); return new THREE.Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1); };
    const pick = (e: PointerEvent): string | null => {
      raycaster.setFromCamera(ndc(e), camera);
      const meshes: THREE.Object3D[] = [];
      for (const inst of instances) for (const v of inst.parts.values()) meshes.push(v.mesh);
      const hit = raycaster.intersectObjects(meshes, false)[0];
      return hit ? (hit.object.userData.partId as string) : null;
    };
    const onDown = (e: PointerEvent) => { drag = { x: e.clientX, y: e.clientY, moved: 0 }; dom.setPointerCapture(e.pointerId); };
    const onMove = (e: PointerEvent) => {
      if (drag) {
        const dx = e.clientX - drag.x; const dy = e.clientY - drag.y;
        drag.x = e.clientX; drag.y = e.clientY; drag.moved += Math.abs(dx) + Math.abs(dy);
        goal.theta -= dx * 0.006;
        goal.phi = Math.min(1.5, Math.max(0.04, goal.phi - dy * 0.005));
        userMoved = true;
      } else {
        hovered = pick(e);
        dom.style.cursor = hovered ? 'pointer' : 'grab';
      }
    };
    const onUp = (e: PointerEvent) => {
      if (drag && drag.moved < 4) P.current.onSelect?.(pick(e));
      drag = null;
    };
    const onWheel = (e: WheelEvent) => { e.preventDefault(); goal.radius = Math.min(30, Math.max(2, goal.radius * (1 + e.deltaY * 0.0012))); userMoved = true; };
    dom.style.cursor = 'grab';
    dom.addEventListener('pointerdown', onDown);
    dom.addEventListener('pointermove', onMove);
    dom.addEventListener('pointerup', onUp);
    dom.addEventListener('wheel', onWheel, { passive: false });

    const applyInsets = () => {
      const w = container.clientWidth || 800; const h = container.clientHeight || 500;
      const ins = P.current.insets;
      if (ins && (ins.left || ins.right)) camera.setViewOffset(w, h, -(ins.left - ins.right) / 2, 0, w, h); else camera.clearViewOffset();
    };
    const resize = () => {
      const w = container.clientWidth || 800; const h = container.clientHeight || 500;
      renderer.setSize(w, h, false);
      dom.style.width = '100%'; dom.style.height = '100%';
      camera.aspect = w / h; camera.updateProjectionMatrix();
      applyInsets();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(container);
    resize();

    // ---------- frame loop ----------
    const frame = () => {
      raf = requestAnimationFrame(frame);
      const props = P.current;
      const dt = Math.min(clock.getDelta(), 0.05);
      const now = clock.elapsedTime;

      // instances follow the props
      const wanted = props.compare ? 2 : 1;
      while (instances.length < wanted) instances.push(makeInstance());
      while (instances.length > wanted) { const i = instances.pop()!; scene.remove(i.root); }
      const box0 = new THREE.Box3();
      props.parts.forEach(p => { const s = dimsOf(p); const [x, y, z] = p.pos ?? [0, 0.5, 0]; box0.expandByPoint(new THREE.Vector3(x - s[0] / 2, y - s[1] / 2, z - s[2] / 2)); box0.expandByPoint(new THREE.Vector3(x + s[0] / 2, y + s[1] / 2, z + s[2] / 2)); });
      const width = box0.isEmpty() ? 0 : box0.max.x - box0.min.x;
      instances.forEach((inst, i) => {
        inst.offsetX = i === 0 ? (props.compare ? -(width / 2 + 0.9) : 0) : width / 2 + 0.9;
        inst.root.position.x += (inst.offsetX - inst.root.position.x) * 0.15;
        syncParts(inst, props);
      });

      // look
      const eng = props.look === 'Engineering';
      realisticFloor.visible = !eng; engineeringFloor.visible = eng;
      bg.set(eng ? 0x0b1626 : 0x030814);
      fog.color.copy(bg);
      useHdri(props.environmentAssetId ?? null);
      // The HDRI is the backdrop in the Realistic look; Engineering keeps its neutral floor and grid.
      scene.background = hdr && !eng ? hdr.sky : bg;
      scene.backgroundBlurriness = hdr && !eng ? 0.25 : 0;
      fog.near = hdr && !eng ? 1000 : 14; fog.far = hdr && !eng ? 2000 : 40;
      const shadows = props.qualityTier !== 'Low';
      renderer.shadowMap.enabled = shadows; key.castShadow = shadows;

      // camera goals
      const fitKey = instances.map(i => i.parts.size).join('/') + (props.compare ? 'c' : '');
      if (fitKey !== lastFitKey) { lastFitKey = fitKey; if (!userMoved) fit(); }
      if (props.fitSignal !== lastFitSignal) { lastFitSignal = props.fitSignal; lastPreset = undefined; }
      if (props.cameraPreset !== lastPreset) { lastPreset = props.cameraPreset; applyPreset(props.cameraPreset); }
      if (props.focusId !== lastFocus) {
        lastFocus = props.focusId;
        const v = props.focusId ? instances[0]?.parts.get(props.focusId) : undefined;
        if (v) { goal.target.copy(v.target).add(new THREE.Vector3(instances[0].offsetX, 0, 0)); goal.radius = Math.max(4.6, Math.max(...v.size) * 3.6); }
        else if (!userMoved) fit();
      }
      const k = 1 - Math.pow(0.001, dt);
      cam.target.lerp(goal.target, k);
      cam.radius += (goal.radius - cam.radius) * k;
      cam.theta += (goal.theta - cam.theta) * k;
      cam.phi += (goal.phi - cam.phi) * k;
      camera.position.set(
        cam.target.x + cam.radius * Math.sin(cam.phi) * Math.sin(cam.theta),
        cam.target.y + cam.radius * Math.cos(cam.phi),
        cam.target.z + cam.radius * Math.sin(cam.phi) * Math.cos(cam.theta)
      );
      camera.lookAt(cam.target);
      applyInsets();

      // parts
      let hotPos: THREE.Vector3 | null = null;
      let hotAmount = 0;
      instances.forEach((inst, idx) => {
        const series = idx === 0 ? props.series : props.compare!.series;
        for (const [id, v] of inst.parts) {
          const s: PartVisualState = stateFor(id, props.bindings, series, props.t);
          // arrival: slide in and grow
          const age = Math.min(1, (now - v.born) / 0.5);
          const ease = 1 - Math.pow(1 - age, 3);
          v.group.position.lerp(v.target, 1 - Math.pow(0.0005, dt));
          v.group.scale.setScalar(0.2 + 0.8 * ease);
          // bound motion
          v.body.rotation.set(0, 0, 0);
          v.body.position.set(0, 0, 0);
          if (s.rotation) v.body.rotation[s.rotation.axis] = s.rotation.angle;
          if (s.shake !== null) {
            const amp = 0.012 + Math.min(1.6, s.shake) * 0.03;
            v.body.position.set(Math.sin(props.t * 61 + hash01(v.size[0]) * 6) * amp, Math.sin(props.t * 47) * amp * 0.6, Math.cos(props.t * 53) * amp);
          }
          if (s.slide) { const ext = Math.max(...v.size) * 0.5; v.body.position[s.slide.axis] += s.slide.metres !== null ? s.slide.metres : s.slide.amount * ext; }
          v.group.visible = s.visible;
          // rotation marker: a bright tick on the side so spin is visible on a symmetric part
          if (s.rotation && !v.marker) {
            const ax = s.rotation.axis;
            const tick = new THREE.Mesh(new THREE.BoxGeometry(0.12, 0.12, 0.12), new THREE.MeshBasicMaterial({ color: 0xffffff }));
            tick.position.set(ax === 'x' ? 0 : v.size[0] / 2, ax === 'x' ? v.size[1] / 2 : 0, 0);
            v.body.add(tick); v.marker = tick;
          }
          // colour and glow
          const mat = v.mesh.material as THREE.MeshPhysicalMaterial;
          const f = s.heat ?? 0;
          if (v.importMats.length === 0) { heatColor(f, v.base, tmpColor); mat.color.copy(tmpColor); }
          const failing = idx === 0 && !!props.failure && props.failure.partId === id && props.t >= props.failure.t;
          const over = failing || f >= 1;
          let emis = 0;
          if (s.heat !== null && f > 0.55) emis = (f - 0.55) * 1.6;
          if (s.glow !== null) emis = Math.max(emis, Math.min(1, s.glow) * 0.55);
          if (over) emis = 1.1 + 0.6 * Math.sin(props.t * 9 + now * 6);
          const flash = Math.max(0, 1 - (now - v.born) / 0.9) * 0.7;
          mat.emissive.set(over ? 0xef4444 : s.glow !== null && s.heat === null ? TRACE : f > 0.55 ? 0xff7a3d : 0x2f7bff);
          mat.emissiveIntensity = emis + flash;
          for (const im of v.importMats) { im.emissive.copy(mat.emissive); im.emissiveIntensity = emis + flash; im.transparent = props.draft || v.placeholder; im.opacity = props.draft ? 0.45 : 1; }
          mat.transparent = props.draft || v.placeholder;
          mat.opacity = props.draft ? 0.45 : v.placeholder ? 0.6 : 1;
          // edges: selection, chain highlight, hover
          const sel = props.selectedId === id; const hl = props.highlightIds?.includes(id);
          const em = v.edges.material as THREE.LineBasicMaterial;
          em.color.set(over ? 0xff8a8a : sel ? 0xffffff : hl ? TRACE : hovered === id ? 0x9cc2ff : ACCENT);
          em.opacity = sel || hl || over ? 1 : props.draft ? 0.9 : 0.5;
          // label
          const showAll = props.labels === 'always' || eng;
          v.label.visible = showAll || sel || hovered === id || over || !!hl;
          // failure label
          if (failing && !v.alert) { v.alert = labelSprite(props.failure!.text, 'alert'); v.alert.position.set(0, v.size[1] / 2 + 0.72, 0); v.group.add(v.alert); }
          if (v.alert) v.alert.visible = failing;
          if (failing || (f >= 0.85 && idx === 0)) { const w = new THREE.Vector3(); v.group.getWorldPosition(w); if (!hotPos || f > hotAmount) { hotPos = w.add(new THREE.Vector3(0, v.size[1] / 2, 0)); hotAmount = Math.max(f, failing ? 1.2 : f); } }
        }
      });

      // heat shimmer rising from the hottest part, a pure function of t
      if (hotPos && hotAmount >= 0.85) {
        heat.visible = true;
        const arr = heatGeo.attributes.position.array as Float32Array;
        const count = Math.round(PN * Math.min(1, (hotAmount - 0.8) * 3));
        for (let i = 0; i < PN; i++) {
          const ph = (hash01(i) + props.t * 0.7) % 1;
          arr[i * 3] = hotPos.x + (hash01(i + 9) - 0.5) * 0.9 + Math.sin(props.t * 3 + i) * 0.05;
          arr[i * 3 + 1] = i < count ? hotPos.y + ph * 1.6 : -50;
          arr[i * 3 + 2] = hotPos.z + (hash01(i + 21) - 0.5) * 0.9;
        }
        heatGeo.attributes.position.needsUpdate = true;
        (heat.material as THREE.PointsMaterial).opacity = 0.9 * (1 - 0.0);
      } else heat.visible = false;

      // titles over each copy when comparing
      instances.forEach((inst, i) => {
        const text = i === 0 ? (props.primaryLabel ?? '') : props.compare?.label ?? '';
        if (props.compare && text) {
          if (!inst.title || inst.title.userData.text !== text) {
            if (inst.title) inst.root.remove(inst.title);
            inst.title = labelSprite(text, 'normal'); inst.title.userData.text = text; inst.root.add(inst.title);
          }
          inst.title.position.set(box0.isEmpty() ? 0 : (box0.min.x + box0.max.x) / 2, (box0.isEmpty() ? 1 : box0.max.y) + 1.3, 0);
          inst.title.visible = true;
        } else if (inst.title) inst.title.visible = false;
      });

      // labels keep a steady size on screen whatever the camera distance
      const wp = new THREE.Vector3();
      scene.traverse(o => {
        if ((o as THREE.Sprite).isSprite && o.userData.base) {
          o.getWorldPosition(wp);
          const k2 = Math.min(1.6, Math.max(0.5, camera.position.distanceTo(wp) / 9));
          o.scale.set(o.userData.base[0] * k2, o.userData.base[1] * k2, 1);
        }
      });
      renderer.render(scene, camera);
    };
    raf = requestAnimationFrame(frame);

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      dom.removeEventListener('pointerdown', onDown);
      dom.removeEventListener('pointermove', onMove);
      dom.removeEventListener('pointerup', onUp);
      dom.removeEventListener('wheel', onWheel);
      scene.traverse(o => { const m = o as THREE.Mesh; m.geometry?.dispose?.(); });
      envTex.dispose();
      if (hdr) { hdr.env.dispose(); hdr.sky.dispose(); }
      pmrem.dispose();
      renderer.dispose();
      renderer.forceContextLoss();
      container.removeChild(renderer.domElement);
    };
  }, []);

  return <div ref={mount} className={`w-full h-full ${props.className ?? ''}`} data-testid="model-scene" />;
};
