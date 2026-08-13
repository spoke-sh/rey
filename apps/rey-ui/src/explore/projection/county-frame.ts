import type { AdmittedRegionalScene, RegionalBounds } from "../../domain";

export const COUNTY_FRAME_PROJECTION_REVISION = "rey.county-frame-projection@1";
export const COUNTY_CAMERA_PITCH_DEGREES = 35.26439;
export const COUNTY_CAMERA_YAW_DEGREES = 45;

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
  let longitude = (bounds.west_microdegrees + east) / 2;
  if (longitude > 180_000_000) longitude -= 360_000_000;
  return Object.freeze([
    longitude,
    (bounds.south_microdegrees + bounds.north_microdegrees) / 2,
  ]);
}

function longitudeOffset(origin: number, longitude: number) {
  let offset = longitude - origin;
  if (offset > 180_000_000) offset -= 360_000_000;
  if (offset < -180_000_000) offset += 360_000_000;
  return offset;
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
