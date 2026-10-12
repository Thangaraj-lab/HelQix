export const ENGINEERING_ASSISTANT_SCHEMA = 'helqix.engineering-assistant.v1' as const;

export type ProposalKind =
  | 'scenario'
  | 'parameter-change'
  | 'failure-hypothesis'
  | 'experiment'
  | 'optimization'
  | 'verification';

export type ProposalRisk = 'low' | 'medium' | 'high';

export interface EngineeringAssistantContext {
  projectId: string;
  realityRevision: string;
  stateRevision?: string;
  logicalTimeMs?: number;
  selectedComponentIds?: string[];
  activeScenarioId?: string;
  observedFailureIds?: string[];
  requirements?: string[];
}

export interface EngineeringProposal {
  id: string;
  schema: typeof ENGINEERING_ASSISTANT_SCHEMA;
  kind: ProposalKind;
  title: string;
  rationale: string;
  confidence: number;
  risk: ProposalRisk;
  requiresApproval: true;
  changes: Array<{ path: string; from?: unknown; to: unknown; reason: string }>;
  evidence: Array<{ source: string; reference: string; relevance: string }>;
  validation: Array<{ check: string; required: boolean }>;
}

export interface AssistantRequest {
  schema: typeof ENGINEERING_ASSISTANT_SCHEMA;
  id: string;
  intent: string;
  context: EngineeringAssistantContext;
}

export interface AssistantResponse {
  schema: typeof ENGINEERING_ASSISTANT_SCHEMA;
  request: AssistantRequest;
  proposals: EngineeringProposal[];
  authority: 'proposal-only';
}

function requireText(value: string, field: string): string {
  const normalized = value.trim();
  if (!normalized) throw new Error(`${field} is required.`);
  return normalized;
}

function clampConfidence(value: number): number {
  if (!Number.isFinite(value)) throw new Error('Proposal confidence must be finite.');
  return Math.min(1, Math.max(0, value));
}

export function createAssistantRequest(intent: string, context: EngineeringAssistantContext, id = 'assistant-request'): AssistantRequest {
  requireText(intent, 'Assistant intent');
  requireText(context.projectId, 'projectId');
  requireText(context.realityRevision, 'realityRevision');
  if (context.logicalTimeMs !== undefined && (!Number.isFinite(context.logicalTimeMs) || context.logicalTimeMs < 0)) throw new Error('logicalTimeMs must be a non-negative finite number.');
  return { schema: ENGINEERING_ASSISTANT_SCHEMA, id: requireText(id, 'request id'), intent: intent.trim(), context: { ...context, selectedComponentIds: context.selectedComponentIds?.slice(), observedFailureIds: context.observedFailureIds?.slice(), requirements: context.requirements?.slice() } };
}

export function createProposal(input: Omit<EngineeringProposal, 'schema' | 'requiresApproval' | 'confidence'> & { confidence: number }): EngineeringProposal {
  if (!input.id.trim()) throw new Error('Proposal id is required.');
  if (!input.title.trim()) throw new Error('Proposal title is required.');
  if (!input.rationale.trim()) throw new Error('Proposal rationale is required.');
  if (!input.changes.length) throw new Error('Proposal must contain at least one proposed change.');
  return { ...input, schema: ENGINEERING_ASSISTANT_SCHEMA, confidence: clampConfidence(input.confidence), requiresApproval: true, changes: input.changes.map((change) => ({ ...change })), evidence: input.evidence.map((item) => ({ ...item })), validation: input.validation.map((item) => ({ ...item })) };
}

export function createAssistantResponse(request: AssistantRequest, proposals: EngineeringProposal[]): AssistantResponse {
  if (request.schema !== ENGINEERING_ASSISTANT_SCHEMA) throw new Error('Unsupported engineering assistant request schema.');
  for (const proposal of proposals) {
    if (!proposal.requiresApproval || proposal.schema !== ENGINEERING_ASSISTANT_SCHEMA) throw new Error('Engineering proposals must remain approval-gated and schema-compatible.');
    if (proposal.confidence < 0 || proposal.confidence > 1) throw new Error('Proposal confidence must be between 0 and 1.');
  }
  return { schema: ENGINEERING_ASSISTANT_SCHEMA, request, proposals: proposals.map((proposal) => ({ ...proposal, changes: proposal.changes.map((change) => ({ ...change })), evidence: proposal.evidence.map((item) => ({ ...item })), validation: proposal.validation.map((item) => ({ ...item })) })), authority: 'proposal-only' };
}

export function approveProposal(response: AssistantResponse, proposalId: string): EngineeringProposal {
  if (response.authority !== 'proposal-only') throw new Error('Invalid assistant authority contract.');
  const proposal = response.proposals.find((candidate) => candidate.id === proposalId);
  if (!proposal) throw new Error(`Proposal '${proposalId}' was not found.`);
  if (!proposal.requiresApproval) throw new Error(`Proposal '${proposalId}' is missing approval gating.`);
  return { ...proposal, requiresApproval: true };
}
