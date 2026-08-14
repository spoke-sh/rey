import {
  compileContextGlobe,
  compileContinuousRelief,
  ExplorerCanvas,
  type ExplorerCanvasContent,
  type ExplorerCanvasReport,
  type RendererPreference,
  type RendererStatus,
} from "@rey/explorer";
import { useEffect, useMemo, useRef } from "react";
import type { GlobeCameraView } from "../engine/camera";
import {
  activeExplorerRenderPasses,
  type ExplorerRenderVisibility,
} from "../engine/render-graph";
import type { SceneSnapshot } from "../engine/scene";
import {
  terrainPatchRequestsForView,
  type TerrainCameraView,
} from "../terrain/compile";
import { TerrainPatchCache } from "../terrain/patch-cache";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";

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
  gpu_bytes: number;
  gpu_budget_bytes: number;
  parity_revision: string;
  parity_samples: number;
  field_evaluation_ms: number;
  geometry_compilation_ms: number;
  render_submission_ms: number;
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
    gpu_bytes: 0,
    gpu_budget_bytes: 0,
    parity_revision: "unbound",
    parity_samples: 0,
    field_evaluation_ms: 0,
    geometry_compilation_ms: 0,
    render_submission_ms: 0,
    measurement_authority: "transient_cpu_unretained",
  } satisfies AcceleratedTerrainReport,
);

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
}: {
  onReport: (report: AcceleratedTerrainReport) => void;
  snapshot: SceneSnapshot;
  view: TerrainCameraView;
  visible: boolean;
  renderVisibility: ExplorerRenderVisibility;
  globeView?: GlobeCameraView;
}) {
  const patchCacheRef = useRef<
    | {
        limits: string;
        cache: TerrainPatchCache;
      }
    | undefined
  >(undefined);
  const semanticGlobe =
    snapshot.scene.regime === "world" ? snapshot.scene.globe : null;
  const preference = rendererPreference(globalThis.location?.search ?? "");
  const programTotals = useMemo(
    () =>
      snapshot.scene.terrain_programs.reduce(
        (result, program) => ({
          cells:
            result.cells +
            program.projection.terrain_program.working_set.max_cells,
          bytes:
            result.bytes +
            program.projection.terrain_program.working_set.max_bytes,
        }),
        { cells: 0, bytes: 0 },
      ),
    [snapshot.snapshot_id],
  );
  const limitsRevision = `${programTotals.cells}:${programTotals.bytes}`;
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
      view.viewport_height,
      view.viewport_width,
    ],
  );
  const workingSetRevision = workingSetRequests
    .flatMap((requests) => requests.map((request) => request.working_set_id))
    .join("|");
  const fieldProjection = useMemo(() => {
    const evaluationStarted = measurementNow();
    if (!semanticGlobe && patchCacheRef.current?.limits !== limitsRevision)
      patchCacheRef.current = {
        limits: limitsRevision,
        cache: new TerrainPatchCache(programTotals.cells, programTotals.bytes),
      };
    const fields = snapshot.scene.terrain_programs.flatMap((program, index) =>
      semanticGlobe
        ? []
        : workingSetRequests[index]!.map((request) =>
            patchCacheRef.current!.cache.materialize(program, request),
          ),
    );
    return Object.freeze({
      fields: Object.freeze(fields),
      evaluation_ms: measurementNow() - evaluationStarted,
      cells: fields.reduce((total, field) => total + field.field_cells, 0),
      bytes: fields.reduce((total, field) => total + field.field_bytes, 0),
      active_band_ids: Object.freeze(
        [
          ...(semanticGlobe ? ["semantic_globe"] : []),
          ...new Set(fields.flatMap((field) => field.active_band_ids)),
        ].sort((left, right) => left.localeCompare(right)),
      ),
    });
  }, [limitsRevision, semanticGlobe, snapshot.snapshot_id, workingSetRevision]);
  const globeCompilation = useMemo(
    () => (semanticGlobe ? compileContextGlobe(semanticGlobe) : null),
    [semanticGlobe],
  );
  const terrainCompilation = useMemo(
    () =>
      semanticGlobe || fieldProjection.fields.length === 0
        ? null
        : compileContinuousRelief(fieldProjection.fields),
    [fieldProjection.fields, semanticGlobe],
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
    globeCompilation?.material_revision ??
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
        view: globeView,
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
      ? `orthographic-globe:${globeView.yaw_degrees}:${globeView.pitch_degrees}`
      : `orthographic:${view.viewport_width}x${view.viewport_height}:${view.rendered_scale}:${view.pan_x}:${view.pan_y}`,
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
      program_count: snapshot.scene.terrain_programs.length,
      working_set_limit_cells: programTotals.cells,
      working_set_limit_bytes: programTotals.bytes,
      draw_calls: canvasReport.draw_calls,
      triangles: statistics.triangles,
      render_graph_id: snapshot.render_graph.graph_id,
      active_render_passes: activeRenderPassIds,
      gpu_bytes: statistics.gpu_bytes,
      gpu_budget_bytes: statistics.gpu_budget_bytes,
      parity_revision: statistics.parity_revision,
      parity_samples: statistics.parity_samples,
      field_evaluation_ms: fieldProjection.evaluation_ms,
      geometry_compilation_ms: statistics.geometry_compilation_ms,
      render_submission_ms: canvasReport.render_submission_ms,
      measurement_authority: "transient_cpu_unretained",
    });

  useEffect(() => {
    if (content) return;
    completeReport({
      status: {
        ...REFERENCE_TERRAIN_REPORT.status,
        lifecycle: "ready",
      },
      draw_calls: 0,
      render_submission_ms: 0,
    });
  }, [content, snapshot.snapshot_id]);

  return content ? (
    <ExplorerCanvas
      className={sx(styles.acceleratedTerrainCanvas)}
      content={content}
      frame={frame}
      onReport={completeReport}
      preference={preference}
      readyClassName={sx(styles.acceleratedTerrainCanvasReady)}
      visible={visible}
    />
  ) : null;
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}
