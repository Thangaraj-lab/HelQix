/** Live State Inspector — displays authoritative state values only. */
import type { AuthoritativeState, ModelComponent } from "../../api/types";
import { formatLogicalTime, trimFloat } from "../../utils/format";
import { Badge, Skeleton } from "../common";

interface Props {
  loading: boolean;
  state: AuthoritativeState | null;
  selectedComponent: ModelComponent | null;
  unavailable?: boolean;
}

export function StatePanel({ loading, state, selectedComponent, unavailable }: Props) {
  return (
    <aside className="hq-panel" aria-label="Live state inspector" style={{ display: "flex", flexDirection: "column", overflow: "hidden" }}>
      <div className="hq-panel__header">
        <span className="label">Live State</span>
        {unavailable ? <Badge tone="warning">No state feed</Badge> : state ? <Badge tone="active">{formatLogicalTime(state.time?.nanos)}</Badge> : <Badge tone="normal">Awaiting</Badge>}
      </div>
      <div className="hq-panel__body" style={{ overflowY: "auto" }}>
        {loading ? (
          <div style={{ display: "grid", gap: 8 }}>{[0, 1, 2, 3].map((i) => <Skeleton key={i} height={18} />)}</div>
        ) : unavailable || !state ? (
          <p className="page__subtitle">
            {unavailable
              ? "The backend state endpoint is not available yet. The inspector binds to the Layer-04 state contract and will populate automatically."
              : "No state received yet for this reality."}
          </p>
        ) : (
          <>
            {selectedComponent ? (
              <>
                <div className="label" style={{ marginBottom: 6 }}>Selected: {selectedComponent.name}</div>
                <table className="prop-table"><tbody>
                  {state.values
                    .filter((v) => v.componentId === selectedComponent.id)
                    .map((v) => (
                      <tr key={`${v.componentId}:${v.property}`}>
                        <td>{v.property}</td>
                        <td className="mono">{trimFloat(v.quantity.value)} {v.quantity.unit}</td>
                      </tr>
                    ))}
                </tbody></table>
              </>
            ) : null}
            <div className="label" style={{ margin: "var(--hq-sp-4) 0 6px" }}>System values</div>
            <table className="prop-table"><tbody>
              {state.values.slice(0, 40).map((v) => (
                <tr key={`${v.componentId}:${v.property}`}>
                  <td>{v.componentName} · {v.property}</td>
                  <td className="mono">{trimFloat(v.quantity.value)} {v.quantity.unit}</td>
                </tr>
              ))}
            </tbody></table>
            <p className="mono" style={{ marginTop: 8, fontSize: "var(--hq-text-xs)", color: "var(--hq-text-muted)" }}>
              state {state.stateId.slice(0, 12)} · identity frozen per snapshot
            </p>
          </>
        )}
      </div>
    </aside>
  );
}
