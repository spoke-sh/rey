export type LensRegime =
  "world" | "atlas" | "landscape" | "neighborhoods" | "objects" | "evidence";

export interface CameraPoint {
  x: number;
  y: number;
}

export interface WorldExtent {
  width: number;
  height: number;
}

export interface GlobeCameraView {
  yaw_degrees: number;
  pitch_degrees: number;
}

export const DEFAULT_GLOBE_VIEW: GlobeCameraView = Object.freeze({
  yaw_degrees: 0,
  pitch_degrees: 0,
});

export function draggedGlobeView(
  origin: GlobeCameraView,
  delta: CameraPoint,
): GlobeCameraView {
  return {
    yaw_degrees: origin.yaw_degrees + delta.x * 0.22,
    pitch_degrees: Math.min(
      62,
      Math.max(-62, origin.pitch_degrees - delta.y * 0.18),
    ),
  };
}

export const MIN_LENS_ZOOM = 0.05;
export const MAX_LENS_ZOOM = 5.4;
export const WORLD_LENS_ZOOM = 0.1;
export const DEFAULT_LENS_ZOOM = 0.26;
export const LANDSCAPE_LENS_ZOOM = 0.58;
export const NEIGHBORHOOD_LENS_ZOOM = 1.08;
export const OBJECT_LENS_ZOOM = 2.05;
export const EVIDENCE_LENS_ZOOM = 3.55;
export const WORLD_ATLAS_MORPH_START_ZOOM = 0.14;
export const WORLD_ATLAS_MORPH_END_ZOOM = 0.24;
export const WORLD_GLOBE_RADIUS_RATIO = 0.41;
export const WORLD_GLOBE_ATMOSPHERE_SCALE = 1.09;

const LENS_HYSTERESIS = 0.05;
const LENS_ORDER: readonly LensRegime[] = [
  "world",
  "atlas",
  "landscape",
  "neighborhoods",
  "objects",
  "evidence",
];
const LENS_BOUNDARIES = [0.19, 0.42, 0.82, 1.52, 2.8] as const;

export function lensRegimeForZoom(
  zoom: number,
  previous?: LensRegime,
): LensRegime {
  const rawIndex = LENS_BOUNDARIES.findIndex((boundary) => zoom < boundary);
  const nextIndex = rawIndex === -1 ? LENS_ORDER.length - 1 : rawIndex;
  if (!previous) return LENS_ORDER[nextIndex]!;
  const previousIndex = LENS_ORDER.indexOf(previous);
  if (previousIndex < 0 || previousIndex === nextIndex)
    return LENS_ORDER[nextIndex]!;
  if (nextIndex > previousIndex) {
    const boundary = LENS_BOUNDARIES[previousIndex];
    if (boundary !== undefined && zoom < boundary + LENS_HYSTERESIS)
      return previous;
  } else {
    const boundary = LENS_BOUNDARIES[nextIndex];
    if (boundary !== undefined && zoom > boundary - LENS_HYSTERESIS)
      return previous;
  }
  return LENS_ORDER[nextIndex]!;
}

export function clampLensZoom(zoom: number): number {
  return Math.min(MAX_LENS_ZOOM, Math.max(MIN_LENS_ZOOM, zoom));
}

export function worldAtlasMorphProgress(zoom: number): number {
  return Math.max(
    0,
    Math.min(
      1,
      (zoom - WORLD_ATLAS_MORPH_START_ZOOM) /
        (WORLD_ATLAS_MORPH_END_ZOOM - WORLD_ATLAS_MORPH_START_ZOOM),
    ),
  );
}

export function recenterWrappedChartPan(
  pan: CameraPoint,
  renderedChartWidth: number,
): CameraPoint {
  if (!Number.isFinite(renderedChartWidth) || renderedChartWidth <= 0)
    throw new Error("wrapped chart recentering requires a positive width");
  return {
    x:
      ((((pan.x + renderedChartWidth / 2) % renderedChartWidth) +
        renderedChartWidth) %
        renderedChartWidth) -
      renderedChartWidth / 2,
    y: pan.y,
  };
}

