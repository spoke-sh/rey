import {
  compileContextGlobe,
  ExplorerCanvas,
  type ExplorerCanvasContent,
  type ExplorerCanvasReport,
  type RendererPreference,
  type RendererStatus,
} from "@rey/explorer";
import { useEffect, useMemo, useRef, useState } from "react";
import type { GlobeCameraView } from "../engine/camera";
import {
  activeExplorerRenderPasses,
  type ExplorerRenderVisibility,
} from "../engine/render-graph";
import { compileTerrainRenderPasses } from "../engine/terrain-passes";
import type { SceneSnapshot } from "../engine/scene";
import {
  terrainPatchRequestsForView,
  type TerrainCameraView,
} from "../terrain/compile";
import {
  MAX_TERRAIN_TILE_CPU_BYTES,
  MAX_TERRAIN_TILE_GPU_BYTES,
  TerrainTileResidency,
  type TerrainTileResidencyStats,
} from "../terrain/residency";
import { TerrainCompilationWorkerClient } from "../terrain/worker-client";
import type { TerrainCompilationResult } from "../terrain/worker";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";
import type { TopologyGlobe } from "../../topology";

export type { RendererPreference } from "@rey/explorer";

export interface AcceleratedTerrainReport {
  status: RendererStatus;
  preference: RendererPreference;
  active_band_ids: readonly string[];
  field_sets: number;
  field_cells: number;
  field_bytes: number;
  program_count: number;
  working_set_limit_cells: number;
  working_set_limit_bytes: number;
  draw_calls: number;
  triangles: number;
  render_graph_id: string;
  active_render_passes: readonly string[];
  render_pass_set_id: string;
  render_pass_area_count: number;
  render_pass_line_count: number;
  render_pass_point_count: number;
  render_pass_kinds: readonly string[];
  source_valid_vertices: number;
  source_no_data_vertices: number;
  source_elevation_minimum: number | null;
  source_elevation_maximum: number | null;
  source_elevation_span: number | null;
  gpu_bytes: number;
  gpu_budget_bytes: number;
  parity_revision: string;
  parity_samples: number;
  field_evaluation_ms: number;
  geometry_compilation_ms: number;
  render_submission_ms: number;
  terrain_update_ms: number;
  terrain_decode_ms: number;
  terrain_tile_projection_ms: number;
  terrain_maximum_screen_error_pixels: number;
  terrain_tile_seam_mismatches: number;
  terrain_no_data_leak_triangles: number;
  terrain_worker_execution:
    "dedicated_worker" | "main_thread_fallback" | "none";
  terrain_worker_revision: string;
  active_tile_count: number;
  active_tile_levels: readonly number[];
  resident_tile_count: number;
  resident_cpu_bytes: number;
  resident_cpu_budget_bytes: number;
  resident_gpu_bytes: number;
  resident_gpu_budget_bytes: number;
  resident_hits: number;
  resident_misses: number;
  resident_evictions: number;
  gpu_timing_ms: null;
  gpu_timing_authority: "unavailable_without_capable_gpu_timer";
  render_pass_omissions: readonly string[];
  measurement_authority: "transient_cpu_unretained";
}

export const REFERENCE_TERRAIN_REPORT: AcceleratedTerrainReport = Object.freeze(
  {
    status: {
      lifecycle: "idle",
      backend: "reference",
      renderer_revision: "rey.reference-renderer@1",
      degraded: false,
      detail: "the deterministic reference terrain is active",
    },
    preference: "auto",
    active_band_ids: Object.freeze([]),
    field_sets: 0,
    field_cells: 0,
    field_bytes: 0,
    program_count: 0,
    working_set_limit_cells: 0,
    working_set_limit_bytes: 0,
    draw_calls: 0,
    triangles: 0,
    render_graph_id: "unbound",
    active_render_passes: Object.freeze([]),
    render_pass_set_id: "unbound",
    render_pass_area_count: 0,
    render_pass_line_count: 0,
    render_pass_point_count: 0,
    render_pass_kinds: Object.freeze([]),
    source_valid_vertices: 0,
    source_no_data_vertices: 0,
    source_elevation_minimum: null,
    source_elevation_maximum: null,
    source_elevation_span: null,
    gpu_bytes: 0,
    gpu_budget_bytes: 0,
    parity_revision: "unbound",
    parity_samples: 0,
    field_evaluation_ms: 0,
    geometry_compilation_ms: 0,
    render_submission_ms: 0,
    terrain_update_ms: 0,
    terrain_decode_ms: 0,
    terrain_tile_projection_ms: 0,
    terrain_maximum_screen_error_pixels: 0,
    terrain_tile_seam_mismatches: 0,
    terrain_no_data_leak_triangles: 0,
    terrain_worker_execution: "none",
    terrain_worker_revision: "unbound",
    active_tile_count: 0,
    active_tile_levels: Object.freeze([]),
    resident_tile_count: 0,
    resident_cpu_bytes: 0,
    resident_cpu_budget_bytes: MAX_TERRAIN_TILE_CPU_BYTES,
    resident_gpu_bytes: 0,
    resident_gpu_budget_bytes: MAX_TERRAIN_TILE_GPU_BYTES,
    resident_hits: 0,
    resident_misses: 0,
    resident_evictions: 0,
    gpu_timing_ms: null,
    gpu_timing_authority: "unavailable_without_capable_gpu_timer",
    render_pass_omissions: Object.freeze([]),
    measurement_authority: "transient_cpu_unretained",
  } satisfies AcceleratedTerrainReport,
);

