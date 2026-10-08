/**
 * useAsyncResource — predictable data-fetching state machine.
 * States: idle → loading → (ready | unavailable | error).
 * "unavailable" is distinct from "error": it means the backend has not
 * implemented the capability yet (BACKEND_UNAVAILABLE) and the UI must show
 * an honest integration gap instead of fake or empty data.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../api/errors";

export type ResourcePhase = "idle" | "loading" | "ready" | "unavailable" | "error";

export interface ResourceState<T> {
  phase: ResourcePhase;
  data: T | null;
  error: ApiError | Error | null;
}

export interface AsyncResourceResult<T> extends ResourceState<T> {
  reload: () => void;
}

export function useAsyncResource<T>(
  fetcher: (() => Promise<T>) | null,
  deps: readonly unknown[],
): AsyncResourceResult<T> {
  const [state, setState] = useState<ResourceState<T>>({ phase: "idle", data: null, error: null });
  const [nonce, setNonce] = useState(0);
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  useEffect(() => {
    if (!fetcherRef.current) {
      setState({ phase: "idle", data: null, error: null });
      return;
    }
    let cancelled = false;
    setState((s) => ({ phase: "loading", data: s.data, error: null }));
    fetcherRef
      .current()
      .then((data) => {
        if (!cancelled) setState({ phase: "ready", data, error: null });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        if (error instanceof ApiError && error.category === "BACKEND_UNAVAILABLE") {
          setState({ phase: "unavailable", data: null, error });
        } else {
          setState({ phase: "error", data: null, error: error instanceof Error ? error : new Error(String(error)) });
        }
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, nonce]);

  const reload = useCallback(() => setNonce((n) => n + 1), []);
  return { ...state, reload };
}
