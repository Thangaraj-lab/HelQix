import * as THREE from 'three';

/**
 * High-Fidelity Procedural PBR Textures for Reality Studio 3D Canvas
 * Generates photorealistic normal, roughness, and bump maps for engineering materials:
 * - Brushed Aerospace Aluminum (6061-T6)
 * - Concentric Lathe-Turned Steel End-Caps
 * - Enameled Copper Magnet Wire Stator Windings
 * - Twill Weave Carbon Fiber Composite
 * - Infrared Thermal FEA Stress/Heat Gradient
 */

// 1. Brushed Aluminum Anisotropic Normal & Roughness Map
export function createBrushedMetalTextures(): {
  colorMap: THREE.CanvasTexture;
  normalMap: THREE.CanvasTexture;
  roughnessMap: THREE.CanvasTexture;
} {
  const size = 512;
  const canvasColor = document.createElement('canvas');
  canvasColor.width = size;
  canvasColor.height = size;
  const ctxColor = canvasColor.getContext('2d')!;

  const canvasNormal = document.createElement('canvas');
  canvasNormal.width = size;
  canvasNormal.height = size;
  const ctxNormal = canvasNormal.getContext('2d')!;

  // Base metallic fill
  ctxColor.fillStyle = '#9cb3c9';
  ctxColor.fillRect(0, 0, size, size);

  // Brushed streak noise
  for (let y = 0; y < size; y++) {
    const val = 150 + Math.floor((Math.random() - 0.5) * 45);
    ctxColor.fillStyle = `rgb(${val}, ${val + 5}, ${val + 12})`;
    ctxColor.fillRect(0, y, size, 1);
  }

  // Subtle cross-scratches
  ctxColor.fillStyle = 'rgba(255, 255, 255, 0.08)';
  for (let i = 0; i < 40; i++) {
    const y = Math.random() * size;
    ctxColor.fillRect(0, y, size, 1);
  }

  // Normal map generation (subtle horizontal grooves)
  const imgData = ctxNormal.createImageData(size, size);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const idx = (y * size + x) * 4;
      // Perturb ny slightly based on row variation
      const ny = 128 + Math.floor((Math.random() - 0.5) * 20);
      imgData.data[idx] = 128;     // nx (neutral)
      imgData.data[idx + 1] = ny;  // ny (brushed groove)
      imgData.data[idx + 2] = 255; // nz (up)
      imgData.data[idx + 3] = 255;
    }
  }
  ctxNormal.putImageData(imgData, 0, 0);

  const colorMap = new THREE.CanvasTexture(canvasColor);
  colorMap.wrapS = THREE.RepeatWrapping;
  colorMap.wrapT = THREE.RepeatWrapping;

  const normalMap = new THREE.CanvasTexture(canvasNormal);
  normalMap.wrapS = THREE.RepeatWrapping;
  normalMap.wrapT = THREE.RepeatWrapping;

  const roughnessMap = colorMap.clone();

  return { colorMap, normalMap, roughnessMap };
}

// 2. Concentric Lathe-Turned Machined Steel (for rotor shafts and end-plates)
export function createMachinedRadialTexture(size = 512): THREE.CanvasTexture {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  const center = size / 2;
  ctx.fillStyle = '#64748b';
  ctx.fillRect(0, 0, size, size);

  // Draw fine concentric rings
  for (let r = 1; r < center; r += 1.5) {
    const alpha = 0.05 + Math.random() * 0.12;
    const isBright = Math.random() > 0.5;
    ctx.strokeStyle = isBright ? `rgba(255, 255, 255, ${alpha})` : `rgba(0, 0, 0, ${alpha})`;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(center, center, r, 0, Math.PI * 2);
    ctx.stroke();
  }

  const texture = new THREE.CanvasTexture(canvas);
  return texture;
}

