/**
 * Authentication boundary.
 * --------------------------------------------------------------------------
 * Login/signup are delegated to the Layer-22 auth endpoints. If the backend
 * does not implement them (capability absent or route missing), the UI shows
 * an honest "authentication service unavailable" state — it NEVER fabricates
 * a fake logged-in user. The token lives in memory; sessionStorage is used
 * only when the user explicitly chooses "Remember me".
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { request, setAccessToken, endpoints } from "../api/client";
import { ApiError } from "../api/errors";
import type { SessionUser } from "./AppState";

interface LoginResponse {
  readonly token: string;
  readonly user: SessionUser;
}

export interface AuthContextValue {
  readonly user: SessionUser | null;
  readonly status: "checking" | "signedOut" | "signedIn" | "unavailable";
  login(email: string, password: string, remember: boolean): Promise<void>;
  signup(name: string, email: string, password: string, organization: string): Promise<void>;
  logout(): void;
}

const AuthContext = createContext<AuthContextValue | null>(null);
const STORAGE_KEY = "helqix.session";

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<SessionUser | null>(null);
  const [status, setStatus] = useState<AuthContextValue["status"]>("checking");

  const restore = useCallback(() => {
    try {
      const raw = sessionStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as LoginResponse;
        if (typeof parsed.token === "string" && parsed.user?.email) {
          setAccessToken(parsed.token);
          setUser(parsed.user);
          setStatus("signedIn");
          return;
        }
      }
    } catch {
      /* corrupt storage → ignore */
    }
    setStatus("signedOut");
  }, []);

  useEffect(() => {
    restore();
  }, [restore]);

  const login = useCallback(async (email: string, password: string, remember: boolean) => {
    try {
      const res = await request<LoginResponse>(endpoints.login, {
        method: "POST",
        body: { email, password },
      });
      setAccessToken(res.token);
      setUser(res.user);
      setStatus("signedIn");
      if (remember) sessionStorage.setItem(STORAGE_KEY, JSON.stringify(res));
      else sessionStorage.removeItem(STORAGE_KEY);
    } catch (error) {
      if (error instanceof ApiError && error.category === "BACKEND_UNAVAILABLE") {
        setStatus("unavailable");
      }
      throw error;
    }
  }, []);

  const signup = useCallback(
    async (name: string, email: string, password: string, organization: string) => {
      try {
        await request(endpoints.signup, {
          method: "POST",
          body: { name, email, password, organization },
        });
      } catch (error) {
        if (error instanceof ApiError && error.category === "BACKEND_UNAVAILABLE") {
          setStatus("unavailable");
        }
        throw error;
      }
    },
    [],
  );

  const logout = useCallback(() => {
    setAccessToken(null);
    sessionStorage.removeItem(STORAGE_KEY);
    setUser(null);
    setStatus("signedOut");
  }, []);

  const value = useMemo(
    () => ({ user, status, login, signup, logout }),
    [user, status, login, signup, logout],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside AuthProvider");
  return ctx;
}
