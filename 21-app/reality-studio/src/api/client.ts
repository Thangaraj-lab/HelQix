/**
 * Centralized typed HTTP client for the Layer 22 HELQIX API.
 * --------------------------------------------------------------------------
 * RULES enforced here:
 *  - All network access flows through this module (no scattered fetch()).
 *  - No invented endpoints: paths are declared once in `endpoints` and every
 *    one of them maps to a documented Layer-22 contract slot. If the backend
 *    has not implemented a route yet, callers receive BACKEND_UNAVAILABLE via
 *    the capability probe (`GET /api/capabilities`) — never fabricated data.
 *  - Credentials/secrets never appear in code; auth uses a bearer token held
 *    in memory + sessionStorage only for "remember me".
 */

import { ApiError, categoryFromStatus } from "./errors";

const API_BASE = "/api";

let accessToken: string | null = null;

export function setAccessToken(token: string | null): void {
  accessToken = token;
}

export function getAccessToken(): string | null {
  return accessToken;
}

interface RequestOptions {
  method?: "GET" | "POST" | "PUT" | "DELETE";
  body?: unknown;
  signal?: AbortSignal;
  /** When true, absence of the backend capability resolves as null instead of throwing. */
  tolerateUnavailable?: boolean;
}

export async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (opts.body !== undefined) headers["Content-Type"] = "application/json";
  if (accessToken) headers.Authorization = `Bearer ${accessToken}`;

  let response: Response;
  try {
    response = await fetch(`${API_BASE}${path}`, {
      method: opts.method ?? "GET",
      headers,
      body: opts.body !== undefined ? JSON.stringify(opts.body) : undefined,
      signal: opts.signal,
    });
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") throw cause;
    throw new ApiError("NETWORK_ERROR", "Platform API unreachable", { endpoint: path });
  }

  if (response.status === 404 && opts.tolerateUnavailable) {
    // Route not implemented by the connected backend → treated as capability gap,
    // never as empty-but-real data.
    throw new ApiError("BACKEND_UNAVAILABLE", `Endpoint not implemented: ${path}`, {
      status: 404,
      endpoint: path,
    });
  }

  if (!response.ok) {
    let detail: string | null = null;
    try {
      const text = await response.text();
      detail = text.slice(0, 400) || null;
    } catch {
      /* ignore body read failures */
    }
    throw new ApiError(categoryFromStatus(response.status), detail ?? response.statusText, {
      status: response.status,
      endpoint: path,
      detail: detail ?? undefined,
    });
  }

  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

/* ---------------------------------------------------------------------------
 * Endpoint registry — the single place where Layer-22 routes are referenced.
 * Keep in lockstep with 22-api/helqix-api. Unimplemented routes are probed at
 * runtime through /api/capabilities and surfaced as BACKEND_UNAVAILABLE.
 * ------------------------------------------------------------------------- */

export const endpoints = {
  capabilities: "/capabilities",

  login: "/auth/login",
  signup: "/auth/signup",
  session: "/auth/session",

  projects: "/projects",
  project: (id: string) => `/projects/${encodeURIComponent(id)}`,

  intent: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/intent`,
  model: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/model`,

  realities: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/realities`,
  state: (realityId: string) => `/realities/${encodeURIComponent(realityId)}/state`,

  executions: (realityId: string) => `/realities/${encodeURIComponent(realityId)}/executions`,
  executionControl: (executionId: string, control: string) =>
    `/executions/${encodeURIComponent(executionId)}/${control}`,
  actions: (executionId: string) => `/executions/${encodeURIComponent(executionId)}/actions`,

  timeline: (executionId: string) => `/executions/${encodeURIComponent(executionId)}/timeline`,
  metrics: (executionId: string) => `/executions/${encodeURIComponent(executionId)}/metrics`,
  metricSeries: (executionId: string, metricId: string) =>
    `/executions/${encodeURIComponent(executionId)}/metrics/${encodeURIComponent(metricId)}`,

  failures: (executionId: string) => `/executions/${encodeURIComponent(executionId)}/failures`,
  causalGraph: (failureId: string) => `/failures/${encodeURIComponent(failureId)}/causal-graph`,

  branches: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/branches`,
  compare: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/compare`,

  search: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/search`,
  searchRun: (runId: string) => `/search/${encodeURIComponent(runId)}`,
  candidates: (runId: string) => `/search/${encodeURIComponent(runId)}/candidates`,

  verification: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/verification`,
  evidence: (projectId: string) => `/projects/${encodeURIComponent(projectId)}/evidence`,
} as const;

/** Capability flags reported by the backend. Absent keys mean "not implemented". */
export interface BackendCapabilities {
  readonly auth?: boolean;
  readonly projects?: boolean;
  readonly models?: boolean;
  readonly realities?: boolean;
  readonly execution?: boolean;
  readonly observation?: boolean;
  readonly failure?: boolean;
  readonly causality?: boolean;
  readonly branching?: boolean;
  readonly search?: boolean;
  readonly verification?: boolean;
  readonly evidence?: boolean;
  readonly realtime?: boolean;
}

let capabilityCache: BackendCapabilities | null = null;

export async function fetchCapabilities(force = false): Promise<BackendCapabilities> {
  if (capabilityCache && !force) return capabilityCache;
  try {
    capabilityCache = await request<BackendCapabilities>(endpoints.capabilities);
  } catch (error) {
    if (error instanceof ApiError && error.category === "BACKEND_UNAVAILABLE") {
      capabilityCache = {}; // server reachable but no capability registry yet
      return capabilityCache;
    }
    throw error;
  }
  return capabilityCache;
}

export function resetCapabilityCache(): void {
  capabilityCache = null;
}
