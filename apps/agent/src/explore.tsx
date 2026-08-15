import { KineticButton } from "@hifi/kinetic";
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import type { WorkloadList } from "./domain";
import {
  explorerViewPath,
  type ExplorerViewResolution,
} from "./explorer-coordinate";
import {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  WORLD_ATLAS_MORPH_END_ZOOM,
  WORLD_LENS_ZOOM,
  clampLensZoom,
  DEFAULT_GLOBE_VIEW,
  draggedGlobeView,
  fitScaleForViewport,
  lensRegimeForZoom,
  panForFocusedPoint,
  panForScaleAtPoint,
  pointerWithinRenderedGlobeAtmosphere,
  recenterWrappedChartPan,
  renderedSceneScale,
  smoothZoomStep,
  stepLensZoom,
  wheelZoomDelta,
  worldAtlasMorphProgress,
  type LensRegime,
  type GlobeCameraView,
} from "./explore/engine/camera";
import { LastGoodSceneCompiler } from "./explore/engine/scene";
import { admittedTopographies } from "./explore/projection/topography-projector";
import { admittedRegionalScenes } from "./explore/projection/regional-scene-projector";
import {
  countyFrameView,
  countyLocalToNativePosition,
  invertCountyScreen,
} from "./explore/projection/county-frame";
import { invertViewAlignedSemanticMercator } from "./explore/projection/semantic-mercator";
import {
  AcceleratedTerrainSurface,
  REFERENCE_TERRAIN_REPORT,
  type AcceleratedTerrainReport,
} from "./explore/renderers/accelerated-terrain";
import {
  ReferenceMapReading,
  ReferenceRenderer,
  type FocusableTopologyObject,
  type ReferenceLayerVisibility,
} from "./explore/renderers/reference";
import { exploreStyles as styles } from "./stylex/explore.stylex";
import { className as sx } from "./stylex/shared.stylex";
import type { TopologyScene } from "./topology";

interface ContextCanvasProps {
  portfolio: WorkloadList;
  coordinate?: ExplorerViewResolution;
}

interface Point {
  x: number;
  y: number;
}

const zeroPoint: Point = { x: 0, y: 0 };
const visibleReferenceLayers: ReferenceLayerVisibility = {
  relief: true,
  water: true,
  weather: true,
  probes: true,
};
const EXPLORER_NOTICE_DURATION_MS = 4_800;
const EXPLORER_ATTENTION_DURATION_MS = 7_200;

export type ExplorerFooterNoticeTone = "guide" | "update" | "attention";

export interface ExplorerFooterNotice {
  id: string;
  message: string;
  tone: ExplorerFooterNoticeTone;
  auto_hide_ms: number | null;
}

export interface ExplorerFooterState {
  has_interacted: boolean;
  next_notice_sequence: number;
  notice: ExplorerFooterNotice | null;
}

export type ExplorerFooterAction =
  | { type: "interact" }
  | {
      type: "publish";
      message: string;
      tone: Exclude<ExplorerFooterNoticeTone, "guide">;
      auto_hide_ms: number;
    }
  | { type: "expire"; notice_id: string };

export function initialExplorerFooterState(): ExplorerFooterState {
  return {
    has_interacted: false,
    next_notice_sequence: 1,
    notice: {
      id: "explorer-notice:onboarding",
      message:
        "WHEEL / + − TO CHANGE LENS · DRAG TO ORBIT · SELECT TO TRAVERSE",
      tone: "guide",
      auto_hide_ms: null,
    },
  };
}

export function explorerFooterReducer(
  state: ExplorerFooterState,
  action: ExplorerFooterAction,
): ExplorerFooterState {
  if (action.type === "interact") {
    if (state.has_interacted && state.notice === null) return state;
    return { ...state, has_interacted: true, notice: null };
  }
  if (action.type === "publish") {
    return {
      has_interacted: state.has_interacted,
      next_notice_sequence: state.next_notice_sequence + 1,
      notice: {
        id: `explorer-notice:${state.next_notice_sequence}`,
        message: action.message,
        tone: action.tone,
        auto_hide_ms: action.auto_hide_ms,
      },
    };
  }
  if (state.notice?.id !== action.notice_id) return state;
  return { ...state, notice: null };
}

export function ExplorePage({ portfolio, coordinate }: ContextCanvasProps) {
  return (
    <main className={sx(styles.explorePage)}>
      {coordinate && coordinate.status !== "current" ? (
        <div className={sx(styles.coordinateBoundary)} role="status">
          <strong>{coordinate.status.toUpperCase()} COORDINATE</strong>
          <code>{explorerViewPath(coordinate.view)}</code>
          <span>
            {coordinate.actual_revision
              ? `CURRENT BINDING / ${coordinate.actual_revision}`
              : "NO CURRENT OBJECT SATISFIES THIS IDENTITY"}
          </span>
        </div>
      ) : null}
      <ContextCanvas coordinate={coordinate} portfolio={portfolio} />
    </main>
  );
}

