import {
  executeTerrainCompilationJob,
  terrainCompilationResultForWorkerTransfer,
  terrainCompilationResultWithTransferMetrics,
  terrainCompilationTransferableBuffers,
  type TerrainCompilationJob,
  type TerrainCompilationResult,
} from "./worker";

type TerrainCompilationViewJob = Omit<TerrainCompilationJob, "fields">;

export type TerrainWorkerRequestMessage =
  | { type: "compile-source"; job: TerrainCompilationJob }
  | { type: "compile-view"; job: TerrainCompilationViewJob };

export type TerrainWorkerResponseMessage =
  | { type: "complete"; result: TerrainCompilationResult }
  | { type: "failed"; job_id: string; error: string };

const workerScope = globalThis as unknown as {
  onmessage:
    ((event: MessageEvent<TerrainWorkerRequestMessage>) => void) | null;
  postMessage(
    message: TerrainWorkerResponseMessage,
    transfer?: readonly Transferable[],
  ): void;
};

let registeredSource:
  | {
      source_key: string;
      fields: TerrainCompilationJob["fields"];
    }
  | undefined;

workerScope.onmessage = (event) => {
  try {
    const sourcePayload =
      event.data.type === "compile-source"
        ? ("full_source" as const)
        : ("registered_source" as const);
    const job =
      event.data.type === "compile-source"
        ? event.data.job
        : registeredSource?.source_key === event.data.job.source_key
          ? { ...event.data.job, fields: registeredSource.fields }
          : null;
    if (!job)
      throw new Error(
        "terrain worker view request has no matching registered source",
      );
    if (event.data.type === "compile-source")
      registeredSource = {
        source_key: job.source_key,
        fields: job.fields,
      };
    let result = terrainCompilationResultForWorkerTransfer(
      executeTerrainCompilationJob(job, "dedicated_worker"),
      sourcePayload,
    );
    const transfer = terrainCompilationTransferableBuffers(result);
    result = terrainCompilationResultWithTransferMetrics(result, transfer);
    workerScope.postMessage(
      {
        type: "complete",
        result,
      },
      transfer,
    );
  } catch (error) {
    workerScope.postMessage({
      type: "failed",
      job_id: event.data.job.job_id,
      error: error instanceof Error ? error.message : String(error),
    });
  }
};
