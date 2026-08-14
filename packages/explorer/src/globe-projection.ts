import type { GlobeCameraView } from "./types";
import { GLOBE_RADIUS } from "./three-globe";

export const SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES = 85.051129;
export const GLOBE_CAMERA_HALF_HEIGHT = 2.12;
export const GLOBE_ATLAS_HORIZONTAL_WRAP_INDEXES = Object.freeze([
  -1, 0, 1,
] as const);
export const GLOBE_ATLAS_REPEAT_DISSOLVE_START = 0.58;
export const GLOBE_ATLAS_REPEAT_MAX_DEPTH = GLOBE_RADIUS * 0.72;
export const GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT = 0.72;
export const GLOBE_SURFACE_FADE_START = 0.38;
export const GLOBE_SURFACE_FADE_END = 0.62;

export interface GlobeProjectionWorld {
  width: number;
  height: number;
}

export interface GlobeProjectionPoint {
  position: readonly [number, number, number];
  normal: readonly [number, number, number];
  sphere_position: readonly [number, number, number];
  atlas_position: readonly [number, number, number];
  progress: number;
}

export interface ProjectedGlobeMesh {
  positions: Float32Array;
  normals: Float32Array;
  indices: Uint32Array;
}

export interface GlobeProjectionBounds {
  west_degrees: number;
  south_degrees: number;
  east_degrees: number;
  north_degrees: number;
  crosses_antimeridian: boolean;
}

export interface GlobeAtlasViewCenter {
  longitude_degrees: number;
  latitude_degrees: number;
}

export function globeAtlasWidth(world: GlobeProjectionWorld) {
  verifyWorld(world);
  return GLOBE_CAMERA_HALF_HEIGHT * (world.width / world.height) * 0.985 * 2;
}

/** The current east-to-west seam period while the sphere unfurls. */
export function globeAtlasRepeatPeriod(
  world: GlobeProjectionWorld,
  progress: number,
) {
  return globeAtlasWidth(world) * (1 - globeProjectionMorphRemaining(progress));
}

/** Positions a planar side chart so its inner edge follows the live seam. */
export function globeAtlasRepeatOffset(
  world: GlobeProjectionWorld,
  progress: number,
  wrapIndex: number,
) {
  if (!Number.isInteger(wrapIndex) || Math.abs(wrapIndex) > 1)
    throw new Error("globe Atlas repeat index must be -1, 0, or 1");
  if (wrapIndex === 0) return 0;
  return (
    (wrapIndex *
      (globeAtlasWidth(world) + globeAtlasRepeatPeriod(world, progress))) /
    2
  );
}

/**
 * Weights one repeated chart from its connected seam toward its outer edge.
 * Chart position is normalized west-to-east; negative repeats connect on the
 * east and positive repeats connect on the west.
 */
export function globeAtlasRepeatSeamWeight(
  normalizedChartX: number,
  wrapIndex: number,
) {
  if (!Number.isFinite(normalizedChartX))
    throw new Error("globe Atlas repeat position must be finite");
  if (!Number.isInteger(wrapIndex))
    throw new Error("globe Atlas repeat index must be an integer");
  if (wrapIndex === 0) return 1;
  const boundedPosition = Math.max(0, Math.min(1, normalizedChartX));
  return smoothstep(wrapIndex < 0 ? boundedPosition : 1 - boundedPosition);
}

/** Recedes overlapping repeat fabric while preserving a coplanar seam. */
export function globeAtlasRepeatDepthOffset(
  progress: number,
  seamWeight: number,
) {
  if (!Number.isFinite(seamWeight))
    throw new Error("globe Atlas repeat seam weight must be finite");
  const boundedWeight = Math.max(0, Math.min(1, seamWeight));
  const morphRemaining = globeProjectionMorphRemaining(progress);
  if (
    boundedWeight >= GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT ||
    morphRemaining === 0
  )
    return 0;
  const recessionProgress = smoothstep(
    1 - boundedWeight / GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT,
  );
  return -GLOBE_ATLAS_REPEAT_MAX_DEPTH * recessionProgress * morphRemaining;
}

