// EXAMPLE scene content (plan rule 2: domain parts live only in src/mock and are labelled Example).
// The Studio canvas draws this fixed demonstration assembly only when it has no model of the
// project's own to show, and then labels the canvas "Example scene".
import * as THREE from 'three';

export interface ExampleAssemblyContext {
  getMaterial: (cfg: THREE.MeshStandardMaterialParameters, ghostColor?: number) => THREE.Material;
  textures: {
    machinedRadial: THREE.Texture;
    copperMaps: { colorMap: THREE.Texture; normalMap: THREE.Texture };
    brushedMaps: { colorMap: THREE.Texture; normalMap: THREE.Texture };
    carbonFiberMap: THREE.Texture;
  };
  variant: string;
  renderMode: 'realistic' | 'ghost' | 'wireframe';
  explodedOffset: number;
  isRepaired: boolean;
  isRepairing: boolean;
  repairProgress: number;
}

export function createExampleAssembly(ctx: ExampleAssemblyContext, faultMode: boolean, bypassCooled: boolean) {
  const { getMaterial, variant, renderMode, explodedOffset, isRepaired, isRepairing, repairProgress } = ctx;
  const { machinedRadial, copperMaps, brushedMaps, carbonFiberMap } = ctx.textures;
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
    emissiveIntensity: isOverheated ? (2.2) : (isRepaired ? 0.35 : 0.0)
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
      emissiveIntensity: 1.2
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
}
