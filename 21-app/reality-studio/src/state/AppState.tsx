/**
 * Frontend presentation state ONLY.
 * --------------------------------------------------------------------------
 * This context deliberately holds no engineering data: no state values, no
 * physics, no execution results. Those always come from the API layer and are
 * cached per-view via useAsyncResource. What lives here is navigation context
 * (selected project/reality/execution/component), session, and UI prefs.
 */
import { createContext, useContext, useMemo, useReducer, type ReactNode } from "react";

export interface SessionUser {
  readonly email: string;
  readonly name: string;
}

export interface UiState {
  readonly user: SessionUser | null;
  readonly projectId: string | null;
  readonly realityId: string | null;
  readonly executionId: string | null;
  readonly selectedComponentId: string | null;
  readonly navCollapsed: boolean;
}

type Action =
  | { type: "session/set"; user: SessionUser | null }
  | { type: "context/project"; projectId: string | null }
  | { type: "context/reality"; realityId: string | null }
  | { type: "context/execution"; executionId: string | null }
  | { type: "context/selectComponent"; componentId: string | null }
  | { type: "ui/toggleNav" };

const initialState: UiState = {
  user: null,
  projectId: null,
  realityId: null,
  executionId: null,
  selectedComponentId: null,
  navCollapsed: false,
};

function reducer(state: UiState, action: Action): UiState {
  switch (action.type) {
    case "session/set":
      return { ...state, user: action.user };
    case "context/project":
      return { ...state, projectId: action.projectId, realityId: null, executionId: null };
    case "context/reality":
      return { ...state, realityId: action.realityId, executionId: null };
    case "context/execution":
      return { ...state, executionId: action.executionId };
    case "context/selectComponent":
      return { ...state, selectedComponentId: action.componentId };
    case "ui/toggleNav":
      return { ...state, navCollapsed: !state.navCollapsed };
  }
}

interface AppContextValue {
  state: UiState;
  dispatch: React.Dispatch<Action>;
}

const AppContext = createContext<AppContextValue | null>(null);

export function AppStateProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, initialState);
  const value = useMemo(() => ({ state, dispatch }), [state]);
  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useAppState(): AppContextValue {
  const ctx = useContext(AppContext);
  if (!ctx) throw new Error("useAppState must be used inside AppStateProvider");
  return ctx;
}