export function globeAtlasRepeatOpacity(progress: number) {
  if (!Number.isFinite(progress))
    throw new Error("globe Atlas repeat progress must be finite");
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const dissolveProgress = Math.max(
    0,
    Math.min(
      1,
      (boundedProgress - GLOBE_ATLAS_REPEAT_DISSOLVE_START) /
        (1 - GLOBE_ATLAS_REPEAT_DISSOLVE_START),
    ),
  );
  return smoothstep(dissolveProgress);
}

/** Expands repeat visibility outward without fading its connected seam. */
export function globeAtlasRepeatVisibility(
  progress: number,
  seamWeight: number,
) {
  if (!Number.isFinite(seamWeight))
    throw new Error("globe Atlas repeat seam weight must be finite");
  const repeatOpacity = globeAtlasRepeatOpacity(progress);
  if (repeatOpacity === 0) return 0;
  const boundedWeight = Math.max(0, Math.min(1, seamWeight));
  const visibleStart = 1 - repeatOpacity;
  if (boundedWeight <= visibleStart) return 0;
  if (boundedWeight >= 1) return 1;
  return smoothstep((boundedWeight - visibleStart) / repeatOpacity);
}

export function globeProjectionMorphRemaining(progress: number) {
  if (!Number.isFinite(progress))
    throw new Error("globe projection progress must be finite");
  return 1 - smoothstep(Math.max(0, Math.min(1, progress)));
}

export function globeSurfaceOpacity(progress: number) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  const fadeProgress = Math.max(
    0,
    Math.min(
      1,
      (progress - GLOBE_SURFACE_FADE_START) /
        (GLOBE_SURFACE_FADE_END - GLOBE_SURFACE_FADE_START),
    ),
  );
  return morphRemaining * (1 - smoothstep(fadeProgress));
}

export function globeAtmosphereOpacity(progress: number) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  return morphRemaining ** 5;
}

/** The semantic coordinate facing the camera after the globe's yaw/pitch. */
export function globeAtlasViewCenter(
  view: Pick<GlobeCameraView, "yaw_degrees" | "pitch_degrees">,
): GlobeAtlasViewCenter {
  if (
    !Number.isFinite(view.yaw_degrees) ||
    !Number.isFinite(view.pitch_degrees)
  )
    throw new Error("globe view center requires finite orientation");
  const pitch = (view.pitch_degrees * Math.PI) / 180;
  const yaw = (view.yaw_degrees * Math.PI) / 180;
  const x = -Math.sin(yaw);
  const y = Math.sin(pitch) * Math.cos(yaw);
  const z = Math.cos(pitch) * Math.cos(yaw);
  return Object.freeze({
    longitude_degrees: (Math.atan2(x, z) * 180) / Math.PI,
    latitude_degrees: (Math.asin(Math.max(-1, Math.min(1, y))) * 180) / Math.PI,
  });
}