export function ContextCanvas({ portfolio, coordinate }: ContextCanvasProps) {
  const shellRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const wheelHandlerRef = useRef<(event: globalThis.WheelEvent) => void>(
    () => undefined,
  );
  const animatedZoomRef = useRef<(zoom: number, client?: Point) => void>(
    () => undefined,
  );
  const wheelAnimationFrameRef = useRef<number | null>(null);
  const wheelAnimationTimestampRef = useRef<number | null>(null);
  const wheelPointerRef = useRef<Point | undefined>(undefined);
  const dragRef = useRef<
    | {
        pointerId: number;
        origin: Point;
        pan: Point;
        globeView: GlobeCameraView;
        distance: number;
        mode: "orbit" | "pan";
      }
    | undefined
  >(undefined);
  const initialZoom =
    admittedTopographies(portfolio).length === 0 &&
    admittedRegionalScenes(portfolio).length === 0
      ? WORLD_LENS_ZOOM
      : DEFAULT_LENS_ZOOM;
  const [zoom, setZoom] = useState(
    coordinate ? coordinate.view.scale : initialZoom,
  );
  const zoomRef = useRef(zoom);
  const wheelTargetZoomRef = useRef(zoom);
  const [pan, setPan] = useState<Point>(zeroPoint);
  const [globeView, setGlobeView] =
    useState<GlobeCameraView>(DEFAULT_GLOBE_VIEW);
  const [fitScale, setFitScale] = useState(1);
  const [viewportSize, setViewportSize] = useState({ width: 1, height: 1 });
  const [focusId, setFocusId] = useState(
    coordinate?.focus_id ?? "cluster:portfolio",
  );
  const [isDragging, setIsDragging] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [terrainRenderer, setTerrainRenderer] =
    useState<AcceleratedTerrainReport>(REFERENCE_TERRAIN_REPORT);
  const [footerState, dispatchFooter] = useReducer(
    explorerFooterReducer,
    undefined,
    initialExplorerFooterState,
  );
  const suppressNextRegimeNoticeRef = useRef(false);
  const layers = visibleReferenceLayers;
  const [sceneCompiler] = useState(() => new LastGoodSceneCompiler());
  const retainedRegime = useRef<LensRegime | undefined>(undefined);
  const regime = lensRegimeForZoom(zoom, retainedRegime.current);
  useEffect(() => {
    retainedRegime.current = regime;
  }, [regime]);
  const measuredSceneProjection = useMemo(() => {
    const started = globalThis.performance?.now() ?? Date.now();
    const projection = sceneCompiler.compile(
      portfolio,
      DEFAULT_LENS_ZOOM,
      focusId,
      regime,
    );
    return {
      compilation_ms: (globalThis.performance?.now() ?? Date.now()) - started,
      projection,
    };
  }, [focusId, portfolio, regime, sceneCompiler]);
  const sceneProjection = measuredSceneProjection.projection;
  const snapshot = sceneProjection.snapshot;
  const scene = snapshot.scene;
  const projectionMorphProgress = worldAtlasMorphProgress(zoom);
  const projectionMorphActive =
    scene.world_atlas_transition !== null &&
    projectionMorphProgress > 0 &&
    projectionMorphProgress < 1;
  const wrappedAtlasActive =
    scene.regime === "atlas" &&
    scene.world_atlas_transition !== null &&
    projectionMorphProgress >= 1;
  const renderedScale = renderedSceneScale(
    scene.terrain || scene.county_frame !== null,
    fitScale,
    zoom,
    scene.regime,
  );
  const acceleratedReady =
    terrainRenderer.status.lifecycle === "ready" &&
    (terrainRenderer.status.backend === "webgpu" ||
      terrainRenderer.status.backend === "webgl2");
  const renderVisibility = {
    contours: layers.relief,
    water: layers.water,
    weather: layers.weather,
    probes: layers.probes,
  };
  const geographicCoordinate = explorerGeographicCoordinate(
    scene,
    pan,
    renderedScale,
    globeView,
  );
  const sourceRevisionKey = snapshot.source_revisions.join(",");
  const previousNoticeRegimeRef = useRef(regime);
  const previousSourceRevisionKeyRef = useRef(sourceRevisionKey);
  const previousRevalidationNoticeRef = useRef("");
  const previousRendererNoticeRef = useRef("");

  const publishFooterNotice = (
    message: string,
    tone: Exclude<ExplorerFooterNoticeTone, "guide"> = "update",
    autoHideMs = EXPLORER_NOTICE_DURATION_MS,
  ) =>
    dispatchFooter({
      type: "publish",
      message,
      tone,
      auto_hide_ms: autoHideMs,
    });

  const acknowledgeMapInteraction = () => {
    const firstInteraction = !footerState.has_interacted;
    dispatchFooter({ type: "interact" });
    return firstInteraction;
  };

  const cancelWheelZoom = () => {
    if (wheelAnimationFrameRef.current !== null)
      window.cancelAnimationFrame(wheelAnimationFrameRef.current);
    wheelAnimationFrameRef.current = null;
    wheelAnimationTimestampRef.current = null;
    wheelTargetZoomRef.current = zoomRef.current;
  };

  const commitImmediateZoom = (nextZoom: number) => {
    cancelWheelZoom();
    const boundedZoom = clampLensZoom(nextZoom);
    zoomRef.current = boundedZoom;
    wheelTargetZoomRef.current = boundedZoom;
    setZoom(boundedZoom);
  };

  useEffect(() => {
    if (!coordinate) return;
    setFocusId(coordinate.focus_id);
    commitImmediateZoom(coordinate.view.scale);
    setPan(zeroPoint);
  }, [coordinate?.focus_id, coordinate?.status, coordinate?.view.scale]);

  useEffect(() => {
    const notice = footerState.notice;
    if (!notice || notice.auto_hide_ms === null) return;
    const timeout = window.setTimeout(
      () => dispatchFooter({ type: "expire", notice_id: notice.id }),
      notice.auto_hide_ms,
    );
    return () => window.clearTimeout(timeout);
  }, [footerState.notice?.auto_hide_ms, footerState.notice?.id]);

  useEffect(() => {
    const previousRegime = previousNoticeRegimeRef.current;
    previousNoticeRegimeRef.current = regime;
    if (previousRegime === regime) return;
    if (suppressNextRegimeNoticeRef.current) {
      suppressNextRegimeNoticeRef.current = false;
      return;
    }
    if (!footerState.has_interacted) return;
    publishFooterNotice(`LENS / ${lensLabel(regime)} · ${scene.label}`);
  }, [footerState.has_interacted, regime, scene.label]);

  useEffect(() => {
    const previousKey = previousSourceRevisionKeyRef.current;
    previousSourceRevisionKeyRef.current = sourceRevisionKey;
    if (previousKey === sourceRevisionKey || !footerState.has_interacted)
      return;
    const sourceCount = snapshot.source_revisions.length;
    publishFooterNotice(
      `MAP UPDATED / ${sourceCount} BOUND SOURCE REVISION${sourceCount === 1 ? "" : "S"}`,
    );
  }, [footerState.has_interacted, sourceRevisionKey]);

  useEffect(() => {
    const revalidationKey = sceneProjection.retained_last_good
      ? (sceneProjection.error?.message ?? "last-good scene retained")
      : "";
    const previousKey = previousRevalidationNoticeRef.current;
    previousRevalidationNoticeRef.current = revalidationKey;
    if (!revalidationKey || previousKey === revalidationKey) return;
    publishFooterNotice(
      "SCENE REVALIDATION DELAYED · LAST-GOOD MAP RETAINED",
      "attention",
      EXPLORER_ATTENTION_DURATION_MS,
    );
  }, [sceneProjection.error?.message, sceneProjection.retained_last_good]);

  useEffect(() => {
    const status = terrainRenderer.status;
    const needsAttention = status.degraded || status.lifecycle === "failed";
    const rendererKey = needsAttention
      ? `${status.lifecycle}:${status.backend}:${status.detail}`
      : "";
    const previousKey = previousRendererNoticeRef.current;
    previousRendererNoticeRef.current = rendererKey;
    if (!rendererKey || previousKey === rendererKey) return;
    publishFooterNotice(
      `RENDERER DEGRADED / ${status.backend?.toUpperCase() ?? "REFERENCE"} · ${status.detail}`,
      "attention",
      EXPLORER_ATTENTION_DURATION_MS,
    );
  }, [
    terrainRenderer.status.backend,
    terrainRenderer.status.degraded,
    terrainRenderer.status.detail,
    terrainRenderer.status.lifecycle,
  ]);

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const measure = () => {
      const { width, height } = viewport.getBoundingClientRect();
      setViewportSize({ width, height });
      const next = fitScaleForViewport({ width, height }, scene.fit_world);
      setFitScale(next);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    return () => observer.disconnect();
  }, [scene.fit_world.height, scene.fit_world.width]);

  useEffect(() => {
    const syncFullscreen = () =>
      setIsFullscreen(document.fullscreenElement === shellRef.current);
    document.addEventListener("fullscreenchange", syncFullscreen);
    return () =>
      document.removeEventListener("fullscreenchange", syncFullscreen);
  }, []);

  const applyZoomAt = (nextZoom: number, client?: Point) => {
    const currentZoom = zoomRef.current;
    const boundedZoom = clampLensZoom(nextZoom);
    if (boundedZoom === currentZoom) return;
    const nextRegime = lensRegimeForZoom(boundedZoom, regime);
    const viewport = viewportRef.current;
    if (client && viewport) {
      const rect = viewport.getBoundingClientRect();
      const pointer = {
        x: client.x - rect.left - rect.width / 2,
        y: client.y - rect.top - rect.height / 2,
      };
      const scaleUsesTerrain = scene.terrain || scene.county_frame !== null;
      const currentRenderedScale = renderedSceneScale(
        scaleUsesTerrain,
        fitScale,
        currentZoom,
        regime,
      );
      const nextRenderedScale = renderedSceneScale(
        scaleUsesTerrain,
        fitScale,
        boundedZoom,
        nextRegime,
      );
      setPan((currentPan) =>
        panForScaleAtPoint(
          currentPan,
          pointer,
          currentRenderedScale,
          nextRenderedScale,
        ),
      );
    }
    zoomRef.current = boundedZoom;
    setZoom(boundedZoom);
  };
  animatedZoomRef.current = applyZoomAt;

  const setZoomAt = (nextZoom: number, client?: Point) => {
    const firstInteraction = acknowledgeMapInteraction();
    cancelWheelZoom();
    const boundedZoom = clampLensZoom(nextZoom);
    const nextRegime = lensRegimeForZoom(boundedZoom, regime);
    if (firstInteraction && nextRegime !== regime)
      suppressNextRegimeNoticeRef.current = true;
    applyZoomAt(boundedZoom, client);
    wheelTargetZoomRef.current = boundedZoom;
  };

  const focusNode = (node: FocusableTopologyObject) => {
    if (dragRef.current?.distance && dragRef.current.distance > 4) return;
    const firstInteraction = acknowledgeMapInteraction();
    cancelWheelZoom();
    if (!firstInteraction)
      publishFooterNotice(`FOCUS / ${focusNoticeLabel(scene, node.focus_id)}`);
    if (node.focus_id.startsWith("beacon:")) {
      setFocusId(node.focus_id);
      return;
    }
    let nextZoom = zoom;
    if (scene.regime === "world") nextZoom = DEFAULT_LENS_ZOOM;
    else if (scene.regime === "atlas") nextZoom = LANDSCAPE_LENS_ZOOM;
    else if (scene.regime === "landscape") nextZoom = NEIGHBORHOOD_LENS_ZOOM;
    else if (scene.regime === "neighborhoods") nextZoom = OBJECT_LENS_ZOOM;
    else if (scene.regime === "objects") nextZoom = EVIDENCE_LENS_ZOOM;
    if (lensRegimeForZoom(nextZoom, regime) !== regime)
      suppressNextRegimeNoticeRef.current = true;
    setFocusId(node.focus_id);
    if (scene.terrain) {
      const nextScale = renderedSceneScale(
        true,
        fitScale,
        nextZoom,
        lensRegimeForZoom(nextZoom, regime),
      );
      setPan(panForFocusedPoint(node, scene.world, nextScale));
    } else setPan(zeroPoint);
    commitImmediateZoom(nextZoom);
  };

  const startWheelZoom = () => {
    if (wheelAnimationFrameRef.current !== null) return;
    const animate = (timestamp: number) => {
      const previousTimestamp = wheelAnimationTimestampRef.current;
      const elapsedMs =
        previousTimestamp === null
          ? 16
          : Math.max(0, timestamp - previousTimestamp);
      wheelAnimationTimestampRef.current = timestamp;
      const targetZoom = wheelTargetZoomRef.current;
      const nextZoom = smoothZoomStep(zoomRef.current, targetZoom, elapsedMs);
      animatedZoomRef.current(nextZoom, wheelPointerRef.current);
      if (nextZoom === targetZoom) {
        wheelAnimationFrameRef.current = null;
        wheelAnimationTimestampRef.current = null;
        return;
      }
      wheelAnimationFrameRef.current = window.requestAnimationFrame(animate);
    };
    wheelAnimationFrameRef.current = window.requestAnimationFrame(animate);
  };

  const handleWheel = (event: globalThis.WheelEvent) => {
    event.preventDefault();
    const firstInteraction = acknowledgeMapInteraction();
    const targetZoom = clampLensZoom(
      wheelTargetZoomRef.current +
        wheelZoomDelta(
          wheelTargetZoomRef.current,
          event.deltaY,
          event.deltaMode,
        ),
    );
    if (firstInteraction && lensRegimeForZoom(targetZoom, regime) !== regime)
      suppressNextRegimeNoticeRef.current = true;
    wheelTargetZoomRef.current = targetZoom;
    wheelPointerRef.current = { x: event.clientX, y: event.clientY };
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches) {
      animatedZoomRef.current(targetZoom, wheelPointerRef.current);
      return;
    }
    startWheelZoom();
  };
  wheelHandlerRef.current = handleWheel;

  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const onWheel = (event: globalThis.WheelEvent) =>
      wheelHandlerRef.current(event);
    viewport.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      viewport.removeEventListener("wheel", onWheel);
      if (wheelAnimationFrameRef.current !== null)
        window.cancelAnimationFrame(wheelAnimationFrameRef.current);
    };
  }, []);

  const beginPan = (event: PointerEvent<HTMLDivElement>) => {
    const target = event.target;
    if (
      event.button !== 0 ||
      (target instanceof Element && target.closest("button, a"))
    )
      return;
    acknowledgeMapInteraction();
    event.currentTarget.setPointerCapture(event.pointerId);
    const bounds = event.currentTarget.getBoundingClientRect();
    const mode =
      scene.globe &&
      pointerWithinRenderedGlobeAtmosphere(
        { x: event.clientX - bounds.left, y: event.clientY - bounds.top },
        { width: bounds.width, height: bounds.height },
        scene.world,
        renderedScale,
        pan,
      )
        ? "orbit"
        : "pan";
    dragRef.current = {
      pointerId: event.pointerId,
      origin: { x: event.clientX, y: event.clientY },
      pan,
      globeView,
      distance: 0,
      mode,
    };
    setIsDragging(true);
  };

  const movePan = (event: PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const delta = {
      x: event.clientX - drag.origin.x,
      y: event.clientY - drag.origin.y,
    };
    drag.distance = Math.hypot(delta.x, delta.y);
    if (drag.mode === "orbit")
      setGlobeView(draggedGlobeView(drag.globeView, delta));
    else {
      const nextPan = { x: drag.pan.x + delta.x, y: drag.pan.y + delta.y };
      setPan(
        wrappedAtlasActive
          ? recenterWrappedChartPan(nextPan, scene.world.width * renderedScale)
          : nextPan,
      );
    }
  };

  const endPan = (event: PointerEvent<HTMLDivElement>) => {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    event.currentTarget.releasePointerCapture(event.pointerId);
    setIsDragging(false);
    window.setTimeout(() => {
      dragRef.current = undefined;
    }, 0);
  };

  const resetView = () => {
    acknowledgeMapInteraction();
    const nextZoom =
      scene.globe?.posture === "orientation"
        ? WORLD_LENS_ZOOM
        : DEFAULT_LENS_ZOOM;
    if (lensRegimeForZoom(nextZoom, regime) !== regime)
      suppressNextRegimeNoticeRef.current = true;
    commitImmediateZoom(nextZoom);
    setPan(zeroPoint);
    setGlobeView(DEFAULT_GLOBE_VIEW);
    setFocusId("cluster:portfolio");
  };

  const toggleFullscreen = async () => {
    acknowledgeMapInteraction();
    const shell = shellRef.current;
    if (!shell) return;
    if (document.fullscreenElement === shell) await document.exitFullscreen();
    else await shell.requestFullscreen();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      setZoomAt(stepLensZoom(zoom, 1));
    } else if (event.key === "-") {
      event.preventDefault();
      setZoomAt(stepLensZoom(zoom, -1));
    } else if (event.key === "0") {
      event.preventDefault();
      resetView();
    }
  };

  const sceneStyle = {
    "--rey-terrain-counter-scale":
      scene.terrain || scene.county_frame
        ? (scene.regime === "world"
            ? WORLD_LENS_ZOOM
            : WORLD_ATLAS_MORPH_END_ZOOM) / zoom
        : 1,
    height: scene.world.height,
    transform: `translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${renderedScale})`,
    width: scene.world.width,
  } as CSSProperties;

  return (
    <section
      className={sx(
        styles.canvasShell,
        scene.globe?.posture === "orientation" && styles.orientationCanvasShell,
        isFullscreen && styles.canvasShellFullscreen,
      )}
      data-scene-compilers={snapshot.compiler_revisions.join(",")}
      data-scene-compilation-ms={measuredSceneProjection.compilation_ms}
      data-scene-focus={snapshot.focus_id}
      data-scene-snapshot={snapshot.snapshot_id}
      data-scene-sources={snapshot.source_revisions.join(",")}
      ref={shellRef}
    >
      {sceneProjection.retained_last_good ? (
        <div
          className={sx(styles.coordinateBoundary)}
          data-scene-projection="last-good"
          role="status"
        >
          <strong>SCENE REVALIDATION DELAYED</strong>
          <code>{snapshot.snapshot_id}</code>
          <span>
            LAST-GOOD IMMUTABLE SCENE RETAINED /{" "}
            {sceneProjection.error?.message}
          </span>
        </div>
      ) : null}
      <CanvasToolbar
        isFullscreen={isFullscreen}
        onFit={resetView}
        onFullscreen={() => void toggleFullscreen()}
        onZoomIn={() => setZoomAt(stepLensZoom(zoom, 1))}
        onZoomOut={() => setZoomAt(stepLensZoom(zoom, -1))}
        scene={scene}
        zoom={zoom}
      />
      <div
        aria-label="Interactive context topology map. Drag the globe to orbit or the surrounding canvas to pan; use the mouse wheel or plus and minus keys to move through semantic lens levels."
        className={sx(
          styles.canvasViewport,
          (scene.terrain || scene.county_frame) && styles.terrainViewport,
          scene.globe?.posture === "orientation" && styles.orientationViewport,
          isDragging && styles.canvasViewportDragging,
        )}
        onKeyDown={handleKeyDown}
        onPointerCancel={endPan}
        onPointerDown={beginPan}
        onPointerMove={movePan}
        onPointerUp={endPan}
        ref={viewportRef}
        role="application"
        tabIndex={0}
        data-camera-pan-x={pan.x}
        data-camera-pan-y={pan.y}
        data-camera-rendered-scale={renderedScale}
        data-camera-zoom={zoom}
        data-globe-pitch={scene.globe ? globeView.pitch_degrees : undefined}
        data-globe-yaw={scene.globe ? globeView.yaw_degrees : undefined}
      >
        {scene.terrain && scene.globe === null ? (
          <AcceleratedTerrainSurface
            onReport={setTerrainRenderer}
            renderVisibility={renderVisibility}
            snapshot={snapshot}
            view={{
              world_width: scene.world.width,
              world_height: scene.world.height,
              viewport_width: viewportSize.width,
              viewport_height: viewportSize.height,
              rendered_scale: renderedScale,
              pan_x: pan.x,
              pan_y: pan.y,
            }}
            visible={!projectionMorphActive}
          />
        ) : null}
        <div
          className={sx(styles.scene, isDragging && styles.sceneDragging)}
          style={sceneStyle}
        >
          {scene.globe ||
          (scene.regime === "atlas" && scene.world_atlas_transition) ? (
            <AcceleratedTerrainSurface
              globeView={globeView}
              onReport={setTerrainRenderer}
              projectionMorphProgress={projectionMorphProgress}
              renderVisibility={renderVisibility}
              snapshot={snapshot}
              view={{
                world_width: scene.world.width,
                world_height: scene.world.height,
                viewport_width: viewportSize.width,
                viewport_height: viewportSize.height,
                rendered_scale: renderedScale,
                pan_x: pan.x,
                pan_y: pan.y,
              }}
              visible
            />
          ) : null}
          <ReferenceRenderer
            accelerated={acceleratedReady}
            globeView={globeView}
            layers={layers}
            onFocus={focusNode}
            projectionMorphProgress={projectionMorphProgress}
            renderGraph={snapshot.render_graph}
            pickingIndex={snapshot.picking_index}
            scene={scene}
          />
        </div>
        <div
          className={sx(
            styles.canvasCoordinates,
            scene.globe?.posture === "orientation" &&
              styles.orientationCoordinates,
            footerState.notice && styles.canvasCoordinatesFooterVisible,
          )}
          data-renderer-active-band-ids={terrainRenderer.active_band_ids.join(
            ",",
          )}
          data-renderer-active-tile-count={terrainRenderer.active_tile_count}
          data-renderer-active-tile-levels={terrainRenderer.active_tile_levels.join(
            ",",
          )}
          data-renderer-active-render-passes={terrainRenderer.active_render_passes.join(
            ",",
          )}
          data-renderer-backend={terrainRenderer.status.backend}
          data-renderer-degraded={String(terrainRenderer.status.degraded)}
          data-renderer-diagnostics="rey.explorer-renderer-diagnostics.v1"
          data-renderer-field-bytes={terrainRenderer.field_bytes}
          data-renderer-field-cells={terrainRenderer.field_cells}
          data-renderer-field-evaluation-ms={
            terrainRenderer.field_evaluation_ms
          }
          data-renderer-draw-calls={terrainRenderer.draw_calls}
          data-renderer-geometry-compilation-ms={
            terrainRenderer.geometry_compilation_ms
          }
          data-renderer-gpu-budget-bytes={terrainRenderer.gpu_budget_bytes}
          data-renderer-gpu-bytes={terrainRenderer.gpu_bytes}
          data-renderer-lifecycle={terrainRenderer.status.lifecycle}
          data-renderer-measurement-authority={
            terrainRenderer.measurement_authority
          }
          data-renderer-parity-samples={terrainRenderer.parity_samples}
          data-renderer-parity-revision={terrainRenderer.parity_revision}
          data-renderer-preference={terrainRenderer.preference}
          data-renderer-program-count={terrainRenderer.program_count}
          data-renderer-resident-cpu-budget-bytes={
            terrainRenderer.resident_cpu_budget_bytes
          }
          data-renderer-resident-cpu-bytes={terrainRenderer.resident_cpu_bytes}
          data-renderer-resident-evictions={terrainRenderer.resident_evictions}
          data-renderer-resident-gpu-budget-bytes={
            terrainRenderer.resident_gpu_budget_bytes
          }
          data-renderer-resident-gpu-bytes={terrainRenderer.resident_gpu_bytes}
          data-renderer-resident-hits={terrainRenderer.resident_hits}
          data-renderer-resident-misses={terrainRenderer.resident_misses}
          data-renderer-resident-tile-count={
            terrainRenderer.resident_tile_count
          }
          data-renderer-render-graph-id={terrainRenderer.render_graph_id}
          data-renderer-revision={terrainRenderer.status.renderer_revision}
          data-renderer-submission-ms={terrainRenderer.render_submission_ms}
          data-renderer-terrain-decode-ms={terrainRenderer.terrain_decode_ms}
          data-renderer-terrain-tile-projection-ms={
            terrainRenderer.terrain_tile_projection_ms
          }
          data-renderer-terrain-update-ms={terrainRenderer.terrain_update_ms}
          data-renderer-terrain-worker-execution={
            terrainRenderer.terrain_worker_execution
          }
          data-renderer-terrain-worker-revision={
            terrainRenderer.terrain_worker_revision
          }
          data-renderer-triangles={terrainRenderer.triangles}
          data-renderer-working-set-limit-bytes={
            terrainRenderer.working_set_limit_bytes
          }
          data-renderer-working-set-limit-cells={
            terrainRenderer.working_set_limit_cells
          }
          aria-hidden="true"
        >
          <span>ZOOM {Math.round(zoom * 100)}%</span>
          <span data-coordinate-authority={geographicCoordinate?.authority}>
            {formatGeographicCoordinate(geographicCoordinate)}
          </span>
        </div>
        <div
          className={sx(
            styles.lensLegend,
            scene.globe?.posture === "orientation" &&
              styles.orientationLensLegend,
          )}
        >
          <LensStep active={scene.regime === "world"} label="WORLD" />
          <i className={sx(styles.legendLine)} />
          <LensStep active={scene.regime === "atlas"} label="ATLAS" />
          <i className={sx(styles.legendLine)} />
          <LensStep active={scene.regime === "landscape"} label="LANDSCAPE" />
          <i className={sx(styles.legendLine)} />
          <LensStep
            active={scene.regime === "neighborhoods"}
            label="NEIGHBORHOODS"
          />
          <i className={sx(styles.legendLine)} />
          <LensStep active={scene.regime === "objects"} label="OBJECTS" />
          <i className={sx(styles.legendLine)} />
          <LensStep active={scene.regime === "evidence"} label="EVIDENCE" />
        </div>
        <ReferenceMapReading scene={scene} />
      </div>
      <CanvasFooter
        coordinate={coordinate}
        notice={footerState.notice}
        scene={scene}
      />
    </section>
  );
}

