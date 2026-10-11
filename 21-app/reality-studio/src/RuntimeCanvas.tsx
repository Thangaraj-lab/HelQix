import type { RuntimeEntity, RuntimeViewPreset } from './visualRuntime';
import { useVehicleRuntime, useVisualRuntime } from './visualRuntime';
import type { RealityConnection, RealityInterface } from './api';

export function RuntimeCanvas({ entities, interfaces, connections, selectedEntityId, onSelectEntity, viewPreset, showGrid, showAxes, fitRevision }: { entities?: RuntimeEntity[]; interfaces?: RealityInterface[]; connections?: RealityConnection[]; selectedEntityId?: string | null; onSelectEntity?: (id: string | null) => void; viewPreset?: RuntimeViewPreset; showGrid?: boolean; showAxes?: boolean; fitRevision?: number }) {
  const canvas = useVisualRuntime(entities, interfaces, connections, onSelectEntity, selectedEntityId, viewPreset, showGrid, showAxes, fitRevision);
  return <canvas ref={canvas} className={`runtime-canvas ${onSelectEntity ? 'runtime-canvas-interactive' : ''}`} aria-label={onSelectEntity ? 'Interactive engineering reality component viewport. Drag to orbit, scroll to zoom, and click a part to inspect it.' : 'Engineering reality component viewport'} />;
}

export function VehicleRuntimeCanvas({ distanceMeters, totalDistanceMeters }: { distanceMeters: number; totalDistanceMeters: number }) {
  const canvas = useVehicleRuntime(distanceMeters, totalDistanceMeters);
  return <canvas ref={canvas} className="vehicle-runtime-canvas" aria-label="Three-dimensional vehicle moving along the recorded drive-cycle distance" />;
}
