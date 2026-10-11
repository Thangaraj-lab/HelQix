export const API_BASE = import.meta.env.VITE_HELQIX_API_URL ?? 'http://127.0.0.1:8091';

export interface User { id: string; name: string; email: string; organization?: string }
export class HelqixRequestError extends Error { constructor(message: string, public readonly runId?: string) { super(message); this.name = 'HelqixRequestError'; } }
export interface Resource<T = Record<string, unknown>> {
  id: string; kind: string; project_id: string | null; version: number;
  payload: T; created_at: string; updated_at: string;
}
export interface IntentCandidate { key: string; label: string; value: number | string; unit: string | null; source_text: string; start: number; end: number; confidence: string; review_status: string }
export interface AIReview { objectives: Array<{ statement: string; source_quote: string; review_status: 'needs_review' }>; questions: Array<{ question: string; reason: string }>; provider: string; model: string; provenance: 'external_model_proposal'; review_required: true }
export interface IntentAnalysis { parser_version: string; method: string; scope: string; source_text: string; candidates: IntentCandidate[]; conflicts: string[]; not_inferred: string[]; review_required: boolean; ai_review_status?: { available: boolean; provider: string | null; model: string | null; note: string }; ai_review?: AIReview; user_review?: { accepted_candidate_indices: number[]; reviewed_at: string; accepted_ai_objective_indices?: number[] } }
export interface ProjectPayload { name: string; description?: string; intent?: string; intent_analysis?: IntentAnalysis; domain?: string; engineering_reality?: EngineeringReality; ev_vehicle?: EVBatteryVehicleConfiguration }
export interface EVDriveCyclePoint { timeS: number; speedMS: number; roadGradePercent: number }
export interface EVBatteryVehicleConfiguration {
  schema: 'helqix.ev-battery-vehicle.v1'; vehicleName: string; vehicleMassKg: number; dragCoefficient: number; frontalAreaM2: number;
  rollingResistanceCoefficient: number; drivetrainEfficiency: number; accessoryPowerW: number;
  batteryPack: { seriesCells: number; parallelCells: number; cellCapacityAh: number; cellNominalVoltageV: number; cellCutoffVoltageV: number; cellMaxContinuousCurrentA: number; initialSoc: number };
  driveCycle: EVDriveCyclePoint[]; modelId: 'pybamm.lithium-ion.spm'; parameterSet: 'Chen2020';
}
export interface RealityProperty { name: string; value: string | number | boolean; unit?: string; origin: 'user' | 'catalog' | 'import' | 'derived' | 'model'; sourceRef?: string }
export interface RealityComponent { id: string; name: string; componentType: string; properties: RealityProperty[] }
export interface ModelBinding { id: string; modelId: string; modelVersion: string; scopeComponentId: string; declaredFidelity: string }
export interface VerificationRule { metric: 'duration_h' | 'discharged_capacity_ah' | 'final_voltage_v'; operator: '>=' | '<=' | '>' | '<' | '=='; threshold: number; unit: 'h' | 'Ah' | 'V' }
export interface EngineeringRequirement { id: string; statement: string; priority: 'must' | 'should' | 'could'; reviewStatus: 'draft' | 'accepted'; sourceRef?: string; verificationRule?: VerificationRule }
export interface RealityInterface { id: string; componentId: string; name: string; direction: 'in' | 'out' | 'bidirectional'; domain: string }
export interface RealityConnection { id: string; sourceInterfaceId: string; targetInterfaceId: string; connectionType: string }
export interface EngineeringReality { schema: 'helqix.engineering-reality.v1'; name: string; domain: string; description: string; revision: number; unitsSystem: 'SI'; components: RealityComponent[]; interfaces: RealityInterface[]; connections: RealityConnection[]; requirements: EngineeringRequirement[]; modelBindings: ModelBinding[] }
export interface IntegrationRun { id: string; created_at: number; tool: string; operation: string; inputs: Record<string, unknown>; result: Record<string, unknown>; provenance: string; project_id: string | null; project_snapshot?: Record<string, unknown> | null }
export interface ActivityEvent { seq: number; id: string; created_at: number; action: string; subject_type: string; subject_id: string | null; outcome: string; details: Record<string, string | number> }
export interface EngineeringBranchPayload { schema: 'helqix.engineering-branch.v1'; name: string; description: string; base_reality_revision: number; changes: Array<{ component_id: string; property_name: string; value: string | number | boolean; unit?: string }> }
export interface BoundedSearchCandidate { current_a: number; state: 'completed' | 'failed'; branch_id?: string | null; run_id?: string | null; duration_h?: number | null; discharged_capacity_ah?: number | null; final_voltage_v?: number | null; error?: string | null }
export interface BoundedSearchPayload { schema: 'helqix.bounded-search.v1'; name: string; status: 'completed' | 'partial' | 'failed'; base_reality_revision: number; binding_id: string; variable: { component_id: string; property_name: 'current_a'; unit: 'A'; values: number[] }; objective: { metric: 'discharged_capacity_ah'; direction: 'maximize' }; constraint: { metric: 'duration_h'; operator: '>='; value_h: number | null }; candidate_count: number; completed_count: number; best_evaluated_candidate_a: number | null; candidates: BoundedSearchCandidate[]; applicability: string }
export interface VerificationRecord extends Resource<{ schema?: string; requirement_id?: string; requirementTitle?: string; verdict?: 'pass' | 'fail' | 'inconclusive'; observedValue?: string; observed_value?: number | null; conditionFormula?: string; source_run_id?: string; assurance?: string }> {}
export interface ComponentTypeDefinition { schema: 'helqix.component-type.v1'; id: string; label: string; category: string; domains: string[]; description: string; supportedModels: Array<{ modelId: string; version: string; scope: string; requiredProperties: string[] }> }
export interface ComponentTypeCatalog { schema: 'helqix.component-type-catalog.v1'; domain: string; entries: ComponentTypeDefinition[]; scope: string }

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      ...init,
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', ...init.headers },
    });
  } catch {
    throw new Error('HELQIX service is unreachable. Start the local gateway and try again.');
  }
  if (!response.ok) {
    let detail = `Request failed (${response.status})`;
    let runId: string | undefined;
    try {
      const body = await response.json();
      if (typeof body.detail === 'string') detail = body.detail;
      else if (body.detail && typeof body.detail === 'object') {
        if (typeof body.detail.message === 'string') detail = body.detail.message;
        if (typeof body.detail.run_id === 'string') {
          const recordedRunId = body.detail.run_id as string;
          runId = recordedRunId;
          detail += ` · recorded attempt ${recordedRunId.slice(0, 8)}`;
        }
      }
    } catch { /* Keep the status-based message. */ }
    throw new HelqixRequestError(detail, runId);
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

export const api = {
  health: () => request<{ status: string }>('/health/live'),
  me: () => request<{ user: User }>('/v1/auth/me'),
  login: (email: string, password: string) => request<{ user: User }>('/v1/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
  signup: (name: string, email: string, password: string, organization: string) => request<{ user: User }>('/v1/auth/signup', { method: 'POST', body: JSON.stringify({ name, email, password, organization }) }),
  logout: () => request<void>('/v1/auth/logout', { method: 'POST' }),
  projects: () => request<{ data: Resource<ProjectPayload>[] }>('/v1/projects?limit=100'),
  createProject: (payload: ProjectPayload) => request<{ data: Resource<ProjectPayload> }>('/v1/projects', { method: 'POST', body: JSON.stringify({ payload }) }),
  updateProject: (project: Resource<ProjectPayload>, payload: ProjectPayload) => request<{ data: Resource<ProjectPayload> }>(`/v1/projects/${project.id}`, { method: 'PUT', headers: { 'If-Match': `"${project.version}"` }, body: JSON.stringify({ payload }) }),
  integrations: () => request<{ integrations: Array<{ id: string; label: string; status: string; available: boolean; note?: string }> }>('/v1/integrations'),
  componentTypes: (domain?: string) => request<ComponentTypeCatalog>(`/v1/libraries/component-types${domain ? `?domain=${encodeURIComponent(domain)}` : ''}`),
  parseIntent: (intent: string, domain?: string, assisted = false) => request<IntentAnalysis>('/v1/intelligence/intent/parse', { method: 'POST', body: JSON.stringify({ intent, domain, ...(assisted ? { assisted: true } : {}) }) }),
  runs: (projectId: string) => request<{ runs: IntegrationRun[] }>(`/v1/integrations/runs?limit=100&project_id=${encodeURIComponent(projectId)}`),
  activity: (projectId: string) => request<{ events: ActivityEvent[]; next_cursor: number; has_more: boolean }>(`/v1/projects/${encodeURIComponent(projectId)}/activity?limit=50`),
  verifications: (projectId: string) => request<{ data: VerificationRecord[] }>(`/v1/verifications?project_id=${encodeURIComponent(projectId)}&limit=100`),
  resources: <T,>(resource: 'branches' | 'searches', projectId: string) => request<{ data: Resource<T>[] }>(`/v1/${resource}?project_id=${encodeURIComponent(projectId)}&limit=100`),
  createResource: <T,>(resource: 'branches' | 'searches', projectId: string, payload: T) => request<{ data: Resource<T> }>(`/v1/${resource}`, { method: 'POST', body: JSON.stringify({ project_id: projectId, payload }) }),
  execute: (toolId: string, operation: string, inputs: Record<string, unknown>) => request<{ run: IntegrationRun }>(`/v1/integrations/${encodeURIComponent(toolId)}/execute`, { method: 'POST', body: JSON.stringify({ operation, inputs }) }),
  exportEvidence: async (projectId: string) => {
    const response = await fetch(`${API_BASE}/v1/projects/${encodeURIComponent(projectId)}/evidence/export`, { credentials: 'include' });
    if (!response.ok) {
      let detail = `Evidence export failed (${response.status})`;
      try { const body = await response.json(); if (typeof body.detail === 'string') detail = body.detail; } catch { /* Keep status message. */ }
      throw new Error(detail);
    }
    return response.blob();
  },
};