interface ResolvedTerrainCompilation {
  job_id: string;
  snapshot_id: string;
  source_key: string;
  result: TerrainCompilationResult;
  fields: TerrainCompilationResult["fields"];
  compiled: TerrainCompilationResult["compiled"];
  residency: TerrainTileResidencyStats;
}

export function rendererPreference(search: string): RendererPreference {
  const requested = new URLSearchParams(search).get("renderer");
  if (
    requested === "webgpu" ||
    requested === "webgl2" ||
    requested === "reference"
  )
    return requested;
  return "auto";
}

export function AcceleratedTerrainSurface({
  onReport,
  snapshot,
  view,
  visible,
  renderVisibility,
  globeView = { yaw_degrees: 0, pitch_degrees: 0 },
  projectionMorphProgress = 0,
  contentMode = "auto",
  canvasOpacity = 1,
}: {
  onReport: (report: AcceleratedTerrainReport) => void;
  snapshot: SceneSnapshot;
  view: TerrainCameraView;
  visible: boolean;
  renderVisibility: ExplorerRenderVisibility;
  globeView?: GlobeCameraView;
  projectionMorphProgress?: number;
  contentMode?: "auto" | "globe" | "terrain";
  canvasOpacity?: number;
}) {
  const workerClientRef = useRef<TerrainCompilationWorkerClient | null>(null);
  const residencyRef = useRef<TerrainTileResidency | null>(null);
  if (!workerClientRef.current)
    workerClientRef.current = new TerrainCompilationWorkerClient();
  if (!residencyRef.current) residencyRef.current = new TerrainTileResidency();
  const retainedTransitionGlobeRef = useRef<
    | {
        atlas_revision: string;
        globe: TopologyGlobe;
      }
    | undefined
  >(undefined);
  const atlasTransition = snapshot.scene.world_atlas_transition;
  if (
    snapshot.scene.regime === "world" &&
    snapshot.scene.globe &&
    atlasTransition
  )
    retainedTransitionGlobeRef.current = {
      atlas_revision: atlasTransition.atlas_revision,
      globe: snapshot.scene.globe,
    };
  const retainedTransitionGlobe =
    atlasTransition &&
    retainedTransitionGlobeRef.current?.atlas_revision ===
      atlasTransition.atlas_revision
      ? retainedTransitionGlobeRef.current.globe
      : null;
  const semanticGlobe = useMemo(() => {
    if (contentMode === "terrain") return null;
    if (snapshot.scene.regime === "world") return snapshot.scene.globe;
    const transition = snapshot.scene.world_atlas_transition;
    if (
      (snapshot.scene.regime !== "atlas" &&
        !snapshot.scene.atlas_landscape_transition) ||
      !transition
    )
      return null;
    if (retainedTransitionGlobe) return retainedTransitionGlobe;
    return {
      schema: "rey.semantic-globe-scene.v1" as const,
      posture: "semantic_atlas" as const,
      globe_id: `atlas-map:${transition.atlas_revision}`,
      source_revision: transition.atlas_revision,
      compiler_revision: transition.projection_revision,
      coordinate_authority: transition.authority,
      clusters: [],
      regions: transition.points.map((point) => ({
        id: point.identity,
        cluster_id: "atlas-map",
        focus_id: point.focus_id,
        workload_id: point.identity,
        label: point.label,
        detail: "retained regional identity on the semantic Mercator atlas",
        longitude_degrees: point.longitude_microdegrees / 1_000_000,
        latitude_degrees: point.latitude_microdegrees / 1_000_000,
        angular_radius_degrees: 0,
        tone: point.tone,
      })),
      beacons: [],
    };
  }, [contentMode, retainedTransitionGlobe, snapshot.scene]);
  const projectionGlobe = useMemo(
    () =>
      semanticGlobe
        ? {
            ...semanticGlobe,
            sectors: (atlasTransition?.sectors ?? []).map((sector) => ({
              id: sector.identity,
              label: sector.label,
              west_degrees: sector.west_microdegrees / 1_000_000,
              south_degrees: sector.south_microdegrees / 1_000_000,
              east_degrees: sector.east_microdegrees / 1_000_000,
              north_degrees: sector.north_microdegrees / 1_000_000,
              crosses_antimeridian: sector.crosses_antimeridian,
              tone: sector.tone,
            })),
          }
        : null,
    [
      atlasTransition?.atlas_revision,
      atlasTransition?.projection_revision,
      semanticGlobe,
    ],
  );
  const preference = rendererPreference(globalThis.location?.search ?? "");
  const programTotals = useMemo(() => {
    const dynamic = snapshot.scene.terrain_programs.reduce(
      (result, program) => ({
        cells:
          result.cells +
          program.projection.terrain_program.working_set.max_cells,
        bytes:
          result.bytes +
          program.projection.terrain_program.working_set.max_bytes,
      }),
      { cells: 0, bytes: 0 },
    );
    const total = snapshot.scene.terrain_fields.reduce(
      (result, field) => ({
        cells: result.cells + field.field_cells,
        bytes: result.bytes + field.field_bytes,
      }),
      dynamic,
    );
    return total;
  }, [snapshot.snapshot_id]);
  const sourceTerrainSummary = useMemo(() => {
    let validVertices = 0;
    let noDataVertices = 0;
    let minimum = Number.POSITIVE_INFINITY;
    let maximum = Number.NEGATIVE_INFINITY;
    for (const field of snapshot.scene.terrain_fields) {
      for (let index = 0; index < field.field_cells; index += 1) {
        if (field.validity.values[index] === 0) {
          noDataVertices += 1;
          continue;
        }
        validVertices += 1;
        const elevation =
          field.elevation.values[index]! * field.elevation_scale;
        minimum = Math.min(minimum, elevation);
        maximum = Math.max(maximum, elevation);
      }
    }
    const hasElevation = validVertices > 0;
    return Object.freeze({
      valid_vertices: validVertices,
      no_data_vertices: noDataVertices,
      elevation_minimum: hasElevation ? minimum : null,
      elevation_maximum: hasElevation ? maximum : null,
      elevation_span: hasElevation ? maximum - minimum : null,
    });
  }, [snapshot.snapshot_id]);
  const workingSetRequests = useMemo(
    () =>
      semanticGlobe
        ? []
        : snapshot.scene.terrain_programs.map((program) =>
            terrainPatchRequestsForView(program, view),
          ),
    [
      semanticGlobe,
      snapshot.snapshot_id,
      view.pan_x,
      view.pan_y,
      view.rendered_scale,
      view.pitch_degrees,
      view.yaw_degrees,
      view.model_transform?.elevation_scale,
      view.model_transform?.scale_x,
      view.model_transform?.scale_z,
      view.model_transform?.translate_x,
      view.model_transform?.translate_z,
      view.viewport_height,
      view.viewport_width,
    ],
  );
  const workingSetRevision = workingSetRequests
    .flatMap((requests) => requests.map((request) => request.working_set_id))
    .join("|");
  const terrainSourceKey = [
    ...snapshot.scene.terrain_fields.map((field) => field.field_set_id),
    ...snapshot.scene.terrain_programs.map((program) => program.program_id),
  ]
    .sort((left, right) => left.localeCompare(right))
    .join("|");
  const terrainCompilationView = useMemo(
    () => ({
      ...view,
      rendered_scale: 2 ** (Math.round(Math.log2(view.rendered_scale) * 8) / 8),
      pan_x: Math.round(view.pan_x / 32) * 32,
      pan_y: Math.round(view.pan_y / 32) * 32,
      pitch_degrees:
        view.pitch_degrees === undefined
          ? undefined
          : Math.round(view.pitch_degrees / 3) * 3,
      yaw_degrees:
        view.yaw_degrees === undefined
          ? undefined
          : Math.round(view.yaw_degrees / 4) * 4,
    }),
    [
      view.pan_x,
      view.pan_y,
      view.rendered_scale,
      view.viewport_height,
      view.viewport_width,
      view.world_height,
      view.world_width,
    ],
  );
  const terrainJobId = [
    snapshot.snapshot_id,
    workingSetRevision,
    `${terrainCompilationView.viewport_width}x${terrainCompilationView.viewport_height}`,
    terrainCompilationView.rendered_scale,
    terrainCompilationView.pan_x,
    terrainCompilationView.pan_y,
    terrainCompilationView.pitch_degrees ?? 90,
    terrainCompilationView.yaw_degrees ?? 0,
  ].join("|");
  const [resolvedTerrain, setResolvedTerrain] =
    useState<ResolvedTerrainCompilation | null>(null);
  const [terrainFailure, setTerrainFailure] = useState<Error | null>(null);
  useEffect(() => {
    if (
      semanticGlobe ||
      (snapshot.scene.terrain_fields.length === 0 &&
        snapshot.scene.terrain_programs.length === 0)
    ) {
      setTerrainFailure(null);
      return;
    }
    const abort = new AbortController();
    setTerrainFailure(null);
    const client = workerClientRef.current!;
    void client
      .compile(
        {
          job_id: terrainJobId,
          workload_id: "explorer-landscape-interaction-v1",
          fields: snapshot.scene.terrain_fields,
          programs: snapshot.scene.terrain_programs.map((program, index) => ({
            program,
            requests: workingSetRequests[index]!,
          })),
          view: terrainCompilationView,
          maximum_cpu_bytes: MAX_TERRAIN_TILE_CPU_BYTES,
          maximum_gpu_bytes: MAX_TERRAIN_TILE_GPU_BYTES,
        },
        abort.signal,
      )
      .then((result) => {
        if (abort.signal.aborted) return;
        const activeTiles = residencyRef.current!.admit(
          result.compiled_tiles,
          result.active_tile_ids,
        );
        const residentById = new Map(
          activeTiles.map((tile) => [tile.descriptor.tile_id, tile]),
        );
        const fields = Object.freeze(
          result.fields.map(
            (field) => residentById.get(field.field_set_id)?.fields ?? field,
          ),
        );
        const compiled = Object.freeze({
          ...result.compiled,
          meshes: Object.freeze(
            result.compiled.meshes.map((mesh) => {
              const resident = residentById.get(mesh.field_set_id);
              return resident
                ? Object.freeze({
                    field_set_id: resident.fields.field_set_id,
                    data: resident.mesh,
                  })
                : mesh;
            }),
          ),
        });
        setResolvedTerrain({
          job_id: terrainJobId,
          snapshot_id: snapshot.snapshot_id,
          source_key: terrainSourceKey,
          result,
          fields,
          compiled,
          residency: residencyRef.current!.stats(),
        });
      })
      .catch((error: unknown) => {
        if (abort.signal.aborted) return;
        setTerrainFailure(
          error instanceof Error ? error : new Error(String(error)),
        );
      });
    return () => abort.abort();
  }, [semanticGlobe, snapshot.snapshot_id, terrainJobId, workingSetRevision]);
  const activeTerrain =
    !semanticGlobe && resolvedTerrain?.source_key === terrainSourceKey
      ? resolvedTerrain
      : null;
  const fieldProjection = useMemo(() => {
    const fields = activeTerrain?.fields ?? [];
    return Object.freeze({
      fields,
      evaluation_ms: activeTerrain?.result.metrics.field_evaluation_ms ?? 0,
      cells: fields.reduce((total, field) => total + field.field_cells, 0),
      bytes: fields.reduce((total, field) => total + field.field_bytes, 0),
      active_band_ids: Object.freeze(
        [
          ...new Set([
            ...(semanticGlobe ? ["semantic_globe"] : []),
            ...fields.flatMap((field) => field.active_band_ids),
          ]),
        ].sort((left, right) => left.localeCompare(right)),
      ),
    });
  }, [activeTerrain, semanticGlobe]);
  const globeCompilation = useMemo(
    () => (projectionGlobe ? compileContextGlobe(projectionGlobe) : null),
    [projectionGlobe],
  );
  const terrainRenderPasses = useMemo(
    () =>
      compileTerrainRenderPasses(
        snapshot.scene,
        snapshot.render_graph,
        renderVisibility,
      ),
    [
      renderVisibility.contours,
      renderVisibility.probes,
      renderVisibility.water,
      renderVisibility.weather,
      snapshot.snapshot_id,
    ],
  );
  const terrainCompilation = useMemo(() => {
    if (semanticGlobe || !activeTerrain?.compiled) return null;
    if (!terrainRenderPasses) return activeTerrain.compiled;
    return Object.freeze({
      ...activeTerrain.compiled,
      material_revision: `${activeTerrain.compiled.material_revision}:${terrainRenderPasses.pass_set_id}`,
      render_passes: terrainRenderPasses,
    });
  }, [activeTerrain, semanticGlobe, terrainRenderPasses]);
  const terrainMetrics = activeTerrain?.result.metrics;
  const residency = activeTerrain?.residency;
  const activeTileLevels = Object.freeze(
    [
      ...new Set(
        activeTerrain?.result.selections.map((selection) => selection.level) ??
          [],
      ),
    ].sort((left, right) => left - right),
  );
  const statistics = globeCompilation?.statistics ??
    terrainCompilation?.statistics ?? {
      field_sets: 0,
      triangles: 0,
      field_bytes: fieldProjection.bytes,
      gpu_bytes: 0,
      gpu_budget_bytes: 0,
      parity_revision: "unbound",
      parity_samples: 0,
      geometry_compilation_ms: 0,
    };
  const materialRevision =
    (globeCompilation
      ? `${globeCompilation.material_revision}:${globeCompilation.projection_revision}`
      : undefined) ??
    terrainCompilation?.material_revision ??
    "unbound";
  const activeRenderPassIds = useMemo(
    () =>
      Object.freeze(
        activeExplorerRenderPasses(snapshot.render_graph, renderVisibility).map(
          ({ id }) => id,
        ),
      ),
    [
      renderVisibility.contours,
      renderVisibility.probes,
      renderVisibility.water,
      renderVisibility.weather,
      snapshot.render_graph.graph_id,
    ],
  );
  const content: ExplorerCanvasContent | null = globeCompilation
    ? {
        kind: "globe",
        compiled: globeCompilation,
        view: {
          ...globeView,
          projection_morph_progress: projectionMorphProgress,
        },
        world: snapshot.scene.world,
      }
    : terrainCompilation
      ? {
          kind: "terrain",
          compiled: terrainCompilation,
          view,
          world: snapshot.scene.world,
        }
      : null;
  const frame = {
    snapshot_id: snapshot.snapshot_id,
    camera_revision: semanticGlobe
      ? `orthographic-globe:${globeView.yaw_degrees}:${globeView.pitch_degrees}:${projectionMorphProgress}`
      : `terrain-orbit:${view.viewport_width}x${view.viewport_height}:${view.rendered_scale}:${view.pan_x}:${view.pan_y}:${view.pitch_degrees ?? 90}:${view.yaw_degrees ?? 0}:${view.model_transform ? Object.values(view.model_transform).join(":") : "identity"}`,
    material_revision: materialRevision,
    render_graph_id: snapshot.render_graph.graph_id,
  };
  const completeReport = (canvasReport: ExplorerCanvasReport) =>
    onReport({
      status: canvasReport.status as RendererStatus,
      preference,
      active_band_ids: fieldProjection.active_band_ids,
      field_sets: statistics.field_sets,
      field_cells: fieldProjection.cells,
      field_bytes: statistics.field_bytes,
      program_count: new Set([
        ...snapshot.scene.terrain_programs.map((program) => program.program_id),
        ...snapshot.scene.terrain_fields.map((field) => field.program_id),
      ]).size,
      working_set_limit_cells: programTotals.cells,
      working_set_limit_bytes: programTotals.bytes,
      draw_calls: canvasReport.draw_calls,
      triangles: statistics.triangles,
      render_graph_id: snapshot.render_graph.graph_id,
      active_render_passes: activeRenderPassIds,
      render_pass_set_id:
        terrainCompilation?.render_passes?.pass_set_id ?? "unbound",
      render_pass_area_count:
        terrainCompilation?.render_passes?.areas.length ?? 0,
      render_pass_line_count:
        terrainCompilation?.render_passes?.lines.length ?? 0,
      render_pass_point_count:
        terrainCompilation?.render_passes?.points.length ?? 0,
      render_pass_kinds: Object.freeze(
        [
          ...new Set([
            ...(terrainCompilation?.render_passes?.areas.map(
              ({ kind }) => kind,
            ) ?? []),
            ...(terrainCompilation?.render_passes?.lines.map(
              ({ kind }) => kind,
            ) ?? []),
            ...(terrainCompilation?.render_passes?.points.map(
              ({ kind }) => kind,
            ) ?? []),
          ]),
        ].sort((left, right) => left.localeCompare(right)),
      ),
      source_valid_vertices: sourceTerrainSummary.valid_vertices,
      source_no_data_vertices: sourceTerrainSummary.no_data_vertices,
      source_elevation_minimum: sourceTerrainSummary.elevation_minimum,
      source_elevation_maximum: sourceTerrainSummary.elevation_maximum,
      source_elevation_span: sourceTerrainSummary.elevation_span,
      gpu_bytes: statistics.gpu_bytes,
      gpu_budget_bytes: statistics.gpu_budget_bytes,
      parity_revision: statistics.parity_revision,
      parity_samples: statistics.parity_samples,
      field_evaluation_ms: fieldProjection.evaluation_ms,
      geometry_compilation_ms: statistics.geometry_compilation_ms,
      render_submission_ms: canvasReport.render_submission_ms,
      terrain_update_ms: terrainMetrics?.update_ms ?? 0,
      terrain_decode_ms: terrainMetrics?.decode_ms ?? 0,
      terrain_tile_projection_ms: terrainMetrics?.tile_projection_ms ?? 0,
      terrain_maximum_screen_error_pixels:
        terrainMetrics?.maximum_screen_error_pixels ?? 0,
      terrain_tile_seam_mismatches: terrainMetrics?.tile_seam_mismatches ?? 0,
      terrain_no_data_leak_triangles:
        terrainMetrics?.no_data_leak_triangles ?? 0,
      terrain_worker_execution:
        activeTerrain?.result.execution ?? (semanticGlobe ? "none" : "none"),
      terrain_worker_revision:
        activeTerrain?.result.worker_revision ?? "unbound",
      active_tile_count: activeTerrain?.result.active_tile_ids.length ?? 0,
      active_tile_levels: activeTileLevels,
      resident_tile_count: residency?.entries ?? 0,
      resident_cpu_bytes: residency?.cpu_bytes ?? 0,
      resident_cpu_budget_bytes:
        residency?.cpu_budget_bytes ?? MAX_TERRAIN_TILE_CPU_BYTES,
      resident_gpu_bytes: residency?.gpu_bytes ?? 0,
      resident_gpu_budget_bytes:
        residency?.gpu_budget_bytes ?? MAX_TERRAIN_TILE_GPU_BYTES,
      resident_hits: residency?.hits ?? 0,
      resident_misses: residency?.misses ?? 0,
      resident_evictions: residency?.evictions ?? 0,
      gpu_timing_ms: terrainMetrics?.gpu_timing_ms ?? null,
      gpu_timing_authority:
        terrainMetrics?.gpu_timing_authority ??
        "unavailable_without_capable_gpu_timer",
      render_pass_omissions:
        terrainCompilation?.render_passes?.omissions ?? Object.freeze([]),
      measurement_authority: "transient_cpu_unretained",
    });

  useEffect(() => {
    if (content) return;
    completeReport({
      status: terrainFailure
        ? {
            lifecycle: "failed",
            backend: "reference",
            renderer_revision: "rey.reference-renderer@1",
            degraded: true,
            detail: `terrain worker failed; reference terrain retained: ${terrainFailure.message}`,
          }
        : {
            ...REFERENCE_TERRAIN_REPORT.status,
            lifecycle: "ready",
          },
      draw_calls: 0,
      render_submission_ms: 0,
    });
  }, [content, snapshot.snapshot_id, terrainFailure]);

  return content ? (
    <ExplorerCanvas
      className={sx(styles.acceleratedTerrainCanvas)}
      content={content}
      frame={frame}
      onReport={completeReport}
      opacity={canvasOpacity}
      preference={preference}
      readyClassName={sx(styles.acceleratedTerrainCanvasReady)}
      visible={visible}
    />
  ) : null;
}
