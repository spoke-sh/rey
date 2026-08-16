import type {
  AdmittedRegionalScene,
  RegionalBounds,
  RegionalFootprint,
} from "../../domain";

export const COUNTY_FRAME_PROJECTION_REVISION = "rey.county-frame-projection@1";
export const COUNTY_FOOTPRINT_PROJECTION_REVISION =
  "rey.county-footprint-projection@1";
export const COUNTY_CAMERA_PITCH_DEGREES = 88;
export const COUNTY_CAMERA_YAW_DEGREES = 0;

export interface CountyLocalPoint {
  east: number;
  north: number;
  up: number;
}

export interface CountyScreenPoint extends CountyLocalPoint {
  x: number;
  y: number;
}

export interface CountyFrame {
  schema: "rey.county-frame.v1";
  frame_id: string;
  scene_id: string;
  source_bounds: RegionalBounds;
  source_origin: readonly [number, number];
  target_origin: readonly [number, number, number];
  transform_id: string;
  transform_revision: number;
  transform_digest: string;
  pitch_degrees: number;
  yaw_degrees: number;
  authority: string;
}

export interface CountyFootprint {
  footprint_id: string;
  scene_id: string;
  source_object_id: string;
  source_artifact_id: string;
  source_object_revision: string;
  native_bounds: RegionalBounds;
  rings: ReadonlyArray<ReadonlyArray<readonly [number, number]>>;
  coordinate_count: number;
  authority: string;
}

export interface ProjectedCountyFootprint extends CountyFootprint {
  path: string;
  screen_rings: ReadonlyArray<ReadonlyArray<CountyScreenPoint>>;
}

export function compileCountyFrame(scene: AdmittedRegionalScene): CountyFrame {
  const matches = scene.projection.transforms.filter(
    (transform) =>
      transform.source_space === "native_crs84" &&
      transform.target_space === "county_local",
  );
  const transform = matches[0];
  const expectedOrigin = regionalBoundsCenter(scene.native_bounds);
  const binding = scene.projection.coordinate_bindings.find(
    ({ space }) => space === "county_local",
  );
  if (
    matches.length !== 1 ||
    !transform ||
    transform.source_origin.length !== 2 ||
    transform.source_origin[0] !== expectedOrigin[0] ||
    transform.source_origin[1] !== expectedOrigin[1] ||
    transform.target_origin.length !== 3 ||
    transform.target_origin.some((value) => value !== 0) ||
    transform.parameters.join(",") !== "east_north_up_microunits" ||
    !transform.inverse_policy.includes("bounded analytic inverse") ||
    !transform.distortion ||
    !binding ||
    binding.status !== "bound" ||
    binding.dimensions.join(",") !== "east,north,up" ||
    binding.units.join(",") !==
      "local_microunit,local_microunit,local_microunit"
  )
    throw new Error(
      "admitted scene does not bind one exact County-local frame",
    );
  const sourceOrigin = Object.freeze([
    transform.source_origin[0],
    transform.source_origin[1],
  ]) as readonly [number, number];
  const targetOrigin = Object.freeze([
    transform.target_origin[0],
    transform.target_origin[1],
    transform.target_origin[2],
  ]) as readonly [number, number, number];
  return Object.freeze({
    schema: "rey.county-frame.v1",
    frame_id: [
      scene.scene_id,
      transform.transform.semantic_digest,
      ...sourceOrigin,
      ...targetOrigin,
    ].join("|"),
    scene_id: scene.scene_id,
    source_bounds: Object.freeze({ ...scene.native_bounds }),
    source_origin: sourceOrigin,
    target_origin: targetOrigin,
    transform_id: transform.transform.id,
    transform_revision: transform.transform.revision,
    transform_digest: transform.transform.semantic_digest,
    pitch_degrees: COUNTY_CAMERA_PITCH_DEGREES,
    yaw_degrees: COUNTY_CAMERA_YAW_DEGREES,
    authority:
      "bounded local tangent presentation over the exact admitted native envelope; envelope bounds do not reconstruct source footprint geometry or prove physical distance",
  });
}

