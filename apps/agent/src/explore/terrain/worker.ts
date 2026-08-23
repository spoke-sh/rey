import {
  compileContinuousRelief,
  deriveLandscapeReliefField,
  landscapePyramidContentId,
  landscapeReliefFieldByteLength,
  terrainNoDataLeakTriangleCount,
  type CompiledContinuousRelief,
  type LandscapePyramidEnvelope,
  type TerrainLineFeatureInput,
} from "@rey/explorer";
import type { LensRegime } from "../engine/camera";
import {
  materializeTerrainWorkingSet,
  type TerrainCameraView,
  type TerrainFieldSet,
  type TerrainProgram,
  type TerrainWorkingSetRequest,
} from "./compile";
import type { CompiledTerrainTile } from "./residency";
import type { MaterializedLandscapeHeightHierarchy } from "./height-pyramid";
import { LANDSCAPE_HEIGHT_HIERARCHY_REVISION } from "./height-pyramid";
import {
  compileMaterializedLandscapePyramid,
  LANDSCAPE_RELIEF_HIERARCHY_REVISION,
  type MaterializedLandscapePyramid,
} from "./relief-pyramid";
import {
  refineRegionalTerrainField,
  REGIONAL_TERRAIN_REFINEMENT_REVISION,
} from "./refinement";
import {
  deriveRegionalTerrainGeography,
  deriveRegionalTerrainPresentationLines,
  REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
} from "./regional-geography";
import {
  materializeTerrainTile,
  materializeTerrainTileRelief,
  projectMaterializedLandscapeTilePyramid,
  selectTerrainTilesForView,
  terrainTileReliefPartitionMismatchCount,
  terrainTileReliefSeamMismatchCount,
  terrainTileSeamMismatchCount,
  type TerrainTilePyramid,
  type TerrainTileSelection,
} from "./tiles";

export const TERRAIN_COMPILATION_WORKER_REVISION =
  "rey.terrain.compilation-worker@12" as const;
export const MAX_TERRAIN_COMPILATION_OUTPUT_BYTES = 112 * 1024 * 1024;
export const MAX_MATERIALIZED_LANDSCAPE_CACHE_BYTES = 80 * 1024 * 1024;

export interface TerrainProgramWorkerRequest {
  program: TerrainProgram;
  requests: readonly TerrainWorkingSetRequest[];
}

export interface TerrainCompilationJob {
  job_id: string;
  workload_id: string;
  regime: LensRegime;
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
  relief_seam_mismatches: number;
  relief_partition_mismatches: number;
  no_data_leak_triangles: number;
  height_hierarchy_levels: number;
  height_hierarchy_bytes: number;
  relief_hierarchy_levels: number;
  relief_hierarchy_bytes: number;
  relief_derived_bytes: number;
  relief_halo_source_cells: number;
  selected_tile_cpu_bytes: number;
  selected_tile_gpu_bytes: number;
  materialized_pyramid_cache_hits: number;
  materialized_pyramid_cache_misses: number;
  relief_border_digest_mismatches: number;
  gpu_timing_ms: null;
  gpu_timing_authority: "unavailable_without_capable_gpu_timer";
}

