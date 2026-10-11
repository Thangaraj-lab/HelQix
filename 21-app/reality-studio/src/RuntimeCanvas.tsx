import type { RuntimeEntity, RuntimeViewPreset } from './visualRuntime';
import { useVehicleRuntime, useVisualRuntime } from './visualRuntime';
import type { EngineeringReality, RealityConnection, RealityInterface } from './api';
import type { AuthoritativeEngineeringState } from './engineering/authoritativeState';
import { bindAuthoritativeStateToReality } from './engineering/visualStateBinding';
import { useMemo } from 'react';

export interface RuntimeCanvasProps {
  entities?: RuntimeEntity[];
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

export function RuntimeCanvas({ entities = [], interfaces, connections, reality, authoritativeState, selectedEntityId, onSelectEntity, viewPreset, showGrid, showAxes, fitRevision }: RuntimeCanvasProps) {
  const boundEntities = useMemo(() => {
    if (!reality || !authoritativeState) return entities;
    return bindAuthoritativeStateToReality(reality, authoritativeState, selectedEntityId).entities;
  }, [entities, reality, authoritativeState, selectedEntityId]);

  const canvas = useVisualRuntime(boundEntities, interfaces, connections, onSelectEntity, selectedEntityId, viewPreset, showGrid, showAxes, fitRevision);
  return <canvas ref={canvas} className={`runtime-canvas ${onSelectEntity ? 'runtime-canvas-interactive' : ''}`} aria-label={onSelectEntity ? 'Interactive engineering reality component viewport. Drag to orbit, scroll to zoom, and click a part to inspect it.' : 'Engineering reality component viewport'} />;
}

export function VehicleRuntimeCanvas({ distanceMeters, totalDistanceMeters }: { distanceMeters: number; totalDistanceMeters: number }) {
  const canvas = useVehicleRuntime(distanceMeters, totalDistanceMeters);
  return <canvas ref={canvas} className="vehicle-runtime-canvas" aria-label="Three-dimensional vehicle moving along the recorded drive-cycle distance" />;
}
