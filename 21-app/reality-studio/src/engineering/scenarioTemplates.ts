import type { EngineeringReality } from '../api';
import { createScenario, type ExecutableScenario, type ScenarioAction } from './scenarioEngine';

export function actionId(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

export function createInspectionScenario(reality: EngineeringReality): ExecutableScenario {
  const first = reality.components[0];
  const actions: ScenarioAction[] = first
    ? [
        { id: actionId('start'), type: 'set_status', atMs: 0, componentId: first.id, status: 'running' },
        { id: actionId('checkpoint'), type: 'checkpoint', atMs: 1000, checkpointId: 'inspection-start' },
        { id: actionId('stop'), type: 'set_status', atMs: 2000, componentId: first.id, status: 'completed' },
      ]
    : [{ id: actionId('checkpoint'), type: 'checkpoint', atMs: 0, checkpointId: 'inspection-start' }];
  return createScenario('Inspection run', reality, actions, 'Generic HELQIX runtime smoke scenario: state transition, logical time, and checkpoint.');
}

export function createFaultInjectionScenario(reality: EngineeringReality, componentId: string, faultId = 'fault.injected'): ExecutableScenario {
  if (!reality.components.some((component) => component.id === componentId)) throw new Error(`Unknown component: ${componentId}`);
  const actions: ScenarioAction[] = [
    { id: actionId('start'), type: 'set_status', atMs: 0, componentId, status: 'running' },
    { id: actionId('fault'), type: 'inject_fault', atMs: 1000, componentId, faultId },
    { id: actionId('checkpoint'), type: 'checkpoint', atMs: 1000, checkpointId: 'fault-state' },
  ];
  return createScenario('Fault injection', reality, actions, 'Generic fault-state scenario for validating observation, checkpointing, and replay boundaries.');
}

/** Deterministic motion smoke test used by reference systems such as EV and drone models. */
export function createMotionScenario(reality: EngineeringReality, componentId: string): ExecutableScenario {
  if (!reality.components.some((component) => component.id === componentId)) throw new Error(`Unknown component: ${componentId}`);
  const component = reality.components.find((item) => item.id === componentId)!;
  const names = new Set(component.properties.map((property) => property.name.toLowerCase().replace(/[\s_-]/g, '')));
  const rpmName = ['rpm', 'rotational_speed', 'rotations_per_minute', 'shaft_speed'].find((name) => names.has(name.replace(/[\s_-]/g, ''))) ?? 'rpm';
  const actions: ScenarioAction[] = [
    { id: actionId('motion-start'), type: 'set_status', atMs: 0, componentId, status: 'running' },
    { id: actionId('motion-1500'), type: 'set_signal', atMs: 1000, componentId, signalName: rpmName, value: 1500, unit: 'rpm', signalSource: 'scenario' },
    { id: actionId('motion-checkpoint'), type: 'checkpoint', atMs: 2000, checkpointId: 'motion-1500rpm' },
    { id: actionId('motion-4000'), type: 'set_signal', atMs: 3000, componentId, signalName: rpmName, value: 4000, unit: 'rpm', signalSource: 'scenario' },
    { id: actionId('motion-stop'), type: 'set_signal', atMs: 4000, componentId, signalName: rpmName, value: 0, unit: 'rpm', signalSource: 'scenario' },
    { id: actionId('motion-complete'), type: 'set_status', atMs: 5000, componentId, status: 'completed' },
  ];
  return createScenario('Deterministic motion test', reality, actions, 'Reference-system scenario proving engineering signal changes propagate through authoritative state and into visual runtime motion.');
}

export function createPropertySweepScenario(reality: EngineeringReality, componentId: string, propertyName: string, values: Array<string | number | boolean>): ExecutableScenario {
  if (!reality.components.some((component) => component.id === componentId)) throw new Error(`Unknown component: ${componentId}`);
  if (!propertyName.trim() || values.length === 0) throw new Error('Property name and at least one value are required.');
  const actions: ScenarioAction[] = values.map((value, index) => ({ id: actionId('sweep'), type: 'set_property' as const, atMs: index * 1000, componentId, propertyName, value }));
  return createScenario(`Property sweep · ${propertyName}`, reality, actions, 'Generic deterministic scenario for bounded what-if exploration.');
}