export interface TerrainCompilationResult {
  job_id: string;
  worker_revision: typeof TERRAIN_COMPILATION_WORKER_REVISION;
  execution: "dedicated_worker" | "main_thread_fallback";
  pyramids: readonly TerrainTilePyramid[];
  landscape_pyramids: readonly LandscapePyramidEnvelope[];
  materialized_landscape_pyramids: readonly MaterializedLandscapePyramid[];
  height_hierarchies: readonly MaterializedLandscapeHeightHierarchy[];
  selections: readonly TerrainTileSelection[];
  active_tile_ids: readonly string[];
  compiled_tiles: readonly CompiledTerrainTile[];
  fields: readonly TerrainFieldSet[];
  compiled: CompiledContinuousRelief;
  derived_lines: readonly TerrainLineFeatureInput[];
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
  const admittedSourceFields = job.fields.filter((field) =>
    field.active_band_ids.includes("admitted_dem"),
  );
  const materializedResults = admittedSourceFields.map((field) =>
    cachedMaterializedLandscapePyramid(field),
  );
  const admittedFields = materializedResults.map(({ field }) => field);
  const materializedLandscapePyramids = materializedResults.map(
    ({ pyramid }) => pyramid,
  );
  const heightHierarchies = materializedLandscapePyramids.map(
    ({ height_hierarchy }) => height_hierarchy,
  );
  const passthroughFields = job.fields.filter(
    (field) => !field.active_band_ids.includes("admitted_dem"),
  );
  const pyramids = materializedLandscapePyramids.map((pyramid) =>
    projectMaterializedLandscapeTilePyramid(pyramid),
  );
  const selections = pyramids.map((pyramid) =>
    selectTerrainTilesForView(pyramid, job.view),
  );
  const derivedLines = admittedFields.flatMap((field) =>
    deriveRegionalTerrainPresentationLines(field, job.regime),
  );
  const fieldById = new Map(
    materializedLandscapePyramids.flatMap((pyramid) =>
      pyramid.relief_levels.map(
        (level) => [level.field.field_set_id, level.field] as const,
      ),
    ),
  );
  const reliefById = new Map(
    materializedLandscapePyramids.flatMap((pyramid) =>
      pyramid.relief_levels.map(
        (level) => [level.field.field_set_id, level.relief] as const,
      ),
    ),
  );
  const landscapePyramids = materializedLandscapePyramids.map(
    ({ envelope }) => envelope,
  );
  const compiledTiles = selections.flatMap((selection, pyramidIndex) => {
    const pyramid = pyramids[pyramidIndex]!;
    return selection.tiles.map((descriptor) => {
      const source = fieldById.get(descriptor.field_set_id);
      if (!source)
        throw new Error("terrain tile pyramid lost its hierarchy field");
      const sourceRelief = reliefById.get(descriptor.field_set_id);
      if (
        !sourceRelief ||
        sourceRelief.relief_field_id !== descriptor.relief_field_id
      )
        throw new Error("terrain tile pyramid lost its exact relief field");
      const fields = materializeTerrainTile(source, descriptor);
      const relief = materializeTerrainTileRelief(
        source,
        sourceRelief,
        descriptor,
      );
      return Object.freeze({
        descriptor,
        fields,
        relief,
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
  const reliefFields = [
    ...compiledTiles.map((tile) => tile.relief),
    ...passthroughFields.map(deriveLandscapeReliefField),
    ...evaluatedFields.map(deriveLandscapeReliefField),
  ];
  const cpuBytes =
    fields.reduce((total, field) => total + field.field_bytes, 0) +
    materializedLandscapePyramids.reduce(
      (total, pyramid) => total + pyramid.byte_length,
      0,
    ) +
    reliefFields.reduce(
      (total, relief) => total + landscapeReliefFieldByteLength(relief),
      0,
    );
  if (cpuBytes > job.maximum_cpu_bytes)
    throw new Error(
      `terrain worker output ${cpuBytes} exceeds CPU budget ${job.maximum_cpu_bytes}`,
    );
  const meshStarted = measurementNow();
  const compiled = compileContinuousRelief(
    fields,
    job.maximum_gpu_bytes,
    undefined,
    reliefFields,
    landscapePyramids,
  );
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
    landscape_pyramids: Object.freeze(landscapePyramids),
    materialized_landscape_pyramids: Object.freeze(
      materializedLandscapePyramids,
    ),
    height_hierarchies: Object.freeze(heightHierarchies),
    selections: Object.freeze(selections),
    active_tile_ids: Object.freeze(
      selections.flatMap((selection) => selection.tile_ids),
    ),
    compiled_tiles: Object.freeze(tiles),
    fields: Object.freeze(fields),
    compiled,
    derived_lines: Object.freeze(derivedLines),
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
      relief_seam_mismatches: terrainTileReliefSeamMismatchCount(tiles),
      relief_partition_mismatches: [...reliefById].reduce(
        (total, [fieldSetId, relief]) =>
          total +
          terrainTileReliefPartitionMismatchCount(
            relief,
            tiles.filter(
              ({ descriptor }) => descriptor.field_set_id === fieldSetId,
            ),
          ),
        0,
      ),
      no_data_leak_triangles: noDataLeakTriangles,
      height_hierarchy_levels: heightHierarchies.reduce(
        (total, hierarchy) => total + hierarchy.levels.length,
        0,
      ),
      height_hierarchy_bytes: heightHierarchies.reduce(
        (total, hierarchy) => total + hierarchy.byte_length,
        0,
      ),
      relief_hierarchy_levels: materializedLandscapePyramids.reduce(
        (total, pyramid) => total + pyramid.relief_levels.length,
        0,
      ),
      relief_hierarchy_bytes: materializedLandscapePyramids.reduce(
        (total, pyramid) =>
          total +
          pyramid.relief_levels.reduce(
            (levelTotal, level) => levelTotal + level.byte_length,
            0,
          ),
        0,
      ),
      relief_derived_bytes: materializedLandscapePyramids.reduce(
        (total, pyramid) =>
          total +
          pyramid.relief_levels.reduce(
            (levelTotal, level) =>
              levelTotal + landscapeReliefFieldByteLength(level.relief),
            0,
          ),
        0,
      ),
      relief_halo_source_cells: materializedLandscapePyramids.reduce(
        (total, pyramid) =>
          total +
          pyramid.relief_levels.reduce(
            (levelTotal, level) =>
              levelTotal +
              level.tiles.reduce(
                (tileTotal, tile) =>
                  tileTotal +
                  (tile.source_window.column_end -
                    tile.source_window.column_start +
                    1) *
                    (tile.source_window.row_end -
                      tile.source_window.row_start +
                      1),
                0,
              ),
            0,
          ),
        0,
      ),
      selected_tile_cpu_bytes: tiles.reduce(
        (total, tile) =>
          total +
          tile.fields.field_bytes +
          landscapeReliefFieldByteLength(tile.relief),
        0,
      ),
      selected_tile_gpu_bytes: tiles.reduce(
        (total, tile) => total + tile.descriptor.gpu_bytes,
        0,
      ),
      materialized_pyramid_cache_hits: materializedResults.filter(
        ({ cache_hit }) => cache_hit,
      ).length,
      materialized_pyramid_cache_misses: materializedResults.filter(
        ({ cache_hit }) => !cache_hit,
      ).length,
      relief_border_digest_mismatches: materializedLandscapePyramids.reduce(
        (total, pyramid) => total + pyramid.border_mismatches,
        0,
      ),
      gpu_timing_ms: null,
      gpu_timing_authority: "unavailable_without_capable_gpu_timer",
    }),
  });
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}

interface MaterializedLandscapeCacheEntry {
  field: TerrainFieldSet;
  pyramid: MaterializedLandscapePyramid;
  last_requested_generation: number;
}

const materializedLandscapeCache = new Map<
  string,
  MaterializedLandscapeCacheEntry
>();
let materializedLandscapeCacheGeneration = 0;
let materializedLandscapeCacheBytes = 0;

function cachedMaterializedLandscapePyramid(sourceField: TerrainFieldSet): {
  field: TerrainFieldSet;
  pyramid: MaterializedLandscapePyramid;
  cache_hit: boolean;
} {
  materializedLandscapeCacheGeneration += 1;
  const key = materializedLandscapeCacheKey(sourceField);
  const retained = materializedLandscapeCache.get(key);
  if (retained) {
    retained.last_requested_generation = materializedLandscapeCacheGeneration;
    return {
      field: retained.field,
      pyramid: retained.pyramid,
      cache_hit: true,
    };
  }
  const field = deriveRegionalTerrainGeography(
    refineRegionalTerrainField(sourceField),
  );
  const pyramid = compileMaterializedLandscapePyramid(field);
  if (pyramid.byte_length <= MAX_MATERIALIZED_LANDSCAPE_CACHE_BYTES) {
    materializedLandscapeCache.set(key, {
      field,
      pyramid,
      last_requested_generation: materializedLandscapeCacheGeneration,
    });
    materializedLandscapeCacheBytes += pyramid.byte_length;
    const candidates = [...materializedLandscapeCache.entries()].sort(
      ([leftKey, left], [rightKey, right]) =>
        left.last_requested_generation - right.last_requested_generation ||
        leftKey.localeCompare(rightKey),
    );
    for (const [candidateKey, candidate] of candidates) {
      if (
        materializedLandscapeCacheBytes <=
        MAX_MATERIALIZED_LANDSCAPE_CACHE_BYTES
      )
        break;
      if (candidateKey === key) continue;
      materializedLandscapeCache.delete(candidateKey);
      materializedLandscapeCacheBytes -= candidate.pyramid.byte_length;
    }
  }
  return { field, pyramid, cache_hit: false };
}

function materializedLandscapeCacheKey(field: TerrainFieldSet): string {
  const metadata = new TextEncoder().encode(
    JSON.stringify({
      worker_revision: TERRAIN_COMPILATION_WORKER_REVISION,
      refinement_revision: REGIONAL_TERRAIN_REFINEMENT_REVISION,
      geography_revision: REGIONAL_TERRAIN_GEOGRAPHY_REVISION,
      height_revision: LANDSCAPE_HEIGHT_HIERARCHY_REVISION,
      relief_revision: LANDSCAPE_RELIEF_HIERARCHY_REVISION,
      field_set_id: field.field_set_id,
      source_revision: field.source_revision,
      grid: field.grid,
      elevation_scale: field.elevation_scale,
      relief_metrics: field.relief_metrics,
      landscape_reference: field.landscape_reference,
      landscape_mosaic: field.landscape_mosaic,
      channels: [
        field.elevation.implementation_revision,
        field.rainfall.implementation_revision,
        field.flow_direction.implementation_revision,
        field.flow_accumulation.implementation_revision,
        field.erosion.implementation_revision,
        field.normal.implementation_revision,
        field.curvature.implementation_revision,
        field.material.implementation_revision,
      ],
    }),
  );
  return landscapePyramidContentId("materialized-landscape-cache", [
    metadata,
    field.validity.values,
    field.validity_classification?.values ?? new Uint8Array(),
    field.elevation.values,
    field.rainfall.values,
    field.flow_direction.values,
    field.flow_accumulation.values,
    field.erosion.values,
    field.normal.values,
    field.curvature.values,
    field.material.tint,
    field.material.occlusion,
    field.material.roughness,
  ]);
}
