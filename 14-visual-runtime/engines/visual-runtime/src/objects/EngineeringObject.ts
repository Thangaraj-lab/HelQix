/**
 * HELQIX Visual Runtime — EngineeringObject
 * Layer 14: 14-visual-runtime
 *
 * High-performance container wrapping Three.js Object3D for engineering components.
 * Tracks metadata, hierarchical ports, visual state, and telemetry tags.
 */

import * as THREE from 'three';
import { ComponentVisualAttributes, SpatialSnapshot, HealthStatus } from '../state/VisualState';

export interface EngineeringObjectConfig {
  id: string;
  name: string;
  subsystem: string;
  category: string;
  /** Ownership of the supplied hierarchy and its geometry/material resources transfers to this object. */
  rootMesh?: THREE.Object3D;
  initialAttributes?: Partial<ComponentVisualAttributes>;
}

export class EngineeringObject {
  public readonly id: string;
  public readonly name: string;
  public readonly subsystem: string;
  public readonly category: string;
  public readonly root3D: THREE.Group;

  private _attributes: ComponentVisualAttributes;
  private _originalMaterials = new Map<THREE.Mesh, THREE.Material[]>();
  private _visualMaterials = new Map<THREE.Mesh, THREE.Material[]>();
  private _thermalRatio: number | null = null;

  constructor(config: EngineeringObjectConfig) {
    this.id = config.id;
    this.name = config.name;
    this.subsystem = config.subsystem;
    this.category = config.category;

    this.root3D = new THREE.Group();
    this.root3D.name = `EngineeringObject:${this.id}`;
    this.root3D.userData = {
      helqixId: this.id,
      name: this.name,
      subsystem: this.subsystem,
      isEngineeringObject: true
    };

    if (config.rootMesh) {
      this.root3D.add(config.rootMesh);
      this._cacheOriginalMaterials(config.rootMesh);
    } else {
      // Default procedural engineering bounding representation if no mesh provided
      const defaultMesh = this._createProceduralPlaceholder();
      this.root3D.add(defaultMesh);
      this._cacheOriginalMaterials(defaultMesh);
    }

    this._attributes = {
      visible: true,
      wireframe: false,
      highlighted: false,
      selected: false,
      health: 'unknown',
      opacity: 1.0,
      ...config.initialAttributes
    };
    this._thermalRatio = config.initialAttributes?.thermal?.normalizedHeatRatio ?? null;

    this.applyAttributes(this._attributes);
  }

  public get attributes(): Readonly<ComponentVisualAttributes> {
    return this._attributes;
  }

  public updateSpatial(spatial: SpatialSnapshot): void {
    this.root3D.position.set(spatial.position.x, spatial.position.y, spatial.position.z);
    this.root3D.quaternion.set(spatial.rotation.x, spatial.rotation.y, spatial.rotation.z, spatial.rotation.w);
    this.root3D.scale.set(spatial.scale.x, spatial.scale.y, spatial.scale.z);
  }

  public applyAttributes(attrs: Partial<ComponentVisualAttributes>): void {
    this._attributes = { ...this._attributes, ...attrs };
    if (attrs.thermal) this._thermalRatio = attrs.thermal.normalizedHeatRatio;
    this.root3D.visible = this._attributes.visible;
    for (const [mesh, originals] of this._originalMaterials) {
      const visualMaterials = this._visualMaterials.get(mesh);
      if (!visualMaterials) continue;
      const currentMaterials = visualMaterials.map((visual, index) => {
        const original = originals[index];
        const colorMaterial = visual as THREE.Material & { color?: THREE.Color; emissive?: THREE.Color; emissiveIntensity?: number };
        const baseColorMaterial = original as THREE.Material & { color?: THREE.Color; emissive?: THREE.Color; emissiveIntensity?: number };
        if (colorMaterial.color && baseColorMaterial.color) {
          colorMaterial.color.copy(baseColorMaterial.color);
          const tint = this._visualTint();
          if (tint) colorMaterial.color.lerp(tint.color, tint.strength);
        }
        if (colorMaterial.emissive) {
          if (this._attributes.health === 'failed' || this._attributes.health === 'critical') {
            colorMaterial.emissive.set(this._attributes.health === 'failed' ? 0x8b1e1e : 0x9e4a24);
            colorMaterial.emissiveIntensity = this._attributes.health === 'failed' ? 0.5 : 0.3;
          } else if (this._attributes.health === 'warning') {
            colorMaterial.emissive.set(0x8a661d);
            colorMaterial.emissiveIntensity = 0.2;
          } else if (this._attributes.selected) {
            colorMaterial.emissive.set(0x789b35);
            colorMaterial.emissiveIntensity = 0.3;
          } else if (baseColorMaterial.emissive) {
            colorMaterial.emissive.copy(baseColorMaterial.emissive);
            colorMaterial.emissiveIntensity = baseColorMaterial.emissiveIntensity ?? 1;
          }
        }
        if ('wireframe' in visual) (visual as THREE.Material & { wireframe: boolean }).wireframe = this._attributes.wireframe;
        visual.transparent = original.transparent || this._attributes.opacity < 1;
        visual.opacity = original.opacity * this._attributes.opacity;
        visual.depthWrite = this._attributes.opacity < 1 ? false : original.depthWrite;
        return visual;
      });
      mesh.material = Array.isArray(mesh.material) ? currentMaterials : currentMaterials[0];
    }
  }

