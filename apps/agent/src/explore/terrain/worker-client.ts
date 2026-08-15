import type { TerrainWorkerResponseMessage } from "./worker-entry";
import {
  executeTerrainCompilationJob,
  type TerrainCompilationJob,
  type TerrainCompilationResult,
} from "./worker";

export const TERRAIN_WORKER_CLIENT_REVISION =
  "rey.terrain.worker-client@1" as const;

export class TerrainCompilationWorkerClient {
  #activeWorker: Worker | null = null;

  compile(
    job: TerrainCompilationJob,
    signal: AbortSignal,
  ): Promise<TerrainCompilationResult> {
    this.cancel();
    if (signal.aborted) return Promise.reject(abortError());
    if (typeof Worker === "undefined")
      return new Promise((resolve, reject) => {
        queueMicrotask(() => {
          if (signal.aborted) reject(abortError());
          else {
            try {
              resolve(executeTerrainCompilationJob(job));
            } catch (error) {
              reject(error);
            }
          }
        });
      });
    const worker = new Worker(new URL("./worker-entry.ts", import.meta.url), {
      name: "rey-terrain-compilation",
      type: "module",
    });
    this.#activeWorker = worker;
    return new Promise((resolve, reject) => {
      const cleanup = () => {
        signal.removeEventListener("abort", abort);
        worker.terminate();
        if (this.#activeWorker === worker) this.#activeWorker = null;
      };
      const abort = () => {
        cleanup();
        reject(abortError());
      };
      signal.addEventListener("abort", abort, { once: true });
      worker.onerror = (event) => {
        cleanup();
        reject(new Error(event.message || "terrain worker failed"));
      };
      worker.onmessage = (
        event: MessageEvent<TerrainWorkerResponseMessage>,
      ) => {
        if (
          event.data.type === "complete" &&
          event.data.result.job_id === job.job_id
        ) {
          cleanup();
          resolve(event.data.result);
        } else if (
          event.data.type === "failed" &&
          event.data.job_id === job.job_id
        ) {
          cleanup();
          reject(new Error(event.data.error));
        }
      };
      worker.postMessage({ type: "compile", job });
    });
  }

  cancel() {
    this.#activeWorker?.terminate();
    this.#activeWorker = null;
  }
}

function abortError(): Error {
  return new DOMException("terrain compilation cancelled", "AbortError");
}
