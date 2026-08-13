import { useEffect, useRef, useState } from "react";
import type { RendererStatus } from "../engine/renderer";
import type { SceneSnapshot } from "../engine/scene";
import type { GlobeCameraView } from "../engine/camera";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";
import {
  terrainPatchRequestsForView,
  type TerrainCameraView,
} from "../terrain/compile";
import { TerrainPatchCache } from "../terrain/patch-cache";

export type RendererPreference = "auto" | "webgpu" | "webgl2" | "reference";

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
  triangles: number;
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
    triangles: 0,
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
  globeView = { yaw_degrees: 0, pitch_degrees: 0 },
}: {
  onReport: (report: AcceleratedTerrainReport) => void;
  snapshot: SceneSnapshot;
  view: TerrainCameraView;
  visible: boolean;
  globeView?: GlobeCameraView;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const globeViewRef = useRef(globeView);
  globeViewRef.current = globeView;
  const adapterRef = useRef<
    import("./three-webgpu").ThreeWebGpuRendererAdapter | undefined
  >(undefined);
  const bundleRef = useRef<
    import("./three-terrain").ThreeTerrainBundle | undefined
  >(undefined);
  const [ready, setReady] = useState(false);
  const patchCacheRef = useRef<{
    limits: string;
    cache: TerrainPatchCache;
  } | null>(null);
  const semanticGlobe =
    snapshot.scene.regime === "world" ? snapshot.scene.globe : null;
  const workingSetRequests = semanticGlobe
    ? []
    : snapshot.scene.terrain_programs.map((program) =>
        terrainPatchRequestsForView(program, view),
      );
  const workingSetRevision = workingSetRequests
    .flatMap((requests) => requests.map((request) => request.working_set_id))
    .join("|");

  useEffect(() => {
    const canvas = canvasRef.current;
    if (
      !canvas ||
      (snapshot.scene.terrain_programs.length === 0 && semanticGlobe === null)
    ) {
      setReady(false);
      onReport(REFERENCE_TERRAIN_REPORT);
      return;
    }
    const programTotals = snapshot.scene.terrain_programs.reduce(
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
    const limitsRevision = `${programTotals.cells}:${programTotals.bytes}`;
    if (!semanticGlobe && patchCacheRef.current?.limits !== limitsRevision)
      patchCacheRef.current = {
        limits: limitsRevision,
        cache: new TerrainPatchCache(programTotals.cells, programTotals.bytes),
      };
    const runtimeFields = snapshot.scene.terrain_programs.flatMap(
      (program, index) => {
        if (semanticGlobe) return [];
        return workingSetRequests[index]!.map((request) =>
          patchCacheRef.current!.cache.materialize(program, request),
        );
      },
    );
    const activeBandIds = Object.freeze(
      [
        ...(semanticGlobe ? ["semantic_globe"] : []),
        ...new Set(runtimeFields.flatMap((fields) => fields.active_band_ids)),
      ].sort((left, right) => left.localeCompare(right)),
    );
    const totals = runtimeFields.reduce(
      (result, fields) => ({
        field_cells: result.field_cells + fields.field_cells,
        field_bytes: result.field_bytes + fields.field_bytes,
      }),
      { field_cells: 0, field_bytes: 0 },
    );
    const preference = rendererPreference(window.location.search);
    if (preference === "reference") {
      setReady(false);
      onReport({
        ...REFERENCE_TERRAIN_REPORT,
        preference,
        status: {
          ...REFERENCE_TERRAIN_REPORT.status,
          lifecycle: "ready",
          detail: "the reference renderer was selected by the view envelope",
        },
        active_band_ids: activeBandIds,
        field_sets: runtimeFields.length,
        field_cells: totals.field_cells,
        field_bytes: totals.field_bytes,
        program_count: snapshot.scene.terrain_programs.length,
        working_set_limit_cells: programTotals.cells,
        working_set_limit_bytes: programTotals.bytes,
      });
      return;
    }

    let cancelled = false;
    let adapter:
      import("./three-webgpu").ThreeWebGpuRendererAdapter | undefined;
    let bundle: import("./three-terrain").ThreeTerrainBundle | undefined;
    const report = (
      status: RendererStatus,
      statistics?: { field_sets: number; triangles: number },
    ) => {
      if (cancelled) return;
      onReport({
        status,
        preference,
        active_band_ids: activeBandIds,
        field_sets: statistics?.field_sets ?? runtimeFields.length,
        field_cells: totals.field_cells,
        field_bytes: totals.field_bytes,
        program_count: snapshot.scene.terrain_programs.length,
        working_set_limit_cells: programTotals.cells,
        working_set_limit_bytes: programTotals.bytes,
        triangles: statistics?.triangles ?? 0,
      });
    };
    report({
      lifecycle: "initializing",
      backend: null,
      renderer_revision: "three@0.185.1:webgpu+tsl",
      degraded: false,
      detail: "initializing the continuous-relief renderer",
    });

    const handleContextLoss = (event: Event) => {
      event.preventDefault();
      setReady(false);
      report({
        lifecycle: "failed",
        backend: "reference",
        renderer_revision: "three@0.185.1:webgpu+tsl",
        degraded: true,
        detail: "graphics context lost; the reference terrain remains active",
      });
    };
    canvas.addEventListener("webglcontextlost", handleContextLoss);

    void (async () => {
      try {
        const rendererModule = await import("./three-webgpu");
        if (cancelled) return;
        adapter = new rendererModule.ThreeWebGpuRendererAdapter();
        adapterRef.current = adapter;
        adapter.resize({
          width: semanticGlobe
            ? snapshot.scene.world.width
            : view.viewport_width,
          height: semanticGlobe
            ? snapshot.scene.world.height
            : view.viewport_height,
          device_pixel_ratio: window.devicePixelRatio,
        });
        const status = await adapter.initialize(
          canvas,
          preference === "auto" ? "auto" : preference,
        );
        if (cancelled) return;
        if (status.lifecycle !== "ready") {
          setReady(false);
          report(status);
          return;
        }
        bundle = semanticGlobe
          ? (await import("./three-globe")).createContextGlobeBundle(
              semanticGlobe,
              snapshot.scene.world,
              globeViewRef.current,
            )
          : (await import("./three-terrain")).createContinuousReliefBundle(
              runtimeFields,
              snapshot.scene.world,
              view,
            );
        bundleRef.current = bundle;
        adapter.render(bundle.scene, bundle.camera, {
          snapshot_id: snapshot.snapshot_id,
          camera_revision: semanticGlobe
            ? `perspective-globe:${snapshot.scene.world.width}x${snapshot.scene.world.height}`
            : `orthographic:${view.viewport_width}x${view.viewport_height}:${view.rendered_scale}:${view.pan_x}:${view.pan_y}`,
          material_revision: bundle.material_revision,
        });
        setReady(true);
        report(status, bundle.statistics);
      } catch (error) {
        setReady(false);
        report({
          lifecycle: "failed",
          backend: "reference",
          renderer_revision: "three@0.185.1:webgpu+tsl",
          degraded: true,
          detail: error instanceof Error ? error.message : String(error),
        });
      }
    })();

    return () => {
      cancelled = true;
      canvas.removeEventListener("webglcontextlost", handleContextLoss);
      bundle?.dispose();
      adapter?.dispose();
      if (bundleRef.current === bundle) bundleRef.current = undefined;
      if (adapterRef.current === adapter) adapterRef.current = undefined;
    };
  }, [onReport, snapshot.snapshot_id, workingSetRevision]);

  useEffect(() => {
    const adapter = adapterRef.current;
    const bundle = bundleRef.current;
    if (!adapter || !bundle || !semanticGlobe) return;
    bundle.updateGlobeView?.(globeView);
    adapter.render(bundle.scene, bundle.camera, {
      snapshot_id: snapshot.snapshot_id,
      camera_revision: `orthographic-globe:${globeView.yaw_degrees}:${globeView.pitch_degrees}`,
      material_revision: bundle.material_revision,
    });
  }, [
    globeView.pitch_degrees,
    globeView.yaw_degrees,
    semanticGlobe,
    snapshot.snapshot_id,
  ]);

  useEffect(() => {
    const adapter = adapterRef.current;
    const bundle = bundleRef.current;
    if (!adapter || !bundle || semanticGlobe) return;
    adapter.resize({
      width: view.viewport_width,
      height: view.viewport_height,
      device_pixel_ratio: window.devicePixelRatio,
    });
    bundle.updateView?.(view);
    adapter.render(bundle.scene, bundle.camera, {
      snapshot_id: snapshot.snapshot_id,
      camera_revision: `orthographic:${view.viewport_width}x${view.viewport_height}:${view.rendered_scale}:${view.pan_x}:${view.pan_y}`,
      material_revision: bundle.material_revision,
    });
  }, [
    semanticGlobe,
    snapshot.snapshot_id,
    view.pan_x,
    view.pan_y,
    view.rendered_scale,
    view.viewport_height,
    view.viewport_width,
  ]);

  return (
    <canvas
      aria-hidden="true"
      className={sx(
        styles.acceleratedTerrainCanvas,
        ready && visible && styles.acceleratedTerrainCanvasReady,
      )}
      data-renderer="three-webgpu"
      ref={canvasRef}
    />
  );
}