export function projectGlobeCoordinate(
  longitudeDegrees: number,
  latitudeDegrees: number,
  view: GlobeCameraView,
  world: GlobeProjectionWorld,
  progress = view.projection_morph_progress ?? 0,
  radius = GLOBE_RADIUS,
  planeDepth = 0,
): GlobeProjectionPoint {
  verifyWorld(world);
  for (const value of [
    longitudeDegrees,
    latitudeDegrees,
    view.yaw_degrees,
    view.pitch_degrees,
    radius,
    planeDepth,
  ])
    if (!Number.isFinite(value))
      throw new Error("globe projection requires finite coordinates");
  if (latitudeDegrees < -90 || latitudeDegrees > 90 || radius <= 0)
    throw new Error("globe projection coordinate is outside its bounds");

  const boundedProgress = Math.max(0, Math.min(1, progress));
  const longitude = (longitudeDegrees * Math.PI) / 180;
  const latitude = (latitudeDegrees * Math.PI) / 180;
  const local = [
    Math.sin(longitude) * Math.cos(latitude),
    Math.sin(latitude),
    Math.cos(longitude) * Math.cos(latitude),
  ] as const;
  const pitch = (view.pitch_degrees * Math.PI) / 180;
  const yaw = (view.yaw_degrees * Math.PI) / 180;
  const pitchY = local[1] * Math.cos(pitch) - local[2] * Math.sin(pitch);
  const pitchZ = local[1] * Math.sin(pitch) + local[2] * Math.cos(pitch);
  const sphereNormal = [
    local[0] * Math.cos(yaw) + pitchZ * Math.sin(yaw),
    pitchY,
    -local[0] * Math.sin(yaw) + pitchZ * Math.cos(yaw),
  ] as const;
  const spherePosition = sphereNormal.map((value) => value * radius) as [
    number,
    number,
    number,
  ];

  const latitudeCutoff = Math.max(
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES,
    Math.min(SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES, latitudeDegrees),
  );
  const latitudeRadians = (latitudeCutoff * Math.PI) / 180;
  const aspect = world.width / world.height;
  const halfHeight = GLOBE_CAMERA_HALF_HEIGHT * 0.985;
  const halfWidth = GLOBE_CAMERA_HALF_HEIGHT * aspect * 0.985;
  const atlasCenter = globeAtlasViewCenter(view);
  const relativeLongitude = wrapLongitude(
    longitudeDegrees - atlasCenter.longitude_degrees,
  );
  const centerLatitude = Math.max(
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES,
    Math.min(
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES,
      atlasCenter.latitude_degrees,
    ),
  );
  const centerLatitudeRadians = (centerLatitude * Math.PI) / 180;
  const atlasPosition = [
    (relativeLongitude / 180) * halfWidth,
    ((Math.log(Math.tan(Math.PI / 4 + latitudeRadians / 2)) -
      Math.log(Math.tan(Math.PI / 4 + centerLatitudeRadians / 2))) /
      Math.PI) *
      halfHeight,
    planeDepth,
  ] as const;
  const progressEased = 1 - globeProjectionMorphRemaining(boundedProgress);
  const position = interpolateVector(
    spherePosition,
    atlasPosition,
    progressEased,
  );
  const normal = normalizeVector(
    interpolateVector(sphereNormal, [0, 0, 1], progressEased),
  );
  return Object.freeze({
    position: Object.freeze(position),
    normal: Object.freeze(normal),
    sphere_position: Object.freeze(spherePosition),
    atlas_position: Object.freeze([...atlasPosition]) as readonly [
      number,
      number,
      number,
    ],
    progress: boundedProgress,
  });
}

/**
 * Projects a side copy as planar Mercator in local chart coordinates, bending
 * only its narrow inner connection band onto the canonical unfurling seam.
 */
export function projectGlobeAtlasRepeatCoordinate(
  longitudeDegrees: number,
  latitudeDegrees: number,
  view: GlobeCameraView,
  world: GlobeProjectionWorld,
  progress: number,
  wrapIndex: number,
  radius = GLOBE_RADIUS,
  planeDepth = 0,
): GlobeProjectionPoint {
  if (wrapIndex === 0)
    return projectGlobeCoordinate(
      longitudeDegrees,
      latitudeDegrees,
      view,
      world,
      progress,
      radius,
      planeDepth,
    );
  if (!Number.isInteger(wrapIndex) || Math.abs(wrapIndex) !== 1)
    throw new Error("globe Atlas repeat projection requires index -1 or 1");

  const planar = projectGlobeCoordinate(
    longitudeDegrees,
    latitudeDegrees,
    view,
    world,
    1,
    radius,
    planeDepth,
  );
  const atlasWidth = globeAtlasWidth(world);
  const normalizedChartX = planar.atlas_position[0] / atlasWidth + 0.5;
  const seamWeight = globeAtlasRepeatSeamWeight(normalizedChartX, wrapIndex);
  const connectionProgress =
    seamWeight <= GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT
      ? 0
      : smoothstep(
          (seamWeight - GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT) /
            (1 - GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT),
        );
  const atlasCenter = globeAtlasViewCenter(view);
  const connectedSeam = projectGlobeCoordinate(
    atlasCenter.longitude_degrees + wrapIndex * 180,
    latitudeDegrees,
    view,
    world,
    progress,
    radius,
    planeDepth,
  );
  const repeatOffset = globeAtlasRepeatOffset(world, progress, wrapIndex);
  const planarSeamX = wrapIndex < 0 ? atlasWidth / 2 : -atlasWidth / 2;
  const seamCorrection = [
    connectedSeam.position[0] - (repeatOffset + planarSeamX),
    connectedSeam.position[1] - planar.position[1],
    connectedSeam.position[2] - planar.position[2],
  ] as const;
  const position = [
    planar.position[0] + seamCorrection[0] * connectionProgress,
    planar.position[1] + seamCorrection[1] * connectionProgress,
    planar.position[2] +
      seamCorrection[2] * connectionProgress +
      globeAtlasRepeatDepthOffset(progress, seamWeight),
  ] as [number, number, number];
  const normal = normalizeVector(
    interpolateVector(planar.normal, connectedSeam.normal, connectionProgress),
  );
  return Object.freeze({
    position: Object.freeze(position),
    normal: Object.freeze(normal),
    sphere_position: planar.sphere_position,
    atlas_position: planar.atlas_position,
    progress: Math.max(0, Math.min(1, progress)),
  });
}

