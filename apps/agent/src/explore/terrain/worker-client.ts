import type { TerrainWorkerResponseMessage } from "./worker-entry";
import {
  executeTerrainCompilationJob,
  type TerrainCompilationJob,
  type TerrainCompilationResult,
} from "./worker";

export const TERRAIN_WORKER_CLIENT_REVISION =
  "rey.terrain.worker-client@2" as const;

interface QueuedTerrainCompilation {
  job: TerrainCompilationJob;
  signal: AbortSignal;
  resolve: (result: TerrainCompilationResult) => void;
  reject: (error: unknown) => void;
}

/**
 * Reuses one dedicated worker for the lifetime of the client instead of
 * creating and `terminate()`-ing a fresh one on every `compile()` call.
 * Panning (or zooming) retriggers `compile()` roughly every 32px/⅛ octave —
 * a `type: "module"` worker's boot cost (fetching, parsing, and executing
 * its whole module graph from scratch) paid on every one of those was the
 * dominant cost behind Neighborhoods panning feeling "extremely slow and
 * jerky," independent of how fast any individual compile actually is.
 *
 * Since the worker can't cooperatively cancel a job already in progress
 * (executeTerrainCompilationJob runs to completion synchronously inside
 * it), a newer request while one is in flight is queued as `#next` rather
 * than dispatched immediately or blindly forwarded — collapsing any
 * requests that arrive faster than the worker can drain them down to just
 * the latest, instead of letting a `postMessage` backlog build up and make
 * the view lag behind wherever panning actually stopped.
 */
export class TerrainCompilationWorkerClient {
  #worker: Worker | null = null;
  #active: QueuedTerrainCompilation | null = null;
  #next: QueuedTerrainCompilation | null = null;

  compile(
    job: TerrainCompilationJob,
    signal: AbortSignal,
  ): Promise<TerrainCompilationResult> {
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
    return new Promise((resolve, reject) => {
      const entry: QueuedTerrainCompilation = {
        job,
        signal,
        resolve,
        reject,
      };
      signal.addEventListener(
        "abort",
        () => {
          if (this.#next === entry) this.#next = null;
          reject(abortError());
        },
        { once: true },
      );
      if (this.#active) {
        // A request already waiting for the active job to finish gets
        // displaced (not just silently dropped) — otherwise its promise
        // would never resolve or reject, since #next only ever remembers
        // one entry and this overwrites it.
        this.#next?.reject(abortError());
        this.#next = entry;
        return;
      }
      this.#dispatch(entry);
    });
  }

  #dispatch(entry: QueuedTerrainCompilation) {
    this.#active = entry;
    this.#ensureWorker().postMessage({ type: "compile", job: entry.job });
  }

  #ensureWorker(): Worker {
    if (this.#worker) return this.#worker;
    const worker = new Worker(new URL("./worker-entry.ts", import.meta.url), {
      name: "rey-terrain-compilation",
      type: "module",
    });
    worker.onmessage = (event: MessageEvent<TerrainWorkerResponseMessage>) => {
      const active = this.#active;
      this.#active = null;
      if (active && !active.signal.aborted) {
        if (
          event.data.type === "complete" &&
          event.data.result.job_id === active.job.job_id
        )
          active.resolve(event.data.result);
        else if (
          event.data.type === "failed" &&
          event.data.job_id === active.job.job_id
        )
          active.reject(new Error(event.data.error));
      }
      this.#advance();
    };
    worker.onerror = (event) => {
      const active = this.#active;
      this.#active = null;
      if (active && !active.signal.aborted)
        active.reject(new Error(event.message || "terrain worker failed"));
      this.#advance();
    };
    this.#worker = worker;
    return worker;
  }

  #advance() {
    const next = this.#next;
    this.#next = null;
    // The abort listener registered in compile() already clears #next the
    // instant its signal aborts, so an already-aborted entry can't reach
    // here in practice — this guard is defensive only.
    if (next && !next.signal.aborted) this.#dispatch(next);
  }

  cancel() {
    this.#worker?.terminate();
    this.#worker = null;
    this.#active = null;
    this.#next = null;
  }
}

function abortError(): Error {
  return new DOMException("terrain compilation cancelled", "AbortError");
}
