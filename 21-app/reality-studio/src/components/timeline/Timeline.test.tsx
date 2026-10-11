import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Timeline } from "./Timeline";
import type { TimelineItem } from "../../api/types";

const items: TimelineItem[] = [
  { id: "t1", executionId: "e1", type: "action", time: { nanos: "5000000000" }, label: "Payload +4 kg", severity: "info", referenceId: null },
  { id: "t2", executionId: "e1", type: "failure", time: { nanos: "12480000000" }, label: "Thermal limit exceeded", severity: "failure", referenceId: "f1" },
];

describe("Timeline", () => {
  it("renders markers and rows from actual events with real timestamps", () => {
    render(<Timeline items={items} selectedId={null} onSelectItem={() => {}} />);
    expect(screen.getByText("Payload +4 kg")).toBeInTheDocument();
    expect(screen.getByText(/5\.000 s/)).toBeInTheDocument();
    expect(screen.getByText(/12\.480 s/)).toBeInTheDocument();
  });

  it("selecting a timestamp reports the item upstream for authoritative state retrieval", async () => {
    const onSelect = vi.fn();
    render(<Timeline items={items} selectedId={null} onSelectItem={onSelect} />);
    await userEvent.click(screen.getByRole("option", { name: /Thermal limit exceeded/i }));
    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: "t2", type: "failure" }));
  });
});