export function buildProjectedGlobeMesh(
  view: GlobeCameraView,
  world: GlobeProjectionWorld,
  progress = view.projection_morph_progress ?? 0,
  longitudeSegments = 160,
  latitudeSegments = 96,
  radius = GLOBE_RADIUS,
  planeDepth = 0,
): ProjectedGlobeMesh {
  if (
    !Number.isInteger(longitudeSegments) ||
    !Number.isInteger(latitudeSegments) ||
    longitudeSegments < 3 ||
    latitudeSegments < 2
  )
    throw new Error("projected globe mesh segments are invalid");
  const vertexCount = (longitudeSegments + 1) * (latitudeSegments + 1);
  const positions = new Float32Array(vertexCount * 3);
  const normals = new Float32Array(vertexCount * 3);
  const atlasCenter = globeAtlasViewCenter(view);
  const west = atlasCenter.longitude_degrees - 180;
  for (let row = 0; row <= latitudeSegments; row += 1) {
    const latitude = -90 + (row / latitudeSegments) * 180;
    for (let column = 0; column <= longitudeSegments; column += 1) {
      const longitude = west + (column / longitudeSegments) * 360;
      const point = projectGlobeCoordinate(
        longitude,
        latitude,
        view,
        world,
        progress,
        radius,
        planeDepth,
      );
      const index = (row * (longitudeSegments + 1) + column) * 3;
      positions.set(point.position, index);
      normals.set(point.normal, index);
    }
  }
  const indices = new Uint32Array(longitudeSegments * latitudeSegments * 6);
  let offset = 0;
  for (let row = 0; row < latitudeSegments; row += 1) {
    for (let column = 0; column < longitudeSegments; column += 1) {
      const northwest = row * (longitudeSegments + 1) + column;
      const northeast = northwest + 1;
      const southwest = northwest + longitudeSegments + 1;
      const southeast = southwest + 1;
      indices.set(
        [northwest, southwest, northeast, northeast, southwest, southeast],
        offset,
      );
      offset += 6;
    }
  }
  return Object.freeze({ positions, normals, indices });
}

export function buildProjectedBoundsMeshes(
  bounds: GlobeProjectionBounds,
  view: GlobeCameraView,
  world: GlobeProjectionWorld,
  progress = view.projection_morph_progress ?? 0,
  longitudeSegments = 16,
  latitudeSegments = 10,
  wrapIndex = 0,
): readonly ProjectedGlobeMesh[] {
  if (
    bounds.south_degrees < -90 ||
    bounds.north_degrees > 90 ||
    bounds.south_degrees >= bounds.north_degrees
  )
    throw new Error("projected globe bounds are invalid");
  const spans = globeAtlasLongitudeSpans(bounds, view);
  return Object.freeze(
    spans.map(([west, east]) =>
      buildProjectionPatch(
        west,
        east,
        bounds.south_degrees,
        bounds.north_degrees,
        view,
        world,
        progress,
        longitudeSegments,
        latitudeSegments,
        wrapIndex,
      ),
    ),
  );
}

