/** Persistent authenticated application shell: header + grouped navigation. */
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { useEffect, useState } from "react";
import { useAuth } from "../state/auth";
import { useAppState } from "../state/AppState";
import { Badge } from "../components/common";
import "./shell.css";

interface NavItem { readonly to: string; readonly label: string; readonly icon: string }
interface NavGroup { readonly label: string; readonly items: NavItem[] }

/** Full-chrome nav is for the Studio workflow pages; Home renders its own minimal header. */
const CHROMELESS_PATHS = ["/home"];

const NAV_GROUPS: NavGroup[] = [
  {
    label: "Workspace",
    items: [
      { to: "/home", label: "Home", icon: "▦" },
      { to: "/projects", label: "Projects", icon: "⌗" },
    ],
  },
  {
    label: "Engineering",
    items: [
      { to: "intent", label: "Intent", icon: "◎" },
      { to: "model", label: "Model", icon: "⬡" },
      { to: "architecture", label: "Architecture", icon: "⇄" },
      { to: "reality", label: "Reality", icon: "◈" },
    ],
  },
  {
    label: "Runtime",
    items: [
      { to: "execution", label: "Execution", icon: "▶" },
      { to: "observation", label: "Observation", icon: "∿" },
      { to: "timeline", label: "Timeline", icon: "→|" },
    ],
  },
  {
    label: "Analysis",
    items: [
      { to: "failure", label: "Failure", icon: "⚠" },
      { to: "causal", label: "Causality", icon: "↯" },
      { to: "branches", label: "Branches", icon: "⑃" },
      { to: "compare", label: "Compare", icon: "⇋" },
      { to: "search", label: "Search", icon: "⌕" },
    ],
  },
  {
    label: "Assurance",
    items: [
      { to: "verify", label: "Verification", icon: "✓" },
      { to: "evidence", label: "Evidence", icon: "§" },
    ],
  },
];

export function AppShell() {
  const { user, logout } = useAuth();
  const { state, dispatch } = useAppState();
  const location = useLocation();
  const [mobileNavOpen, setMobileNavOpen] = useState(false);

  useEffect(() => setMobileNavOpen(false), [location.pathname]);

  // Keep presentation context in sync with the route (project scope).
  const projectMatch = location.pathname.match(/^\/projects\/([^/]+)/);
  const routeProjectId = projectMatch?.[1] ? decodeURIComponent(projectMatch[1]) : null;
  useEffect(() => {
    if (routeProjectId && routeProjectId !== state.projectId) {
      dispatch({ type: "context/project", projectId: routeProjectId });
    }
  }, [routeProjectId, state.projectId, dispatch]);

  const engineeringBase = routeProjectId ? `/projects/${routeProjectId}` : null;

  if (CHROMELESS_PATHS.includes(location.pathname)) {
    return (
      <div className="app-shell app-shell--chromeless">
        <main className="app-shell__main">
          <Outlet />
        </main>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <header className="hq-header">
        <button
          className="hq-button hq-button--ghost hq-button--sm"
          aria-label="Toggle navigation"
          style={{ display: "none" }}
          id="nav-toggle"
          onClick={() => setMobileNavOpen((o) => !o)}
        >
          ☰
        </button>
        <span className="hq-header__brand">HELQIX</span>
        <nav className="hq-header__crumbs" aria-label="Breadcrumb">
          <span>/</span>
          <span>{routeProjectId ? `Project ${routeProjectId.slice(0, 8)}` : "Workspace"}</span>
          {state.realityId ? (
            <>
              <span>/</span>
              <span>Reality {state.realityId.slice(0, 8)}</span>
            </>
          ) : null}
        </nav>
        <div className="hq-header__spacer" />
        <div className="hq-header__status">
          <Badge tone={navigator.onLine ? "success" : "failure"}>{navigator.onLine ? "Connected" : "Offline"}</Badge>
          <Badge tone={state.executionId ? "active" : "normal"}>
            {state.executionId ? "Execution active" : "No execution"}
          </Badge>
          <button className="hq-button hq-button--ghost hq-button--sm" onClick={logout}>
            {user?.email ?? "Sign out"}
          </button>
        </div>
      </header>

      <nav className="hq-nav" data-open={mobileNavOpen} aria-label="Primary">
        {NAV_GROUPS.map((group) => (
          <div className="hq-nav__group" key={group.label}>
            <div className="label hq-nav__group-label">{group.label}</div>
            {group.items.map((item) => {
              const to =
                group.label === "Workspace" || item.to === "/settings"
                  ? item.to
                  : engineeringBase
                    ? `${engineeringBase}/${item.to}`
                    : "/projects";
              return (
                <NavLink
                  key={`${group.label}:${item.to}`}
                  className="hq-nav__item"
                  to={to}
                  title={!engineeringBase && group.label !== "Workspace" ? "Open a project first" : undefined}
                >
                  <span className="hq-nav__icon" aria-hidden>{item.icon}</span>
                  <span>{item.label}</span>
                </NavLink>
              );
            })}
          </div>
        ))}
        <div className="hq-nav__group">
          <div className="label hq-nav__group-label">System</div>
          <NavLink className="hq-nav__item" to="/settings">
            <span className="hq-nav__icon" aria-hidden>⚙</span>
            <span>Settings</span>
          </NavLink>
        </div>
      </nav>

      <main className="app-shell__main">
        <Outlet />
      </main>
    </div>
  );
}
