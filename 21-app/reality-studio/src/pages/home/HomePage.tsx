/**
 * HELQIX Home — the entry point into the Engineering Reality Studio.
 * One job: start a new engineering project, or open an existing one.
 * Real backend data only; empty means "No projects yet" — never fabricated rows.
 */
import { useState } from "react";
import { Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { listProjects } from "../../api/resources";
import { ApiError } from "../../api/errors";
import { ErrorPanel, LoadingBlock, UnavailablePanel } from "../../components/common";
import { CreateProjectModal } from "./CreateProjectModal";
import "./home.css";

export function HomePage() {
  const [showCreate, setShowCreate] = useState(false);
  const projects = useAsyncResource(() => listProjects(), []);

  return (
    <div className="home">
      <header className="home__header">
        <span className="home__brand" aria-label="HELQIX Engineering Reality Studio">
          HELQIX
        </span>
        <nav className="home__nav" aria-label="Primary">
          <Link to="/projects" className="home__nav-link">
            Projects
          </Link>
          <Link to="/settings" className="home__nav-icon" aria-label="Settings" title="Settings">
            ⚙
          </Link>
        </nav>
      </header>

      <main className="home__main">
        <h1 className="home__wordmark" aria-label="HELQIX">
          {"HELQIX".split("").join(" ")}
          <span className="home__tagline">Engineering Reality Studio</span>
          <span className="home__pipeline">Build&thinsp;·&thinsp;Execute&thinsp;·&thinsp;Understand&thinsp;·&thinsp;Verify</span>
        </h1>

        <button
          type="button"
          className="home__new-project"
          onClick={() => setShowCreate(true)}
        >
          <span aria-hidden>+&nbsp;&nbsp;</span>NEW PROJECT
        </button>

        <section className="home__recent" aria-labelledby="home-recent-heading">
          <h2 className="label home__recent-heading" id="home-recent-heading">
            Recent Projects
          </h2>
          {projects.phase === "idle" || projects.phase === "loading" ? (
            <LoadingBlock rows={2} />
          ) : projects.phase === "unavailable" ? (
            <UnavailablePanel feature="Project listing" detail="GET /api/projects" />
          ) : projects.phase === "error" ? (
            <ErrorPanel error={projects.error} onRetry={projects.reload} />
          ) : projects.data === null || projects.data.length === 0 ? (
            <p className="home__empty" role="status">
              No projects yet
            </p>
          ) : (
            <ul className="home__project-list">
              {projects.data.slice(0, 5).map((p) => (
                <li key={p.id}>
                  <Link className="home__project-row" to={`/projects/${encodeURIComponent(p.id)}`}>
                    <span className="home__project-name">{p.name}</span>
                    <span className="home__project-domain">{p.domain}</span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
          {projects.phase === "ready" && projects.data !== null && projects.data.length > 0 ? (
            <Link to="/projects" className="home__all-projects">
              All projects
            </Link>
          ) : null}
        </section>
      </main>

      {showCreate ? <CreateProjectModal onClose={() => setShowCreate(false)} /> : null}
      {/* Screen-reader context for the honest capability gap surfaced above. */}
      <span className="sr-only">
        {projects.phase === "unavailable" && projects.error instanceof ApiError
          ? `Backend capability unavailable: ${projects.error.userMessage}`
          : ""}
      </span>
    </div>
  );
}
