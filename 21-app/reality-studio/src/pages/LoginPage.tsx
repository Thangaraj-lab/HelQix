import { useState, type FormEvent } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { useAuth } from "../state/auth";
import { ApiError } from "../api/errors";
import "./auth.css";

export function LoginPage() {
  const { login, status } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const from = (location.state as { from?: string } | null)?.from ?? "/workspace";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await login(email, password, remember);
      navigate(from, { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.userMessage : "Sign-in failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth">
      <aside className="auth__brand">
        <Link to="/" className="hq-header__brand">HELQIX</Link>
        <p className="auth__statement">
          Engineering reality, <strong>made executable</strong>. Observe state, execute actions, branch from
          exact moments, and verify with evidence.
        </p>
        <span className="label">Executable-Reality Platform</span>
      </aside>
      <div className="auth__panelwrap">
        <form className="auth__panel" onSubmit={onSubmit}>
          <h1>Welcome back</h1>
          {status === "unavailable" ? (
            <div className="hq-banner hq-banner--warn" role="alert" style={{ marginBottom: "var(--hq-sp-4)" }}>
              The authentication service is not available on the connected backend yet.
            </div>
          ) : null}
          {error ? <div className="hq-banner hq-banner--error" role="alert" style={{ marginBottom: "var(--hq-sp-4)" }}>{error}</div> : null}
          <label className="hq-field">
            <span className="label">Email</span>
            <input className="hq-input" type="email" required autoComplete="email" value={email} onChange={(e) => setEmail(e.target.value)} />
          </label>
          <label className="hq-field">
            <span className="label">Password</span>
            <input className="hq-input" type="password" required autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
          </label>
          <div className="auth__row">
            <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
              <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} /> Remember me
            </label>
            <Link to="/recover">Forgot password?</Link>
          </div>
          <button className="hq-button hq-button--primary" style={{ width: "100%", justifyContent: "center" }} disabled={busy}>
            {busy ? "SIGNING IN…" : "SIGN IN"}
          </button>
          <p className="auth__foot">
            Don&apos;t have an account? <Link to="/signup">Create account</Link>
          </p>
        </form>
      </div>
    </div>
  );
}
