import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "../state/auth";
import { ApiError } from "../api/errors";
import "./auth.css";

export function SignupPage() {
  const { signup } = useAuth();
  const navigate = useNavigate();
  const [form, setForm] = useState({ name: "", email: "", password: "", confirm: "", organization: "" });
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);

  function set(field: keyof typeof form) {
    return (v: string) => setForm((f) => ({ ...f, [field]: v }));
  }

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    const fe: Record<string, string> = {};
    if (!form.name.trim()) fe.name = "Name is required.";
    if (!/.+@.+\..+/.test(form.email)) fe.email = "Enter a valid email address.";
    if (form.password.length < 8) fe.password = "Password must be at least 8 characters.";
    if (form.password !== form.confirm) fe.confirm = "Passwords do not match.";
    setFieldErrors(fe);
    if (Object.keys(fe).length > 0) return;
    setBusy(true);
    try {
      await signup(form.name, form.email, form.password, form.organization);
      navigate("/login", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.userMessage : "Account creation failed.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth">
      <aside className="auth__brand">
        <Link to="/" className="hq-header__brand">HELQIX</Link>
        <p className="auth__statement">Create your engineering reality. Intent in — verified evidence out.</p>
        <span className="label">Executable-Reality Platform</span>
      </aside>
      <div className="auth__panelwrap">
        <form className="auth__panel" onSubmit={onSubmit}>
          <h1>Create account</h1>
          {error ? <div className="hq-banner hq-banner--error" role="alert" style={{ marginBottom: "var(--hq-sp-4)" }}>{error}</div> : null}
          <label className="hq-field"><span className="label">Name</span>
            <input className="hq-input" value={form.name} onChange={(e) => set("name")(e.target.value)} autoComplete="name" />
            {fieldErrors.name ? <span className="hq-field__error">{fieldErrors.name}</span> : null}</label>
          <label className="hq-field"><span className="label">Email</span>
            <input className="hq-input" type="email" value={form.email} onChange={(e) => set("email")(e.target.value)} autoComplete="email" />
            {fieldErrors.email ? <span className="hq-field__error">{fieldErrors.email}</span> : null}</label>
          <label className="hq-field"><span className="label">Organization</span>
            <input className="hq-input" value={form.organization} onChange={(e) => set("organization")(e.target.value)} autoComplete="organization" /></label>
          <label className="hq-field"><span className="label">Password</span>
            <input className="hq-input" type="password" value={form.password} onChange={(e) => set("password")(e.target.value)} autoComplete="new-password" />
            {fieldErrors.password ? <span className="hq-field__error">{fieldErrors.password}</span> : null}</label>
          <label className="hq-field"><span className="label">Confirm password</span>
            <input className="hq-input" type="password" value={form.confirm} onChange={(e) => set("confirm")(e.target.value)} autoComplete="new-password" />
            {fieldErrors.confirm ? <span className="hq-field__error">{fieldErrors.confirm}</span> : null}</label>
          <button className="hq-button hq-button--primary" style={{ width: "100%", justifyContent: "center" }} disabled={busy}>
            {busy ? "CREATING…" : "CREATE ACCOUNT"}
          </button>
          <p className="auth__foot">Already have an account? <Link to="/login">Sign in</Link></p>
        </form>
      </div>
    </div>
  );
}
