import React, { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
type QualityTier = 'Auto' | 'High' | 'Low';
import { GeometryKind } from '../contracts';
import { createExampleAssembly, ExampleAssemblyContext } from '../mock/exampleAssembly';
import { ExampleTag, IconButton, Tabs } from '../components/ui/UiKit';
import { Pause, Play } from 'lucide-react';
import {
  createBrushedMetalTextures,
  createMachinedRadialTexture,
  createCopperWindingTexture,
  createCarbonFiberTexture,
  createThermalGradientTexture,
  createStudioEnvironment
} from './materials';

export type RenderMode = 'realistic' | 'ghost' | 'wireframe';

/** A part of the project's own model, as the build reports it (file 35 section 8 geometry). */
export interface ScenePart {
  id: string;
  name: string;
  geometry: GeometryKind;
  /** Position in metres; drawn 1 scene unit per metre. */
  pos?: [number, number, number];
  placeholder?: boolean;
}

// Primitive dimensions are millimetres, drawn at 1 scene unit per 100 mm so small parts stay
// visible next to metre-scale placement (a schematic scale, not a true-scale view). Imported
// assets are not loaded into this canvas yet, so they show as an outline box.
function partObject(
  p: ScenePart,
  getMaterial: (cfg: THREE.MeshStandardMaterialParameters, ghost?: number) => THREE.Material
): THREE.Object3D {
  const g = p.geometry;
  const d = (i: number) => (g.kind === 'primitive' && g.dims[i] ? g.dims[i].v : 100) / 100;
  let geo: THREE.BufferGeometry;
  if (g.kind === 'primitive' && g.shape === 'cylinder') geo = new THREE.CylinderGeometry(d(0) / 2, d(0) / 2, d(1), 32);
  else if (g.kind === 'primitive' && g.shape === 'sphere') geo = new THREE.SphereGeometry(d(0) / 2, 24, 24);
  else if (g.kind === 'primitive' && g.shape === 'plate') geo = new THREE.BoxGeometry(d(0), Math.max(d(2), 0.02), d(1));
  else geo = new THREE.BoxGeometry(d(0), d(1), d(2));
  const color = p.placeholder || g.kind === 'import' ? 0x64748b : 0x2f7bff;
  const mat = getMaterial(
    { color, metalness: 0.6, roughness: 0.35, transparent: !!p.placeholder, opacity: p.placeholder ? 0.55 : 1 },
    color
  );
  const mesh = new THREE.Mesh(geo, mat);
  const [x, y, z] = p.pos ?? [0, 0.5, 0];
  mesh.position.set(x, y, z);
  mesh.name = p.id;
  return mesh;
}

export interface HeroSceneProps {
  variant?: 'landing' | 'workspace' | 'model_editor' | 'simulation_run' | 'simulation_results' | 'failure' | 'compare';
  interactive?: boolean;
  qualityTier?: QualityTier;
  thermalHotspot?: boolean;
  explodedOffset?: number; // 0 to 1
  isGhostMode?: boolean;
  initialRenderMode?: RenderMode;
  /** Realistic is the dark cinematic floor; Engineering is a neutral studio floor with a grid. */
  look?: 'Realistic' | 'Engineering';
  cameraPreset?: 'Perspective' | 'Top' | 'Front' | 'Right' | 'Follow' | 'Orbit' | 'FPV';
  repairState?: 'none' | 'fault' | 'repairing' | 'repaired';
  /** The project's parts. When absent, the canvas shows the labelled Example scene instead. */
  parts?: ScenePart[] | null;
  className?: string;
}

export const HeroScene: React.FC<HeroSceneProps> = ({
  variant = 'workspace',
  interactive = true,
  qualityTier = 'High',
  thermalHotspot = false,
  explodedOffset = 0,
  isGhostMode = false,
  initialRenderMode = 'realistic',
  look = 'Realistic',
  cameraPreset = 'Perspective',
  repairState = 'none',
  parts = null,
  className = ''
}) => {
  const mountRef = useRef<HTMLDivElement>(null);
  const partsKey = parts ? JSON.stringify(parts) : '';
  const [isRotating, setIsRotating] = useState<boolean>(true);
  
  // Shading modes: Ghost (draft), Wireframe, Realistic
  const [renderMode, setRenderMode] = useState<RenderMode>(isGhostMode ? 'ghost' : initialRenderMode);

  // Example-scene look per stage (fault colours in Understand, changed design in Improve)
  const [localRepairState, setLocalRepairState] = useState<'none' | 'fault' | 'repairing' | 'repaired'>(repairState);
  const repairProgress = 1;

  // Sync external repairState prop
  useEffect(() => {
    if (repairState !== 'none') {
      setLocalRepairState(repairState);
    }
  }, [repairState]);

  // Sync external isGhostMode prop (a draft is drawn as ghost; a committed model as solid)
  useEffect(() => {
    setRenderMode(isGhostMode ? 'ghost' : initialRenderMode);
  }, [isGhostMode, initialRenderMode]);

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    const width = container.clientWidth || 800;
    const height = container.clientHeight || 500;

    // 1. Scene & Camera
    const scene = new THREE.Scene();
    const engineering = look === 'Engineering';
    const bg = engineering ? 0x0b1626 : 0x020612;
    scene.background = new THREE.Color(bg);
    scene.fog = new THREE.FogExp2(bg, variant === 'landing' ? 0.025 : engineering ? 0.02 : 0.035);

    const camera = new THREE.PerspectiveCamera(45, width / height, 0.1, 100);
    if (variant === 'landing') {
      camera.position.set(0, 1.8, 8.5);
    } else if (variant === 'model_editor') {
      camera.position.set(4.5, 3.8, 6.0);
    } else if (variant === 'compare') {
      camera.position.set(0, 3.2, 7.8);
    } else {
      camera.position.set(3.8, 3.0, 5.2);
    }
    // Camera presets (Perspective is the stage default above)
    const dist = camera.position.length();
    if (cameraPreset === 'Top') camera.position.set(0, dist, 0.01);
    else if (cameraPreset === 'Front') camera.position.set(0, 1.4, dist * 0.95);
    else if (cameraPreset === 'Right') camera.position.set(dist * 0.95, 1.4, 0);
    camera.lookAt(0, 0.5, 0);

    // 2. WebGL renderer
    const renderer = new THREE.WebGLRenderer({
      antialias: qualityTier !== 'Low',
      powerPreference: 'high-performance',
      alpha: true
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, qualityTier === 'Low' ? 1 : 2));
    
    // ACES Filmic tone mapping (three.js)
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.6;
    renderer.shadowMap.enabled = qualityTier !== 'Low';
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(renderer.domElement);

    // 3. Studio environment reflections and lights
    const studioEnv = createStudioEnvironment(renderer);
    scene.environment = studioEnv.texture;

    const ambientLight = new THREE.AmbientLight(0x0f2042, 1.5);
    scene.add(ambientLight);

    const keyLight = new THREE.DirectionalLight(0xe0f2fe, 3.8);
    keyLight.position.set(6, 9, 5);
    keyLight.castShadow = qualityTier !== 'Low';
    scene.add(keyLight);

    const cyanRim = new THREE.PointLight(0x22d3ee, 5.2, 18);
    cyanRim.position.set(-5, 3, 3);
    scene.add(cyanRim);

    const violetRim = new THREE.PointLight(0x8b5cf6, 3.6, 15);
    violetRim.position.set(0, -2, -5);
    scene.add(violetRim);

    // 4. Procedural High-Fidelity PBR Textures
    const brushedMaps = createBrushedMetalTextures();
    const machinedRadial = createMachinedRadialTexture(512);
    const copperMaps = createCopperWindingTexture(512, 512);
    const carbonFiberMap = createCarbonFiberTexture(256);

    // 5. Circular reflective platform
    const platformGroup = new THREE.Group();

    // Base disc with brushed finish
    const platformGeo = new THREE.CylinderGeometry(3.8, 3.9, 0.14, 64);
    const platformMat = new THREE.MeshStandardMaterial({
      color: 0x030a18,
      metalness: 0.9,
      roughness: 0.18,
      map: brushedMaps.colorMap,
      normalMap: brushedMaps.normalMap
    });
    const platformMesh = new THREE.Mesh(platformGeo, platformMat);
    platformMesh.position.y = -0.07;
    platformMesh.receiveShadow = true;
    platformGroup.add(platformMesh);

    // Glowing Precision Concentric Reticle Rings
    const ringRadii = [3.8, 2.9, 1.9, 0.95];
    ringRadii.forEach((r, idx) => {
      const ringGeo = new THREE.RingGeometry(r - 0.015, r + 0.015, 64);
      const ringMat = new THREE.MeshBasicMaterial({
        color: idx === 0 ? 0x22d3ee : idx === 1 ? 0x2f7bff : 0x38bdf8,
        side: THREE.DoubleSide,
        transparent: true,
        opacity: idx === 0 ? 0.75 : 0.4
      });
      const ringMesh = new THREE.Mesh(ringGeo, ringMat);
      ringMesh.rotation.x = -Math.PI / 2;
      ringMesh.position.y = 0.005;
      platformGroup.add(ringMesh);
    });

    // Radial coordinate ticks
    const ticksCount = 36;
    for (let i = 0; i < ticksCount; i++) {
      const angle = (i / ticksCount) * Math.PI * 2;
      const isMajor = i % 6 === 0;
      const tickGeo = new THREE.PlaneGeometry(isMajor ? 0.45 : 0.22, 0.015);
      const tickMat = new THREE.MeshBasicMaterial({
        color: isMajor ? 0x22d3ee : 0x1e40af,
        transparent: true,
        opacity: isMajor ? 0.8 : 0.35,
        side: THREE.DoubleSide
      });
      const tickMesh = new THREE.Mesh(tickGeo, tickMat);
      tickMesh.rotation.x = -Math.PI / 2;
      tickMesh.rotation.z = angle;
      tickMesh.position.set(Math.cos(angle) * 3.3, 0.006, Math.sin(angle) * 3.3);
      platformGroup.add(tickMesh);
    }
    scene.add(platformGroup);

    // 6. Assembly Model Generator with Render Mode Support (Ghost, Wireframe, Realistic)
    const isRepaired = localRepairState === 'repaired';
    const isFaulty = localRepairState === 'fault' || variant === 'failure' || thermalHotspot;
    const isRepairing = localRepairState === 'repairing';

    const getMaterial = (baseConfig: THREE.MeshStandardMaterialParameters, ghostColor = 0x06b6d4) => {
      if (renderMode === 'wireframe') {
        return new THREE.MeshBasicMaterial({
          color: baseConfig.color || 0x38bdf8,
          wireframe: true
        });
      }
      if (renderMode === 'ghost') {
        return new THREE.MeshStandardMaterial({
          color: ghostColor,
          emissive: ghostColor,
          emissiveIntensity: 0.65,
          wireframe: true,
          transparent: true,
          opacity: 0.45
        });
      }
      // Realistic PBR
      return new THREE.MeshPhysicalMaterial({
        ...baseConfig,
        clearcoat: 0.4,
        clearcoatRoughness: 0.1
      });
    };

    // 7. The model: the project's own parts when the Studio has them, otherwise the Example scene
    const modelGroup = new THREE.Group();
    let windingRef: any = null;
    const exampleCtx: ExampleAssemblyContext = {
      getMaterial,
      textures: { machinedRadial, copperMaps, brushedMaps, carbonFiberMap },
      variant,
      renderMode,
      explodedOffset,
      isRepaired,
      isRepairing,
      repairProgress
    };

    if (parts) {
      parts.forEach(p => modelGroup.add(partObject(p, getMaterial)));
    } else if (variant === 'landing') {
      const portalRadius = 3.2;
      const portalTube = 0.038;
      const portalGeo = new THREE.TorusGeometry(portalRadius, portalTube, 32, 100);
      const portalMat = new THREE.MeshBasicMaterial({ color: 0x38bdf8, transparent: true, opacity: 0.95 });
      const portalMesh = new THREE.Mesh(portalGeo, portalMat);
      portalMesh.position.set(0, 2.2, -0.5);
      scene.add(portalMesh);

      const assembly = createExampleAssembly(exampleCtx, false, false);
      assembly.group.scale.set(1.15, 1.15, 1.15);
      assembly.group.position.y = 0.4;
      modelGroup.add(assembly.group);
      windingRef = assembly.windingMat;
    } else if (variant === 'compare') {
      const leftAssembly = createExampleAssembly(exampleCtx, true, false);
      leftAssembly.group.position.set(-1.8, 0, 0);
      modelGroup.add(leftAssembly.group);

      const rightAssembly = createExampleAssembly(exampleCtx, false, true);
      rightAssembly.group.position.set(1.8, 0, 0);
      modelGroup.add(rightAssembly.group);
      windingRef = leftAssembly.windingMat;
    } else {
      const assembly = createExampleAssembly(exampleCtx, isFaulty, isRepaired);
      modelGroup.add(assembly.group);
      windingRef = assembly.windingMat;
    }

    scene.add(modelGroup);

    // 8. Dynamic Sparks / Thermal Particle Swarm
    const particleCount = qualityTier === 'Low' ? 30 : (isFaulty || isRepairing ? 180 : 70);
    const particleGeo = new THREE.BufferGeometry();
    const positions = new Float32Array(particleCount * 3);
    for (let i = 0; i < particleCount * 3; i += 3) {
      positions[i] = (Math.random() - 0.5) * 4.5;
      positions[i + 1] = Math.random() * 3.2;
      positions[i + 2] = (Math.random() - 0.5) * 4.5;
    }
    particleGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const particleMat = new THREE.PointsMaterial({
      color: isRepairing ? 0x22d3ee : (isFaulty && !isRepaired ? 0xef4444 : (isRepaired ? 0x22c55e : 0x38bdf8)),
      size: isRepairing ? 0.06 : 0.04,
      transparent: true,
      opacity: 0.8
    });
    const particles = new THREE.Points(particleGeo, particleMat);
    scene.add(particles);

    // 9. Coordinate Orientation Gizmo
    const gizmoGroup = new THREE.Group();
    gizmoGroup.position.set(-2.5, 0.1, 1.8);
    const axisLen = 0.45;
    const xLine = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0, 0, 0), new THREE.Vector3(axisLen, 0, 0)]),
      new THREE.LineBasicMaterial({ color: 0xef4444, linewidth: 2 })
    );
    const yLine = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0, 0, 0), new THREE.Vector3(0, axisLen, 0)]),
      new THREE.LineBasicMaterial({ color: 0x22c55e, linewidth: 2 })
    );
    const zLine = new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0, 0, 0), new THREE.Vector3(0, 0, axisLen)]),
      new THREE.LineBasicMaterial({ color: 0x38bdf8, linewidth: 2 })
    );
    gizmoGroup.add(xLine, yLine, zLine);
    scene.add(gizmoGroup);

    // 10. Mouse Drag & Orbit Controls
    let isDragging = false;
    let prevMouseX = 0;
    let prevMouseY = 0;
    let targetRotationY = 0;
    let targetRotationX = 0;

    const onMouseDown = (e: MouseEvent) => {
      if (!interactive) return;
      isDragging = true;
      prevMouseX = e.clientX;
      prevMouseY = e.clientY;
      setIsRotating(false);
    };

    const onMouseMove = (e: MouseEvent) => {
      if (!isDragging || !interactive) return;
      const deltaX = e.clientX - prevMouseX;
      const deltaY = e.clientY - prevMouseY;
      prevMouseX = e.clientX;
      prevMouseY = e.clientY;

      targetRotationY += deltaX * 0.007;
      targetRotationX += deltaY * 0.005;
      targetRotationX = Math.max(-0.6, Math.min(0.8, targetRotationX));
    };

    const onMouseUp = () => {
      isDragging = false;
    };

    const onWheel = (e: WheelEvent) => {
      if (!interactive) return;
      e.preventDefault();
      const zoomFactor = e.deltaY * 0.003;
      camera.position.z = Math.max(3.0, Math.min(12.0, camera.position.z + zoomFactor));
    };

    container.addEventListener('mousedown', onMouseDown);
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
    container.addEventListener('wheel', onWheel, { passive: false });


    // 11. Animation loop
    let animationFrameId: number;
    let lastTime = performance.now();
    let frameCount = 0;

    const animate = () => {
      animationFrameId = requestAnimationFrame(animate);
      const now = performance.now();
      frameCount++;
      if (now - lastTime >= 1000) {
        frameCount = 0;
        lastTime = now;
      }

      if (isRotating && variant !== 'landing') {
        targetRotationY += 0.0035;
      }
      modelGroup.rotation.y += (targetRotationY - modelGroup.rotation.y) * 0.1;
      modelGroup.rotation.x += (targetRotationX - modelGroup.rotation.x) * 0.1;

      if (windingRef && isFaulty && !isRepaired && windingRef.emissiveIntensity !== undefined) {
        const pulse = 0.8 + Math.sin(now * 0.006) * 0.45;
        windingRef.emissiveIntensity = pulse;
      }

      const posAttr = particleGeo.attributes.position as THREE.BufferAttribute;
      const arr = posAttr.array as Float32Array;
      const speed = isRepairing ? 0.025 : (isFaulty ? 0.012 : 0.004);
      for (let i = 1; i < particleCount * 3; i += 3) {
        arr[i] += speed;
        if (arr[i] > 3.6) arr[i] = 0.1;
      }
      posAttr.needsUpdate = true;

      renderer.render(scene, camera);
    };

    animate();

    const handleResize = () => {
      if (!container) return;
      const w = container.clientWidth;
      const h = container.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    window.addEventListener('resize', handleResize);

    return () => {
      cancelAnimationFrame(animationFrameId);
      window.removeEventListener('resize', handleResize);
      container.removeEventListener('mousedown', onMouseDown);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      container.removeEventListener('wheel', onWheel);
      if (renderer.domElement && container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
      studioEnv.dispose();
      renderer.dispose();
      renderer.forceContextLoss(); // release the WebGL context now; rebuilds would otherwise pile them up
    };
    // Rebuilt only when the parts' content changes, not on every new array from the build stream.
  }, [variant, qualityTier, thermalHotspot, explodedOffset, interactive, isRotating, renderMode, localRepairState, partsKey, look, cameraPreset]);

  const modeTab = renderMode === 'wireframe' ? 'wireframe' : renderMode === 'ghost' ? 'ghost' : 'realistic';

  return (
    <div className={`relative w-full h-full overflow-hidden select-none ${className}`}>
      <div ref={mountRef} className="w-full h-full cursor-grab active:cursor-grabbing" />

      <div className="absolute top-4 left-4 z-10 flex items-center gap-2 pointer-events-none">
        {parts ? (
          <span data-testid="canvas-source" className="h-6 px-2.5 inline-flex items-center rounded-full text-[11px] text-ink bg-black/40 border border-white/10 backdrop-blur">
            {parts.length} {parts.length === 1 ? 'part' : 'parts'}
          </span>
        ) : (
          <span data-testid="canvas-source" className="pointer-events-auto"><ExampleTag label="Example scene" /></span>
        )}
      </div>

      {interactive && (
        <div className="absolute top-4 right-4 z-20 flex items-center gap-2 pointer-events-auto">
          <Tabs
            tabs={[{ id: 'realistic', label: 'Solid' }, { id: 'wireframe', label: 'Wireframe' }, { id: 'ghost', label: 'Ghost' }]}
            activeTab={modeTab}
            onChange={id => setRenderMode(id as RenderMode)}
            className="!bg-black/40 backdrop-blur"
          />
          <IconButton
            label={isRotating ? 'Pause rotation' : 'Resume rotation'}
            active={isRotating}
            onClick={() => setIsRotating(!isRotating)}
            className="!bg-black/40 backdrop-blur border border-white/10"
          >
            {isRotating ? <Pause size={15} /> : <Play size={15} />}
          </IconButton>
        </div>
      )}
    </div>
  );
};
