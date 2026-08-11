import { useEffect, useRef, useState } from "react";
import type { RendererStatus } from "../engine/renderer";
import type { SceneSnapshot } from "../engine/scene";
import { exploreStyles as styles } from "../../stylex/explore.stylex";
import { className as sx } from "../../stylex/shared.stylex";

export type RendererPreference = "auto" | "webgpu" | "webgl2" | "reference";

export interface AcceleratedTerrainReport {
  status: RendererStatus;
  preference: RendererPreference;
  active_level_ids: readonly string[];
  field_sets: number;
  field_cells: number;
  field_bytes: number;
  pyramid_levels: number;
  pyramid_cells: number;
  pyramid_bytes: number;
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
    active_level_ids: Object.freeze([]),
    field_sets: 0,
    field_cells: 0,
    field_bytes: 0,
    pyramid_levels: 0,
    pyramid_cells: 0,
    pyramid_bytes: 0,
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
  visible,
}: {
  onReport: (report: AcceleratedTerrainReport) => void;
  snapshot: SceneSnapshot;
  visible: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    const semanticGlobe =
      snapshot.scene.regime === "world" ? snapshot.scene.globe : null;
    if (
      !canvas ||
      (snapshot.scene.terrain_fields.length === 0 && semanticGlobe === null)
    ) {
      setReady(false);
      onReport(REFERENCE_TERRAIN_REPORT);
      return;
    }
    const activeLevelIds = Object.freeze(
      [
        ...(semanticGlobe ? ["semantic_globe"] : []),
        ...new Set(
          snapshot.scene.terrain_fields.map((fields) => fields.level_id),
        ),
      ].sort((left, right) => left.localeCompare(right)),
    );
    const totals = snapshot.scene.terrain_fields.reduce(
      (result, fields) => ({
        field_cells: result.field_cells + fields.field_cells,
        field_bytes: result.field_bytes + fields.field_bytes,
      }),
      { field_cells: 0, field_bytes: 0 },
    );
    const pyramidTotals = snapshot.scene.terrain_pyramids.reduce(
      (result, pyramid) => ({
        levels: result.levels + pyramid.levels.length,
        cells: result.cells + pyramid.total_cells,
        bytes: result.bytes + pyramid.total_bytes,
      }),
      { levels: 0, cells: 0, bytes: 0 },
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
        active_level_ids: activeLevelIds,
        field_sets: snapshot.scene.terrain_fields.length,
        field_cells: totals.field_cells,
        field_bytes: totals.field_bytes,
        pyramid_levels: pyramidTotals.levels,
        pyramid_cells: pyramidTotals.cells,
        pyramid_bytes: pyramidTotals.bytes,
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
        active_level_ids: activeLevelIds,
        field_sets:
          statistics?.field_sets ?? snapshot.scene.terrain_fields.length,
        field_cells: totals.field_cells,
        field_bytes: totals.field_bytes,
        pyramid_levels: pyramidTotals.levels,
        pyramid_cells: pyramidTotals.cells,
        pyramid_bytes: pyramidTotals.bytes,
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
        adapter.resize({
          width: snapshot.scene.world.width,
          height: snapshot.scene.world.height,
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
          ? (await import("./three-globe")).createSemanticGlobeBundle(
              semanticGlobe,
              snapshot.scene.world,
            )
          : (await import("./three-terrain")).createContinuousReliefBundle(
              snapshot.scene.terrain_fields,
              snapshot.scene.world,
            );
        adapter.render(bundle.scene, bundle.camera, {
          snapshot_id: snapshot.snapshot_id,
          camera_revision: semanticGlobe
            ? `perspective-globe:${snapshot.scene.world.width}x${snapshot.scene.world.height}`
            : `orthographic:${snapshot.scene.world.width}x${snapshot.scene.world.height}`,
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
    };
  }, [onReport, snapshot.snapshot_id]);

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
