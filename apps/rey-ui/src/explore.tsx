import { KineticButton } from "@hifi/kinetic";
import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
  type WheelEvent,
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
  WORLD_LENS_ZOOM,
  clampLensZoom,
  DEFAULT_GLOBE_VIEW,
  draggedGlobeView,
  fitScaleForViewport,
  lensRegimeForZoom,
  panForFocusedPoint,
  panForZoomAtPoint,
  recenterWrappedChartPan,
  renderedSceneScale,
  stepLensZoom,
  worldAtlasMorphProgress,
  type LensRegime,
  type GlobeCameraView,
} from "./explore/engine/camera";
import { compileSceneSnapshot } from "./explore/engine/scene";
import { admittedTopographies } from "./explore/projection/topography-projector";
import { admittedRegionalScenes } from "./explore/projection/regional-scene-projector";
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
  const dragRef = useRef<
    | {
        pointerId: number;
        origin: Point;
        pan: Point;
        globeView: GlobeCameraView;
        distance: number;
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
  const [layers, setLayers] = useState<ReferenceLayerVisibility>({
    relief: true,
    water: true,
    weather: true,
    probes: true,
  });
  const retainedRegime = useRef<LensRegime | undefined>(undefined);
  const regime = lensRegimeForZoom(zoom, retainedRegime.current);
  useEffect(() => {
    retainedRegime.current = regime;
  }, [regime]);
  const snapshot = useMemo(
    () => compileSceneSnapshot(portfolio, DEFAULT_LENS_ZOOM, focusId, regime),
    [focusId, portfolio, regime],
  );
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
    scene.terrain,
    fitScale,
    zoom,
    scene.regime,
  );
  const acceleratedReady =
    terrainRenderer.status.lifecycle === "ready" &&
    (terrainRenderer.status.backend === "webgpu" ||
      terrainRenderer.status.backend === "webgl2");

  useEffect(() => {
    if (!coordinate) return;
    setFocusId(coordinate.focus_id);
    setZoom(coordinate.view.scale);
    setPan(zeroPoint);
  }, [coordinate?.focus_id, coordinate?.status, coordinate?.view.scale]);

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

  const setZoomAt = (nextZoom: number, client?: Point) => {
    const boundedZoom = clampLensZoom(nextZoom);
    if (boundedZoom === zoom) return;
    const viewport = viewportRef.current;
    if (client && viewport) {
      const rect = viewport.getBoundingClientRect();
      const pointer = {
        x: client.x - rect.left - rect.width / 2,
        y: client.y - rect.top - rect.height / 2,
      };
      setPan(panForZoomAtPoint(pan, pointer, zoom, boundedZoom));
    }
    setZoom(boundedZoom);
  };

  const focusNode = (node: FocusableTopologyObject) => {
    if (dragRef.current?.distance && dragRef.current.distance > 4) return;
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
    setFocusId(node.focus_id);
    if (scene.terrain) {
      const nextScale = fitScale * (nextZoom / DEFAULT_LENS_ZOOM);
      setPan(panForFocusedPoint(node, scene.world, nextScale));
    } else setPan(zeroPoint);
    setZoom(nextZoom);
  };

  const handleWheel = (event: WheelEvent<HTMLDivElement>) => {
    event.preventDefault();
    const signedDelta = Math.sign(-event.deltaY);
    const boundedDelta =
      signedDelta * Math.min(0.12, Math.abs(event.deltaY) * 0.0015);
    setZoomAt(zoom + boundedDelta, { x: event.clientX, y: event.clientY });
  };

  const beginPan = (event: PointerEvent<HTMLDivElement>) => {
    const target = event.target;
    if (
      event.button !== 0 ||
      (target instanceof Element && target.closest("button, a"))
    )
      return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      pointerId: event.pointerId,
      origin: { x: event.clientX, y: event.clientY },
      pan,
      globeView,
      distance: 0,
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
    if (scene.globe) setGlobeView(draggedGlobeView(drag.globeView, delta));
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
    setZoom(
      scene.globe?.posture === "orientation"
        ? WORLD_LENS_ZOOM
        : DEFAULT_LENS_ZOOM,
    );
    setPan(zeroPoint);
    setGlobeView(DEFAULT_GLOBE_VIEW);
    setFocusId("cluster:portfolio");
  };

  const toggleFullscreen = async () => {
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
    "--rey-terrain-counter-scale": scene.terrain
      ? (scene.regime === "world" ? WORLD_LENS_ZOOM : DEFAULT_LENS_ZOOM) / zoom
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
      ref={shellRef}
    >
      <CanvasToolbar
        isFullscreen={isFullscreen}
        layers={layers}
        onFit={resetView}
        onFullscreen={() => void toggleFullscreen()}
        onZoomIn={() => setZoomAt(stepLensZoom(zoom, 1))}
        onZoomOut={() => setZoomAt(stepLensZoom(zoom, -1))}
        onToggleLayer={(layer) =>
          setLayers((current) => ({ ...current, [layer]: !current[layer] }))
        }
        scene={scene}
        zoom={zoom}
      />
      <div
        aria-label="Interactive context topology map. Drag to pan, use the mouse wheel or plus and minus keys to move through semantic lens levels."
        className={sx(
          styles.canvasViewport,
          scene.terrain && styles.terrainViewport,
          scene.globe?.posture === "orientation" && styles.orientationViewport,
          isDragging && styles.canvasViewportDragging,
        )}
        onKeyDown={handleKeyDown}
        onPointerCancel={endPan}
        onPointerDown={beginPan}
        onPointerMove={movePan}
        onPointerUp={endPan}
        onWheel={handleWheel}
        ref={viewportRef}
        role="application"
        tabIndex={0}
      >
        {scene.terrain && scene.globe === null ? (
          <AcceleratedTerrainSurface
            onReport={setTerrainRenderer}
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
            visible={layers.relief && !projectionMorphActive}
          />
        ) : null}
        <div
          className={sx(styles.scene, isDragging && styles.sceneDragging)}
          style={sceneStyle}
        >
          {scene.globe ? (
            <AcceleratedTerrainSurface
              globeView={globeView}
              onReport={setTerrainRenderer}
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
              visible={layers.relief && !projectionMorphActive}
            />
          ) : null}
          <ReferenceRenderer
            accelerated={
              acceleratedReady && layers.relief && !projectionMorphActive
            }
            globeView={globeView}
            layers={layers}
            onFocus={focusNode}
            projectionMorphProgress={projectionMorphProgress}
            renderGraph={snapshot.render_graph}
            scene={scene}
          />
        </div>
        <div
          className={sx(
            styles.canvasCoordinates,
            scene.globe?.posture === "orientation" &&
              styles.orientationCoordinates,
          )}
          aria-hidden="true"
        >
          <span>ZOOM {Math.round(zoom * 100)}%</span>
          <span>
            {scene.globe
              ? `LON ${Math.round(globeView.yaw_degrees)}° / LAT ${Math.round(globeView.pitch_degrees)}°`
              : `X ${Math.round(pan.x)} / Y ${Math.round(pan.y)}`}
          </span>
          {scene.terrain ? (
            <span data-renderer-backend={terrainRenderer.status.backend}>
              RENDER / {terrainRenderer.status.backend?.toUpperCase() ?? "INIT"}
              {terrainRenderer.status.degraded ? " / DEGRADED" : ""}
            </span>
          ) : null}
          {terrainRenderer.field_cells > 0 ? (
            <>
              <span
                data-terrain-lod={terrainRenderer.active_band_ids.join(",")}
              >
                LOD /{" "}
                {terrainRenderer.active_band_ids.join(" + ").toUpperCase()}
              </span>
              <span>
                FIELD / {terrainRenderer.field_cells} CELLS /{" "}
                {Math.ceil(terrainRenderer.field_bytes / 1024)} KIB /{" "}
                {terrainRenderer.triangles} TRI
              </span>
              <span>
                PROGRAM / {terrainRenderer.program_count} / WORKING LIMIT /{" "}
                {terrainRenderer.working_set_limit_cells} CELLS /{" "}
                {Math.ceil(terrainRenderer.working_set_limit_bytes / 1024)} KIB
              </span>
              <span data-render-graph={terrainRenderer.render_graph_id}>
                GRAPH / {terrainRenderer.active_render_passes.length} PASSES
              </span>
              {terrainRenderer.gpu_budget_bytes > 0 ? (
                <span>
                  GPU / {Math.ceil(terrainRenderer.gpu_bytes / 1024)} KIB /{" "}
                  LIMIT {Math.ceil(terrainRenderer.gpu_budget_bytes / 1024)} KIB
                </span>
              ) : null}
            </>
          ) : null}
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
      <footer
        className={sx(
          styles.canvasFooter,
          scene.globe?.posture === "orientation" &&
            styles.orientationCanvasFooter,
        )}
      >
        <span>
          WHEEL / + − TO CHANGE LENS ·{" "}
          {scene.globe ? "DRAG TO ORBIT" : "DRAG TO PAN"} · SELECT TO TRAVERSE
        </span>
        <span>
          {scene.omissions.length > 0
            ? `BOUNDED / ${scene.omissions.join(" · ")}`
            : "BOUNDED / NO PROJECTION OMISSIONS"}
        </span>
        {coordinate ? (
          <code className={sx(styles.coordinateUri)}>
            {explorerViewPath(coordinate.view)}
          </code>
        ) : null}
      </footer>
    </section>
  );
}

function CanvasToolbar({
  isFullscreen,
  layers,
  onFit,
  onFullscreen,
  onZoomIn,
  onZoomOut,
  onToggleLayer,
  scene,
  zoom,
}: {
  isFullscreen: boolean;
  layers: ReferenceLayerVisibility;
  onFit: () => void;
  onFullscreen: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onToggleLayer: (layer: keyof ReferenceLayerVisibility) => void;
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
    >
      <div className={sx(styles.lensReadout)}>
        <span className={sx(styles.micro)}>
          LENS / {lensLabel(scene.regime)}
        </span>
        <strong>{scene.label}</strong>
        <small>{scene.detail}</small>
      </div>
      <div className={sx(styles.canvasControls)}>
        {(["relief", "water", "weather", "probes"] as const).map((layer) => (
          <KineticButton
            aria-pressed={layers[layer]}
            className={sx(
              styles.layerButton,
              layers[layer] && styles.layerButtonActive,
            )}
            key={layer}
            onClick={() => onToggleLayer(layer)}
            theme="precision"
          >
            {layer.toUpperCase()}
          </KineticButton>
        ))}
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
