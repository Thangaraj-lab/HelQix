import { describe, expect, it } from "vitest";
import { defaultCamera, hitTest, project } from "./projection";

describe("reality projection", () => {
  it("projects engineering-space meters to deterministic screen coordinates", () => {
    const cam = { ...defaultCamera(), yaw: 0, pitch: 0 };
    const p = project([1, 0, 0], cam, 800, 600);
    // yaw=0,pitch=0 → x maps directly, y centered
    expect(p.x).toBeCloseTo(400 + cam.zoom);
    expect(p.y).toBeCloseTo(300);
  });

  it("hit-testing returns the frontmost object under the cursor", () => {
    const objects = [
      { id: "far", center: { x: 100, y: 100, depth: 5 }, radiusPx: 10 },
      { id: "near", center: { x: 100, y: 100, depth: 1 }, radiusPx: 10 },
    ];
    expect(hitTest(102, 99, objects)).toBe("near");
    expect(hitTest(500, 500, objects)).toBeNull();
  });
});
