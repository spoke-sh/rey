import { KineticButton } from "@hifi/kinetic";
import { Link } from "@tanstack/react-router";
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
import { exploreStyles as styles } from "./stylex/explore.stylex";
import { className as sx } from "./stylex/shared.stylex";
import {
  DEFAULT_LENS_ZOOM,
  EVIDENCE_LENS_ZOOM,
  LANDSCAPE_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  buildTopologyScene,
  clampLensZoom,
  lensRegimeForZoom,
  stepLensZoom,
  type LensRegime,
  type TopologyEdge,
  type TopologyNode,
  type TopologyPointOfInterest,
  type TopologyScene,
  type TopologyTone,
} from "./topology";

interface ContextCanvasProps {
  portfolio: WorkloadList;
  coordinate?: ExplorerViewResolution;
}

interface Point {
  x: number;
  y: number;
}

type FocusableTopologyObject = Pick<
  TopologyNode | TopologyPointOfInterest,
  "focus_id" | "x" | "y"
>;

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
    | { pointerId: number; origin: Point; pan: Point; distance: number }
    | undefined
  >(undefined);
  const [zoom, setZoom] = useState(
    coordinate ? coordinate.view.scale : DEFAULT_LENS_ZOOM,
  );
  const [pan, setPan] = useState<Point>(zeroPoint);
  const [fitScale, setFitScale] = useState(1);
  const [focusId, setFocusId] = useState(
    coordinate?.focus_id ?? "cluster:portfolio",
  );
  const [isDragging, setIsDragging] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const retainedRegime = useRef<LensRegime | undefined>(undefined);
  const regime = lensRegimeForZoom(zoom, retainedRegime.current);
  useEffect(() => {
    retainedRegime.current = regime;
  }, [regime]);
  const scene = useMemo(
    () => buildTopologyScene(portfolio, zoom, focusId, regime),
    [focusId, portfolio, regime, zoom],
  );
  const regimeBase = {
    atlas: DEFAULT_LENS_ZOOM,
    landscape: LANDSCAPE_LENS_ZOOM,
    neighborhoods: NEIGHBORHOOD_LENS_ZOOM,
    objects: OBJECT_LENS_ZOOM,
    evidence: EVIDENCE_LENS_ZOOM,
  }[scene.regime];
  const renderedScale = scene.terrain
    ? fitScale * (zoom / DEFAULT_LENS_ZOOM)
    : fitScale * Math.min(1.16, Math.max(0.84, zoom / regimeBase));

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
      const next = Math.min(
        Math.max(0.2, (width - 36) / scene.fit_world.width),
        Math.max(0.2, (height - 36) / scene.fit_world.height),
        1,
      );
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
      const scaleRatio = boundedZoom / zoom;
      setPan({
        x: pointer.x - (pointer.x - pan.x) * scaleRatio,
        y: pointer.y - (pointer.y - pan.y) * scaleRatio,
      });
    }
    setZoom(boundedZoom);
  };

  const focusNode = (node: FocusableTopologyObject) => {
    if (dragRef.current?.distance && dragRef.current.distance > 4) return;
    let nextZoom = zoom;
    if (scene.regime === "atlas") nextZoom = LANDSCAPE_LENS_ZOOM;
    else if (scene.regime === "landscape") nextZoom = NEIGHBORHOOD_LENS_ZOOM;
    else if (scene.regime === "neighborhoods") nextZoom = OBJECT_LENS_ZOOM;
    else if (scene.regime === "objects") nextZoom = EVIDENCE_LENS_ZOOM;
    setFocusId(node.focus_id);
    if (scene.terrain) {
      const nextScale = fitScale * (nextZoom / DEFAULT_LENS_ZOOM);
      setPan({
        x: -(node.x - scene.world.width / 2) * nextScale,
        y: -(node.y - scene.world.height / 2) * nextScale,
      });
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
    setPan({ x: drag.pan.x + delta.x, y: drag.pan.y + delta.y });
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
    setZoom(DEFAULT_LENS_ZOOM);
    setPan(zeroPoint);
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
    "--rey-terrain-counter-scale": scene.terrain ? DEFAULT_LENS_ZOOM / zoom : 1,
    height: scene.world.height,
    transform: `translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${renderedScale})`,
    width: scene.world.width,
  } as CSSProperties;

  return (
    <section
      className={sx(
        styles.canvasShell,
        isFullscreen && styles.canvasShellFullscreen,
      )}
      ref={shellRef}
    >
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
        aria-label="Interactive context topology map. Drag to pan, use the mouse wheel or plus and minus keys to move through semantic lens levels."
        className={sx(
          styles.canvasViewport,
          scene.terrain && styles.terrainViewport,
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
        <div
          className={sx(styles.scene, isDragging && styles.sceneDragging)}
          style={sceneStyle}
        >
          <SemanticLens onFocus={focusNode} scene={scene} />
        </div>
        <div className={sx(styles.canvasCoordinates)} aria-hidden="true">
          <span>ZOOM {Math.round(zoom * 100)}%</span>
          <span>
            X {Math.round(pan.x)} / Y {Math.round(pan.y)}
          </span>
        </div>
        <div className={sx(styles.lensLegend)}>
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
      </div>
      <footer className={sx(styles.canvasFooter)}>
        <span>
          WHEEL / + − TO CHANGE LENS · DRAG TO PAN · SELECT TO TRAVERSE
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
    <header className={sx(styles.canvasToolbar)}>
      <div className={sx(styles.lensReadout)}>
        <span className={sx(styles.micro)}>
          LENS / {lensLabel(scene.regime)}
        </span>
        <strong>{scene.label}</strong>
        <small>{scene.detail}</small>
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

function SemanticLens({
  onFocus,
  scene,
}: {
  onFocus: (node: FocusableTopologyObject) => void;
  scene: TopologyScene;
}) {
  return (
    <div
      className={sx(
        styles.projection,
        scene.terrain && styles.terrainProjection,
      )}
      data-lens-regime={scene.regime}
    >
      {scene.regions.map((region) => (
        <div
          className={sx(
            styles.region,
            region.variant === "map-boundary" && styles.mapBoundary,
            region.variant === "map-zone" && styles.mapZone,
            toneStyle(region.tone, "region"),
          )}
          key={region.id}
          style={{
            height: region.height,
            left: region.x,
            top: region.y,
            width: region.width,
          }}
        >
          <span>{region.label}</span>
          <small>{region.detail}</small>
        </div>
      ))}
      <ReliefLayer scene={scene} />
      <EdgeLayer
        edges={scene.edges}
        nodes={scene.nodes}
        points={scene.points}
        terrain={scene.terrain}
        world={scene.world}
      />
      {scene.points.map((point) => (
        <PointOfInterest
          key={point.id}
          onFocus={onFocus}
          point={point}
          regime={scene.regime}
        />
      ))}
      {scene.nodes.map((node) => (
        <TopologyObject
          counterScale={scene.terrain}
          key={node.id}
          linkToWorkload={scene.regime === "objects" && !scene.terrain}
          node={node}
          onFocus={onFocus}
        />
      ))}
    </div>
  );
}

function ReliefLayer({ scene }: { scene: TopologyScene }) {
  if (scene.contours.length === 0) return null;
  return (
    <svg
      aria-label={`${scene.contours.length} anchor-derived terrain contour levels`}
      className={sx(styles.reliefLayer)}
      role="img"
      viewBox={`0 0 ${scene.world.width} ${scene.world.height}`}
    >
      <title>
        Relief height is derived from admitted anchor and classified-edge
        influence. It does not assert semantic similarity.
      </title>
      {scene.contours.map((contour) => (
        <path
          className={sx(
            styles.reliefContour,
            contour.level >= 6 && styles.reliefContourPeak,
            contour.level <= 2 && styles.reliefContourLow,
          )}
          d={contour.path}
          data-anchor-count={contour.anchor_count}
          data-relief-level={contour.level}
          key={contour.id}
          style={{
            strokeDasharray:
              contour.level <= 2
                ? "calc(3px * var(--rey-terrain-counter-scale)) calc(3px * var(--rey-terrain-counter-scale))"
                : undefined,
            strokeWidth: `calc(${contour.level >= 6 ? 1.55 : 1.15}px * var(--rey-terrain-counter-scale))`,
          }}
        />
      ))}
    </svg>
  );
}

function EdgeLayer({
  edges,
  nodes,
  points,
  terrain,
  world,
}: {
  edges: TopologyEdge[];
  nodes: TopologyNode[];
  points: TopologyPointOfInterest[];
  terrain: boolean;
  world: TopologyScene["world"];
}) {
  const byId = new Map(
    [...nodes, ...points].map((candidate) => [candidate.id, candidate]),
  );
  return (
    <svg
      aria-hidden="true"
      className={sx(styles.edgeLayer)}
      viewBox={`0 0 ${world.width} ${world.height}`}
    >
      <defs>
        <marker
          id="topology-arrow"
          markerHeight="7"
          markerWidth="7"
          orient="auto-start-reverse"
          refX="6"
          refY="3.5"
        >
          <path d="M0,0 L7,3.5 L0,7 Z" />
        </marker>
      </defs>
      {edges.map((edge) => {
        const from = byId.get(edge.from);
        const to = byId.get(edge.to);
        if (!from || !to) return null;
        const midpoint = {
          x: (from.x + to.x) / 2,
          y: (from.y + to.y) / 2,
        };
        return (
          <g className={sx(styles.edgeGroup)} key={edge.id}>
            <line
              className={sx(
                styles.edge,
                edge.kind === "directs" && styles.edgeDirects,
                edge.kind === "contains" && styles.edgeContains,
              )}
              markerEnd={terrain ? undefined : "url(#topology-arrow)"}
              style={
                terrain
                  ? {
                      strokeDasharray:
                        edge.kind === "contains"
                          ? "calc(6px * var(--rey-terrain-counter-scale)) calc(5px * var(--rey-terrain-counter-scale))"
                          : undefined,
                      strokeWidth: `calc(${edge.kind === "directs" ? 2.4 : 1.5}px * var(--rey-terrain-counter-scale))`,
                    }
                  : undefined
              }
              x1={from.x}
              x2={to.x}
              y1={from.y}
              y2={to.y}
            />
            <text
              className={sx(styles.edgeLabel)}
              style={
                terrain
                  ? {
                      fontSize: "calc(9px * var(--rey-terrain-counter-scale))",
                      strokeWidth:
                        "calc(5px * var(--rey-terrain-counter-scale))",
                    }
                  : undefined
              }
              textAnchor="middle"
              x={midpoint.x}
              y={midpoint.y - 7}
            >
              {edge.label.toUpperCase()}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

function PointOfInterest({
  onFocus,
  point,
  regime,
}: {
  onFocus: (point: TopologyPointOfInterest) => void;
  point: TopologyPointOfInterest;
  regime: LensRegime;
}) {
  const showLabel =
    regime === "atlas"
      ? point.prominence >= 3
      : regime === "landscape"
        ? point.prominence >= 2
        : true;
  const showDetail =
    regime === "neighborhoods" || regime === "objects" || regime === "evidence";
  return (
    <button
      aria-label={`${point.family} point of interest: ${point.label}`}
      className={sx(
        styles.pointOfInterest,
        point.kind === "frontier" && styles.frontierPoint,
        point.prominence >= 3 && styles.prominentPoint,
      )}
      data-prominence={point.prominence}
      onClick={() => onFocus(point)}
      style={{ left: point.x, top: point.y }}
      type="button"
    >
      <i
        className={sx(
          styles.pointMarker,
          point.kind === "frontier" && styles.frontierMarker,
        )}
        aria-hidden="true"
      />
      {showLabel ? (
        <span className={sx(styles.pointLabel)}>
          <small className={sx(styles.pointFamily)}>{point.family}</small>
          <strong className={sx(styles.pointName)}>{point.label}</strong>
          {showDetail ? (
            <em className={sx(styles.pointDetail)}>{point.detail}</em>
          ) : null}
        </span>
      ) : null}
    </button>
  );
}

function TopologyObject({
  counterScale,
  linkToWorkload,
  node,
  onFocus,
}: {
  counterScale: boolean;
  linkToWorkload: boolean;
  node: TopologyNode;
  onFocus: (node: FocusableTopologyObject) => void;
}) {
  const className = sx(styles.topologyObject, toneStyle(node.tone, "node"));
  const style = {
    left: node.x,
    top: node.y,
    ...(counterScale
      ? {
          transform:
            "translate(-50%, -50%) scale(var(--rey-terrain-counter-scale))",
        }
      : {}),
    width: node.width,
  } as CSSProperties;
  const content = (
    <>
      <span className={sx(styles.objectFamily)}>{node.family}</span>
      <strong className={sx(styles.objectLabel)}>{node.label}</strong>
      <small className={sx(styles.objectDetail)}>{node.detail}</small>
      <span className={sx(styles.objectAction)}>
        {node.coordinate_uri
          ? "OPEN COORDINATE ↗"
          : linkToWorkload && node.workload_id
            ? "OPEN RECORD ↗"
            : "FOCUS / ZOOM →"}
      </span>
    </>
  );

  if (node.coordinate_uri) {
    return (
      <a
        className={className}
        href={`/explore?coordinate=${encodeURIComponent(node.coordinate_uri)}&scale=${OBJECT_LENS_ZOOM}`}
        style={style}
      >
        {content}
      </a>
    );
  }

  if (linkToWorkload && node.workload_id) {
    return (
      <Link
        className={className}
        params={{ workloadId: node.workload_id }}
        style={style}
        to="/workloads/$workloadId"
      >
        {content}
      </Link>
    );
  }

  return (
    <button
      className={className}
      onClick={() => onFocus(node)}
      style={style}
      type="button"
    >
      {content}
    </button>
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
  if (regime === "atlas") return "ORBITAL";
  if (regime === "landscape") return "TELESCOPE";
  if (regime === "neighborhoods") return "MESOSCOPIC";
  if (regime === "objects") return "MICROSCOPE";
  return "EVIDENTIARY";
}

function toneStyle(tone: TopologyTone, kind: "node" | "region") {
  if (kind === "region") {
    if (tone === "accent") return styles.regionAccent;
    if (tone === "attention" || tone === "blocked")
      return styles.regionAttention;
    if (tone === "healthy") return styles.regionHealthy;
    if (tone === "unknown") return styles.regionUnknown;
    if (tone === "omitted") return styles.regionOmitted;
    if (tone === "stale") return styles.regionStale;
    if (tone === "unsupported") return styles.regionUnsupported;
    if (tone === "frontier") return styles.regionFrontier;
    return styles.regionNeutral;
  }
  if (tone === "accent") return styles.objectAccent;
  if (tone === "attention") return styles.objectAttention;
  if (tone === "blocked") return styles.objectBlocked;
  if (tone === "healthy") return styles.objectHealthy;
  if (tone === "unknown") return styles.objectUnknown;
  if (tone === "omitted") return styles.objectOmitted;
  if (tone === "stale") return styles.objectStale;
  if (tone === "unsupported") return styles.objectUnsupported;
  if (tone === "frontier") return styles.objectFrontier;
  return styles.objectNeutral;
}
