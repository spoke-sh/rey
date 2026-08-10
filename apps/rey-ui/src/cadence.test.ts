import { describe, expect, it } from "vitest";
import { formatCadenceTime } from "./cadence";

describe("cadence projection", () => {
  it("renders exact UTC instants while preserving order-only clocks", () => {
    expect(formatCadenceTime(1_786_335_192)).toBe("2026-08-10 04:13:12Z");
    expect(formatCadenceTime(null)).toBe("ORDER ONLY");
  });
});