function focusNoticeLabel(scene: TopologyScene, focusId: string): string {
  const object =
    scene.points.find(({ focus_id }) => focus_id === focusId) ??
    scene.nodes.find(({ focus_id }) => focus_id === focusId) ??
    scene.globe?.regions.find(({ focus_id }) => focus_id === focusId) ??
    scene.globe?.beacons.find(({ focus_id }) => focus_id === focusId);
  return object?.label ?? focusId;
}

export interface ExplorerGeographicCoordinate {
  latitude_degrees: number;
  longitude_degrees: number;
  authority: "globe_view" | "semantic_mercator" | "native_crs84";
}

export function explorerGeographicCoordinate(
  scene: TopologyScene,
  pan: Point,
  renderedScale: number,
  globeView: GlobeCameraView,
): ExplorerGeographicCoordinate | null {
  if (scene.globe) {
    return {
      latitude_degrees: globeView.pitch_degrees,
      longitude_degrees: normalizeLongitudeDegrees(globeView.yaw_degrees),
      authority: "globe_view",
    };
  }
  if (!Number.isFinite(renderedScale) || renderedScale <= 0) return null;
  const cameraCenter = {
    x: scene.world.width / 2 - pan.x / renderedScale,
    y: scene.world.height / 2 - pan.y / renderedScale,
  };
  if (scene.county_frame) {
    const local = invertCountyScreen(
      scene.county_frame,
      cameraCenter,
      countyFrameView(scene.county_frame, scene.world),
    );
    const native = countyLocalToNativePosition(scene.county_frame, local);
    return {
      latitude_degrees: native[1] / 1_000_000,
      longitude_degrees: native[0] / 1_000_000,
      authority: "native_crs84",
    };
  }
  if (scene.regime === "atlas") {
    const semantic = invertViewAlignedSemanticMercator(
      cameraCenter,
      {
        x: 0,
        y: 0,
        width: scene.world.width,
        height: scene.world.height,
      },
      globeView,
    ).coordinate;
    return {
      latitude_degrees: semantic.latitude_microdegrees / 1_000_000,
      longitude_degrees: semantic.longitude_microdegrees / 1_000_000,
      authority: "semantic_mercator",
    };
  }
  return null;
}

