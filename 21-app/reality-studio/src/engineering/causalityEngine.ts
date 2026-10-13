import type { ObservationAnomaly } from './observationFabric';

export const CAUSALITY_ENGINE_SCHEMA = 'helqix.causality-engine.v1' as const;

export interface CausalNode {
  id: string;
  kind: 'component' | 'parameter' | 'signal' | 'environment' | 'failure' | 'event';
  label: string;
  reference: string;
}

export interface CausalEdge {
  from: string;
  to: string;
  relation: 'causes' | 'contributes' | 'depends_on' | 'correlates';
  confidence: number;
  evidenceIds: string[];
}

export interface CausalFailure {
  id: string;
  anomalyId: string;
  rootCauseNodeId?: string;
  path: string[];
  confidence: number;
  explanation: string;
}

export interface CausalityGraph {
  schema: typeof CAUSALITY_ENGINE_SCHEMA;
  nodes: CausalNode[];
  edges: CausalEdge[];
  failures: CausalFailure[];
}

function clone<T>(value: T): T { return structuredClone(value); }
function confidence(value: number): number { return Math.min(1, Math.max(0, value)); }

export class CausalityEngine {
  private readonly nodes = new Map<string, CausalNode>();
  private readonly edges: CausalEdge[] = [];
  private readonly failures: CausalFailure[] = [];

  addNode(node: CausalNode): void {
    if (!node.id.trim()) throw new Error('Causal node id is required.');
    if (this.nodes.has(node.id)) return;
    this.nodes.set(node.id, clone(node));
  }

  addEdge(edge: CausalEdge): void {
    if (!this.nodes.has(edge.from) || !this.nodes.has(edge.to)) throw new Error('Both causal edge endpoints must exist.');
    if (edge.from === edge.to) throw new Error('Causal self-edge is not allowed.');
    if (edge.confidence < 0 || edge.confidence > 1) throw new Error('Causal confidence must be between 0 and 1.');
    const duplicate = this.edges.some((item) => item.from === edge.from && item.to === edge.to && item.relation === edge.relation);
    if (!duplicate) this.edges.push({ ...clone(edge), confidence: confidence(edge.confidence) });
  }

  ingestAnomaly(anomaly: ObservationAnomaly, candidateNodeIds: string[] = []): CausalFailure {
    const failureNodeId = `failure:${anomaly.id}`;
    this.addNode({ id: failureNodeId, kind: 'failure', label: anomaly.message, reference: anomaly.id });
    const candidates = candidateNodeIds.filter((id) => this.nodes.has(id));
    for (const nodeId of candidates) this.addEdge({ from: nodeId, to: failureNodeId, relation: 'contributes', confidence: anomaly.severity === 'critical' ? 0.75 : 0.45, evidenceIds: [anomaly.id] });
    const root = this.selectRoot(failureNodeId);
    const path = root ? this.findPath(root, failureNodeId) : [failureNodeId];
    const result: CausalFailure = { id: `causal:${anomaly.id}`, anomalyId: anomaly.id, ...(root ? { rootCauseNodeId: root } : {}), path, confidence: root ? 0.75 : 0.2, explanation: root ? `Candidate root cause is ${this.nodes.get(root)?.label ?? root}, supported by the recorded causal graph.` : 'No connected upstream cause is available; additional instrumentation or simulation evidence is required.' };
    this.failures.push(clone(result));
    return result;
  }

  private selectRoot(target: string): string | undefined {
    const upstream = this.edges.filter((edge) => edge.to === target).sort((a, b) => b.confidence - a.confidence || a.from.localeCompare(b.from));
    if (!upstream.length) return undefined;
    let current = upstream[0].from;
    const visited = new Set<string>([target]);
    while (!visited.has(current)) {
      visited.add(current);
      const next = this.edges.filter((edge) => edge.to === current).sort((a, b) => b.confidence - a.confidence || a.from.localeCompare(b.from))[0];
      if (!next) break;
      current = next.from;
    }
    return current;
  }

  private findPath(from: string, to: string): string[] {
    const queue: Array<{ node: string; path: string[] }> = [{ node: from, path: [from] }];
    const visited = new Set<string>([from]);
    while (queue.length) {
      const current = queue.shift()!;
      if (current.node === to) return current.path;
      for (const edge of this.edges.filter((item) => item.from === current.node).sort((a, b) => a.to.localeCompare(b.to))) {
        if (visited.has(edge.to)) continue;
        visited.add(edge.to);
        queue.push({ node: edge.to, path: [...current.path, edge.to] });
      }
    }
    return [from, to];
  }

  graph(): CausalityGraph { return { schema: CAUSALITY_ENGINE_SCHEMA, nodes: [...this.nodes.values()].map(clone), edges: this.edges.map(clone), failures: this.failures.map(clone) }; }
  failure(anomalyId: string): CausalFailure | undefined { return this.failures.find((failure) => failure.anomalyId === anomalyId); }
}
