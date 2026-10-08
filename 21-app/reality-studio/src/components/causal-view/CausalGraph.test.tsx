import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CausalGraph } from "./CausalGraph";
import type { CausalGraphView } from "../../api/types";

const graph: CausalGraphView = {
  failureId: "f1",
  rootCauseIds: ["n1"],
  nodes: [
    { id: "n1", label: "Payload increase", kind: "root", componentId: "c1", propertyName: "mass", valueChange: "100→104 kg", time: { nanos: "9000000000" }, evidenceRefs: [] },
    { id: "n2", label: "Temperature rise", kind: "intermediate", componentId: "c2", propertyName: "temp", valueChange: "61→103 °C", time: { nanos: "11000000000" }, evidenceRefs: ["s1"] },
    { id: "n3", label: "Limit exceeded", kind: "failure", componentId: "c2", propertyName: "temp", valueChange: "103 > 100", time: { nanos: "12480000000" }, evidenceRefs: [] },
  ],
  edges: [{ from: "n1", to: "n2", relation: "increases_load_on" }, { from: "n2", to: "n3", relation: "violates" }],
};

describe("CausalGraph", () => {
  it("draws every node and edge supplied by the causality engine", () => {
    render(<CausalGraph graph={graph} selectedNodeId={null} onSelect={() => {}} />);
    expect(screen.getByLabelText(/Causal graph: 3 events, 2 relations/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Payload increase/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Limit exceeded/ })).toBeInTheDocument();
  });

  it("selection bubbles the exact node id for the inspector panel", async () => {
    const onSelect = vi.fn();
    render(<CausalGraph graph={graph} selectedNodeId={null} onSelect={onSelect} />);
    await userEvent.click(screen.getByRole("button", { name: /Temperature rise/ }));
    expect(onSelect).toHaveBeenCalledWith("n2");
  });
});
