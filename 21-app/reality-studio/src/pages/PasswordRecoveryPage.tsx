import { useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { request, endpoints } from "../api/client";
import { ApiError } from "../api/errors";
import "./auth.css";

export function PasswordRecoveryPage() {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<"idle" | "sent" | "unavailable" | "error">("idle");
  const [message, setMessage] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    try {
      await request(endpoints.login.replace("/login", "/password-recovery"), { method: "POST", body: { email } });
      setState("sent");
    } catch (err) {
      if (err instanceof ApiError && (err.category === "BACKEND_UNAVAILABLE" || err.status === 404)) setState("unavailable");
      else { setState("error"); setMessage(err instanceof ApiError ? err.userMessage : "Recovery request failed."); }
    }
  }

  return (
    <div className="auth">
      <aside className="auth__brand">
        <Link to="/" className="hq-header__brand">HELQIX</Link>
        <p className="auth__statement">Recover access to your engineering realities.</p>
        <span className="label">Executable-Reality Platform</span>
      </aside>
      <div className="auth__panelwrap">
        <form className="auth__panel" onSubmit={onSubmit}>
          <h1>Password recovery</h1>
          {state === "sent" ? <div className="hq-banner hq-banner--info" role="status">If the account exists, a recovery link has been sent.</div> : null}
          {state === "unavailable" ? <div className="hq-banner hq-banner--warn" role="alert">The recovery service is not available on the connected backend yet.</div> : null}
          {state === "error" && message ? <div className="hq-banner hq-banner--error" role="alert">{message}</div> : null}
          <label className="hq-field"><span className="label">Email</span>
            <input className="hq-input" type="email" required value={email} onChange={(e) => setEmail(e.target.value)} /></label>
          <button className="hq-button hq-button--primary" style={{ width: "100%", justifyContent: "center" }}>SEND RECOVERY LINK</button>
          <p className="auth__foot"><Link to="/login">Back to sign in</Link></p>
        </form>
      </div>
    </div>
  );
}
