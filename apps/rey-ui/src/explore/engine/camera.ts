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

export const MIN_LENS_ZOOM = 0.05;
export const MAX_LENS_ZOOM = 5.4;
export const WORLD_LENS_ZOOM = 0.1;
export const DEFAULT_LENS_ZOOM = 0.26;
export const LANDSCAPE_LENS_ZOOM = 0.58;
export const NEIGHBORHOOD_LENS_ZOOM = 1.08;
export const OBJECT_LENS_ZOOM = 2.05;
export const EVIDENCE_LENS_ZOOM = 3.55;

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

export function panForZoomAtPoint(
  pan: CameraPoint,
  pointerFromViewportCenter: CameraPoint,
  currentZoom: number,
  nextZoom: number,
): CameraPoint {
  const scaleRatio = clampLensZoom(nextZoom) / currentZoom;
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
