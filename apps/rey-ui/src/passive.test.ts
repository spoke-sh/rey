import { afterEach, describe, expect, it, vi } from "vitest";
import { startPassiveRevalidation } from "./passive";

afterEach(() => {
  vi.useRealTimers();
});

describe("passive revalidation", () => {
  it("publishes in mounted state without overlapping refreshes", async () => {
    vi.useFakeTimers();
    let resolveFirst: ((value: string) => void) | undefined;
    const load = vi
      .fn<() => Promise<string>>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockResolvedValue("third");
    const publish = vi.fn();
    const reportError = vi.fn();
    const stop = startPassiveRevalidation({
      intervalMs: 5_000,
      load,
      publish,
      reportError,
    });

    await vi.advanceTimersByTimeAsync(10_000);
    expect(load).toHaveBeenCalledTimes(1);
    resolveFirst?.("second");
    await Promise.resolve();
    expect(publish).toHaveBeenCalledWith("second");

    await vi.advanceTimersByTimeAsync(5_000);
    expect(load).toHaveBeenCalledTimes(2);
    expect(publish).toHaveBeenLastCalledWith("third");
    expect(reportError).toHaveBeenLastCalledWith(null);
    stop();
  });

  it("retains the last document when a background refresh fails", async () => {
    vi.useFakeTimers();
    const publish = vi.fn();
    const reportError = vi.fn();
    const stop = startPassiveRevalidation({
      intervalMs: 5_000,
      load: vi.fn().mockRejectedValue(new Error("offline")),
      publish,
      reportError,
    });

    await vi.advanceTimersByTimeAsync(5_000);
    expect(publish).not.toHaveBeenCalled();
    expect(reportError).toHaveBeenCalledWith(new Error("offline"));
    stop();
  });
});
