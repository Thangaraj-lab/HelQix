/**
 * Typed resource API — one function per Layer-22 contract slot.
 * Components/hooks consume ONLY this module (never `client.request` directly).
 */

import { endpoints, request } from "./client";
import type {
  ActionDescriptor,
  AuthoritativeState,
  BranchNode,
  CandidateRow,
  CausalGraphView,
  ComparisonTable,
  CreateBranchRequest,
  CreateProjectRequest,
  EngineeringModel,
  EvidenceRecord,
  ExecutionSummary,
  ExecuteActionRequest,
  ExecuteActionResponse,
  FailureRecord,
  IntentStatement,
  MetricDescriptor,
  MetricSeries,
  ProjectSummary,
  RealitySummary,
  SearchDefinition,
  SearchRunStatus,
  TimelineItem,
  VerificationCheck,
} from "./types";

/* Projects / Realities */
export const listProjects = () =>
  request<ProjectSummary[]>(endpoints.projects, { tolerateUnavailable: true });
export const getProject = (id: string) =>
  request<ProjectSummary>(endpoints.project(id), { tolerateUnavailable: true });
export const createProject = (body: CreateProjectRequest) =>
  request<ProjectSummary>(endpoints.projects, { method: "POST", body });
export const listRealities = (projectId: string) =>
  request<RealitySummary[]>(endpoints.realities(projectId), { tolerateUnavailable: true });

/* Intent / Model */
export const getIntent = (projectId: string) =>
  request<IntentStatement>(endpoints.intent(projectId), { tolerateUnavailable: true });
export const saveIntent = (projectId: string, text: string) =>
  request<IntentStatement>(endpoints.intent(projectId), { method: "PUT", body: { text } });
export const getModel = (projectId: string) =>
  request<EngineeringModel>(endpoints.model(projectId), { tolerateUnavailable: true });

/* State (Layer 04 via API) */
export const getCurrentState = (realityId: string) =>
  request<AuthoritativeState>(endpoints.state(realityId), { tolerateUnavailable: true });

/* Execution (Layer 05) */
export const listExecutions = (realityId: string) =>
  request<ExecutionSummary[]>(endpoints.executions(realityId), { tolerateUnavailable: true });
export const runExecution = (executionId: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "run"), { method: "POST" });
export const pauseExecution = (executionId: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "pause"), { method: "POST" });
export const stepExecution = (executionId: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "step"), { method: "POST" });
export const stopExecution = (executionId: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "stop"), { method: "POST" });
export const replayExecution = (executionId: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "replay"), { method: "POST" });
export const rewindExecution = (executionId: string, toNanos: string) =>
  request<ExecutionSummary>(endpoints.executionControl(executionId, "rewind"), {
    method: "POST",
    body: { to_nanos: toNanos },
  });
export const listAvailableActions = (executionId: string) =>
  request<ActionDescriptor[]>(endpoints.actions(executionId), { tolerateUnavailable: true });
export const executeAction = (body: ExecuteActionRequest) =>
  request<ExecuteActionResponse>(endpoints.actions(body.executionId), { method: "POST", body });

/* Timeline / Observation */
export const getTimeline = (executionId: string) =>
  request<TimelineItem[]>(endpoints.timeline(executionId), { tolerateUnavailable: true });
export const listMetrics = (executionId: string) =>
  request<MetricDescriptor[]>(endpoints.metrics(executionId), { tolerateUnavailable: true });
export const getMetricSeries = (executionId: string, metricId: string) =>
  request<MetricSeries>(endpoints.metricSeries(executionId, metricId), {
    tolerateUnavailable: true,
  });

/* Failure / Causality */
export const listFailures = (executionId: string) =>
  request<FailureRecord[]>(endpoints.failures(executionId), { tolerateUnavailable: true });
export const getCausalGraph = (failureId: string) =>
  request<CausalGraphView>(endpoints.causalGraph(failureId), { tolerateUnavailable: true });

/* Branching / Compare */
export const listBranches = (projectId: string) =>
  request<BranchNode[]>(endpoints.branches(projectId), { tolerateUnavailable: true });
export const createBranch = (body: CreateBranchRequest) =>
  request<BranchNode>(endpoints.branches(body.projectId), { method: "POST", body });
export const compareBranches = (projectId: string, left: string, right: string) =>
  request<ComparisonTable>(`${endpoints.compare(projectId)}?left=${encodeURIComponent(left)}&right=${encodeURIComponent(right)}`, {
    tolerateUnavailable: true,
  });

/* Search */
export const startSearch = (projectId: string, definition: SearchDefinition) =>
  request<SearchRunStatus>(endpoints.search(projectId), { method: "POST", body: definition });
export const getSearchStatus = (runId: string) =>
  request<SearchRunStatus>(endpoints.searchRun(runId), { tolerateUnavailable: true });
export const listCandidates = (runId: string) =>
  request<CandidateRow[]>(endpoints.candidates(runId), { tolerateUnavailable: true });

/* Verification / Evidence */
export const getVerification = (projectId: string) =>
  request<VerificationCheck[]>(endpoints.verification(projectId), { tolerateUnavailable: true });
export const listEvidence = (projectId: string) =>
  request<EvidenceRecord[]>(endpoints.evidence(projectId), { tolerateUnavailable: true });
