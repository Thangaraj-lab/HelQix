import type { RuntimeEntity, RuntimeViewPreset } from './visualRuntime';
import { useVehicleRuntime, useVisualRuntime } from './visualRuntime';
import type { EngineeringReality, RealityConnection, RealityInterface } from './api';
import type { AuthoritativeEngineeringState } from './engineering/authoritativeState';
import { bindAuthoritativeStateToReality, type RuntimeEntityState } from './engineering/visualStateBinding';
import { useMemo } from 'react';

export interface RuntimeCanvasProps {
  entities?: RuntimeEntity[] | RuntimeEntityState[];
  interfaces?: RealityInterface[];
  connections?: RealityConnection[];
  reality?: EngineeringReality;
  authoritativeState?: AuthoritativeEngineeringState;
  selectedEntityId?: string | null;
  onSelectEntity?: (id: string | null) => void;
  viewPreset?: RuntimeViewPreset;
  showGrid?: boolean;
  showAxes?: boolean;
  fitRevision?: number;
}

function normalizeEntities(entities: RuntimeCanvasProps['entities']): RuntimeEntityState[] {
  return (entities ?? []).map((entity) => ({
    ...entity, status: 'unknown' as const, propertyValues: {}, faults: [], signals: {}, selected: false,
    stateRevision: 0, logicalTimeMs: 0, motionEvents: [],
  }));
}

export function RuntimeCanvas({ entities = [], interfaces, connections, reality, authoritativeState, selectedEntityId, onSelectEntity, viewPreset, showGrid, showAxes, fitRevision }: RuntimeCanvasProps) {
  const boundEntities = useMemo<RuntimeEntityState[]>(() => reality && authoritativeState
    ? bindAuthoritativeStateToReality(reality, authoritativeState, selectedEntityId).entities
    : normalizeEntities(entities), [entities, reality, authoritativeState, selectedEntityId]);
  const canvas = useVisualRuntime(boundEntities, interfaces, connections, onSelectEntity, selectedEntityId, viewPreset, showGrid, showAxes, fitRevision);
  return <canvas ref={canvas} className={`runtime-canvas ${onSelectEntity ? 'runtime-canvas-interactive' : ''}`} aria-label={onSelectEntity ? 'Interactive engineering reality component viewport. Drag to orbit, scroll to zoom, and click a part to inspect it.' : 'Engineering reality component viewport'} />;
}

export function VehicleRuntimeCanvas({ distanceMeters, totalDistanceMeters }: { distanceMeters: number; totalDistanceMeters: number }) {
  const canvas = useVehicleRuntime(distanceMeters, totalDistanceMeters);
  return <canvas ref={canvas} className="vehicle-runtime-canvas" aria-label="Three-dimensional vehicle moving along the recorded drive-cycle distance" />;
}
