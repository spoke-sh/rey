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

export interface TerrainOrbitView {
  yaw_degrees: number;
  pitch_degrees: number;
}

export const DEFAULT_GLOBE_VIEW: GlobeCameraView = Object.freeze({
  yaw_degrees: 0,
  pitch_degrees: 0,
});
export const DEFAULT_TERRAIN_ORBIT: TerrainOrbitView = Object.freeze({
  yaw_degrees: 0,
  pitch_degrees: 82,
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

export function draggedTerrainOrbit(
  origin: TerrainOrbitView,
  delta: CameraPoint,
): TerrainOrbitView {
  if (
    !Number.isFinite(origin.yaw_degrees) ||
    !Number.isFinite(origin.pitch_degrees) ||
    !Number.isFinite(delta.x) ||
    !Number.isFinite(delta.y)
  )
    throw new Error("terrain orbit requires finite camera values");
  let yaw = origin.yaw_degrees + delta.x * 0.18;
  while (yaw > 180) yaw -= 360;
  while (yaw < -180) yaw += 360;
  return Object.freeze({
    yaw_degrees: yaw,
    pitch_degrees: Math.min(
      88,
      Math.max(28, origin.pitch_degrees - delta.y * 0.14),
    ),
  });
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
export const WORLD_GLOBE_RADIUS_RATIO = 0.4;
export const WORLD_GLOBE_ATMOSPHERE_SCALE = 1.09;

const WHEEL_LINE_PIXELS = 16;
const WHEEL_PAGE_PIXELS = 240;
const WHEEL_MIN_ZOOM_SENSITIVITY = 0.00045;
const WHEEL_RELATIVE_ZOOM_SENSITIVITY = 0.0009;
const WHEEL_MIN_MAX_STEP = 0.045;
const WHEEL_MAX_RELATIVE_STEP = 0.09;
const WHEEL_ZOOM_TIME_CONSTANT_MS = 50;
const WHEEL_ZOOM_MAX_PRESENTATION_STEP_MS = 20;
const WHEEL_ZOOM_SETTLE_EPSILON = 0.00005;

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

export function wheelZoomDelta(
  zoom: number,
  deltaY: number,
  deltaMode = 0,
): number {
  if (
    !Number.isFinite(zoom) ||
    !Number.isFinite(deltaY) ||
    !Number.isFinite(deltaMode) ||
    deltaY === 0
  )
    return 0;
  const modeScale =
    deltaMode === 1
      ? WHEEL_LINE_PIXELS
      : deltaMode === 2
        ? WHEEL_PAGE_PIXELS
        : 1;
  const boundedZoom = clampLensZoom(zoom);
  const sensitivity = Math.max(
    WHEEL_MIN_ZOOM_SENSITIVITY,
    boundedZoom * WHEEL_RELATIVE_ZOOM_SENSITIVITY,
  );
  const maximumStep = Math.max(
    WHEEL_MIN_MAX_STEP,
    boundedZoom * WHEEL_MAX_RELATIVE_STEP,
  );
  return Math.max(
    -maximumStep,
    Math.min(maximumStep, -deltaY * modeScale * sensitivity),
  );
}

export function smoothZoomStep(
  currentZoom: number,
  targetZoom: number,
  elapsedMs: number,
): number {
  if (
    !Number.isFinite(currentZoom) ||
    !Number.isFinite(targetZoom) ||
    !Number.isFinite(elapsedMs)
  )
    throw new Error("smooth zoom requires finite camera values");
  const current = clampLensZoom(currentZoom);
  const target = clampLensZoom(targetZoom);
  if (elapsedMs <= 0 || current === target) return current;
  const interpolation =
    1 -
    Math.exp(
      -Math.min(WHEEL_ZOOM_MAX_PRESENTATION_STEP_MS, elapsedMs) /
        WHEEL_ZOOM_TIME_CONSTANT_MS,
    );
  const next = current + (target - current) * interpolation;
  return Math.abs(target - next) <= WHEEL_ZOOM_SETTLE_EPSILON ? target : next;
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

export function panForTerrainTarget(
  point: CameraPoint,
  world: WorldExtent,
  renderedScale: number,
  orbit: TerrainOrbitView,
): CameraPoint {
  if (
    !Number.isFinite(renderedScale) ||
    renderedScale <= 0 ||
    !Number.isFinite(orbit.pitch_degrees) ||
    !Number.isFinite(orbit.yaw_degrees)
  )
    throw new Error("terrain focus requires finite bounded camera values");
  const yaw = (orbit.yaw_degrees * Math.PI) / 180;
  const pitch = (orbit.pitch_degrees * Math.PI) / 180;
  const deltaX = point.x - world.width / 2;
  const deltaY = point.y - world.height / 2;
  return {
    x: (-Math.cos(yaw) * deltaX + Math.sin(yaw) * deltaY) * renderedScale,
    y:
      -Math.max(0.2, Math.sin(pitch)) *
      (Math.sin(yaw) * deltaX + Math.cos(yaw) * deltaY) *
      renderedScale,
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
  if (
    !terrain &&
    (regime === "world" || regime === "atlas") &&
    zoom <= WORLD_ATLAS_MORPH_END_ZOOM
  ) {
    const boundedWorldZoom = Math.min(zoom, WORLD_ATLAS_MORPH_START_ZOOM);
    const magnified =
      boundedWorldZoom <= WORLD_LENS_ZOOM
        ? 1 -
          ((WORLD_LENS_ZOOM - boundedWorldZoom) /
            (WORLD_LENS_ZOOM - MIN_LENS_ZOOM)) *
            0.16
        : 1 +
          ((boundedWorldZoom - WORLD_LENS_ZOOM) /
            (WORLD_ATLAS_MORPH_START_ZOOM - WORLD_LENS_ZOOM)) *
            0.16;
    const progress = worldAtlasMorphProgress(zoom);
    return fitScale * (magnified + (1 - magnified) * progress);
  }
  if (regime === "world") return fitScale * (zoom / WORLD_LENS_ZOOM);
  return fitScale * (zoom / WORLD_ATLAS_MORPH_END_ZOOM);
}
