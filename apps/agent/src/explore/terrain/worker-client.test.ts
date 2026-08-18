import { afterEach, describe, expect, it, vi } from "vitest";
import { executeTerrainCompilationJob } from "./worker";
import { TerrainCompilationWorkerClient } from "./worker-client";
import { admittedField, terrainTileView } from "./tiles.fixture";

class MockWorker {
  static instances: MockWorker[] = [];
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  posted: { type: string; job: { job_id: string } }[] = [];
  terminated = false;
  url: URL;
  options?: unknown;

  constructor(url: URL, options?: unknown) {
    this.url = url;
    this.options = options;
    MockWorker.instances.push(this);
  }

  postMessage(message: { type: string; job: { job_id: string } }) {
    this.posted.push(message);
  }

  terminate() {
    this.terminated = true;
  }

  respond(jobId: string) {
    const result = executeTerrainCompilationJob(job(jobId));
    this.onmessage?.({
      data: { type: "complete", result },
    } as MessageEvent);
  }
}

function job(jobId: string) {
  return {
    job_id: jobId,
    workload_id: "neighborhoods-pan-fixture",
    regime: "neighborhoods" as const,
    fields: [admittedField()],
    programs: [],
    view: terrainTileView(4),
    maximum_cpu_bytes: 8 * 1024 * 1024,
    maximum_gpu_bytes: 8 * 1024 * 1024,
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  MockWorker.instances = [];
});

describe("terrain compilation worker client", () => {
  it("reuses one worker across sequential compiles instead of rebuilding it per job", async () => {
    vi.stubGlobal("Worker", MockWorker);
    const client = new TerrainCompilationWorkerClient();

    const first = client.compile(job("pan:1"), new AbortController().signal);
    expect(MockWorker.instances).toHaveLength(1);
    MockWorker.instances[0]!.respond("pan:1");
    await expect(first).resolves.toMatchObject({ job_id: "pan:1" });

    const second = client.compile(job("pan:2"), new AbortController().signal);
    // Panning triggers many of these in a row (every ~32px) — before this
    // fix, every one tore down the previous worker and booted a fresh
    // module worker from scratch, which is what made panning feel slow and
    // jerky independent of how fast any individual compile actually was.
    expect(MockWorker.instances).toHaveLength(1);
    expect(MockWorker.instances[0]!.terminated).toBe(false);
    MockWorker.instances[0]!.respond("pan:2");
    await expect(second).resolves.toMatchObject({ job_id: "pan:2" });
  });

  it("collapses requests that arrive while one is in flight down to just the latest", async () => {
    vi.stubGlobal("Worker", MockWorker);
    const client = new TerrainCompilationWorkerClient();

    const first = client.compile(job("pan:a"), new AbortController().signal);
    const second = client.compile(job("pan:b"), new AbortController().signal);
    const third = client.compile(job("pan:c"), new AbortController().signal);
    // Only the first job is actually posted to the worker while it's busy —
    // pan:b and pan:c collapse into a single pending slot instead of both
    // queuing up behind pan:a, which would otherwise make the view lag
    // behind wherever the pointer actually stopped.
    expect(MockWorker.instances[0]!.posted.map((m) => m.job.job_id)).toEqual([
      "pan:a",
    ]);

    MockWorker.instances[0]!.respond("pan:a");
    await expect(first).resolves.toMatchObject({ job_id: "pan:a" });
    // pan:b was superseded before it ever reached the worker, so it neither
    // resolves nor gets posted — only pan:c (the latest) is dispatched next.
    expect(MockWorker.instances[0]!.posted.map((m) => m.job.job_id)).toEqual([
      "pan:a",
      "pan:c",
    ]);

    MockWorker.instances[0]!.respond("pan:c");
    await expect(third).resolves.toMatchObject({ job_id: "pan:c" });
    expect(MockWorker.instances).toHaveLength(1);

    // The superseded request must not hang forever — it's rejected as soon
    // as pan:c displaces it from the pending slot, not left dangling.
    await expect(second).rejects.toThrow();
  });

  it("aborting a still-queued (not yet dispatched) request drops it without posting it", async () => {
    vi.stubGlobal("Worker", MockWorker);
    const client = new TerrainCompilationWorkerClient();
    const abortController = new AbortController();

    void client.compile(job("pan:x"), new AbortController().signal);
    const queued = client.compile(job("pan:y"), abortController.signal);
    abortController.abort();
    await expect(queued).rejects.toThrow();

    MockWorker.instances[0]!.respond("pan:x");
    // pan:y was aborted while only queued (never posted) — draining the
    // queue after pan:x completes must not post it anyway.
    expect(MockWorker.instances[0]!.posted.map((m) => m.job.job_id)).toEqual([
      "pan:x",
    ]);
  });
});