function formatGeographicCoordinate(
  coordinate: ExplorerGeographicCoordinate | null,
): string {
  if (!coordinate) return "LAT — / LON —";
  return `LAT ${coordinate.latitude_degrees.toFixed(4)}° / LON ${coordinate.longitude_degrees.toFixed(4)}°`;
}

function normalizeLongitudeDegrees(longitude: number): number {
  let normalized = longitude;
  while (normalized > 180) normalized -= 360;
  while (normalized < -180) normalized += 360;
  return normalized;
}

export function CanvasFooter({
  coordinate,
  notice,
  scene,
}: {
  coordinate?: ExplorerViewResolution;
  notice: ExplorerFooterNotice | null;
  scene: TopologyScene;
}) {
  return (
    <footer
      aria-atomic="true"
      aria-live="polite"
      className={sx(
        styles.canvasFooter,
        scene.globe?.posture === "orientation" &&
          styles.orientationCanvasFooter,
        notice && styles.canvasFooterVisible,
        notice?.tone === "attention" && styles.canvasFooterAttention,
      )}
      data-explorer-footer=""
      data-notice-id={notice?.id}
      data-notice-tone={notice?.tone ?? "quiet"}
      data-visible={String(notice !== null)}
      role="status"
    >
      {notice ? (
        <span
          className={sx(
            styles.canvasFooterNotice,
            coordinate && styles.canvasFooterNoticeWithCoordinate,
          )}
          key={notice.id}
        >
          {notice.message}
        </span>
      ) : null}
      {notice && coordinate ? (
        <code className={sx(styles.coordinateUri)}>
          {explorerViewPath(coordinate.view)}
        </code>
      ) : null}
    </footer>
  );
}

