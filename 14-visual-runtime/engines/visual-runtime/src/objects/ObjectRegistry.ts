/**
 * HELQIX Visual Runtime — ObjectRegistry
 * Layer 14: 14-visual-runtime
 *
 * Central registry maintaining 1-to-1 parity between core engineering IDs and 3D visual objects.
 */

import * as THREE from 'three';
import { EngineeringObject } from './EngineeringObject';

export class ObjectRegistry {
  private _objects: Map<string, EngineeringObject> = new Map();
  private _rootSceneGroup: THREE.Group;
  private _onRemove?: (id: string) => void;

  constructor(rootSceneGroup: THREE.Group, onRemove?: (id: string) => void) {
    this._rootSceneGroup = rootSceneGroup;
    this._onRemove = onRemove;
  }

  public register(object: EngineeringObject): void {
    const existing = this._objects.get(object.id);
    if (existing === object) {
      this._rootSceneGroup.add(object.root3D);
      return;
    }
    if (existing) this.unregister(object.id);

    this._objects.set(object.id, object);
    this._rootSceneGroup.add(object.root3D);
  }

  public unregister(id: string): boolean {
    const obj = this._objects.get(id);
    if (!obj) return false;

    this._rootSceneGroup.remove(obj.root3D);
    this._objects.delete(id);
    this._onRemove?.(id);
    obj.dispose();
    return true;
  }

  public get(id: string): EngineeringObject | undefined {
    return this._objects.get(id);
  }

  public getAll(): EngineeringObject[] {
    return Array.from(this._objects.values());
  }

  public getBySubsystem(subsystem: string): EngineeringObject[] {
    return this.getAll().filter((obj) => obj.subsystem === subsystem);
  }

  public findByRaycastHit(intersectedObject: THREE.Object3D): EngineeringObject | undefined {
    let curr: THREE.Object3D | null = intersectedObject;
    while (curr) {
      if (curr.userData && curr.userData.isEngineeringObject && curr.userData.helqixId) {
        return this._objects.get(curr.userData.helqixId);
      }
      curr = curr.parent;
    }
    return undefined;
  }

  public clear(): void {
    for (const obj of this._objects.values()) {
      this._rootSceneGroup.remove(obj.root3D);
      this._onRemove?.(obj.id);
      obj.dispose();
    }
    this._objects.clear();
  }

  /** Detaches and disposes every registered object owned by this registry. */
  public dispose(): void {
    this.clear();
  }

  public get count(): number {
    return this._objects.size;
  }
}
