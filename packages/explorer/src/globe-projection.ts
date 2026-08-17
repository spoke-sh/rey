import type { GlobeCameraView } from "./types";
import { GLOBE_RADIUS } from "./three-globe";

export const SEMANTIC_MERCATOR_LATITUDE_CUTOFF_DEGREES = 85.051129;
export const GLOBE_CAMERA_HALF_HEIGHT = 2.12;
export const GLOBE_CAMERA_DISTANCE = 6;
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
  /**
   * One scalar per vertex: the same west-to-east normalized chart position
   * `globeAtlasRepeatSeamWeight` expects, so a repeated wrap copy's shader
   * can compute its own per-vertex seam weight instead of applying one flat
   * scalar to the whole shell. Independent of progress — sphere and atlas
   * endpoints share identical values per vertex index — so it never needs
   * interpolation.
   */
  normalizedChartX: Float32Array;
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

/** Restricts seam bending to the same coplanar band used by repeat depth. */
export function globeAtlasRepeatConnectionProgress(seamWeight: number) {
  if (!Number.isFinite(seamWeight))
    throw new Error("globe Atlas repeat seam weight must be finite");
  const boundedWeight = Math.max(0, Math.min(1, seamWeight));
  if (boundedWeight <= GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT) return 0;
  return smoothstep(
    (boundedWeight - GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT) /
      (1 - GLOBE_ATLAS_REPEAT_DEPTH_CONNECTION_WEIGHT),
  );
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

export function globeAtmosphereShellScale(progress: number) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  return Math.sqrt(morphRemaining);
}

export function globeAtmosphereOpacity(progress: number) {
  const morphRemaining = globeProjectionMorphRemaining(progress);
  return morphRemaining ** 2;
}

