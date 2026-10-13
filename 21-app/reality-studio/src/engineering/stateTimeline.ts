import type { EngineeringReality } from '../api';
import { assertValidState, type AuthoritativeEngineeringState, type StateEvent, type StateSnapshot } from './authoritativeState';

export const STATE_TIMELINE_SCHEMA = 'helqix.state-timeline.v1' as const;
export interface TimelineEntry { sequence: number; logicalTimeMs: number; stateRevision: number; type: StateEvent['type']; componentId?: string; name?: string; actionId?: string; before?: string | number | boolean; after?: string | number | boolean; faultId?: string; source: StateEvent['source']; }
export interface FailureObservation { componentId?: string; faultId?: string; logicalTimeMs: number; stateRevision: number; actionId?: string; eventType: StateEvent['type']; description: string; }
export interface StateTimeline { schema: typeof STATE_TIMELINE_SCHEMA; realityRevision: number; stateRevision: number; logicalTimeMs: number; status: AuthoritativeEngineeringState['status']; entries: TimelineEntry[]; failures: FailureObservation[]; }

function eventStateRevision(state: AuthoritativeEngineeringState, event: StateEvent): number {
  if (event.type === 'state_created') return 1;
  const matching = state.events.filter((candidate) => candidate.sequence <= event.sequence);
  const transitions = matching.filter((candidate) => candidate.type !== 'state_created');
  return Math.min(state.stateRevision, Math.max(1, transitions.length + 1));
}

export function buildStateTimeline(reality: EngineeringReality, state: AuthoritativeEngineeringState): StateTimeline {
  assertValidState(state, reality);
  const entries = state.events.map((event) => ({
    sequence: event.sequence,
    logicalTimeMs: event.logicalTimeMs,
    stateRevision: eventStateRevision(state, event),
    type: event.type,
    ...(event.componentId ? { componentId: event.componentId } : {}),
    ...(event.name ? { name: event.name } : {}),
    ...(event.actionId ? { actionId: event.actionId } : {}),
    ...(event.before !== undefined ? { before: event.before } : {}),
    ...(event.after !== undefined ? { after: event.after } : {}),
    ...(event.faultId ? { faultId: event.faultId } : {}),
    source: event.source,
  }));
  const failures = state.events
    .filter((event) => event.type === 'fault_injected' || event.after === 'failed' || event.after === 'faulted')
    .map((event) => ({
      ...(event.componentId ? { componentId: event.componentId } : {}),
      ...(event.faultId ? { faultId: event.faultId } : {}),
      logicalTimeMs: event.logicalTimeMs,
      stateRevision: eventStateRevision(state, event),
      ...(event.actionId ? { actionId: event.actionId } : {}),
      eventType: event.type,
      description: event.faultId ? `Fault ${event.faultId} observed.` : `Failure state observed${event.componentId ? ` on ${event.componentId}` : ''}.`,
    }));
  return { schema: STATE_TIMELINE_SCHEMA, realityRevision: reality.revision, stateRevision: state.stateRevision, logicalTimeMs: state.logicalTimeMs, status: state.status, entries, failures };
}

export function timelineBetween(timeline: StateTimeline, startMs: number, endMs: number): TimelineEntry[] {
  if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || startMs < 0 || endMs < startMs) throw new Error('Timeline range must be finite and non-negative.');
  return timeline.entries.filter((entry) => entry.logicalTimeMs >= startMs && entry.logicalTimeMs <= endMs);
}
export function latestFailure(timeline: StateTimeline): FailureObservation | undefined { return timeline.failures[timeline.failures.length - 1]; }
export function snapshotAtOrBefore(snapshots: StateSnapshot[], logicalTimeMs: number): StateSnapshot | undefined { return [...snapshots].filter((snapshot) => snapshot.logicalTimeMs <= logicalTimeMs).sort((a, b) => b.logicalTimeMs - a.logicalTimeMs)[0]; }
