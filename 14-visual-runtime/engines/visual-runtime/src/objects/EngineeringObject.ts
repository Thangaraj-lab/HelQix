/**
 * HELQIX Visual Runtime — EngineeringObject
 * Layer 14: 14-visual-runtime
 *
 * High-performance container wrapping Three.js Object3D for engineering components.
 * Tracks metadata, hierarchical ports, visual state, and telemetry tags.
 */

import * as THREE from 'three';
import { ComponentVisualAttributes, SpatialSnapshot } from '../state/VisualState';

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
  private _selected = false;
  private _visible = true;

  constructor(config: EngineeringObjectConfig) {
    this.id = config.id;
    this.name = config.name;
    this.subsystem = config.subsystem;
    this.category = config.category;
    this.root3D = config.rootMesh instanceof THREE.Group ? config.rootMesh : new THREE.Group();
    if (config.rootMesh && !(config.rootMesh instanceof THREE.Group)) this.root3D.add(config.rootMesh);
    this._attributes = {
      selected: false,
      health: 'unknown',
      ...(config.initialAttributes ?? {}),
    };
  }

  get attributes(): ComponentVisualAttributes { return { ...this._attributes }; }
  get selected(): boolean { return this._selected; }
  get visible(): boolean { return this._visible; }
  get worldPosition(): THREE.Vector3 { return this.root3D.getWorldPosition(new THREE.Vector3()); }

  setAttributes(attributes: Partial<ComponentVisualAttributes>): void { this._attributes = { ...this._attributes, ...attributes }; }
  setSelected(selected: boolean): void { this._selected = selected; this._attributes.selected = selected; }
  setVisible(visible: boolean): void { this._visible = visible; this.root3D.visible = visible; }
  addChild(child: THREE.Object3D): void { this.root3D.add(child); }
  removeChild(child: THREE.Object3D): void { this.root3D.remove(child); }
  snapshot(): SpatialSnapshot { return { position: this.root3D.position.clone(), rotation: this.root3D.rotation.clone(), scale: this.root3D.scale.clone() }; }
  applySnapshot(snapshot: SpatialSnapshot): void { this.root3D.position.copy(snapshot.position); this.root3D.rotation.copy(snapshot.rotation); this.root3D.scale.copy(snapshot.scale); }
  dispose(): void {
    this.root3D.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      object.geometry.dispose();
      if (Array.isArray(object.material)) object.material.forEach((material) => material.dispose()); else object.material.dispose();
    });
    this._originalMaterials.clear();
  }
}