export function compileCountyFootprint(
  scene: AdmittedRegionalScene,
): CountyFootprint | null {
  const footprint = scene.projection.footprint;
  if (!footprint) return null;
  const source = scene.projection.objects.find(
    (object) => object.object_id === footprint.source_object_id,
  );
  const positions = footprint.rings.flat();
  if (
    footprint.geometry_kind !== "Polygon" ||
    footprint.rings.length === 0 ||
    footprint.rings.some(
      (ring) => ring.length < 4 || !samePosition(ring[0], ring.at(-1)),
    ) ||
    positions.length !== footprint.coordinate_count ||
    footprint.coordinate_count >
      scene.projection.limits.max_native_coordinates ||
    !sameBounds(footprint.native_bounds, scene.native_bounds) ||
    !sameBounds(boundsForPositions(positions), footprint.native_bounds) ||
    !source ||
    source.layer !== "boundary" ||
    source.geometry_kind !== "Polygon" ||
    source.source_artifact_id !== footprint.source_artifact_id ||
    source.object_revision !== footprint.source_object_revision ||
    !sameBounds(source.native_bounds, footprint.native_bounds) ||
    footprint.authority !==
      "exact admitted native boundary polygon; footprint validity ends at its rings"
  )
    throw new Error("admitted scene County footprint is invalid");
  return Object.freeze({
    footprint_id: footprint.footprint_id,
    scene_id: scene.scene_id,
    source_object_id: footprint.source_object_id,
    source_artifact_id: footprint.source_artifact_id,
    source_object_revision: footprint.source_object_revision,
    native_bounds: Object.freeze({ ...footprint.native_bounds }),
    rings: Object.freeze(
      footprint.rings.map((ring) =>
        Object.freeze(
          ring.map(
            (position) =>
              Object.freeze([position[0], position[1]]) as readonly [
                number,
                number,
              ],
          ),
        ),
      ),
    ),
    coordinate_count: footprint.coordinate_count,
    authority: footprint.authority,
  });
}

export function nativeBoundsToCountyLocal(
  frame: CountyFrame,
  bounds: RegionalBounds,
): CountyLocalPoint {
  const center = regionalBoundsCenter(bounds);
  return {
    east: longitudeOffset(frame.source_origin[0], center[0]),
    north: center[1] - frame.source_origin[1],
    up: 0,
  };
}

export function nativePositionToCountyLocal(
  frame: CountyFrame,
  position: readonly [number, number],
): CountyLocalPoint {
  return {
    east: longitudeOffset(frame.source_origin[0], position[0]),
    north: position[1] - frame.source_origin[1],
    up: 0,
  };
}

export function countyLocalToNativePosition(
  frame: CountyFrame,
  point: CountyLocalPoint,
): readonly [number, number] {
  if (
    !Number.isFinite(point.east) ||
    !Number.isFinite(point.north) ||
    !Number.isFinite(point.up)
  )
    throw new Error("County-local inverse requires finite coordinates");
  return Object.freeze([
    wrapLongitude(frame.source_origin[0] + point.east),
    frame.source_origin[1] + point.north,
  ]);
}

export function projectCountyFootprint(
  frame: CountyFrame,
  footprint: CountyFootprint,
  view: { center: { x: number; y: number }; scale: number },
): ProjectedCountyFootprint {
  if (frame.scene_id !== footprint.scene_id)
    throw new Error("County footprint does not bind the selected frame");
  const screenRings = footprint.rings.map((ring) =>
    ring.map((position) =>
      projectCountyLocal(
        frame,
        nativePositionToCountyLocal(frame, position),
        view,
      ),
    ),
  );
  return Object.freeze({
    ...footprint,
    path: screenRings
      .map(
        (ring) =>
          ring
            .map(
              ({ x, y }, index) =>
                `${index === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`,
            )
            .join(" ") + " Z",
      )
      .join(" "),
    screen_rings: Object.freeze(
      screenRings.map((ring) =>
        Object.freeze(ring.map((position) => Object.freeze(position))),
      ),
    ),
  });
}

export function projectCountyLocal(
  frame: CountyFrame,
  point: CountyLocalPoint,
  view: { center: { x: number; y: number }; scale: number },
): CountyScreenPoint {
  verifyView(view);
  const yaw = (frame.yaw_degrees * Math.PI) / 180;
  const pitch = (frame.pitch_degrees * Math.PI) / 180;
  const horizontal = point.east * Math.cos(yaw) - point.north * Math.sin(yaw);
  const depth = point.east * Math.sin(yaw) + point.north * Math.cos(yaw);
  return {
    ...point,
    x: view.center.x + horizontal * view.scale,
    y:
      view.center.y +
      depth * Math.sin(pitch) * view.scale -
      point.up * Math.cos(pitch) * view.scale,
  };
}

