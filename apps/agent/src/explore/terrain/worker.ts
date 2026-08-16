import {
  compileContinuousRelief,
  terrainNoDataLeakTriangleCount,
  type CompiledContinuousRelief,
} from "@rey/explorer";
import {
  materializeTerrainWorkingSet,
  type TerrainCameraView,
  type TerrainFieldSet,
  type TerrainProgram,
  type TerrainWorkingSetRequest,
} from "./compile";
import type { CompiledTerrainTile } from "./residency";
import { deriveTerrainNormals } from "./normals";
import { refineRegionalTerrainField } from "./refinement";
import {
  materializeTerrainTile,
  projectTerrainTilePyramid,
  selectTerrainTilesForView,
  terrainTileSeamMismatchCount,
  type TerrainTilePyramid,
  type TerrainTileSelection,
} from "./tiles";

export const TERRAIN_COMPILATION_WORKER_REVISION =
  "rey.terrain.compilation-worker@1" as const;
export const TERRAIN_WORKER_RELIEF_REVISION =
  "rey.terrain.worker-relief@1" as const;

export interface TerrainProgramWorkerRequest {
  program: TerrainProgram;
  requests: readonly TerrainWorkingSetRequest[];
}

export interface TerrainCompilationJob {
  job_id: string;
  workload_id: string;
  fields: readonly TerrainFieldSet[];
  programs: readonly TerrainProgramWorkerRequest[];
  view: TerrainCameraView;
  maximum_cpu_bytes: number;
  maximum_gpu_bytes: number;
}

export interface TerrainCompilationMetrics {
  workload_id: string;
  update_ms: number;
  decode_ms: number;
  tile_projection_ms: number;
  field_evaluation_ms: number;
  mesh_preparation_ms: number;
  maximum_screen_error_pixels: number;
  tile_seam_mismatches: number;
  no_data_leak_triangles: number;
  gpu_timing_ms: null;
  gpu_timing_authority: "unavailable_without_capable_gpu_timer";
}

export interface TerrainCompilationResult {
  job_id: string;
  worker_revision: typeof TERRAIN_COMPILATION_WORKER_REVISION;
  execution: "dedicated_worker" | "main_thread_fallback";
  pyramids: readonly TerrainTilePyramid[];
  selections: readonly TerrainTileSelection[];
  active_tile_ids: readonly string[];
  compiled_tiles: readonly CompiledTerrainTile[];
  fields: readonly TerrainFieldSet[];
  compiled: CompiledContinuousRelief;
  metrics: TerrainCompilationMetrics;
}

export function executeTerrainCompilationJob(
  job: TerrainCompilationJob,
  execution: TerrainCompilationResult["execution"] = "main_thread_fallback",
): TerrainCompilationResult {
  const updateStarted = measurementNow();
  if (
    !job.job_id ||
    !job.workload_id ||
    !Number.isSafeInteger(job.maximum_cpu_bytes) ||
    !Number.isSafeInteger(job.maximum_gpu_bytes) ||
    job.maximum_cpu_bytes < 1 ||
    job.maximum_gpu_bytes < 1
  )
    throw new Error("terrain compilation job bounds are invalid");

  const projectionStarted = measurementNow();
  const admittedFields = job.fields
    .filter((field) => field.active_band_ids.includes("admitted_dem"))
    .map((field) => refineRegionalTerrainField(field))
    .map(deriveWorkerRelief);
  const passthroughFields = job.fields.filter(
    (field) => !field.active_band_ids.includes("admitted_dem"),
  );
  const pyramids = admittedFields.map((field) =>
    projectTerrainTilePyramid(field),
  );
  const selections = pyramids.map((pyramid) =>
    selectTerrainTilesForView(pyramid, job.view),
  );
  const fieldById = new Map(
    admittedFields.map((field) => [field.field_set_id, field]),
  );
  const compiledTiles = selections.flatMap((selection, pyramidIndex) => {
    const pyramid = pyramids[pyramidIndex]!;
    const source = fieldById.get(pyramid.field_set_id);
    if (!source) throw new Error("terrain tile pyramid lost its source field");
    return selection.tiles.map((descriptor) => {
      const fields = materializeTerrainTile(source, descriptor);
      return Object.freeze({
        descriptor,
        fields,
      });
    });
  });
  const tileProjectionMs = measurementNow() - projectionStarted;

  const evaluationStarted = measurementNow();
  const evaluatedFields = job.programs.flatMap(({ program, requests }) =>
    requests.map((request) => materializeTerrainWorkingSet(program, request)),
  );
  const fieldEvaluationMs = measurementNow() - evaluationStarted;
  const fields = [
    ...compiledTiles.map((tile) => tile.fields),
    ...passthroughFields,
    ...evaluatedFields,
  ];
  const cpuBytes = fields.reduce(
    (total, field) => total + field.field_bytes,
    0,
  );
  if (cpuBytes > job.maximum_cpu_bytes)
    throw new Error(
      `terrain worker output ${cpuBytes} exceeds CPU budget ${job.maximum_cpu_bytes}`,
    );
  const meshStarted = measurementNow();
  const compiled = compileContinuousRelief(fields, job.maximum_gpu_bytes);
  const meshPreparationMs = measurementNow() - meshStarted;
  const meshById = new Map(
    compiled.meshes.map((mesh) => [mesh.field_set_id, mesh.data]),
  );
  const tiles = compiledTiles.map((tile) =>
    Object.freeze({
      ...tile,
      mesh: meshById.get(tile.fields.field_set_id)!,
    }),
  );
  if (tiles.some((tile) => tile.mesh === undefined))
    throw new Error("terrain worker mesh output lost a selected tile");
  const noDataLeakTriangles = fields.reduce(
    (total, field, index) =>
      total +
      terrainNoDataLeakTriangleCount(field, compiled.meshes[index]!.data),
    0,
  );
  return Object.freeze({
    job_id: job.job_id,
    worker_revision: TERRAIN_COMPILATION_WORKER_REVISION,
    execution,
    pyramids: Object.freeze(pyramids),
    selections: Object.freeze(selections),
    active_tile_ids: Object.freeze(
      selections.flatMap((selection) => selection.tile_ids),
    ),
    compiled_tiles: Object.freeze(tiles),
    fields: Object.freeze(fields),
    compiled,
    metrics: Object.freeze({
      workload_id: job.workload_id,
      update_ms: measurementNow() - updateStarted,
      decode_ms: 0,
      tile_projection_ms: tileProjectionMs,
      field_evaluation_ms: fieldEvaluationMs,
      mesh_preparation_ms: meshPreparationMs,
      maximum_screen_error_pixels: selections.reduce(
        (maximum, selection) =>
          Math.max(maximum, selection.screen_error_pixels),
        0,
      ),
      tile_seam_mismatches: selections.reduce(
        (total, selection) =>
          total + terrainTileSeamMismatchCount(selection.tiles),
        0,
      ),
      no_data_leak_triangles: noDataLeakTriangles,
      gpu_timing_ms: null,
      gpu_timing_authority: "unavailable_without_capable_gpu_timer",
    }),
  });
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}

function deriveWorkerRelief(field: TerrainFieldSet): TerrainFieldSet {
  const relief = deriveTerrainNormals(
    field.elevation,
    field.validity,
    field.elevation_scale,
    {
      normal: `${TERRAIN_WORKER_RELIEF_REVISION}:normal:${field.source_revision}`,
      curvature: `${TERRAIN_WORKER_RELIEF_REVISION}:curvature:${field.source_revision}`,
    },
  );
  return Object.freeze({
    ...field,
    normal: relief.normal,
    curvature: relief.curvature,
  });
}
