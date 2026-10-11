/**
 * Reality data orchestration hook.
 * Fetches model + state + executions for a project's reality and exposes a
 * `refresh()` that re-reads AUTHORITATIVE state from the backend after any
 * execution command or action — the UI never mutates engineering state locally.
 */
import { useCallback, useEffect, useState } from "react";
import { getCurrentState, getModel, listAvailableActions, listExecutions, listRealities } from "../api/resources";
import { useAsyncResource } from "./useAsyncResource";
import type { ActionDescriptor, AuthoritativeState, ExecutionSummary } from "../api/types";

export function useRealityData(projectId: string) {
  const realities = useAsyncResource(() => listRealities(projectId), [projectId]);
  const model = useAsyncResource(() => getModel(projectId), [projectId]);

  const realityId = realities.data?.[0]?.id ?? null;
  const [execution, setExecution] = useState<ExecutionSummary | null>(null);
  const [liveState, setLiveState] = useState<AuthoritativeState | null>(null);
  const [stateUnavailable, setStateUnavailable] = useState(false);
  const [actions, setActions] = useState<ActionDescriptor[]>([]);
  const [actionsUnavailable, setActionsUnavailable] = useState(false);

  // Load executions for the first reality.
  useEffect(() => {
    if (!realityId) return;
    let cancelled = false;
    listExecutions(realityId)
      .then((list) => {
        if (cancelled) return;
        if (list.length > 0) setExecution(list[0] ?? null);
      })
      .catch(() => { /* unavailable handled by controls */ });
    return () => { cancelled = true; };
  }, [realityId]);

  const refresh = useCallback(async () => {
    if (!realityId) return;
    try {
      const s = await getCurrentState(realityId);
      setLiveState(s);
      setStateUnavailable(false);
    } catch (error) {
      setStateUnavailable(true);
    }
    if (execution) {
      try {
        const execs = await listExecutions(realityId);
        const updated = execs.find((e) => e.id === execution.id) ?? execs[0] ?? null;
        setExecution(updated);
      } catch { /* keep last known */ }
    }
  }, [realityId, execution]);

  useEffect(() => { void refresh(); }, [refresh]);

  useEffect(() => {
    if (!execution) return;
    let cancelled = false;
    listAvailableActions(execution.id)
      .then((a) => { if (!cancelled) { setActions(a); setActionsUnavailable(false); } })
      .catch(() => { if (!cancelled) setActionsUnavailable(true); });
    return () => { cancelled = true; };
  }, [execution]);

  return {
    realities, model, realityId, execution, setExecution,
    liveState, stateUnavailable, actions, actionsUnavailable, refresh,
  };
}
