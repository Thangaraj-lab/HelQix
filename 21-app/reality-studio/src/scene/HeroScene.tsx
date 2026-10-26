import React, { useEffect, useRef, useState } from 'react';
import * as THREE from 'three';
import { QualityTier } from '../engineering/stateStore';
import {
  createBrushedMetalTextures,
  createMachinedRadialTexture,
  createCopperWindingTexture,
  createCarbonFiberTexture,
  createThermalGradientTexture,
  createStudioEnvironment
} from './materials';

export type RenderMode = 'realistic' | 'ghost' | 'wireframe';
export type EnginePreset = 'ue5' | 'blender' | 'standard';

export interface ScratchPart3D {
  id: string;
  name: string;
  shape: 'cylinder' | 'box' | 'flange' | 'conduit' | 'sensor';
  color: string;
  position: [number, number, number];
  rotation: [number, number, number];
  scale: [number, number, number];
}

export interface HeroSceneProps {
  variant?: 'landing' | 'workspace' | 'model_editor' | 'simulation_run' | 'simulation_results' | 'failure' | 'compare';
  interactive?: boolean;
  qualityTier?: QualityTier;
  thermalHotspot?: boolean;
  explodedOffset?: number; // 0 to 1
  isGhostMode?: boolean;
  initialRenderMode?: RenderMode;
  initialEnginePreset?: EnginePreset;
  repairState?: 'none' | 'fault' | 'repairing' | 'repaired';
  onExecuteRepair?: () => void;
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
  initialEnginePreset = 'ue5',
  repairState = 'none',
  onExecuteRepair,
  className = ''
}) => {
  const mountRef = useRef<HTMLDivElement>(null);
  const [fps, setFps] = useState<number>(60);
  const [isRotating, setIsRotating] = useState<boolean>(true);
  
  // Shading & Engine Presets (User requested: Ghost, Wireframe, Realistic, Blender, UE5)
  const [renderMode, setRenderMode] = useState<RenderMode>(isGhostMode ? 'ghost' : initialRenderMode);
  const [enginePreset, setEnginePreset] = useState<EnginePreset>(initialEnginePreset);
  const [showShadingMenu, setShowShadingMenu] = useState<boolean>(false);
  const [showScratchCreator, setShowScratchCreator] = useState<boolean>(false);
  const [showBridgeModal, setShowBridgeModal] = useState<'blender' | 'ue5' | null>(null);

  // Scratch 3D Parts dynamic list created by user
  const [scratchParts, setScratchParts] = useState<ScratchPart3D[]>([]);

  // 3D Repair State
  const [localRepairState, setLocalRepairState] = useState<'none' | 'fault' | 'repairing' | 'repaired'>(repairState);
  const [repairProgress, setRepairProgress] = useState<number>(0);

  // Sync external repairState prop
  useEffect(() => {
    if (repairState !== 'none') {
      setLocalRepairState(repairState);
    }
  }, [repairState]);

  // Sync external isGhostMode prop
  useEffect(() => {
    if (isGhostMode) {
      setRenderMode('ghost');
    }
  }, [isGhostMode]);

  // Handle on-canvas 3D physical repair execution
  const handleTriggerRepair = () => {
    setLocalRepairState('repairing');
    let p = 0;
    const interval = setInterval(() => {
      p += 0.05;
      if (p >= 1) {
        clearInterval(interval);
        setRepairProgress(1);
        setLocalRepairState('repaired');
        if (onExecuteRepair) onExecuteRepair();
      } else {
        setRepairProgress(p);
      }
    }, 50);
  };

  // Add a new 3D part from scratch
  const handleAddScratchPart = (shape: ScratchPart3D['shape']) => {
    const newPart: ScratchPart3D = {
      id: `scratch-${Date.now().toString(36)}`,
      name: `${shape.charAt(0).toUpperCase() + shape.slice(1)} Module`,
      shape,
      color: shape === 'conduit' ? '#06b6d4' : shape === 'sensor' ? '#8b5cf6' : '#2f7bff',
      position: [
        (Math.random() - 0.5) * 1.5,
        1.2 + Math.random() * 0.4,
        (Math.random() - 0.5) * 1.5
      ],
      rotation: [0, Math.random() * Math.PI, 0],
      scale: [1, 1, 1]
    };
    setScratchParts(prev => [...prev, newPart]);
    setShowScratchCreator(false);
  };

  useEffect(() => {
    const container = mountRef.current;
    if (!container) return;

    const width = container.clientWidth || 800;
    const height = container.clientHeight || 500;

    // 1. Scene & Camera
    const scene = new THREE.Scene();
    const isDarkVoid = enginePreset === 'ue5' || variant === 'landing';
    scene.background = new THREE.Color(isDarkVoid ? 0x020612 : 0x071120);
    scene.fog = new THREE.FogExp2(isDarkVoid ? 0x020612 : 0x071120, variant === 'landing' ? 0.025 : 0.035);

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
    camera.lookAt(0, 0.5, 0);

    // 2. WebGL Renderer with Unreal Engine 5 / Blender Color Management
    const renderer = new THREE.WebGLRenderer({
      antialias: qualityTier !== 'Low',
      powerPreference: 'high-performance',
      alpha: true
    });
    renderer.setSize(width, height);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, qualityTier === 'Low' ? 1 : 2));
    
    // UE5 Tone Mapping Pipeline: ACES Filmic with enhanced dynamic range
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = enginePreset === 'ue5' ? 1.6 : enginePreset === 'blender' ? 1.35 : 1.25;
    renderer.shadowMap.enabled = qualityTier !== 'Low';
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    container.appendChild(renderer.domElement);

    // 3. Studio Environment Reflections & Physical Lighting (UE5 Lumen style)
    const studioEnv = createStudioEnvironment(renderer);
    scene.environment = studioEnv.texture;

    const ambientLight = new THREE.AmbientLight(0x0f2042, enginePreset === 'ue5' ? 1.5 : 1.1);
    scene.add(ambientLight);

    const keyLight = new THREE.DirectionalLight(0xe0f2fe, enginePreset === 'ue5' ? 3.8 : 3.0);
    keyLight.position.set(6, 9, 5);
    keyLight.castShadow = qualityTier !== 'Low';
    scene.add(keyLight);

    const cyanRim = new THREE.PointLight(0x22d3ee, enginePreset === 'ue5' ? 5.2 : 3.8, 18);
    cyanRim.position.set(-5, 3, 3);
    scene.add(cyanRim);

    const violetRim = new THREE.PointLight(0x8b5cf6, enginePreset === 'ue5' ? 3.6 : 2.5, 15);
    violetRim.position.set(0, -2, -5);
    scene.add(violetRim);

    // 4. Procedural High-Fidelity PBR Textures
    const brushedMaps = createBrushedMetalTextures();
    const machinedRadial = createMachinedRadialTexture(512);
    const copperMaps = createCopperWindingTexture(512, 512);
    const carbonFiberMap = createCarbonFiberTexture(256);

    // 5. Circular Reflective Precision Platform (UE5 Mirror / Blender Grid)
    const platformGroup = new THREE.Group();

    // Base disc with brushed finish
    const platformGeo = new THREE.CylinderGeometry(3.8, 3.9, 0.14, 64);
    const platformMat = new THREE.MeshStandardMaterial({
      color: enginePreset === 'ue5' ? 0x030a18 : 0x061126,
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
      // Realistic PBR (Unreal Engine 5 / Blender)
      return new THREE.MeshPhysicalMaterial({
        ...baseConfig,
        clearcoat: enginePreset === 'ue5' ? 0.4 : 0.2,
        clearcoatRoughness: 0.1
      });
    };

    const createPrecisionAssembly = (faultMode: boolean, bypassCooled: boolean) => {
      const group = new THREE.Group();
      const offset = variant === 'model_editor' ? (explodedOffset > 0 ? explodedOffset : 0.42) : 0;

      // A. Ground Steel Rotor Shaft
      const shaftGeo = new THREE.CylinderGeometry(0.22, 0.22, 3.4, 32);
      const shaftMat = getMaterial({
        color: 0x94a3b8,
        metalness: 0.95,
        roughness: 0.16,
        map: machinedRadial,
        bumpScale: 0.05
      });
      const shaft = new THREE.Mesh(shaftGeo, shaftMat);
      shaft.rotation.z = Math.PI / 2;
      shaft.position.y = 1.2;
      shaft.castShadow = true;
      group.add(shaft);

      // Shaft keyway slot
      const keywayGeo = new THREE.BoxGeometry(0.06, 0.04, 0.8);
      const keywayMat = getMaterial({ color: 0x334155, metalness: 0.9, roughness: 0.4 });
      const keyway = new THREE.Mesh(keywayGeo, keywayMat);
      keyway.position.set(1.2, 1.33, 0);
      group.add(keyway);

      // B. Stator Windings (with dynamic thermal overheating & repair cooling)
      const windingGeo = new THREE.CylinderGeometry(0.72, 0.72, 1.25, 36);
      const isOverheated = faultMode && !isRepaired;
      const windingMat = getMaterial({
        color: isOverheated ? 0xff3b30 : (isRepaired ? 0x22c55e : 0xd97706),
        metalness: 0.85,
        roughness: 0.28,
        map: copperMaps.colorMap,
        normalMap: copperMaps.normalMap,
        normalScale: new THREE.Vector2(1.8, 1.8),
        emissive: isOverheated ? 0xef4444 : (isRepaired ? 0x10b981 : 0x000000),
        emissiveIntensity: isOverheated ? (enginePreset === 'ue5' ? 2.2 : 1.4) : (isRepaired ? 0.35 : 0.0)
      }, isOverheated ? 0xef4444 : 0x06b6d4);
      const winding = new THREE.Mesh(windingGeo, windingMat);
      winding.rotation.z = Math.PI / 2;
      winding.position.set(0, 1.2, 0);
      group.add(winding);

      // C. Laminated Steel Stator Teeth & Core
      const teethCount = 12;
      for (let i = 0; i < teethCount; i++) {
        const theta = (i / teethCount) * Math.PI * 2;
        const toothGeo = new THREE.BoxGeometry(1.26, 0.08, 0.12);
        const toothMat = getMaterial({
          color: 0x475569,
          metalness: 0.9,
          roughness: 0.25
        });
        const tooth = new THREE.Mesh(toothGeo, toothMat);
        tooth.position.set(0, 1.2 + Math.cos(theta) * 0.75, Math.sin(theta) * 0.75);
        tooth.rotation.x = -theta;
        group.add(tooth);
      }

      // D. Outer Casing
      const casingGeo = new THREE.CylinderGeometry(0.98, 0.98, 1.45, 36);
      const casingMat = getMaterial({
        color: bypassCooled || isRepaired ? 0x0284c7 : 0x1e3a8a,
        metalness: 0.88,
        roughness: 0.18,
        map: brushedMaps.colorMap,
        normalMap: brushedMaps.normalMap,
        transparent: true,
        opacity: variant === 'model_editor' || renderMode === 'ghost' ? 0.6 : 0.88
      });
      const casing = new THREE.Mesh(casingGeo, casingMat);
      casing.rotation.z = Math.PI / 2;
      casing.position.set(0, 1.2, 0);
      group.add(casing);

      // Annular Cooling Fins on outer casing (Dynamically expanded if repaired!)
      const finCount = isRepaired ? 11 : 7;
      for (let f = 0; f < finCount; f++) {
        const finX = -0.65 + (f / (finCount - 1)) * 1.3;
        const finGeo = new THREE.CylinderGeometry(isRepaired ? 1.18 : 1.12, isRepaired ? 1.18 : 1.12, 0.04, 36);
        const finMat = getMaterial({
          color: isRepaired ? 0x38bdf8 : 0x334155,
          metalness: 0.92,
          roughness: 0.25
        });
        const fin = new THREE.Mesh(finGeo, finMat);
        fin.rotation.z = Math.PI / 2;
        fin.position.set(finX, 1.2, 0);
        group.add(fin);
      }

      // Bypass Liquid Cooling Jacket Tubes (Mounted dynamically during 3D physical repair!)
      if (bypassCooled || isRepaired || isRepairing) {
        const scaleVal = isRepairing ? Math.max(0.2, repairProgress) : 1.0;
        const tubeCurve = new THREE.CatmullRomCurve3([
          new THREE.Vector3(-0.8, 2.1, 0.6),
          new THREE.Vector3(0.0, 2.2, 0.7),
          new THREE.Vector3(0.8, 2.1, 0.6),
          new THREE.Vector3(0.9, 1.4, 0.8),
          new THREE.Vector3(0.0, 0.4, 0.8),
          new THREE.Vector3(-0.9, 1.4, 0.8),
        ], true);
        const tubeGeo = new THREE.TubeGeometry(tubeCurve, 32, 0.065 * scaleVal, 12, true);
        const tubeMat = getMaterial({
          color: 0x06b6d4,
          metalness: 0.95,
          roughness: 0.15,
          emissive: 0x0891b2,
          emissiveIntensity: enginePreset === 'ue5' ? 1.2 : 0.7
        }, 0x06b6d4);
        const tubeMesh = new THREE.Mesh(tubeGeo, tubeMat);
        group.add(tubeMesh);
      }

      // E. Endbells
      const endbellGeo = new THREE.CylinderGeometry(0.98, 0.98, 0.28, 32);
      const endbellMat = getMaterial({
        color: 0x1e293b,
        metalness: 0.9,
        roughness: 0.2,
        map: carbonFiberMap
      });
      const endbellL = new THREE.Mesh(endbellGeo, endbellMat);
      endbellL.rotation.z = Math.PI / 2;
      endbellL.position.set(-0.88 - offset * 1.6, 1.2, 0);
      group.add(endbellL);

      const endbellR = endbellL.clone();
      endbellR.position.set(0.88 + offset * 1.6, 1.2, 0);
      group.add(endbellR);

      return { group, windingMat };
    };

    // 7. Mount Core Assembly & User-Created Scratch 3D Parts
    const modelGroup = new THREE.Group();
    let windingRef: any = null;

    if (variant === 'landing') {
      const portalRadius = 3.2;
      const portalTube = 0.038;
      const portalGeo = new THREE.TorusGeometry(portalRadius, portalTube, 32, 100);
      const portalMat = new THREE.MeshBasicMaterial({ color: 0x38bdf8, transparent: true, opacity: 0.95 });
      const portalMesh = new THREE.Mesh(portalGeo, portalMat);
      portalMesh.position.set(0, 2.2, -0.5);
      scene.add(portalMesh);

      const assembly = createPrecisionAssembly(false, false);
      assembly.group.scale.set(1.15, 1.15, 1.15);
      assembly.group.position.y = 0.4;
      modelGroup.add(assembly.group);
      windingRef = assembly.windingMat;
    } else if (variant === 'compare') {
      const leftAssembly = createPrecisionAssembly(true, false);
      leftAssembly.group.position.set(-1.8, 0, 0);
      modelGroup.add(leftAssembly.group);

      const rightAssembly = createPrecisionAssembly(false, true);
      rightAssembly.group.position.set(1.8, 0, 0);
      modelGroup.add(rightAssembly.group);
      windingRef = leftAssembly.windingMat;
    } else {
      const assembly = createPrecisionAssembly(isFaulty, isRepaired);
      modelGroup.add(assembly.group);
      windingRef = assembly.windingMat;
    }

    // Mount user scratch parts created live from scratch
    scratchParts.forEach(p => {
      let geo: THREE.BufferGeometry;
      if (p.shape === 'box') geo = new THREE.BoxGeometry(0.8, 0.6, 0.8);
      else if (p.shape === 'flange') geo = new THREE.CylinderGeometry(0.6, 0.6, 0.15, 24);
      else if (p.shape === 'conduit') geo = new THREE.TorusGeometry(0.5, 0.08, 16, 32);
      else if (p.shape === 'sensor') geo = new THREE.SphereGeometry(0.25, 24, 24);
      else geo = new THREE.CylinderGeometry(0.4, 0.4, 0.9, 24);

      const mat = getMaterial({
        color: new THREE.Color(p.color),
        metalness: 0.85,
        roughness: 0.25
      }, new THREE.Color(p.color).getHex());

      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(...p.position);
      mesh.rotation.set(...p.rotation);
      mesh.scale.set(...p.scale);
      modelGroup.add(mesh);
    });

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

    // Blender Shading Pie Menu Hotkey (Z key)
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.code === 'KeyZ') {
        setShowShadingMenu(prev => !prev);
      }
    };
    window.addEventListener('keydown', onKeyDown);

    // 11. Animation Loop with UE5 Lumen Bloom & Thermal Pulsing
    let animationFrameId: number;
    let lastTime = performance.now();
    let frameCount = 0;

    const animate = () => {
      animationFrameId = requestAnimationFrame(animate);
      const now = performance.now();
      frameCount++;
      if (now - lastTime >= 1000) {
        setFps(Math.round((frameCount * 1000) / (now - lastTime)));
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
      window.removeEventListener('keydown', onKeyDown);
      container.removeEventListener('mousedown', onMouseDown);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
      container.removeEventListener('wheel', onWheel);
      if (renderer.domElement && container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
      studioEnv.dispose();
      renderer.dispose();
    };
  }, [variant, qualityTier, thermalHotspot, explodedOffset, interactive, isRotating, renderMode, enginePreset, localRepairState, repairProgress, scratchParts]);

  return (
    <div className={`relative w-full h-full overflow-hidden select-none ${className}`}>
      <div ref={mountRef} className="w-full h-full cursor-grab active:cursor-grabbing" />

      {/* Floating Canvas Badges & Engine Status */}
      <div className="absolute top-3 left-3 flex items-center gap-2 pointer-events-none z-10 font-mono text-[10px]">
        <span className="px-2.5 py-0.5 rounded bg-[#0A1422]/90 border border-[#2F7BFF]/40 text-[#22D3EE] font-bold shadow-[0_0_12px_rgba(47,123,255,0.3)]">
          {enginePreset === 'ue5' ? 'UNREAL ENGINE 5 LUMEN' : enginePreset === 'blender' ? 'BLENDER EEVEE' : 'THREE.JS'}
        </span>
        <span className="px-2 py-0.5 rounded bg-slate-900/80 border border-slate-700 text-slate-300 num">
          {fps} FPS
        </span>
        <span className={`px-2 py-0.5 rounded uppercase font-semibold ${
          renderMode === 'ghost' ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40' :
          renderMode === 'wireframe' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/40' :
          'bg-emerald-500/20 text-emerald-300 border border-emerald-500/40'
        }`}>
          {renderMode} MODE
        </span>
        {localRepairState === 'repaired' && (
          <span className="px-2 py-0.5 rounded bg-emerald-500/20 border border-emerald-500/50 text-emerald-300 flex items-center gap-1 font-semibold">
            <span>✓</span>
            <span>3D REPAIRED</span>
          </span>
        )}
      </div>

      {/* VIEWPORT CONTROLS TOOLBAR: Shading Modes (Ghost, Wireframe, Realistic) & Engine Preset (Blender, UE5) */}
      {interactive && (
        <div className="absolute top-3 right-3 flex items-center gap-2 z-20 pointer-events-auto">
          {/* Shading Mode Switcher: Ghost / Wireframe / Realistic */}
          <div className="p-1 rounded-lg bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] flex items-center gap-1 text-xs font-mono">
            {(['ghost', 'wireframe', 'realistic'] as const).map(mode => (
              <button
                key={mode}
                onClick={() => setRenderMode(mode)}
                className={`px-2.5 py-1 rounded-[6px] capitalize transition-all ${
                  renderMode === mode
                    ? 'bg-[#2F7BFF] text-white font-bold shadow-[0_0_12px_rgba(47,123,255,0.4)]'
                    : 'text-[#7F93AF] hover:text-white hover:bg-white/5'
                }`}
              >
                {mode === 'ghost' ? '👻 Ghost' : mode === 'wireframe' ? '🕸 Wireframe' : '💎 Realistic'}
              </button>
            ))}
          </div>

          {/* Engine Visual Preset: Unreal Engine 5 vs Blender */}
          <div className="p-1 rounded-lg bg-[#0A1422]/90 backdrop-blur-md border border-[#22395A] flex items-center gap-1 text-xs font-mono">
            <button
              onClick={() => {
                setEnginePreset('ue5');
                setShowBridgeModal('ue5');
              }}
              className={`px-2.5 py-1 rounded-[6px] transition-all flex items-center gap-1.5 ${
                enginePreset === 'ue5'
                  ? 'bg-gradient-to-r from-blue-600 to-cyan-500 text-white font-bold shadow-[0_0_12px_rgba(34,211,238,0.4)]'
                  : 'text-[#7F93AF] hover:text-white'
              }`}
              title="Unreal Engine 5 Lumen Lighting & Bridge"
            >
              <span>UE5</span>
            </button>
            <button
              onClick={() => {
                setEnginePreset('blender');
                setShowBridgeModal('blender');
              }}
              className={`px-2.5 py-1 rounded-[6px] transition-all flex items-center gap-1.5 ${
                enginePreset === 'blender'
                  ? 'bg-gradient-to-r from-amber-600 to-orange-500 text-white font-bold shadow-[0_0_12px_rgba(245,158,11,0.4)]'
                  : 'text-[#7F93AF] hover:text-white'
              }`}
              title="Blender EEVEE / Cycles Pipeline & Bridge"
            >
              <span>Blender</span>
            </button>
          </div>

          {/* Create 3D Part From Scratch Button */}
          <button
            onClick={() => setShowScratchCreator(prev => !prev)}
            className="px-3 py-1.5 rounded-lg bg-gradient-to-r from-[#2F7BFF] to-[#8B5CF6] text-white text-xs font-mono font-semibold shadow-[0_0_16px_rgba(139,92,246,0.35)] hover:scale-105 transition-all flex items-center gap-1.5"
            title="Create a new 3D part from scratch"
          >
            <span>+</span>
            <span>Create 3D Part</span>
          </button>

          {/* Auto-Rotate Toggle */}
          <button
            onClick={() => setIsRotating(!isRotating)}
            className={`p-1.5 rounded-lg border backdrop-blur text-xs transition-colors ${
              isRotating
                ? 'bg-blue-600/30 text-cyan-300 border-blue-500/40'
                : 'bg-[#0A1422]/90 border-[#22395A] text-slate-400 hover:text-white'
            }`}
            title="Toggle rotation"
          >
            <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
              <path d="M3 3v5h5" />
              <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
              <path d="M16 21h5v-5" />
            </svg>
          </button>
        </div>
      )}

      {/* CREATE 3D PART FROM SCRATCH POPUP TOOLBAR */}
      {showScratchCreator && (
        <div className="absolute top-16 right-3 z-30 w-72 rounded-xl bg-[#0A1422]/95 backdrop-blur-xl border border-[#2F7BFF]/50 p-4 shadow-[0_0_35px_rgba(47,123,255,0.3)] space-y-3 font-mono text-xs">
          <div className="flex items-center justify-between border-b border-[#22395A] pb-2">
            <span className="font-bold text-white">Create 3D Part (Scratch)</span>
            <button onClick={() => setShowScratchCreator(false)} className="text-[#7F93AF] hover:text-white">✕</button>
          </div>
          <div className="text-[11px] text-[#7F93AF]">
            Choose a shape to dynamically instantiate onto the 3D precision canvas:
          </div>
          <div className="grid grid-cols-2 gap-2">
            {[
              { id: 'cylinder', label: 'Cylinder / Core', icon: '🥫' },
              { id: 'box', label: 'Chassis / Enclosure', icon: '📦' },
              { id: 'flange', label: 'Mounting Flange', icon: '💿' },
              { id: 'conduit', label: 'Cooling Conduit', icon: '➰' },
              { id: 'sensor', label: 'Sensor Probe', icon: '📍' }
            ].map(item => (
              <button
                key={item.id}
                onClick={() => handleAddScratchPart(item.id as ScratchPart3D['shape'])}
                className="p-2.5 rounded-lg bg-[#0E1B2E] hover:bg-[#13253D] border border-[#22395A] hover:border-[#22D3EE] text-left transition-all group flex items-center gap-2"
              >
                <span className="text-sm">{item.icon}</span>
                <span className="text-[11px] text-[#D6E2F3] group-hover:text-[#22D3EE] font-medium">{item.label}</span>
              </button>
            ))}
          </div>
          {scratchParts.length > 0 && (
            <div className="pt-2 border-t border-[#22395A] flex items-center justify-between text-[10px] text-[#7F93AF]">
              <span>{scratchParts.length} custom parts placed</span>
              <button onClick={() => setScratchParts([])} className="text-red-400 hover:underline">Clear all</button>
            </div>
          )}
        </div>
      )}

      {/* BLENDER / UNREAL ENGINE 5 INTEGRATION MODAL */}
      {showBridgeModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/75 backdrop-blur-md animate-fade-in">
          <div className="w-full max-w-lg rounded-2xl bg-[#0A1422] border border-[#2F7BFF]/50 p-6 space-y-4 shadow-[0_0_50px_rgba(47,123,255,0.3)] font-mono text-xs">
            <div className="flex items-center justify-between border-b border-[#22395A] pb-3">
              <div className="flex items-center gap-2">
                <span className="w-2.5 h-2.5 rounded-full bg-[#22C55E]" />
                <span className="text-sm font-bold text-white uppercase">
                  {showBridgeModal === 'ue5' ? 'Unreal Engine 5 Lumen Pipeline' : 'Blender 4.x Interchange'}
                </span>
              </div>
              <button onClick={() => setShowBridgeModal(null)} className="text-[#7F93AF] hover:text-white">✕</button>
            </div>

            {showBridgeModal === 'ue5' ? (
              <div className="space-y-3 text-[#D6E2F3]">
                <p>
                  Unreal Engine 5 real-time Lumen Global Illumination & ACES Filmic tonemapping active. Export physical assets directly into UE5 Datasmith or Nanite USD geometry.
                </p>
                <div className="p-3 rounded-lg bg-[#050B14] border border-[#22395A] space-y-1.5 text-[11px]">
                  <div className="flex justify-between"><span>Illumination:</span><strong className="text-cyan-300">Lumen Real-Time GI</strong></div>
                  <div className="flex justify-between"><span>Reflection:</span><strong className="text-cyan-300">Screen-Space + PMREM Probe</strong></div>
                  <div className="flex justify-between"><span>Interchange:</span><strong className="text-cyan-300">USD / Datasmith AP242</strong></div>
                </div>
              </div>
            ) : (
              <div className="space-y-3 text-[#D6E2F3]">
                <p>
                  Blender EEVEE/Cycles geometry pipeline active. Edit vertices, apply subdivision/bevel modifiers, and synchronize with local Blender instances via glTF 2.0.
                </p>
                <div className="p-3 rounded-lg bg-[#050B14] border border-[#22395A] space-y-1.5 text-[11px]">
                  <div className="flex justify-between"><span>Shading Hotkey:</span><strong className="text-amber-400">Press 'Z' for Pie Menu</strong></div>
                  <div className="flex justify-between"><span>CAD Transform:</span><strong className="text-amber-400">G (Grab), R (Rotate), S (Scale)</strong></div>
                  <div className="flex justify-between"><span>Format:</span><strong className="text-amber-400">.blend / glTF 2.0 PBR</strong></div>
                </div>
              </div>
            )}

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[#22395A]">
              <button
                onClick={() => setShowBridgeModal(null)}
                className="px-4 py-1.5 rounded-lg text-white bg-[#2F7BFF] hover:bg-[#3B82F6] font-semibold"
              >
                Apply Engine Profile
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 3D Physical Repair Execution Overlay (strictly in failure mode) */}
      {localRepairState === 'fault' && variant === 'failure' && (
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-20 pointer-events-auto animate-bounce">
          <button
            onClick={handleTriggerRepair}
            className="px-5 py-2.5 rounded-full font-mono text-xs font-bold text-white bg-gradient-to-r from-red-600 via-amber-500 to-emerald-500 hover:scale-105 shadow-[0_0_25px_rgba(239,68,68,0.7)] transition-all flex items-center gap-2 border border-white/30"
          >
            <span>⚡</span>
            <span>Execute 3D Physical Repair (Mount Bypass Cooling Jacket)</span>
            <span>→</span>
          </button>
        </div>
      )}

      {localRepairState === 'repairing' && variant === 'failure' && (
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-20 pointer-events-auto p-2.5 rounded-xl bg-slate-900/95 border border-cyan-500/80 shadow-[0_0_30px_rgba(34,211,238,0.5)] flex items-center gap-3">
          <div className="w-4 h-4 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin" />
          <span className="text-xs font-mono font-bold text-cyan-300">
            Morphing 3D geometry & welding bypass cooling jacket... {Math.round(repairProgress * 100)}%
          </span>
        </div>
      )}
    </div>
  );
};