function globeAtlasLongitudeSpans(
  bounds: GlobeProjectionBounds,
  view: Pick<GlobeCameraView, "yaw_degrees" | "pitch_degrees">,
): readonly (readonly [number, number])[] {
  const center = globeAtlasViewCenter(view).longitude_degrees;
  const chartWest = center - 180;
  const chartEast = center + 180;
  const sourceWest = bounds.west_degrees;
  const sourceEast =
    bounds.crosses_antimeridian || bounds.east_degrees < bounds.west_degrees
      ? bounds.east_degrees + 360
      : bounds.east_degrees;
  const firstCopy = Math.floor((chartWest - sourceEast) / 360);
  const lastCopy = Math.ceil((chartEast - sourceWest) / 360);
  const spans: [number, number][] = [];
  const copies = Array.from(
    { length: lastCopy - firstCopy + 1 },
    (_, index) => firstCopy + index,
  ).sort((left, right) => Math.abs(left) - Math.abs(right) || left - right);
  for (const copy of copies) {
    const west = Math.max(chartWest, sourceWest + copy * 360);
    const east = Math.min(chartEast, sourceEast + copy * 360);
    if (east > west) spans.push([west, east]);
  }
  return Object.freeze(spans.map((span) => Object.freeze(span)));
}

function buildProjectionPatch(
  west: number,
  east: number,
  south: number,
  north: number,
  view: GlobeCameraView,
  world: GlobeProjectionWorld,
  progress: number,
  longitudeSegments: number,
  latitudeSegments: number,
  wrapIndex: number,
) {
  const positions = new Float32Array(
    (longitudeSegments + 1) * (latitudeSegments + 1) * 3,
  );
  const normals = new Float32Array(positions.length);
  for (let row = 0; row <= latitudeSegments; row += 1) {
    const latitude = south + (row / latitudeSegments) * (north - south);
    for (let column = 0; column <= longitudeSegments; column += 1) {
      const longitude = west + (column / longitudeSegments) * (east - west);
      const point =
        wrapIndex === 0
          ? projectGlobeCoordinate(
              longitude,
              latitude,
              view,
              world,
              progress,
              GLOBE_RADIUS * 1.008,
              0.016,
            )
          : projectGlobeAtlasRepeatCoordinate(
              longitude,
              latitude,
              view,
              world,
              progress,
              wrapIndex,
              GLOBE_RADIUS * 1.008,
              0.016,
            );
      const index = (row * (longitudeSegments + 1) + column) * 3;
      positions.set(point.position, index);
      normals.set(point.normal, index);
    }
  }
  const indices = new Uint32Array(longitudeSegments * latitudeSegments * 6);
  let offset = 0;
  for (let row = 0; row < latitudeSegments; row += 1) {
    for (let column = 0; column < longitudeSegments; column += 1) {
      const northwest = row * (longitudeSegments + 1) + column;
      const northeast = northwest + 1;
      const southwest = northwest + longitudeSegments + 1;
      const southeast = southwest + 1;
      indices.set(
        [northwest, southwest, northeast, northeast, southwest, southeast],
        offset,
      );
      offset += 6;
    }
  }
  return Object.freeze({ positions, normals, indices });
}

function smoothstep(value: number) {
  return value * value * (3 - 2 * value);
}

function interpolateVector(
  source: readonly [number, number, number],
  target: readonly [number, number, number],
  progress: number,
): [number, number, number] {
  return [
    source[0] + (target[0] - source[0]) * progress,
    source[1] + (target[1] - source[1]) * progress,
    source[2] + (target[2] - source[2]) * progress,
  ];
}

function normalizeVector(
  vector: readonly [number, number, number],
): [number, number, number] {
  const length = Math.hypot(...vector);
  return length > 0
    ? [vector[0] / length, vector[1] / length, vector[2] / length]
    : [0, 0, 1];
}

function wrapLongitude(longitude: number) {
  if (longitude === 180) return 180;
  return ((((longitude + 180) % 360) + 360) % 360) - 180;
}

function verifyWorld(world: GlobeProjectionWorld) {
  if (
    !Number.isFinite(world.width) ||
    !Number.isFinite(world.height) ||
    world.width <= 0 ||
    world.height <= 0
  )
    throw new Error("globe projection requires a finite positive world");
}
