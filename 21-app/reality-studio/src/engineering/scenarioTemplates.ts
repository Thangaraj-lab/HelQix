import { createScenario, type ExecutableScenario, type ScenarioAction } from './scenarioEngine';
import type { EngineeringReality } from '../api';

const actionId = (prefix: string) => `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;

export function createInspectionScenario(reality: EngineeringReality, componentId: string): ExecutableScenario {
  const actions: ScenarioAction[] = [
    { id: actionId('ready'), type: 'set_status', atMs: 0, componentId, status: 'ready' },
    { id: actionId('checkpoint'), type: 'checkpoint', atMs: 0, checkpointId: 'inspection-start' },
  ];
  return createScenario('Inspection', reality, actions, 'Generic state inspection scenario. No solver or physical result is inferred.');
}

export function createFaultInjectionScenario(reality: EngineeringReality, componentId: string, faultId = 'test-fault'): ExecutableScenario {
  const actions: ScenarioAction[] = [
    { id: actionId('start'), type: 'set_status', atMs: 0, componentId, status: 'running' },
    { id: actionId('fault'), type: 'inject_fault', atMs: 1000, componentId, faultId },
    { id: actionId('checkpoint'), type: 'checkpoint', atMs: 1000, checkpointId: 'fault-observed' },
  ];
  return createScenario('Fault injection', reality, actions, 'Controlled runtime fault observation. This does not model physical failure propagation.');
}

export function createPropertyChangeScenario(reality: EngineeringReality, componentId: string, propertyName: string, value: string | number | boolean, atMs = 0): ExecutableScenario {
  const component = reality.components.find((item) => item.id === componentId);
  if (!component) throw new Error(`Unknown component ${componentId}.`);
  if (!component.properties.some((property) => property.name === propertyName)) throw new Error(`Property ${propertyName} is not declared on component ${componentId}.`);
  const actions: ScenarioAction[] = [
    { id: actionId('property'), type: 'set_property', atMs, componentId, propertyName, value },
    { id: actionId('checkpoint'), type: 'checkpoint', atMs, checkpointId: 'property-change' },
  ];
  return createScenario('Property change', reality, actions, 'Generic executable property change against the authoritative runtime state.');
}