export function stepLensZoom(zoom: number, direction: 1 | -1): number {
  const regime = lensRegimeForZoom(zoom);
  const stops = [
    WORLD_LENS_ZOOM,
    DEFAULT_LENS_ZOOM,
    LANDSCAPE_LENS_ZOOM,
    NEIGHBORHOOD_LENS_ZOOM,
    OBJECT_LENS_ZOOM,
    EVIDENCE_LENS_ZOOM,
  ] as const;
  const index = LENS_ORDER.indexOf(regime);
  if (direction > 0) {
    return index >= stops.length - 1 ? MAX_LENS_ZOOM : stops[index + 1]!;
  }
  return index <= 0 ? MIN_LENS_ZOOM : stops[index - 1]!;
}

export function fitScaleForViewport(
  viewport: WorldExtent,
  world: WorldExtent,
  padding = 36,
): number {
  return Math.min(
    Math.max(0.2, (viewport.width - padding) / world.width),
    Math.max(0.2, (viewport.height - padding) / world.height),
    1,
  );
}

export function panForScaleAtPoint(
  pan: CameraPoint,
  pointerFromViewportCenter: CameraPoint,
  currentScale: number,
  nextScale: number,
): CameraPoint {
  if (
    !Number.isFinite(currentScale) ||
    currentScale <= 0 ||
    !Number.isFinite(nextScale) ||
    nextScale <= 0
  )
    throw new Error("zoom anchoring requires positive rendered scales");
  const scaleRatio = nextScale / currentScale;
  return {
    x:
      pointerFromViewportCenter.x -
      (pointerFromViewportCenter.x - pan.x) * scaleRatio,
    y:
      pointerFromViewportCenter.y -
      (pointerFromViewportCenter.y - pan.y) * scaleRatio,
  };
}

export function panForFocusedPoint(
  point: CameraPoint,
  world: WorldExtent,
  renderedScale: number,
): CameraPoint {
  return {
    x: -(point.x - world.width / 2) * renderedScale,
    y: -(point.y - world.height / 2) * renderedScale,
  };
}

export function pointerWithinRenderedGlobeAtmosphere(
  pointer: CameraPoint,
  viewport: WorldExtent,
  world: WorldExtent,
  renderedScale: number,
  pan: CameraPoint,
): boolean {
  if (
    !Number.isFinite(pointer.x) ||
    !Number.isFinite(pointer.y) ||
    !Number.isFinite(viewport.width) ||
    !Number.isFinite(viewport.height) ||
    !Number.isFinite(renderedScale) ||
    renderedScale <= 0
  )
    return false;
  const radius =
    Math.min(world.width, world.height) *
    WORLD_GLOBE_RADIUS_RATIO *
    WORLD_GLOBE_ATMOSPHERE_SCALE *
    renderedScale;
  const center = {
    x: viewport.width / 2 + pan.x,
    y: viewport.height / 2 + pan.y,
  };
  return Math.hypot(pointer.x - center.x, pointer.y - center.y) <= radius;
}

export function renderedSceneScale(
  terrain: boolean,
  fitScale: number,
  zoom: number,
  regime: LensRegime,
): number {
  if (terrain)
    return (
      fitScale *
      (zoom / (regime === "world" ? WORLD_LENS_ZOOM : DEFAULT_LENS_ZOOM))
    );
  const regimeBase = {
    world: WORLD_LENS_ZOOM,
    atlas: DEFAULT_LENS_ZOOM,
    landscape: LANDSCAPE_LENS_ZOOM,
    neighborhoods: NEIGHBORHOOD_LENS_ZOOM,
    objects: OBJECT_LENS_ZOOM,
    evidence: EVIDENCE_LENS_ZOOM,
  }[regime];
  return fitScale * Math.min(1.16, Math.max(0.84, zoom / regimeBase));
}
