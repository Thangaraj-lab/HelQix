/**
 * HELQIX Visual Runtime — TransformSync
 * Layer 14: 14-visual-runtime
 *
 * High-precision spatial transform synchronization between discrete solver ticks and 60Hz display frames.
 * Supports exact deterministic snapping or smooth linear/spherical interpolation (SLERP).
 */

import * as THREE from 'three';
import { EngineeringObject } from '../objects/EngineeringObject';
import { SpatialSnapshot } from '../state/VisualState';

export interface InterpolationTarget {
  object: EngineeringObject;
  startPos: THREE.Vector3;
  targetPos: THREE.Vector3;
  startRot: THREE.Quaternion;
  targetRot: THREE.Quaternion;
  startScale: THREE.Vector3;
  targetScale: THREE.Vector3;
  alpha: number; // 0.0 to 1.0
}

export class TransformSync {
  private _interpolationTargets: Map<string, InterpolationTarget> = new Map();
  private _enableInterpolation: boolean;

  constructor(enableInterpolation: boolean = false) {
    this._enableInterpolation = enableInterpolation;
  }

  public setInterpolation(enabled: boolean): void {
    if (this._enableInterpolation && !enabled) {
      for (const target of this._interpolationTargets.values()) this._apply(target, 1);
      this._interpolationTargets.clear();
    }
    this._enableInterpolation = enabled;
  }

  public syncSnapshot(object: EngineeringObject, spatial: SpatialSnapshot): void {
    if (!this._enableInterpolation) {
      this._interpolationTargets.delete(object.id);
      object.updateSpatial(spatial);
      return;
    }

    const currentPos = object.root3D.position.clone();
    const currentRot = object.root3D.quaternion.clone();
    const currentScale = object.root3D.scale.clone();

    const targetPos = new THREE.Vector3(spatial.position.x, spatial.position.y, spatial.position.z);
    const targetRot = new THREE.Quaternion(spatial.rotation.x, spatial.rotation.y, spatial.rotation.z, spatial.rotation.w);
    const targetScale = new THREE.Vector3(spatial.scale.x, spatial.scale.y, spatial.scale.z);

    this._interpolationTargets.set(object.id, {
      object,
      startPos: currentPos,
      targetPos,
      startRot: currentRot,
      targetRot,
      startScale: currentScale,
      targetScale,
      alpha: 0.0
    });
  }

  public update(deltaTimeSeconds: number, blendRate: number = 20.0): void {
    if (!this._enableInterpolation || !Number.isFinite(deltaTimeSeconds) || deltaTimeSeconds < 0 || !Number.isFinite(blendRate) || blendRate <= 0) return;

    for (const [id, target] of this._interpolationTargets.entries()) {
      target.alpha = Math.min(1.0, target.alpha + deltaTimeSeconds * blendRate);
      this._apply(target, target.alpha);
      if (target.alpha >= 1.0) {
        this._interpolationTargets.delete(id);
      }
    }
  }

  /** Cancel a pending pose transition for an object removed from the scene. */
  public cancel(id: string): void {
    this._interpolationTargets.delete(id);
  }

  /** Clear pending interpolation references when a scene or runtime is disposed. */
  public clear(): void {
    this._interpolationTargets.clear();
  }

  private _apply(target: InterpolationTarget, alpha: number): void {
    const object = target.object.root3D;
    object.position.copy(target.startPos).lerp(target.targetPos, alpha);
    object.quaternion.copy(target.startRot).slerp(target.targetRot, alpha);
    object.scale.copy(target.startScale).lerp(target.targetScale, alpha);
  }

  public getInterpolatedPose(id: string): { position: THREE.Vector3; rotation: THREE.Quaternion; scale: THREE.Vector3 } | null {
    const target = this._interpolationTargets.get(id);
    if (!target) return null;

    const position = target.startPos.clone().lerp(target.targetPos, target.alpha);
    const rotation = target.startRot.clone().slerp(target.targetRot, target.alpha);
    const scale = target.startScale.clone().lerp(target.targetScale, target.alpha);

    return { position, rotation, scale };
  }
}