/** Analytic inverse on the declared up=0 County presentation plane. */
export function invertCountyScreen(
  frame: CountyFrame,
  point: { x: number; y: number },
  view: { center: { x: number; y: number }; scale: number },
): CountyLocalPoint {
  verifyView(view);
  const yaw = (frame.yaw_degrees * Math.PI) / 180;
  const pitch = (frame.pitch_degrees * Math.PI) / 180;
  const horizontal = (point.x - view.center.x) / view.scale;
  const depth = (point.y - view.center.y) / (Math.sin(pitch) * view.scale);
  return {
    east: horizontal * Math.cos(yaw) + depth * Math.sin(yaw),
    north: -horizontal * Math.sin(yaw) + depth * Math.cos(yaw),
    up: 0,
  };
}

export function countyFrameView(
  frame: CountyFrame,
  world: { width: number; height: number },
) {
  const longitudeSpan = Math.max(
    1,
    frame.source_bounds.crosses_antimeridian
      ? frame.source_bounds.east_microdegrees +
          360_000_000 -
          frame.source_bounds.west_microdegrees
      : frame.source_bounds.east_microdegrees -
          frame.source_bounds.west_microdegrees,
  );
  const latitudeSpan = Math.max(
    1,
    frame.source_bounds.north_microdegrees -
      frame.source_bounds.south_microdegrees,
  );
  return Object.freeze({
    center: Object.freeze({ x: world.width / 2, y: world.height / 2 }),
    scale: Math.min(
      (world.width - 360) / longitudeSpan,
      (world.height - 260) / latitudeSpan,
    ),
  });
}

export function regionalBoundsCenter(
  bounds: RegionalBounds,
): readonly [number, number] {
  const east = bounds.crosses_antimeridian
    ? bounds.east_microdegrees + 360_000_000
    : bounds.east_microdegrees;
  let longitude = Math.trunc((bounds.west_microdegrees + east) / 2);
  if (longitude > 180_000_000) longitude -= 360_000_000;
  return Object.freeze([
    longitude,
    Math.trunc((bounds.south_microdegrees + bounds.north_microdegrees) / 2),
  ]);
}

function longitudeOffset(origin: number, longitude: number) {
  let offset = longitude - origin;
  if (offset > 180_000_000) offset -= 360_000_000;
  if (offset < -180_000_000) offset += 360_000_000;
  return offset;
}

function wrapLongitude(longitude: number) {
  let wrapped = longitude;
  while (wrapped > 180_000_000) wrapped -= 360_000_000;
  while (wrapped < -180_000_000) wrapped += 360_000_000;
  return wrapped;
}

function samePosition(
  left: readonly [number, number] | undefined,
  right: readonly [number, number] | undefined,
) {
  return !!left && !!right && left[0] === right[0] && left[1] === right[1];
}

function sameBounds(left: RegionalBounds, right: RegionalBounds) {
  return (
    left.west_microdegrees === right.west_microdegrees &&
    left.south_microdegrees === right.south_microdegrees &&
    left.east_microdegrees === right.east_microdegrees &&
    left.north_microdegrees === right.north_microdegrees &&
    left.crosses_antimeridian === right.crosses_antimeridian
  );
}

function boundsForPositions(
  positions: ReadonlyArray<readonly [number, number]>,
): RegionalBounds {
  if (positions.length === 0)
    throw new Error("County footprint has no positions");
  const latitudes = positions.map((position) => position[1]);
  const longitudes = [
    ...new Set(positions.map((position) => position[0])),
  ].sort((left, right) => left - right);
  let west = longitudes[0]!;
  let east = longitudes[0]!;
  let largestGap = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < longitudes.length; index += 1) {
    const current = longitudes[index]!;
    const next =
      index + 1 < longitudes.length
        ? longitudes[index + 1]!
        : longitudes[0]! + 360_000_000;
    if (next - current > largestGap) {
      largestGap = next - current;
      west = longitudes[(index + 1) % longitudes.length]!;
      east = current;
    }
  }
  return {
    west_microdegrees: west,
    south_microdegrees: Math.min(...latitudes),
    east_microdegrees: east,
    north_microdegrees: Math.max(...latitudes),
    crosses_antimeridian: west > east,
  };
}

function verifyView(view: { center: { x: number; y: number }; scale: number }) {
  if (
    !Number.isFinite(view.center.x) ||
    !Number.isFinite(view.center.y) ||
    !Number.isFinite(view.scale) ||
    view.scale <= 0
  )
    throw new Error("County frame view must be finite and positive");
}
