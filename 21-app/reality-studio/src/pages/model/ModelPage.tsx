/** Model workspace — component tree, workspace surface, property panel, validation bar. */
import { useMemo } from "react";
import { useParams, Link } from "react-router-dom";
import { useAsyncResource } from "../../hooks/useAsyncResource";
import { getModel } from "../../api/resources";
import { ResourceGate, PageHeader, Badge } from "../../components/common";
import { ComponentTree } from "../../components/component-tree/ComponentTree";
import { useAppState } from "../../state/AppState";
import { trimFloat } from "../../utils/format";
import "./model.css";

export function ModelPage() {
  const { projectId = "" } = useParams();
  const { state: ui, dispatch } = useAppState();
  const model = useAsyncResource(() => getModel(projectId), [projectId]);

  const selected = useMemo(
    () => model.data?.components.find((c) => c.id === ui.selectedComponentId) ?? null,
    [model.data, ui.selectedComponentId],
  );

  return (
    <div className="page" style={{ maxWidth: "none" }}>
      <PageHeader
        title="Engineering Model"
        actions={<Link className="hq-button hq-button--primary" to="../architecture">ARCHITECTURE →</Link>}
      />
      <ResourceGate phase={model.phase} error={model.error} data={model.data} feature="Engineering model service" onRetry={model.reload}>
        {(m) => (
          <div className="model-layout">
            <section className="hq-panel model-tree" aria-label="Component tree">
              <div className="hq-panel__header"><span className="label">System</span><Badge tone="normal">{m.components.length} components</Badge></div>
              <div style={{ padding: "var(--hq-sp-2)" }}>
                <ComponentTree
                  components={m.components}
                  selectedId={ui.selectedComponentId}
                  onSelect={(id) => dispatch({ type: "context/selectComponent", componentId: id })}
                />
              </div>
            </section>

            <section className="model-canvas" aria-label="Model workspace">
              <div className="model-canvas__grid" aria-hidden />
              <div style={{ position: "relative", padding: "var(--hq-sp-6)", textAlign: "center", color: "var(--hq-text-secondary)" }}>
                {selected ? (
                  <>
                    <h3 style={{ marginBottom: "var(--hq-sp-2)" }}>{selected.name}</h3>
                    <p className="mono" style={{ fontSize: "var(--hq-text-xs)" }}>{selected.kind} · {selected.id.slice(0, 12)}</p>
                    <p style={{ marginTop: "var(--hq-sp-4)", fontSize: "var(--hq-text-sm)" }}>
                      {m.interfaces.filter((i) => i.sourceComponentId === selected.id || i.targetComponentId === selected.id).length} interface(s) connected
                    </p>
                  </>
                ) : (
                  <p>Select a component in the tree to inspect it in the workspace.</p>
                )}
              </div>
            </section>

            <aside className="hq-panel" aria-label="Properties">
              <div className="hq-panel__header"><span className="label">Properties & parameters</span></div>
              <div className="hq-panel__body">
                {selected ? (
                  <table className="prop-table">
                    <tbody>
                      {selected.properties.map((p) => (
                        <tr key={p.name}>
                          <td>{p.name}</td>
                          <td className="mono">{p.quantity ? `${trimFloat(p.quantity.value)} ${p.quantity.unit}` : p.unitSymbol ?? "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <p className="page__subtitle">No component selected.</p>
                )}
              </div>
            </aside>

            <footer className="hq-panel validation-bar" aria-label="Validation status">
              <div style={{ display: "flex", alignItems: "center", gap: "var(--hq-sp-3)", padding: "var(--hq-sp-3) var(--hq-sp-4)" }}>
                <span className="label">Validation</span>
                {m.validationDiagnostics.length === 0 ? (
                  <Badge tone="success">Model consistent</Badge>
                ) : (
                  <Badge tone="warning">{m.validationDiagnostics.length} diagnostic(s)</Badge>
                )}
                <ul style={{ margin: 0, paddingLeft: "var(--hq-sp-4)", color: "var(--hq-text-secondary)", fontSize: "var(--hq-text-xs)" }}>
                  {m.validationDiagnostics.map((d, i) => <li key={i}>{d}</li>)}
                </ul>
              </div>
            </footer>
          </div>
        )}
      </ResourceGate>
    </div>
  );
}
