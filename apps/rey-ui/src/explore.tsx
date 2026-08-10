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
  explorerCoordinatePath,
  zoomForExplorerLens,
  type ExplorerCoordinateResolution,
} from "./explorer-coordinate";
import { exploreStyles as styles } from "./stylex/explore.stylex";
import { className as sx } from "./stylex/shared.stylex";
import {
  DEFAULT_LENS_ZOOM,
  MAX_LENS_ZOOM,
  MIN_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  TOPOLOGY_WORLD,
  buildTopologyScene,
  clampLensZoom,
  stepLensZoom,
  type LensRegime,
  type TopologyEdge,
  type TopologyNode,
  type TopologyScene,
  type TopologyTone,
} from "./topology";

interface ContextCanvasProps {
  portfolio: WorkloadList;
  coordinate?: ExplorerCoordinateResolution;
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
          <code>{explorerCoordinatePath(coordinate.coordinate)}</code>
          <span>
            {coordinate.actual_at
              ? `CURRENT BINDING / ${coordinate.actual_at}`
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
    coordinate
      ? zoomForExplorerLens(coordinate.coordinate.lens)
      : DEFAULT_LENS_ZOOM,
  );
  const [pan, setPan] = useState<Point>(zeroPoint);
  const [fitScale, setFitScale] = useState(1);
  const [focusId, setFocusId] = useState(
    coordinate?.focus_id ?? "cluster:portfolio",
  );
  const [isDragging, setIsDragging] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const scene = useMemo(
    () => buildTopologyScene(portfolio, zoom, focusId),
    [focusId, portfolio, zoom],
  );
  const renderedScale = fitScale * (zoom / DEFAULT_LENS_ZOOM);

  useEffect(() => {
    if (!coordinate) return;
    setFocusId(coordinate.focus_id);
    setZoom(zoomForExplorerLens(coordinate.coordinate.lens));
    setPan(zeroPoint);
  }, [coordinate?.coordinate.lens, coordinate?.focus_id, coordinate?.status]);

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) return;
    const measure = () => {
      const { width, height } = viewport.getBoundingClientRect();
      const next = Math.min(
        Math.max(0.2, (width - 36) / TOPOLOGY_WORLD.width),
        Math.max(0.2, (height - 36) / TOPOLOGY_WORLD.height),
        1,
      );
      setFitScale(next);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);

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

  const focusNode = (node: TopologyNode) => {
    if (dragRef.current?.distance && dragRef.current.distance > 4) return;
    setFocusId(node.focus_id);
    setPan(zeroPoint);
    if (scene.regime === "landscape") setZoom(NEIGHBORHOOD_LENS_ZOOM);
    else if (scene.regime === "neighborhoods") setZoom(OBJECT_LENS_ZOOM);
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
    height: TOPOLOGY_WORLD.height,
    transform: `translate(-50%, -50%) translate(${pan.x}px, ${pan.y}px) scale(${renderedScale})`,
    width: TOPOLOGY_WORLD.width,
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
          <LensStep active={scene.regime === "landscape"} label="LANDSCAPE" />
          <i className={sx(styles.legendLine)} />
          <LensStep
            active={scene.regime === "neighborhoods"}
            label="NEIGHBORHOODS"
          />
          <i className={sx(styles.legendLine)} />
          <LensStep active={scene.regime === "objects"} label="OBJECTS" />
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
            {explorerCoordinatePath(coordinate.coordinate)}
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
  onFocus: (node: TopologyNode) => void;
  scene: TopologyScene;
}) {
  return (
    <div className={sx(styles.projection)} data-lens-regime={scene.regime}>
      {scene.regions.map((region) => (
        <div
          className={sx(styles.region, toneStyle(region.tone, "region"))}
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
      <EdgeLayer edges={scene.edges} nodes={scene.nodes} />
      {scene.nodes.map((node) => (
        <TopologyObject
          key={node.id}
          linkToWorkload={scene.regime === "objects"}
          node={node}
          onFocus={onFocus}
        />
      ))}
    </div>
  );
}

function EdgeLayer({
  edges,
  nodes,
}: {
  edges: TopologyEdge[];
  nodes: TopologyNode[];
}) {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  return (
    <svg
      aria-hidden="true"
      className={sx(styles.edgeLayer)}
      viewBox={`0 0 ${TOPOLOGY_WORLD.width} ${TOPOLOGY_WORLD.height}`}
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
              markerEnd="url(#topology-arrow)"
              x1={from.x}
              x2={to.x}
              y1={from.y}
              y2={to.y}
            />
            <text
              className={sx(styles.edgeLabel)}
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

function TopologyObject({
  linkToWorkload,
  node,
  onFocus,
}: {
  linkToWorkload: boolean;
  node: TopologyNode;
  onFocus: (node: TopologyNode) => void;
}) {
  const className = sx(styles.topologyObject, toneStyle(node.tone, "node"));
  const style = {
    left: node.x,
    top: node.y,
    width: node.width,
  } as CSSProperties;
  const content = (
    <>
      <span className={sx(styles.objectFamily)}>{node.family}</span>
      <strong className={sx(styles.objectLabel)}>{node.label}</strong>
      <small className={sx(styles.objectDetail)}>{node.detail}</small>
      <span className={sx(styles.objectAction)}>
        {linkToWorkload && node.workload_id
          ? "OPEN RECORD ↗"
          : "FOCUS / ZOOM →"}
      </span>
    </>
  );

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
  if (regime === "landscape") return "TELESCOPE";
  if (regime === "neighborhoods") return "MESOSCOPIC";
  return "MICROSCOPE";
}

function toneStyle(tone: TopologyTone, kind: "node" | "region") {
  if (kind === "region") {
    if (tone === "accent") return styles.regionAccent;
    if (tone === "attention" || tone === "blocked")
      return styles.regionAttention;
    if (tone === "healthy") return styles.regionHealthy;
    return styles.regionNeutral;
  }
  if (tone === "accent") return styles.objectAccent;
  if (tone === "attention") return styles.objectAttention;
  if (tone === "blocked") return styles.objectBlocked;
  if (tone === "healthy") return styles.objectHealthy;
  return styles.objectNeutral;
}
