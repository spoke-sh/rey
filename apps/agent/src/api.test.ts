import { afterEach, describe, expect, it, vi } from "vitest";
import { loadPortfolio, loadPortfolioAfterRevision } from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("portfolio revalidation", () => {
  it("keeps the evidence catalog out of the Explorer portfolio read", async () => {
    const fetch = vi.fn().mockImplementation(async (input: string) => ({
      ok: true,
      json: async () => {
        if (input === "/api/v1/health") return { agent: {}, server: {} };
        if (input === "/api/v1/revalidation") {
          return {
            schema: "rey.ui-revalidation.v1",
            revision: "blake3:current",
            poll_after_ms: 5_000,
          };
        }
        return input === "/api/v1/workloads"
          ? { schema: "rey.workload-list.v1" }
          : {};
      },
    }));
    vi.stubGlobal("fetch", fetch);

    await loadPortfolio();
    const targets = fetch.mock.calls.map(([target]) => target);
    expect(targets).toContain("/api/v1/workloads");
    expect(targets).toContain("/api/v1/revalidation");
    expect(targets).not.toContain("/api/v1/workloads/evidence");
  });

  it("does not reload heavy portfolio endpoints when exact sources are unchanged", async () => {
    const fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        schema: "rey.ui-revalidation.v1",
        revision: "blake3:unchanged",
        poll_after_ms: 5_000,
        basis: "exact bounded source bytes",
        source_entries: 1,
        source_bytes: 1,
        scope: ["workloads"],
        authority: "change detection only",
        omissions: [],
      }),
    });
    vi.stubGlobal("fetch", fetch);

    await expect(
      loadPortfolioAfterRevision("blake3:unchanged"),
    ).resolves.toBeNull();
    expect(fetch).toHaveBeenCalledOnce();
    expect(fetch).toHaveBeenCalledWith("/api/v1/revalidation", {
      headers: { Accept: "application/json" },
    });
  });
});