  /** Apply or clear the temperature-to-color overlay without changing the reported health state. */
  public setThermalTint(heatRatio: number | null): void {
    if (heatRatio !== null && !Number.isFinite(heatRatio)) {
      throw new TypeError('Thermal ratio must be finite or null.');
    }
    this._thermalRatio = heatRatio === null ? null : Math.max(0, Math.min(1.5, heatRatio));
    this.applyAttributes({});
  }

  private _visualTint(): { color: THREE.Color; strength: number } | null {
    if (this._attributes.colorOverride) {
      try { return { color: new THREE.Color(this._attributes.colorOverride), strength: 1 }; }
      catch { /* Ignore malformed presentation hints and keep the source material color. */ }
    }
    if (this._attributes.health === 'failed' || this._attributes.health === 'critical') {
      return { color: new THREE.Color(this._attributes.health === 'failed' ? 0xef4444 : 0xf26a38), strength: 0.72 };
    }
    if (this._attributes.health === 'warning') return { color: new THREE.Color(0xfbbf24), strength: 0.55 };
    if (this._thermalRatio !== null) {
      const heat = Math.max(0, Math.min(1, (this._thermalRatio - 0.55) / 0.45));
      if (heat > 0) return { color: new THREE.Color(0x34d399).lerp(new THREE.Color(0xef4444), heat), strength: 0.58 * heat };
    }
    if (this._attributes.selected) return { color: new THREE.Color(0xc2ed74), strength: 0.58 };
    return null;
  }

  private _cacheOriginalMaterials(object: THREE.Object3D): void {
    object.traverse((child) => {
      if (child instanceof THREE.Mesh) {
        const sourceMaterials = (Array.isArray(child.material) ? child.material : [child.material]);
        const visualMaterials = sourceMaterials.map((material) => material.clone());
        this._originalMaterials.set(child, sourceMaterials);
        this._visualMaterials.set(child, visualMaterials);
        child.material = Array.isArray(child.material) ? visualMaterials : visualMaterials[0];
      }
    });
  }

  /** Releases the geometry and materials owned by this visual object. */
  public dispose(): void {
    const geometries = new Set<THREE.BufferGeometry>();
    const materials = new Set<THREE.Material>();
    this.root3D.traverse((child) => {
      if (child instanceof THREE.Mesh) geometries.add(child.geometry);
    });
    for (const material of this._originalMaterials.values()) {
      for (const entry of material) materials.add(entry);
    }
    for (const material of this._visualMaterials.values()) {
      for (const entry of material) materials.add(entry);
    }
    for (const geometry of geometries) geometry.dispose();
    for (const material of materials) material.dispose();
    this._originalMaterials.clear();
    this._visualMaterials.clear();
  }

  private _createProceduralPlaceholder(): THREE.Mesh {
    const geometry = new THREE.BoxGeometry(0.5, 0.3, 0.4);
    const material = new THREE.MeshStandardMaterial({
      color: 0x475569,
      metalness: 0.6,
      roughness: 0.4
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    return mesh;
  }
}
