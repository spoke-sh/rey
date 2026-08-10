import { describe, expect, it } from "vitest";
import { router } from "./router";

describe("operator routes", () => {
  it("matches cadence, agents, and matrix-style Explorer coordinates", () => {
    expect(router.matchRoutes("/cadence").at(-1)?.routeId).toBe("/cadence");
    expect(router.matchRoutes("/agents").at(-1)?.routeId).toBe("/agents");
    const coordinate = router
      .matchRoutes(
        "/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness",
      )
      .at(-1);
    expect(coordinate?.routeId).toBe("/explore/$kind/$coordinate");
    expect(coordinate?.params).toMatchObject({
      kind: "agent",
      coordinate: "codex;at=gpt-5;lens=objects;role=coding_harness",
    });
  });
});