/** Echoes atmosphere only through the bounded horizontal-repeat dissolve. */
export function globeAtmosphereRepeatOpacity(progress: number) {
  const repeatOpacity = globeAtlasRepeatOpacity(progress);
  return repeatOpacity * (1 - repeatOpacity);
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

/**
 * The bearing the flat Mercator chart recenters around: the same view
 * center as `globeAtlasViewCenter`, but always at pitch 0. Pitch has no
 * Mercator analogue (the chart's "north is up" convention is fixed), so
 * every Atlas-position computation anchors to this level bearing instead of
 * the view's live, possibly-pitched one — keeping it a fixed target
 * regardless of progress, matching the sphere source's own eased-to-level
 * trajectory. Longitude still tracks yaw, so a pure-yaw orbit still opens
 * the Atlas around the operator's current heading.
 */
export function globeAtlasProjectionCenter(
  view: Pick<GlobeCameraView, "yaw_degrees" | "pitch_degrees">,
): GlobeAtlasViewCenter {
  return globeAtlasViewCenter({
    yaw_degrees: view.yaw_degrees,
    pitch_degrees: 0,
  });
}

export interface GlobeCameraPose {
  position: readonly [number, number, number];
  rotation: readonly [number, number, number];
}

/**
 * The camera's own pose, not the geometry's: pitch has no Mercator
 * analogue (the flat chart's "north is up" convention is fixed), so it
 * lives entirely here as a screen-relative tilt about a fixed
 * world-horizontal axis, independent of yaw — the same orbit-camera
 * convention `terrainCameraProjection` already uses. Yaw stays out of this
 * function entirely; it's baked into the projected geometry itself (see
 * `projectGlobeCoordinate`'s `Ry(yaw)`), since recentering the mesh around
 * the current bearing also decides where the sphere's unfurl seam lands —
 * a content decision, not a viewing-angle one.
 *
 * Eases from the raw pitch (progress 0, a full orbit tilt) to level
 * (progress 1, exactly the static `(0,0,GLOBE_CAMERA_DISTANCE)` pose the
 * Mercator endpoint has always used) over the same curve the rest of the
 * morph runs on, so the camera visibly re-levels while the globe flattens.
 * Verified exact against Three.js's own Object3D/Quaternion math in
 * globe-projection.test.ts: `rotation=[-pitch,0,0]` always faces the
 * origin from `position`, for every bounded pitch value.
 */
export function globeCameraPose(
  view: Pick<GlobeCameraView, "pitch_degrees">,
  progress: number,
): GlobeCameraPose {
  if (!Number.isFinite(view.pitch_degrees) || !Number.isFinite(progress))
    throw new Error("globe camera pose requires finite orientation");
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const progressEased = 1 - globeProjectionMorphRemaining(boundedProgress);
  const pitch = (view.pitch_degrees * (1 - progressEased) * Math.PI) / 180;
  return Object.freeze({
    position: Object.freeze([
      0,
      GLOBE_CAMERA_DISTANCE * Math.sin(pitch),
      GLOBE_CAMERA_DISTANCE * Math.cos(pitch),
    ]) as readonly [number, number, number],
    rotation: Object.freeze([-pitch, 0, 0]) as readonly [
      number,
      number,
      number,
    ],
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
  const progressEased = 1 - globeProjectionMorphRemaining(boundedProgress);
  const longitude = (longitudeDegrees * Math.PI) / 180;
  const latitude = (latitudeDegrees * Math.PI) / 180;
  const local = [
    Math.sin(longitude) * Math.cos(latitude),
    Math.sin(latitude),
    Math.cos(longitude) * Math.cos(latitude),
  ] as const;
  // Yaw recenters the sphere around the current bearing (and with it, the
  // unfurl seam) — this is the only orientation baked into the geometry.
  // Pitch lives entirely in the camera now (see globeCameraPose).
  const yaw = (view.yaw_degrees * Math.PI) / 180;
  const sphereNormal = [
    local[0] * Math.cos(yaw) + local[2] * Math.sin(yaw),
    local[1],
    -local[0] * Math.sin(yaw) + local[2] * Math.cos(yaw),
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
  // The flat Mercator chart has "north always up" fixed into its definition
  // and cannot represent the view's own pitch. Always recentering against
  // the level (pitch 0) bearing — rather than the live, possibly-pitched
  // view — keeps this fixed regardless of progress, so it agrees with the
  // sphere source's own eased-toward-level trajectory above at every
  // progress value, not just the two endpoints.
  const atlasCenter = globeAtlasProjectionCenter(view);
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
  const connectionProgress = globeAtlasRepeatConnectionProgress(seamWeight);
  const atlasCenter = globeAtlasProjectionCenter(view);
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
  const normalizedChartX = new Float32Array(vertexCount);
  // "West" must use the exact same reference as the Atlas-position formula
  // inside projectGlobeCoordinate (globeAtlasProjectionCenter, fixed at
  // pitch 0): the sphere (progress 0) and atlas (progress 1) endpoint
  // meshes are cached once and later blended per vertex index by
  // interpolateProjectedGlobeMeshes, so both builds must sample the exact
  // same (longitude, latitude) grid — any drift here would mismatch vertex
  // correspondence between the two cached meshes, not just this mesh's own
  // seam placement. At progress 1 the rendered position IS the Atlas
  // position (sphere is fully unweighted there), so this also keeps the
  // chart's own west/east boundary columns landing exactly at the Atlas
  // frame's edges rather than drifting whenever pitch isn't 0.
  const atlasCenter = globeAtlasProjectionCenter(view);
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
      const vertexIndex = row * (longitudeSegments + 1) + column;
      const index = vertexIndex * 3;
      positions.set(point.position, index);
      normals.set(point.normal, index);
      // Derived directly from the grid's own column index rather than
      // point.atlas_position[0]: since "west" (this loop's own longitude
      // span) and globeAtlasProjectionCenter's fixed-pitch recentering
      // (used inside atlas_position) are now independent references
      // whenever pitch isn't 0, deriving from atlas_position would put the
      // discontinuity somewhere mid-grid instead of at this mesh's own
      // rendered seam (column 0/max) — breaking the wrap-copy reveal sweep's
      // assumption that it sweeps in from the rendered edge. The two
      // references are identical when pitch is 0, where this is exactly
      // equivalent to the old atlas_position-derived formula.
      normalizedChartX[vertexIndex] = column / longitudeSegments;
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
  return Object.freeze({ positions, normals, indices, normalizedChartX });
}

export interface ProjectedGlobeMeshInterpolationBuffer {
  positions: Float32Array;
  normals: Float32Array;
}

/**
 * Interpolates precompiled sphere/Mercator endpoint buffers for one frame.
 * Called every animation frame while the globe morphs; pass a retained
 * `output` buffer (sized to match `source`) to write into it in place
 * instead of allocating two fresh Float32Arrays each call. A plain per-
 * vertex lerp is exact here: pitch no longer lives in either endpoint's
 * vertex data (see `globeCameraPose`), so there's no view-dependent
 * rotation for the two cached endpoints to disagree on mid-transition.
 */
export function interpolateProjectedGlobeMeshes(
  source: ProjectedGlobeMesh,
  target: ProjectedGlobeMesh,
  progress: number,
  output?: ProjectedGlobeMeshInterpolationBuffer,
): ProjectedGlobeMesh {
  if (!Number.isFinite(progress))
    throw new Error("globe mesh interpolation progress must be finite");
  if (
    source.positions.length !== target.positions.length ||
    source.normals.length !== target.normals.length ||
    source.indices.length !== target.indices.length ||
    source.normalizedChartX.length !== target.normalizedChartX.length
  )
    throw new Error("globe mesh interpolation endpoints must have equal shape");
  const boundedProgress = Math.max(0, Math.min(1, progress));
  if (boundedProgress === 0) return source;
  if (boundedProgress === 1) return target;
  const reuseOutput =
    output !== undefined &&
    output.positions.length === source.positions.length &&
    output.normals.length === source.normals.length;
  const positions = reuseOutput
    ? output.positions
    : new Float32Array(source.positions.length);
  const normals = reuseOutput
    ? output.normals
    : new Float32Array(source.normals.length);
  for (let index = 0; index < positions.length; index += 1) {
    positions[index] =
      source.positions[index]! +
      (target.positions[index]! - source.positions[index]!) * boundedProgress;
  }
  for (let index = 0; index < normals.length; index += 3) {
    const sourceX = source.normals[index]!;
    const sourceY = source.normals[index + 1]!;
    const sourceZ = source.normals[index + 2]!;
    const x = sourceX + (target.normals[index]! - sourceX) * boundedProgress;
    const y =
      sourceY + (target.normals[index + 1]! - sourceY) * boundedProgress;
    const z =
      sourceZ + (target.normals[index + 2]! - sourceZ) * boundedProgress;
    const length = Math.hypot(x, y, z) || 1;
    normals[index] = x / length;
    normals[index + 1] = y / length;
    normals[index + 2] = z / length;
  }
  // atlas_position (and therefore normalizedChartX) doesn't depend on
  // progress — sphere and atlas endpoints compute bit-identical values per
  // vertex index — so this is exact, not an approximation, and needs no
  // interpolation or retained scratch buffer.
  return Object.freeze({
    positions,
    normals,
    indices: source.indices,
    normalizedChartX: source.normalizedChartX,
  });
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
  // Must match globeAtlasProjectionCenter (used inside projectGlobeCoordinate
  // for the actual rendered Atlas position): this chart window decides which
  // longitude span of a sector gets included and where it's clipped, so
  // using a different reference than the one vertices are actually
  // projected against would clip against the wrong window.
  const center = globeAtlasProjectionCenter(view).longitude_degrees;
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
  const vertexCount = (longitudeSegments + 1) * (latitudeSegments + 1);
  const positions = new Float32Array(vertexCount * 3);
  const normals = new Float32Array(positions.length);
  const normalizedChartX = new Float32Array(vertexCount);
  const atlasWidth = globeAtlasWidth(world);
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
      const vertexIndex = row * (longitudeSegments + 1) + column;
      const index = vertexIndex * 3;
      positions.set(point.position, index);
      normals.set(point.normal, index);
      normalizedChartX[vertexIndex] =
        point.atlas_position[0] / atlasWidth + 0.5;
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
  return Object.freeze({ positions, normals, indices, normalizedChartX });
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
