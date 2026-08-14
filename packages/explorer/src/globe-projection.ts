import type { GlobeCameraView } from "./types";
import { GLOBE_RADIUS } from "./three-globe";

export const SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES = 85.051129;
export const GLOBE_CAMERA_HALF_HEIGHT = 2.12;

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
  const atlasPosition = [
    (wrapLongitude(longitudeDegrees) / 180) * halfWidth,
    (Math.log(Math.tan(Math.PI / 4 + latitudeRadians / 2)) / Math.PI) *
      halfHeight,
    planeDepth,
  ] as const;
  const progressEased = smoothstep(boundedProgress);
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
  for (let row = 0; row <= latitudeSegments; row += 1) {
    const latitude = -90 + (row / latitudeSegments) * 180;
    for (let column = 0; column <= longitudeSegments; column += 1) {
      const longitude = -180 + (column / longitudeSegments) * 360;
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
): readonly ProjectedGlobeMesh[] {
  if (
    bounds.south_degrees < -90 ||
    bounds.north_degrees > 90 ||
    bounds.south_degrees >= bounds.north_degrees
  )
    throw new Error("projected globe bounds are invalid");
  const spans =
    bounds.crosses_antimeridian || bounds.east_degrees < bounds.west_degrees
      ? [
          [bounds.west_degrees, 180] as const,
          [-180, bounds.east_degrees] as const,
        ]
      : [[bounds.west_degrees, bounds.east_degrees] as const];
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
      ),
    ),
  );
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
) {
  const positions = new Float32Array(
    (longitudeSegments + 1) * (latitudeSegments + 1) * 3,
  );
  const normals = new Float32Array(positions.length);
  for (let row = 0; row <= latitudeSegments; row += 1) {
    const latitude = south + (row / latitudeSegments) * (north - south);
    for (let column = 0; column <= longitudeSegments; column += 1) {
      const longitude = west + (column / longitudeSegments) * (east - west);
      const point = projectGlobeCoordinate(
        longitude,
        latitude,
        view,
        world,
        progress,
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
