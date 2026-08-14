import type { GlobeCameraView } from "../engine/camera";
import { globeAtlasViewCenter } from "@rey/explorer/globe-projection";

export const SEMANTIC_MERCATOR_PROJECTION_REVISION =
  "rey.semantic-mercator-projection@2";
export const SEMANTIC_LONGITUDE_WRAP_MICRODEGREES = 360_000_000;
export const SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES = 85_051_129;

const HALF_LONGITUDE_WRAP_MICRODEGREES =
  SEMANTIC_LONGITUDE_WRAP_MICRODEGREES / 2;
const MICRODEGREES_PER_DEGREE = 1_000_000;

export interface SemanticCoordinate {
  longitude_microdegrees: number;
  latitude_microdegrees: number;
}

export interface ProjectionFrame {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SemanticMercatorViewOffset {
  x: number;
  y: number;
}

export interface SemanticBounds {
  west_microdegrees: number;
  south_microdegrees: number;
  east_microdegrees: number;
  north_microdegrees: number;
  crosses_antimeridian: boolean;
}

export type PolarDisclosure = "north_cap" | "south_cap";

export interface SemanticMercatorPoint {
  x: number;
  y: number;
  longitude_microdegrees: number;
  latitude_microdegrees: number;
  source_latitude_microdegrees: number;
  wrap_index: number;
  polar_disclosure: PolarDisclosure | null;
}

export interface SemanticMercatorInverse {
  coordinate: SemanticCoordinate;
  wrap_index: number;
  outside_vertical_extent: boolean;
}

export interface SemanticMercatorPickCandidate {
  identity: string;
  focus_id: string;
  coordinate: SemanticCoordinate;
}

export interface SemanticMercatorPick {
  identity: string;
  focus_id: string;
  coordinate: SemanticCoordinate;
  inverse_coordinate: SemanticCoordinate;
  wrap_index: number;
  distance: number;
}

export interface SemanticMercatorFragment {
  identity: string;
  fragment_id: string;
  fragment_index: number;
  x: number;
  y: number;
  width: number;
  height: number;
  west_microdegrees: number;
  east_microdegrees: number;
  polar_disclosures: readonly PolarDisclosure[];
}

export interface SemanticGlobePoint {
  x: number;
  y: number;
  depth: number;
  visible: boolean;
}

export interface WorldAtlasMorphPoint {
  identity: string;
  focus_id: string;
  progress: number;
  x: number;
  y: number;
  world: SemanticGlobePoint;
  atlas: SemanticMercatorPoint;
}

export interface WorldAtlasMorphFragment {
  identity: string;
  fragment_id: string;
  fragment_index: number;
  points: readonly WorldAtlasMorphVertex[];
  polar_disclosures: readonly PolarDisclosure[];
}

export interface WorldAtlasMorphVertex {
  vertex_index: number;
  progress: number;
  x: number;
  y: number;
  world: SemanticGlobePoint;
  atlas: SemanticMercatorPoint;
}

/**
 * Projects one synthetic semantic coordinate into one horizontal chart wrap.
 * The returned wrap index is view geometry, not part of semantic identity.
 */
export function projectSemanticMercator(
  coordinate: SemanticCoordinate,
  frame: ProjectionFrame,
  chartWrapIndex = 0,
): SemanticMercatorPoint {
  verifyFrame(frame);
  verifyCoordinate(coordinate);
  if (!Number.isInteger(chartWrapIndex))
    throw new Error("semantic Mercator chart wrap index must be an integer");
  const longitude = wrapSemanticLongitude(coordinate.longitude_microdegrees);
  const latitude = Math.max(
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
    Math.min(
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
      coordinate.latitude_microdegrees,
    ),
  );
  const unitX =
    (longitude + HALF_LONGITUDE_WRAP_MICRODEGREES) /
    SEMANTIC_LONGITUDE_WRAP_MICRODEGREES;
  return Object.freeze({
    x: frame.x + (unitX + chartWrapIndex) * frame.width,
    y: mercatorY(latitude, frame),
    longitude_microdegrees: longitude,
    latitude_microdegrees: latitude,
    source_latitude_microdegrees: coordinate.latitude_microdegrees,
    wrap_index: chartWrapIndex,
    polar_disclosure:
      coordinate.latitude_microdegrees >
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
        ? "north_cap"
        : coordinate.latitude_microdegrees <
            -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
          ? "south_cap"
          : null,
  });
}

/** Keeps the coordinate facing the rotated globe at the center of the chart. */
export function projectViewAlignedSemanticMercator(
  coordinate: SemanticCoordinate,
  frame: ProjectionFrame,
  view: GlobeCameraView,
  chartWrapIndex = 0,
): SemanticMercatorPoint {
  verifyFrame(frame);
  verifyCoordinate(coordinate);
  if (!Number.isInteger(chartWrapIndex))
    throw new Error("semantic Mercator chart wrap index must be an integer");
  const center = semanticMercatorViewCenter(view);
  const longitude = wrapViewRelativeLongitude(
    coordinate.longitude_microdegrees - center.longitude_microdegrees,
  );
  const latitude = Math.max(
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
    Math.min(
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
      coordinate.latitude_microdegrees,
    ),
  );
  const centerLatitude = Math.max(
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
    Math.min(
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
      center.latitude_microdegrees,
    ),
  );
  const unitX =
    (longitude + HALF_LONGITUDE_WRAP_MICRODEGREES) /
    SEMANTIC_LONGITUDE_WRAP_MICRODEGREES;
  return Object.freeze({
    x: frame.x + (unitX + chartWrapIndex) * frame.width,
    y:
      mercatorY(latitude, frame) +
      (frame.y + frame.height / 2 - mercatorY(centerLatitude, frame)),
    longitude_microdegrees: wrapSemanticLongitude(
      coordinate.longitude_microdegrees,
    ),
    latitude_microdegrees: latitude,
    source_latitude_microdegrees: coordinate.latitude_microdegrees,
    wrap_index: chartWrapIndex,
    polar_disclosure:
      coordinate.latitude_microdegrees >
      SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
        ? "north_cap"
        : coordinate.latitude_microdegrees <
            -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
          ? "south_cap"
          : null,
  });
}

export function semanticMercatorViewOffset(
  frame: ProjectionFrame,
  view: GlobeCameraView,
): SemanticMercatorViewOffset {
  const center = semanticMercatorViewCenter(view);
  const canonical = projectSemanticMercator(center, frame);
  return Object.freeze({
    x: frame.x + frame.width / 2 - canonical.x,
    y: frame.y + frame.height / 2 - canonical.y,
  });
}

/** Resolves any horizontally repeated chart point to one canonical coordinate. */
export function invertSemanticMercator(
  point: { x: number; y: number },
  frame: ProjectionFrame,
): SemanticMercatorInverse {
  verifyFrame(frame);
  if (!Number.isFinite(point.x) || !Number.isFinite(point.y))
    throw new Error("semantic Mercator inverse requires finite coordinates");
  const rawUnitX = (point.x - frame.x) / frame.width;
  const wrapIndex = Math.floor(rawUnitX);
  const unitX = rawUnitX - wrapIndex;
  const rawUnitY = (point.y - frame.y) / frame.height;
  const unitY = Math.max(0, Math.min(1, rawUnitY));
  const longitude = Math.round(
    unitX * SEMANTIC_LONGITUDE_WRAP_MICRODEGREES -
      HALF_LONGITUDE_WRAP_MICRODEGREES,
  );
  const latitudeRadians = Math.atan(Math.sinh(Math.PI * (1 - 2 * unitY)));
  return Object.freeze({
    coordinate: Object.freeze({
      longitude_microdegrees: wrapSemanticLongitude(longitude),
      latitude_microdegrees: Math.round(
        (latitudeRadians * 180 * MICRODEGREES_PER_DEGREE) / Math.PI,
      ),
    }),
    wrap_index: wrapIndex,
    outside_vertical_extent: rawUnitY < 0 || rawUnitY > 1,
  });
}

export function invertViewAlignedSemanticMercator(
  point: { x: number; y: number },
  frame: ProjectionFrame,
  view: GlobeCameraView,
): SemanticMercatorInverse {
  const offset = semanticMercatorViewOffset(frame, view);
  return invertSemanticMercator(
    { x: point.x - offset.x, y: point.y - offset.y },
    frame,
  );
}

/**
 * Resolves one point in any repeated chart copy to one retained identity. The
 * inverse coordinate is reported separately from the candidate's exact source
 * coordinate and grants no authority beyond selection.
 */
export function pickSemanticMercator(
  point: { x: number; y: number },
  candidates: readonly SemanticMercatorPickCandidate[],
  frame: ProjectionFrame,
  hitRadius = 24,
): SemanticMercatorPick | null {
  if (!Number.isFinite(hitRadius) || hitRadius < 0)
    throw new Error(
      "semantic Mercator pick radius must be finite and non-negative",
    );
  const inverse = invertSemanticMercator(point, frame);
  const matches = candidates
    .map((candidate) => {
      if (!candidate.identity || !candidate.focus_id)
        throw new Error(
          "semantic Mercator pick candidate identity is required",
        );
      const projected = projectSemanticMercator(
        candidate.coordinate,
        frame,
        inverse.wrap_index,
      );
      return {
        candidate,
        distance: Math.hypot(projected.x - point.x, projected.y - point.y),
      };
    })
    .filter(({ distance }) => distance <= hitRadius)
    .sort(
      (left, right) =>
        left.distance - right.distance ||
        left.candidate.identity.localeCompare(right.candidate.identity),
    );
  const selected = matches[0];
  if (!selected) return null;
  return Object.freeze({
    identity: selected.candidate.identity,
    focus_id: selected.candidate.focus_id,
    coordinate: Object.freeze({ ...selected.candidate.coordinate }),
    inverse_coordinate: inverse.coordinate,
    wrap_index: inverse.wrap_index,
    distance: selected.distance,
  });
}

/**
 * Splits antimeridian-crossing bounds into bounded draw fragments while every
 * fragment retains the same semantic identity for focus and inverse picking.
 */
export function projectSemanticMercatorBounds(
  identity: string,
  bounds: SemanticBounds,
  frame: ProjectionFrame,
): readonly SemanticMercatorFragment[] {
  verifyFrame(frame);
  verifyBounds(identity, bounds);
  const longitudeFragments = longitudeBoundsFragments(bounds);
  const north = Math.min(
    bounds.north_microdegrees,
    SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
  );
  const south = Math.max(
    bounds.south_microdegrees,
    -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES,
  );
  const y = mercatorY(north, frame);
  const southY = mercatorY(south, frame);
  const polarDisclosures: PolarDisclosure[] = [];
  if (
    bounds.north_microdegrees > SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
  )
    polarDisclosures.push("north_cap");
  if (
    bounds.south_microdegrees < -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
  )
    polarDisclosures.push("south_cap");
  return Object.freeze(
    longitudeFragments.map(({ west, east }, fragmentIndex) => {
      const x = boundaryX(west, frame);
      return Object.freeze({
        identity,
        fragment_id: `${identity}#mercator-fragment:${fragmentIndex}`,
        fragment_index: fragmentIndex,
        x,
        y,
        width: Math.max(0, boundaryX(east, frame) - x),
        height: Math.max(0, southY - y),
        west_microdegrees: west,
        east_microdegrees: east,
        polar_disclosures: Object.freeze([...polarDisclosures]),
      });
    }),
  );
}

/** Orthographic World endpoint shared by reference picking and Atlas morphs. */
export function projectSemanticGlobe(
  coordinate: SemanticCoordinate,
  center: { x: number; y: number },
  radius: number,
  view: GlobeCameraView,
): SemanticGlobePoint {
  verifyCoordinate(coordinate);
  if (
    !Number.isFinite(center.x) ||
    !Number.isFinite(center.y) ||
    !Number.isFinite(radius) ||
    radius <= 0
  )
    throw new Error(
      "semantic globe projection requires a finite positive frame",
    );
  const longitude =
    (coordinate.longitude_microdegrees * Math.PI) /
    (180 * MICRODEGREES_PER_DEGREE);
  const latitude =
    (coordinate.latitude_microdegrees * Math.PI) /
    (180 * MICRODEGREES_PER_DEGREE);
  const localX = Math.cos(latitude) * Math.sin(longitude);
  const localY = Math.sin(latitude);
  const localZ = Math.cos(latitude) * Math.cos(longitude);
  const pitch = (view.pitch_degrees * Math.PI) / 180;
  const yaw = (view.yaw_degrees * Math.PI) / 180;
  const pitchY = localY * Math.cos(pitch) - localZ * Math.sin(pitch);
  const pitchZ = localY * Math.sin(pitch) + localZ * Math.cos(pitch);
  const rotatedX = localX * Math.cos(yaw) + pitchZ * Math.sin(yaw);
  const depth = -localX * Math.sin(yaw) + pitchZ * Math.cos(yaw);
  return Object.freeze({
    x: center.x + radius * rotatedX,
    y: center.y - radius * pitchY,
    depth,
    visible: depth >= -0.02,
  });
}

/**
 * Produces a deterministic geometry morph without changing semantic or focus
 * identity. Visibility at either endpoint is presentation state only.
 */
export function projectWorldAtlasMorph(
  identity: string,
  focusId: string,
  coordinate: SemanticCoordinate,
  worldFrame: { center: { x: number; y: number }; radius: number },
  atlasFrame: ProjectionFrame,
  view: GlobeCameraView,
  progress: number,
): WorldAtlasMorphPoint {
  if (!identity || !focusId)
    throw new Error(
      "World-to-Atlas morph requires semantic and focus identity",
    );
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const world = projectSemanticGlobe(
    coordinate,
    worldFrame.center,
    worldFrame.radius,
    view,
  );
  const atlas = projectViewAlignedSemanticMercator(
    coordinate,
    atlasFrame,
    view,
  );
  return Object.freeze({
    identity,
    focus_id: focusId,
    progress: boundedProgress,
    x: world.x + (atlas.x - world.x) * boundedProgress,
    y: world.y + (atlas.y - world.y) * boundedProgress,
    world,
    atlas,
  });
}

/** Morphs bounded sector geometry without changing its unsplit identity. */
export function projectWorldAtlasBoundsMorph(
  identity: string,
  bounds: SemanticBounds,
  worldFrame: { center: { x: number; y: number }; radius: number },
  atlasFrame: ProjectionFrame,
  view: GlobeCameraView,
  progress: number,
): readonly WorldAtlasMorphFragment[] {
  verifyFrame(atlasFrame);
  verifyBounds(identity, bounds);
  const polarDisclosures: PolarDisclosure[] = [];
  if (
    bounds.north_microdegrees > SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
  )
    polarDisclosures.push("north_cap");
  if (
    bounds.south_microdegrees < -SEMANTIC_MERCATOR_LATITUDE_CUTOFF_MICRODEGREES
  )
    polarDisclosures.push("south_cap");
  return Object.freeze(
    viewAlignedLongitudeBoundsFragments(bounds, view).map(
      ({ west, east }, fragmentIndex) => {
        const fragmentId = `${identity}#mercator-fragment:${fragmentIndex}`;
        const coordinates = [
          {
            longitude_microdegrees: west,
            latitude_microdegrees: bounds.north_microdegrees,
          },
          {
            longitude_microdegrees: east,
            latitude_microdegrees: bounds.north_microdegrees,
          },
          {
            longitude_microdegrees: east,
            latitude_microdegrees: bounds.south_microdegrees,
          },
          {
            longitude_microdegrees: west,
            latitude_microdegrees: bounds.south_microdegrees,
          },
        ];
        const points = coordinates.map((coordinate, pointIndex) => {
          const world = projectSemanticGlobe(
            coordinate,
            worldFrame.center,
            worldFrame.radius,
            view,
          );
          const atlas = projectViewAlignedSemanticMercator(
            coordinate,
            atlasFrame,
            view,
          );
          const boundedProgress = Math.max(0, Math.min(1, progress));
          return Object.freeze({
            vertex_index: pointIndex,
            progress: boundedProgress,
            x: world.x + (atlas.x - world.x) * boundedProgress,
            y: world.y + (atlas.y - world.y) * boundedProgress,
            world,
            atlas,
          });
        });
        return Object.freeze({
          identity,
          fragment_id: fragmentId,
          fragment_index: fragmentIndex,
          points: Object.freeze(points),
          polar_disclosures: Object.freeze([...polarDisclosures]),
        });
      },
    ),
  );
}

export function wrapSemanticLongitude(longitudeMicrodegrees: number) {
  if (!Number.isFinite(longitudeMicrodegrees))
    throw new Error("semantic longitude must be finite");
  return (
    ((((longitudeMicrodegrees + HALF_LONGITUDE_WRAP_MICRODEGREES) %
      SEMANTIC_LONGITUDE_WRAP_MICRODEGREES) +
      SEMANTIC_LONGITUDE_WRAP_MICRODEGREES) %
      SEMANTIC_LONGITUDE_WRAP_MICRODEGREES) -
    HALF_LONGITUDE_WRAP_MICRODEGREES
  );
}

function mercatorY(latitudeMicrodegrees: number, frame: ProjectionFrame) {
  const latitudeRadians =
    (latitudeMicrodegrees * Math.PI) / (180 * MICRODEGREES_PER_DEGREE);
  const unitY =
    0.5 - Math.log(Math.tan(Math.PI / 4 + latitudeRadians / 2)) / (2 * Math.PI);
  return frame.y + unitY * frame.height;
}

function boundaryX(longitudeMicrodegrees: number, frame: ProjectionFrame) {
  return (
    frame.x +
    ((longitudeMicrodegrees + HALF_LONGITUDE_WRAP_MICRODEGREES) /
      SEMANTIC_LONGITUDE_WRAP_MICRODEGREES) *
      frame.width
  );
}

function longitudeBoundsFragments(bounds: SemanticBounds) {
  const crosses =
    bounds.crosses_antimeridian ||
    bounds.east_microdegrees < bounds.west_microdegrees;
  return crosses
    ? [
        {
          west: bounds.west_microdegrees,
          east: HALF_LONGITUDE_WRAP_MICRODEGREES,
        },
        {
          west: -HALF_LONGITUDE_WRAP_MICRODEGREES,
          east: bounds.east_microdegrees,
        },
      ]
    : [{ west: bounds.west_microdegrees, east: bounds.east_microdegrees }];
}

function semanticMercatorViewCenter(view: GlobeCameraView) {
  const center = globeAtlasViewCenter(view);
  return Object.freeze({
    longitude_microdegrees: Math.round(
      center.longitude_degrees * MICRODEGREES_PER_DEGREE,
    ),
    latitude_microdegrees: Math.round(
      center.latitude_degrees * MICRODEGREES_PER_DEGREE,
    ),
  });
}

function viewAlignedLongitudeBoundsFragments(
  bounds: SemanticBounds,
  view: GlobeCameraView,
) {
  const center = semanticMercatorViewCenter(view).longitude_microdegrees;
  const chartWest = center - HALF_LONGITUDE_WRAP_MICRODEGREES;
  const chartEast = center + HALF_LONGITUDE_WRAP_MICRODEGREES;
  const sourceWest = bounds.west_microdegrees;
  const sourceEast =
    bounds.crosses_antimeridian ||
    bounds.east_microdegrees < bounds.west_microdegrees
      ? bounds.east_microdegrees + SEMANTIC_LONGITUDE_WRAP_MICRODEGREES
      : bounds.east_microdegrees;
  const firstCopy = Math.floor(
    (chartWest - sourceEast) / SEMANTIC_LONGITUDE_WRAP_MICRODEGREES,
  );
  const lastCopy = Math.ceil(
    (chartEast - sourceWest) / SEMANTIC_LONGITUDE_WRAP_MICRODEGREES,
  );
  const spans: { west: number; east: number }[] = [];
  const copies = Array.from(
    { length: lastCopy - firstCopy + 1 },
    (_, index) => firstCopy + index,
  ).sort((left, right) => Math.abs(left) - Math.abs(right) || left - right);
  for (const copy of copies) {
    const shift = copy * SEMANTIC_LONGITUDE_WRAP_MICRODEGREES;
    const west = Math.max(chartWest, sourceWest + shift);
    const east = Math.min(chartEast, sourceEast + shift);
    if (east > west) spans.push({ west, east });
  }
  return spans;
}

function wrapViewRelativeLongitude(longitudeMicrodegrees: number) {
  if (longitudeMicrodegrees === HALF_LONGITUDE_WRAP_MICRODEGREES)
    return HALF_LONGITUDE_WRAP_MICRODEGREES;
  return wrapSemanticLongitude(longitudeMicrodegrees);
}

function verifyFrame(frame: ProjectionFrame) {
  if (
    !Number.isFinite(frame.x) ||
    !Number.isFinite(frame.y) ||
    !Number.isFinite(frame.width) ||
    !Number.isFinite(frame.height) ||
    frame.width <= 0 ||
    frame.height <= 0
  )
    throw new Error(
      "semantic Mercator projection requires a finite positive frame",
    );
}

function verifyCoordinate(coordinate: SemanticCoordinate) {
  if (
    !Number.isFinite(coordinate.longitude_microdegrees) ||
    !Number.isFinite(coordinate.latitude_microdegrees) ||
    coordinate.latitude_microdegrees < -90_000_000 ||
    coordinate.latitude_microdegrees > 90_000_000
  )
    throw new Error(
      "synthetic semantic coordinate is outside its declared bounds",
    );
}

function verifyBounds(identity: string, bounds: SemanticBounds) {
  if (!identity)
    throw new Error("semantic Mercator fragment identity is required");
  for (const value of [
    bounds.west_microdegrees,
    bounds.south_microdegrees,
    bounds.east_microdegrees,
    bounds.north_microdegrees,
  ])
    if (!Number.isFinite(value))
      throw new Error("semantic Mercator bounds must be finite");
  if (
    bounds.west_microdegrees < -HALF_LONGITUDE_WRAP_MICRODEGREES ||
    bounds.west_microdegrees > HALF_LONGITUDE_WRAP_MICRODEGREES ||
    bounds.east_microdegrees < -HALF_LONGITUDE_WRAP_MICRODEGREES ||
    bounds.east_microdegrees > HALF_LONGITUDE_WRAP_MICRODEGREES ||
    bounds.south_microdegrees < -90_000_000 ||
    bounds.north_microdegrees > 90_000_000 ||
    bounds.south_microdegrees > bounds.north_microdegrees
  )
    throw new Error(
      "semantic Mercator bounds are outside their declared domain",
    );
  if (
    bounds.crosses_antimeridian &&
    bounds.east_microdegrees >= bounds.west_microdegrees
  )
    throw new Error("antimeridian-crossing bounds must wrap east before west");
}
