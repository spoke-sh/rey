import { describe, expect, it } from "vitest";
import { rendererPreference } from "./accelerated-terrain";

describe("accelerated terrain browser qualification", () => {
  it("keeps backend forcing in the view envelope rather than semantic identity", () => {
    expect(rendererPreference("?renderer=webgpu")).toBe("webgpu");
    expect(rendererPreference("?renderer=webgl2")).toBe("webgl2");
    expect(rendererPreference("?renderer=reference")).toBe("reference");
    expect(rendererPreference("?renderer=unknown")).toBe("auto");
  });
});
