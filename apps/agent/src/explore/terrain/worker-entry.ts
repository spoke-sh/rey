import {
  executeTerrainCompilationJob,
  type TerrainCompilationJob,
  type TerrainCompilationResult,
} from "./worker";

interface TerrainWorkerRequestMessage {
  type: "compile";
  job: TerrainCompilationJob;
}

export type TerrainWorkerResponseMessage =
  | { type: "complete"; result: TerrainCompilationResult }
  | { type: "failed"; job_id: string; error: string };

const workerScope = globalThis as unknown as {
  onmessage:
    ((event: MessageEvent<TerrainWorkerRequestMessage>) => void) | null;
  postMessage(message: TerrainWorkerResponseMessage): void;
};

workerScope.onmessage = (event) => {
  if (event.data.type !== "compile") return;
  try {
    workerScope.postMessage({
      type: "complete",
      result: executeTerrainCompilationJob(event.data.job, "dedicated_worker"),
    });
  } catch (error) {
    workerScope.postMessage({
      type: "failed",
      job_id: event.data.job.job_id,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
