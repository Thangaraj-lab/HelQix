import { Binding, BuildEvent, Issue, ModelOp, ModelRevision, Project, RunFrame, Source, Transform, Value } from '../contracts';

// Mappers between the UI's types and the HELQIX API's wire format (15-sdk/platform/contracts).
// Pure functions, so they are tested against shapes taken from the Rust serde definitions. Where the
// two differ, the UI keeps its own shape and the difference is handled here and nowhere else.

type Wire = Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any

const AXES: Record<'x' | 'y' | 'z', [number, number, number]> = { x: [1, 0, 0], y: [0, 1, 0], z: [0, 0, 1] };

function axisLetter(v: number[] | undefined): 'x' | 'y' | 'z' | undefined {
  if (!v) return undefined;
  const i = v.findIndex(n => Math.abs(n) > 0.5);
  return (['x', 'y', 'z'] as const)[i];
}

export function toWireBinding(b: Binding): Wire {
  return {
    variable: b.variable,
    node: b.node,
    channel: b.channel,
    ...(b.axis ? { axis: AXES[b.axis] } : {}),
    // A rotation is the integral of an angular rate; everything else maps value to channel linearly.
    mapping: b.channel === 'rotation' ? 'integrate' : 'linear',
    units: b.units,
    ...(b.range ? { range: b.range } : {})
  };
}

export function fromWireBinding(b: Wire): Binding {
  return {
    id: `${b.node}:${b.variable}`,
    variable: b.variable,
    node: b.node,
    channel: b.channel,
    axis: axisLetter(b.axis),
    mapping: b.mapping,
    units: b.units,
    range: b.range
  };
}

export function toWireOp(op: ModelOp): Wire {
  switch (op.op) {
    case 'connection.add':
      return { op: op.op, id: op.id, from: { part: op.from.partId, port: op.from.portId }, to: { part: op.to.partId, port: op.to.portId }, kind: op.kind };
    case 'binding.add':
      return { op: op.op, binding: toWireBinding(op.binding) };
    default:
      return { ...op };
  }
}

export function fromWireOp(op: Wire): ModelOp {
  switch (op.op) {
    case 'connection.add':
      return { op: 'connection.add', id: op.id, from: { partId: op.from.part, portId: op.from.port }, to: { partId: op.to.part, portId: op.to.port }, kind: op.kind };
    case 'binding.add':
      return { op: 'binding.add', binding: fromWireBinding(op.binding) };
    default:
      return op as ModelOp;
  }
}

export function fromWireIssue(i: Wire): Issue {
  return {
    id: i.id,
    partId: Array.isArray(i.subject) ? i.subject[0] : undefined,
    message: i.message,
    severity: i.severity
    // The API describes a fix in words (i.fix); it does not send an op, so the UI offers no Fix button.
  };
}

export function fromWireEvent(e: Wire): BuildEvent {
  return {
    seq: e.seq,
    buildId: e.buildId,
    ts: e.ts,
    kind: e.kind,
    step: e.step,
    op: e.op ? fromWireOp(e.op) : undefined,
    issue: e.issue ? fromWireIssue(e.issue) : undefined,
    question: e.question ? { id: e.question.id, text: e.question.text, defaultVal: e.question.default } : undefined,
    note: e.note
  };
}

const ZERO = (unit: string): Value => ({ v: 0, unit, origin: 'user_stated', status: 'unverified' });
const DEFAULT_TRANSFORM = (): Transform => ({ pos: [ZERO('m'), { ...ZERO('m'), v: 0.5 }, ZERO('m')], rot: [ZERO('deg'), ZERO('deg'), ZERO('deg')], scale: [1, 1, 1], frame: 'world' });

/** Folds a revision's op list into the model the UI reads: parts, ports, connections, requirements, bindings. */
export function foldRevision(summary: Wire, ops: Wire[]): ModelRevision {
  const rev: ModelRevision = {
    id: `r${summary.number}`,
    projectId: summary.projectId,
    revisionNum: summary.number,
    committedAt: summary.committedAt,
    parts: {},
    ports: {},
    connections: {},
    requirements: {},
    bindings: {}
  };
  for (const w of ops) {
    const op = fromWireOp(w);
    if (op.op === 'part.add') rev.parts[op.id] = { id: op.id, name: op.name, geometry: op.geometry, transform: DEFAULT_TRANSFORM(), maturity: op.maturity };
    else if (op.op === 'part.place' && rev.parts[op.id]) rev.parts[op.id].transform = op.transform;
    else if (op.op === 'part.remove') delete rev.parts[op.id];
    else if (op.op === 'port.add') rev.ports[`${op.part}:${op.id}`] = { id: op.id, partId: op.part, kind: op.kind, direction: op.direction, unit: op.unit };
    else if (op.op === 'connection.add') rev.connections[op.id] = { id: op.id, from: op.from, to: op.to, kind: op.kind };
    else if (op.op === 'connection.remove') delete rev.connections[op.id];
    else if (op.op === 'requirement.add') rev.requirements[op.id] = { id: op.id, text: op.text, expr: op.expr };
    else if (op.op === 'binding.add') rev.bindings[op.binding.id] = op.binding;
  }
  return rev;
}

export function fromWireProject(record: Wire, revisionCount: number): Project {
  return {
    id: record.id,
    name: typeof record.payload?.name === 'string' && record.payload.name ? record.payload.name : 'Untitled project',
    createdAt: record.created_at,
    updatedAt: record.updated_at,
    activeRevisionId: revisionCount ? `r${revisionCount}` : undefined,
    modelRevisions: Array.from({ length: revisionCount }, (_, i) => `r${i + 1}`)
  };
}

const SOURCE_KINDS: Source['kind'][] = ['connected_tool', 'cad_repository', 'catalog', 'manual'];
export function fromWireSource(s: Wire): Source {
  return { id: s.id, name: s.example ? `${s.name} (Example)` : s.name, kind: SOURCE_KINDS.includes(s.kind) ? s.kind : 'catalog', status: 'connected', lastSync: s.asOf };
}

export function fromWireFrame(f: Wire): RunFrame {
  const values: Record<string, Value> = {};
  for (const [k, v] of Object.entries(f.state ?? {})) values[k] = v as Value;
  return { t: f.t.v, values };
}
