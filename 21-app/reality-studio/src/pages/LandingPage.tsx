/** Landing page — explains HELQIX in under 30 seconds. */
import { Link } from "react-router-dom";
import { useReveal } from "../hooks/useReveal";
import "./landing.css";

const WORKFLOW = [
  "Intent", "Model", "Reality", "Execute", "Observe", "Failure",
  "Cause", "Branch", "Compare", "Verify", "Evidence",
];

function WorkflowChain() {
  const { ref, visible } = useReveal<HTMLDivElement>();
  return (
    <div className="workflow-chain" ref={ref} aria-label="HELQIX executable-reality workflow">
      {WORKFLOW.map((step, i) => (
        <div
          key={step}
          className={`workflow-chain__row${visible ? " in-view" : ""}`}
          style={{ transitionDelay: `${i * 60}ms` }}
        >
          <span className="workflow-chain__node" aria-hidden />
          <span>{step.toUpperCase()}</span>
          {i < WORKFLOW.length - 1 ? <span className="workflow-chain__arrow" style={{ marginLeft: "auto" }}>↓</span> : null}
        </div>
      ))}
    </div>
  );
}

export function LandingPage() {
  return (
    <div className="landing">
      <nav className="landing__topnav" aria-label="Public navigation">
        <span className="landing__brand">HELQIX</span>
        <div className="landing__links">
          <a href="#product">Product</a>
          <a href="#how">How it works</a>
          <a href="#tech">Technology</a>
          <a href="#cases">Use cases</a>
        </div>
        <div className="landing__cta-group">
          <Link to="/login" className="hq-button hq-button--ghost">Login</Link>
          <Link to="/signup" className="hq-button hq-button--primary">Get Started</Link>
        </div>
      </nav>

      <header className="hero">
        <div>
          <h1 className="hero__title">
            ENGINEERING REALITY,
            <br />
            <em>MADE EXECUTABLE.</em>
          </h1>
          <p className="hero__sub">
            Turn engineering intent into an executable reality. Observe state. Execute actions. Understand
            failure. Branch from exact states. Compare outcomes. Verify results with evidence.
          </p>
          <div className="hero__actions">
            <Link to="/login" className="hq-button hq-button--primary">ENTER HELQIX</Link>
            <a href="#how" className="hq-button">EXPLORE HOW IT WORKS</a>
          </div>
        </div>
        <WorkflowChain />
      </header>

      <section className="section" id="product">
        <h2>Executable reality, not another description of a system</h2>
        <p className="lede">
          Traditional engineering tools describe systems. HELQIX connects the parts that were never wired
          together: model, state, execution, observation, causality, and verification — one continuous,
          reproducible chain of engineering reality.
        </p>
        <div className="grid-3">
          <div className="feature"><h3>MODEL + STATE</h3><p>An engineering model that carries authoritative state, not a static diagram.</p></div>
          <div className="feature"><h3>EXECUTION + OBSERVATION</h3><p>Run realities, measure what actually happens, and watch state change as it occurs.</p></div>
          <div className="feature"><h3>CAUSALITY + VERIFICATION</h3><p>Deterministic cause→effect graphs trace failures to root causes, then verify against requirements with evidence.</p></div>
        </div>
      </section>

      <section className="section" id="how">
        <h2>When something fails, HELQIX knows why</h2>
        <p className="lede">
          Conceptual illustration of causal tracing (not simulation output): an increase propagates through
          loads, currents, and temperatures until a limit is exceeded — and every link is recorded as an
          explicit, traceable causal edge.
        </p>
        <div className="causal-demo" aria-label="Conceptual causal chain">
          <span className="causal-demo__step">Payload increase</span><span className="causal-demo__sep">→</span>
          <span className="causal-demo__step">Load increase</span><span className="causal-demo__sep">→</span>
          <span className="causal-demo__step">Temperature increase</span><span className="causal-demo__sep">→</span>
          <span className="causal-demo__step">Limit exceeded</span><span className="causal-demo__sep">→</span>
          <span className="causal-demo__step causal-demo__step--fail">Performance degradation</span>
        </div>
      </section>

      <section className="section" id="tech">
        <h2>Branch from the exact moment</h2>
        <p className="lede">Freeze the precise state at any instant, branch, change one variable, and compare outcomes — without ever touching the original history.</p>
        <pre className="branch-demo">{`Exact State @ 12.480 s
├── Original run
├── Branch A · payload −4 kg
└── Branch B · cooling +15 %`}</pre>
      </section>

      <section className="section" id="cases">
        <h2>Built for engineering teams operating physical systems</h2>
        <div className="grid-3">
          <div className="feature"><h3>Aerospace</h3><p>Thermal &amp; actuation limits traced to root causes across flight profiles.</p></div>
          <div className="feature"><h3>Robotics</h3><p>Execution timelines with per-state branching for control tuning.</p></div>
          <div className="feature"><h3>Automotive</h3><p>Verification evidence generated from actual executed realities.</p></div>
        </div>
      </section>

      <footer className="final-cta">
        <h2>BUILD YOUR ENGINEERING REALITY</h2>
        <p style={{ margin: "var(--hq-sp-4) 0" }}>
          <Link to="/signup" className="hq-button hq-button--primary">Get Started</Link>
        </p>
      </footer>
    </div>
  );
}
