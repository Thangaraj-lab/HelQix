/**
 * HELQIX Visual Runtime — RealitySynchronizer
 * Layer 14: 14-visual-runtime
 *
 * Master synchronization coordinator.
 * Consumes authoritative engineering snapshots and updates visual representations in real time.
 */

import { SceneManager } from '../scene/SceneManager';
import { StateMapper, RawCoreSnapshot } from '../state/StateMapper';
import { EngineeringVisualSnapshot } from '../state/VisualState';
import { EngineeringObject } from '../objects/EngineeringObject';

export class RealitySynchronizer {
  private _manager: SceneManager;
  private _latestSnapshot: EngineeringVisualSnapshot | null = null;
  private _autoRegisterMissing: boolean;

  constructor(manager: SceneManager, autoRegisterMissing: boolean = true) {
    this._manager = manager;
    this._autoRegisterMissing = autoRegisterMissing;
  }

  public get latestSnapshot(): EngineeringVisualSnapshot | null {
    return this._latestSnapshot;
  }

  public applyCoreSnapshot(raw: RawCoreSnapshot): void {
    const visualSnapshot = StateMapper.mapFromCoreSnapshot(raw);
    this.applyVisualSnapshot(visualSnapshot);
  }

  public applyVisualSnapshot(snapshot: EngineeringVisualSnapshot): void {
    this._latestSnapshot = snapshot;

    for (const [id, entityData] of Object.entries(snapshot.entities)) {
      let obj = this._manager.registry.get(id);

      if (!obj && this._autoRegisterMissing) {
        obj = new EngineeringObject({
          id,
          name: (entityData.telemetry.name as string) || id,
          subsystem: (entityData.telemetry.subsystem as string) || 'default',
          category: (entityData.telemetry.category as string) || 'mechanical',
          initialAttributes: entityData.attributes
        });
        this._manager.registry.register(obj);
      }

      if (obj) {
        this._manager.transformSync.syncSnapshot(obj, entityData.spatial);
        obj.applyAttributes(entityData.attributes);

        obj.setThermalTint(entityData.attributes.thermal?.normalizedHeatRatio ?? null);
      }
    }

    this._manager.renderSingleFrame();
  }

  public clearScene(): void {
    this._manager.transformSync.clear();
    this._manager.registry.clear();
    this._latestSnapshot = null;
    this._manager.renderSingleFrame();
  }
}
