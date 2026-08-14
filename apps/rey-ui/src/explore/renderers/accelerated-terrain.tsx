import { createRoot, type Renderer as FiberRenderer } from "@react-three/fiber";
import { useEffect, useMemo, useRef, useState } from "react";
import type { RendererStatus, RenderFrameIdentity } from "../engine/renderer";
import { boundedViewport, renderFrameInvalidation } from "../engine/renderer";
import type { SceneSnapshot } from "../engine/scene";
import type { GlobeCameraView } from "../engine/camera";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";
import {
  terrainPatchRequestsForView,
  type TerrainCameraView,
} from "../terrain/compile";
import { TerrainPatchCache } from "../terrain/patch-cache";
import {
  activeExplorerRenderPasses,
  type ExplorerRenderVisibility,
} from "../engine/render-graph";
import { ContextGlobeScene, ContinuousReliefScene } from "./fiber-scenes";
import { compileContextGlobe } from "./three-globe";
import { compileContinuousRelief } from "./three-terrain";
import {
  ReactThreeFiberRendererAdapter,
  THREE_RENDERER_REVISION,
} from "./three-webgpu";

export type RendererPreference = "auto" | "webgpu" | "webgl2" | "reference";
export const WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT =
  "rey:qualify-webgpu-device-loss";

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
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rootRef = useRef<ReturnType<typeof createRoot> | undefined>(undefined);
  const adapterRef = useRef<ReactThreeFiberRendererAdapter | undefined>(
    undefined,
  );
  const lastFrameRef = useRef<RenderFrameIdentity | undefined>(undefined);
  const patchCacheRef = useRef<
    | {
        limits: string;
        cache: TerrainPatchCache;
      }
    | undefined
  >(undefined);
  const [rootGeneration, setRootGeneration] = useState(0);
  const [ready, setReady] = useState(false);
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
  const hasAcceleratedScene =
    globeCompilation !== null || terrainCompilation !== null;
  const frame: RenderFrameIdentity = {
    snapshot_id: snapshot.snapshot_id,
    camera_revision: semanticGlobe
      ? `orthographic-globe:${globeView.yaw_degrees}:${globeView.pitch_degrees}`
      : `orthographic:${view.viewport_width}x${view.viewport_height}:${view.rendered_scale}:${view.pan_x}:${view.pan_y}`,
    material_revision: materialRevision,
    render_graph_id: snapshot.render_graph.graph_id,
  };
  const report = (status: RendererStatus) => {
    const adapter = adapterRef.current;
    onReport({
      status,
      preference,
      active_band_ids: fieldProjection.active_band_ids,
      field_sets: statistics.field_sets,
      field_cells: fieldProjection.cells,
      field_bytes: statistics.field_bytes,
      program_count: snapshot.scene.terrain_programs.length,
      working_set_limit_cells: programTotals.cells,
      working_set_limit_bytes: programTotals.bytes,
      draw_calls: adapter?.lastDrawCalls ?? 0,
      triangles: statistics.triangles,
      render_graph_id: snapshot.render_graph.graph_id,
      active_render_passes: activeRenderPassIds,
      gpu_bytes: statistics.gpu_bytes,
      gpu_budget_bytes: statistics.gpu_budget_bytes,
      parity_revision: statistics.parity_revision,
      parity_samples: statistics.parity_samples,
      field_evaluation_ms: fieldProjection.evaluation_ms,
      geometry_compilation_ms: statistics.geometry_compilation_ms,
      render_submission_ms: adapter?.lastSubmissionMs ?? 0,
      measurement_authority: "transient_cpu_unretained",
    });
  };
  const reportRef = useRef(report);
  reportRef.current = report;
  const sceneElement = globeCompilation ? (
    <ContextGlobeScene
      compiled={globeCompilation}
      view={globeView}
      world={snapshot.scene.world}
    />
  ) : terrainCompilation ? (
    <ContinuousReliefScene
      compiled={terrainCompilation}
      view={view}
      world={snapshot.scene.world}
    />
  ) : null;
  const sceneElementRef = useRef(sceneElement);
  sceneElementRef.current = sceneElement;
  const frameRef = useRef(frame);
  frameRef.current = frame;

  useEffect(() => {
    if (preference !== "reference" && hasAcceleratedScene) return;
    setReady(false);
    const status: RendererStatus = {
      ...REFERENCE_TERRAIN_REPORT.status,
      lifecycle: "ready",
      detail:
        preference === "reference"
          ? "the reference renderer was selected by the view envelope"
          : "the deterministic reference terrain is active",
    };
    reportRef.current(status);
  }, [hasAcceleratedScene, preference, snapshot.snapshot_id]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || preference === "reference" || !hasAcceleratedScene) return;
    let cancelled = false;
    let root: ReturnType<typeof createRoot> | undefined;
    const adapter = new ReactThreeFiberRendererAdapter();
    adapterRef.current = adapter;
    setReady(false);
    reportRef.current({
      lifecycle: "initializing",
      backend: null,
      renderer_revision: THREE_RENDERER_REVISION,
      degraded: false,
      detail: "initializing the declarative React Three Fiber renderer",
    });
    const unsubscribeStatus = adapter.onStatusChange((status) => {
      if (status.lifecycle === "failed") {
        rootRef.current?.unmount();
        rootRef.current = undefined;
        lastFrameRef.current = undefined;
        setReady(false);
        reportRef.current(status as RendererStatus);
      }
    });
    const unsubscribeFrame = adapter.onFrameSubmitted(() => {
      if (cancelled) return;
      setReady(true);
      reportRef.current(adapter.status as RendererStatus);
    });
    const handleContextLoss = (event: Event) => {
      event.preventDefault();
      rootRef.current?.unmount();
      rootRef.current = undefined;
      lastFrameRef.current = undefined;
      setReady(false);
      reportRef.current({
        lifecycle: "failed",
        backend: "reference",
        renderer_revision: THREE_RENDERER_REVISION,
        degraded: true,
        detail: "graphics context lost; the reference terrain remains active",
      });
    };
    const handleWebGpuDeviceLossQualification = () => {
      if (preference === "webgpu")
        adapter.destroyWebGpuDeviceForQualification();
    };
    canvas.addEventListener("webglcontextlost", handleContextLoss);
    canvas.addEventListener(
      WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT,
      handleWebGpuDeviceLossQualification,
    );

    void (async () => {
      const status = await adapter.initialize(
        canvas,
        preference === "auto" ? "auto" : preference,
      );
      if (cancelled || status.lifecycle !== "ready" || !adapter.renderer) {
        if (!cancelled) reportRef.current(status as RendererStatus);
        return;
      }
      try {
        root = createRoot(canvas);
        const viewport = acceleratedViewport(snapshot, view, semanticGlobe);
        await root.configure({
          dpr: viewport.device_pixel_ratio,
          flat: true,
          frameloop: "demand",
          gl: adapter.renderer as unknown as FiberRenderer,
          size: {
            width: viewport.width,
            height: viewport.height,
            left: 0,
            top: 0,
          },
        });
        if (cancelled) {
          root.unmount();
          return;
        }
        rootRef.current = root;
        lastFrameRef.current = frameRef.current;
        root.render(sceneElementRef.current);
        setRootGeneration((generation) => generation + 1);
      } catch (error) {
        setReady(false);
        reportRef.current({
          lifecycle: "failed",
          backend: "reference",
          renderer_revision: THREE_RENDERER_REVISION,
          degraded: true,
          detail: error instanceof Error ? error.message : String(error),
        });
      }
    })();

    return () => {
      cancelled = true;
      canvas.removeEventListener("webglcontextlost", handleContextLoss);
      canvas.removeEventListener(
        WEBGPU_DEVICE_LOSS_QUALIFICATION_EVENT,
        handleWebGpuDeviceLossQualification,
      );
      unsubscribeFrame();
      unsubscribeStatus();
      root?.unmount();
      adapter.dispose();
      if (rootRef.current === root) rootRef.current = undefined;
      if (adapterRef.current === adapter) adapterRef.current = undefined;
      lastFrameRef.current = undefined;
    };
  }, [hasAcceleratedScene, preference]);

  useEffect(() => {
    const root = rootRef.current;
    const adapter = adapterRef.current;
    if (!root || !adapter?.renderer || rootGeneration === 0) return;
    const viewport = acceleratedViewport(snapshot, view, semanticGlobe);
    void root.configure({
      dpr: viewport.device_pixel_ratio,
      flat: true,
      frameloop: "demand",
      gl: adapter.renderer as unknown as FiberRenderer,
      size: {
        width: viewport.width,
        height: viewport.height,
        left: 0,
        top: 0,
      },
    });
  }, [
    rootGeneration,
    semanticGlobe,
    snapshot.scene.world.height,
    snapshot.scene.world.width,
    view.viewport_height,
    view.viewport_width,
  ]);

  useEffect(() => {
    const root = rootRef.current;
    if (!root || rootGeneration === 0 || !sceneElement) return;
    if (renderFrameInvalidation(lastFrameRef.current, frame).length === 0) {
      const status = adapterRef.current?.status;
      if (ready && status) reportRef.current(status as RendererStatus);
      return;
    }
    lastFrameRef.current = Object.freeze({ ...frame });
    root.render(sceneElement);
  }, [
    activeRenderPassIds,
    frame.camera_revision,
    frame.material_revision,
    frame.render_graph_id,
    frame.snapshot_id,
    ready,
    rootGeneration,
    sceneElement,
  ]);

  return (
    <canvas
      aria-hidden="true"
      className={sx(
        styles.acceleratedTerrainCanvas,
        ready && visible && styles.acceleratedTerrainCanvasReady,
      )}
      data-renderer="react-three-fiber"
      ref={canvasRef}
    />
  );
}

function acceleratedViewport(
  snapshot: SceneSnapshot,
  view: TerrainCameraView,
  semanticGlobe: SceneSnapshot["scene"]["globe"],
) {
  return boundedViewport({
    width: semanticGlobe ? snapshot.scene.world.width : view.viewport_width,
    height: semanticGlobe ? snapshot.scene.world.height : view.viewport_height,
    device_pixel_ratio: globalThis.devicePixelRatio ?? 1,
  });
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}