export function CanvasToolbar({
  isFullscreen,
  onFit,
  onFullscreen,
  onZoomIn,
  onZoomOut,
  scene,
  zoom,
}: {
  isFullscreen: boolean;
  onFit: () => void;
  onFullscreen: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  scene: TopologyScene;
  zoom: number;
}) {
  return (
    <header
      className={sx(
        styles.canvasToolbar,
        scene.globe?.posture === "orientation" &&
          styles.orientationCanvasToolbar,
      )}
      data-explorer-canvas-header=""
    >
      <div className={sx(styles.lensReadout)}>
        <span className={sx(styles.micro)}>
          LENS / {lensLabel(scene.regime)}
        </span>
        <strong>{scene.label}</strong>
      </div>
      <div className={sx(styles.canvasControls)}>
        <span className={sx(styles.micro, styles.zoomReadout)}>
          {Math.round(zoom * 100)}%
        </span>
        <KineticButton
          aria-label="Zoom out one semantic level"
          className={sx(styles.controlButton)}
          disabled={zoom <= MIN_LENS_ZOOM}
          onClick={onZoomOut}
          theme="precision"
        >
          −
        </KineticButton>
        <KineticButton
          aria-label="Zoom in one semantic level"
          className={sx(styles.controlButton)}
          disabled={zoom >= MAX_LENS_ZOOM}
          onClick={onZoomIn}
          theme="precision"
        >
          +
        </KineticButton>
        <KineticButton
          className={sx(styles.textControlButton)}
          onClick={onFit}
          theme="precision"
        >
          FIT
        </KineticButton>
        <KineticButton
          className={sx(styles.textControlButton)}
          onClick={onFullscreen}
          theme="precision"
        >
          {isFullscreen ? "EXIT FULL" : "FULL SCREEN"}
        </KineticButton>
      </div>
    </header>
  );
}

function LensStep({ active, label }: { active: boolean; label: string }) {
  return (
    <span className={sx(styles.lensStep, active && styles.lensStepActive)}>
      <i /> {label}
    </span>
  );
}

function lensLabel(regime: LensRegime): string {
  if (regime === "world") return "WORLD PROJECTION";
  if (regime === "atlas") return "ORBITAL";
  if (regime === "landscape") return "TELESCOPE";
  if (regime === "neighborhoods") return "MESOSCOPIC";
  if (regime === "objects") return "MICROSCOPE";
  return "EVIDENTIARY";
}
