import { describe, expect, it } from "vitest";
import { formatLogicalTime, nanosToSecondsFloat, trimFloat, formatPercent } from "./format";

describe("engineering time formatting", () => {
  it("formats ns decimal strings as seconds with ms precision", () => {
    expect(formatLogicalTime("12480000000")).toBe("12.480 s");
    expect(formatLogicalTime("0")).toBe("0.000 s");
    expect(formatLogicalTime("1500000000")).toBe("1.500 s");
  });
  it("handles missing values honestly", () => {
    expect(formatLogicalTime(null)).toBe("—");
    expect(formatLogicalTime(undefined)).toBe("—");
    expect(formatLogicalTime("not-a-number")).toBe("—");
  });
  it("converts to float seconds for chart math", () => {
    expect(nanosToSecondsFloat("12480000000")).toBeCloseTo(12.48);
    expect(nanosToSecondsFloat("-5")).toBeNull();
  });
  it("trims floats without rounding surprises", () => {
    expect(trimFloat(82.4, 3)).toBe("82.4");
    expect(trimFloat(100, 3)).toBe("100");
    expect(formatPercent(0.94)).toBe("94%");
  });
});