// 3. Enameled Copper Magnet Wire Stator Windings Texture
export function createCopperWindingTexture(width = 512, height = 512): {
  colorMap: THREE.CanvasTexture;
  normalMap: THREE.CanvasTexture;
} {
  const canvasColor = document.createElement('canvas');
  canvasColor.width = width;
  canvasColor.height = height;
  const ctxColor = canvasColor.getContext('2d')!;

  const canvasNormal = document.createElement('canvas');
  canvasNormal.width = width;
  canvasNormal.height = height;
  const ctxNormal = canvasNormal.getContext('2d')!;

  // Dark amber copper shadow base
  ctxColor.fillStyle = '#451a03';
  ctxColor.fillRect(0, 0, width, height);

  const wireHeight = 8;
  const normalData = ctxNormal.createImageData(width, height);

  for (let y = 0; y < height; y += wireHeight) {
    // Rich copper gradient for each individual wire turn
    const grad = ctxColor.createLinearGradient(0, y, 0, y + wireHeight);
    grad.addColorStop(0, '#78350f');   // crevice shadow
    grad.addColorStop(0.2, '#d97706'); // warm copper highlight
    grad.addColorStop(0.5, '#f59e0b'); // wire peak specular
    grad.addColorStop(0.8, '#b45309'); // mid-tone
    grad.addColorStop(1, '#451a03');   // crevice shadow

    ctxColor.fillStyle = grad;
    ctxColor.fillRect(0, y, width, wireHeight - 1);

    // Normal map cylindrical curve
    for (let wy = 0; wy < wireHeight; wy++) {
      const curY = y + wy;
      if (curY >= height) break;
      const t = wy / (wireHeight - 1); // 0 to 1
      const ny = Math.round(128 + Math.cos(t * Math.PI) * 110);
      const nz = Math.round(Math.sin(t * Math.PI) * 220 + 35);

      for (let x = 0; x < width; x++) {
        const idx = (curY * width + x) * 4;
        normalData.data[idx] = 128;      // nx
        normalData.data[idx + 1] = ny;   // ny (rounded wire curvature)
        normalData.data[idx + 2] = nz;   // nz
        normalData.data[idx + 3] = 255;
      }
    }
  }

  ctxNormal.putImageData(normalData, 0, 0);

  const colorMap = new THREE.CanvasTexture(canvasColor);
  colorMap.wrapS = THREE.RepeatWrapping;
  colorMap.wrapT = THREE.RepeatWrapping;
  colorMap.repeat.set(2, 4);

  const normalMap = new THREE.CanvasTexture(canvasNormal);
  normalMap.wrapS = THREE.RepeatWrapping;
  normalMap.wrapT = THREE.RepeatWrapping;
  normalMap.repeat.set(2, 4);

  return { colorMap, normalMap };
}

// 4. Carbon Fiber Twill Weave Normal Map
export function createCarbonFiberTexture(size = 256): THREE.CanvasTexture {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  ctx.fillStyle = '#0f172a';
  ctx.fillRect(0, 0, size, size);

  const step = 8;
  for (let y = 0; y < size; y += step) {
    for (let x = 0; x < size; x += step) {
      const isShifted = ((x / step + y / step) % 4) < 2;
      ctx.fillStyle = isShifted ? '#1e293b' : '#090d16';
      ctx.fillRect(x, y, step, step);
    }
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(8, 8);
  return texture;
}

// 5. Infrared Thermal FEA Hotspot Gradient (Cold Blue -> Cyan -> Green -> Yellow -> Red -> Incandescent White)
export function createThermalGradientTexture(): THREE.CanvasTexture {
  const canvas = document.createElement('canvas');
  canvas.width = 256;
  canvas.height = 16;
  const ctx = canvas.getContext('2d')!;

  const grad = ctx.createLinearGradient(0, 0, 256, 0);
  grad.addColorStop(0.0, '#1e3a8a'); // 25 °C Ambient
  grad.addColorStop(0.3, '#06b6d4'); // 60 °C
  grad.addColorStop(0.55, '#22c55e'); // 85 °C
  grad.addColorStop(0.75, '#eab308'); // 105 °C Caution
  grad.addColorStop(0.9, '#ef4444');  // 120 °C Trip Limit
  grad.addColorStop(1.0, '#ffffff');  // 135 °C Melting/Flash point

  ctx.fillStyle = grad;
  ctx.fillRect(0, 0, 256, 16);

  const texture = new THREE.CanvasTexture(canvas);
  return texture;
}

// 6. Realistic Procedural Environment Lighting Probe
export function createStudioEnvironment(renderer: THREE.WebGLRenderer): THREE.WebGLRenderTarget {
  const pmremGenerator = new THREE.PMREMGenerator(renderer);
  pmremGenerator.compileEquirectangularShader();

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x050b18);

  // Overhead Key Softbox
  const keyLight = new THREE.DirectionalLight(0xe0f2fe, 3.5);
  keyLight.position.set(5, 10, 5);
  scene.add(keyLight);

  // Blue fill light
  const fillLight = new THREE.DirectionalLight(0x1d4ed8, 2.0);
  fillLight.position.set(-6, 2, -4);
  scene.add(fillLight);

  // Violet rim light
  const rimLight = new THREE.DirectionalLight(0x8b5cf6, 2.2);
  rimLight.position.set(0, -6, 6);
  scene.add(rimLight);

  const envMap = pmremGenerator.fromScene(scene, 0.04);
  pmremGenerator.dispose();

  return envMap;
}
