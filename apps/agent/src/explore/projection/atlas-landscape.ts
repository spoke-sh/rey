import { projectTerrainCoordinate } from "@rey/explorer";
import type { FieldBounds } from "../engine/fields";

export const ATLAS_LANDSCAPE_PROJECTION_REVISION =
  "rey.atlas-landscape-projector@1" as const;
export const ATLAS_LANDSCAPE_MORPH_START_ZOOM = 0.34;
export const ATLAS_LANDSCAPE_MORPH_END_ZOOM = 0.66;

export interface AtlasLandscapeProjectionBinding {
  source_frame: FieldBounds;
  target_frame: FieldBounds;
}

export interface TerrainModelTransform {
  scale_x: number;
  scale_z: number;
  translate_x: number;
  translate_z: number;
  elevation_scale: number;
}

export interface AtlasLandscapePresentation {
  progress: number;
  camera_progress: number;
  composition_scale: number;
  terrain_opacity: number;
  atlas_opacity: number;
  pitch_degrees: number;
  yaw_degrees: number;
  model_transform: TerrainModelTransform;
  css_transform: string;
}

export function atlasLandscapeMorphProgress(zoom: number): number {
  if (!Number.isFinite(zoom))
    throw new Error("Atlas-to-Landscape progress requires finite zoom");
  return smootherstep(
    (zoom - ATLAS_LANDSCAPE_MORPH_START_ZOOM) /
      (ATLAS_LANDSCAPE_MORPH_END_ZOOM - ATLAS_LANDSCAPE_MORPH_START_ZOOM),
  );
}

/**
 * The composition scale atlasLandscapePresentation bakes into the real
 * on-screen render — pulled out on its own so callers that only need to
 * anchor a zoom-to-pointer pan (not the full presentation: pitch/yaw,
 * model transform, opacities) can compute the exact same scale the paint
 * path uses, instead of recomputing pan against a scale that omits this
 * factor. That mismatch previously let the anchored point drift out from
 * under the pointer while zooming through the Atlas-to-Landscape band,
 * compounding every animation frame since pan is derived from its own
 * previous value each frame.
 */
export function atlasLandscapeCompositionScale(progress: number): number {
  if (!Number.isFinite(progress))
    throw new Error(
      "Atlas-to-Landscape composition scale requires finite progress",
    );
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const cameraProgress = smootherstep((boundedProgress - 0.12) / (1 - 0.12));
  return interpolate(1, 1.38, cameraProgress);
}

export function atlasLandscapePresentation(
  binding: AtlasLandscapeProjectionBinding,
  progress: number,
  orbit: { pitch_degrees: number; yaw_degrees: number },
  world: { width: number; height: number },
): AtlasLandscapePresentation {
  verifyFrame(binding.source_frame, "source");
  verifyFrame(binding.target_frame, "target");
  if (
    !Number.isFinite(progress) ||
    !Number.isFinite(orbit.pitch_degrees) ||
    !Number.isFinite(orbit.yaw_degrees)
  )
    throw new Error("Atlas-to-Landscape presentation values are invalid");
  const boundedProgress = Math.max(0, Math.min(1, progress));
  const cameraProgress = smootherstep((boundedProgress - 0.12) / (1 - 0.12));
  const sourceScaleX = binding.source_frame.width / binding.target_frame.width;
  const sourceScaleZ =
    binding.source_frame.height / binding.target_frame.height;
  const scaleX = interpolate(sourceScaleX, 1, boundedProgress);
  const scaleZ = interpolate(sourceScaleZ, 1, boundedProgress);
  const sourceTranslateX =
    binding.source_frame.x - binding.target_frame.x * sourceScaleX;
  const sourceTranslateZ =
    binding.source_frame.y - binding.target_frame.y * sourceScaleZ;
  const translateX = interpolate(sourceTranslateX, 0, boundedProgress);
  const translateZ = interpolate(sourceTranslateZ, 0, boundedProgress);
  const pitchDegrees = interpolate(90, orbit.pitch_degrees, cameraProgress);
  // Yaw is held constant (not animated) through the swoop: sweeping pitch
  // and yaw together makes off-center screen positions non-monotonic during
  // the transition (verified against both this DOM projection and the
  // accelerated path's own orbit camera — it's a property of composing
  // rotation with translation for off-pivot points, not a bug specific to
  // either implementation). Holding yaw fixed and only sweeping pitch across
  // its clamped [22°,90°] range keeps every flat (zero-elevation) point's
  // screen Y a single monotonic quarter-sine, which covers every label.
  const yawDegrees = orbit.yaw_degrees;
  const modelTransform = Object.freeze({
    scale_x: scaleX,
    scale_z: scaleZ,
    translate_x: translateX,
    translate_z: translateZ,
    elevation_scale: smootherstep((boundedProgress - 0.08) / (1 - 0.08)),
  });
  return Object.freeze({
    progress: boundedProgress,
    camera_progress: cameraProgress,
    composition_scale: atlasLandscapeCompositionScale(boundedProgress),
    terrain_opacity: smootherstep(boundedProgress / 0.22),
    atlas_opacity: 1 - smootherstep((boundedProgress - 0.42) / 0.46),
    pitch_degrees: pitchDegrees,
    yaw_degrees: yawDegrees,
    model_transform: modelTransform,
    css_transform: cssTerrainTransform(
      modelTransform,
      pitchDegrees,
      yawDegrees,
      world,
    ),
  });
}

export function projectAtlasLandscapePoint(
  point: { x: number; y: number },
  transform: TerrainModelTransform,
): { x: number; y: number } {
  return Object.freeze({
    x: point.x * transform.scale_x + transform.translate_x,
    y: point.y * transform.scale_z + transform.translate_z,
  });
}

/**
 * Builds the 2D CSS matrix approximating the same real orbit camera
 * `packages/explorer`'s accelerated terrain path renders with
 * (`projectTerrainCoordinate` / `terrainCameraProjection`), rather than an
 * independently hand-derived rotation formula that could silently diverge
 * from it. `projectTerrainCoordinate` is affine in `(x, z)` for a fixed
 * pose (elevation held at 0, since labels are flat), so probing it at the
 * pivot and one unit step along each axis recovers the exact linear
 * coefficients — no approximation, just finite differences of an affine
 * function.
 */
function cssTerrainTransform(
  model: TerrainModelTransform,
  pitchDegrees: number,
  yawDegrees: number,
  world: { width: number; height: number },
): string {
  const view = { pitch_degrees: pitchDegrees, yaw_degrees: yawDegrees };
  const centerX = world.width / 2;
  const centerY = world.height / 2;
  const origin = projectTerrainCoordinate(
    { x: centerX, z: centerY },
    view,
    world,
  );
  const alongX = projectTerrainCoordinate(
    { x: centerX + 1, z: centerY },
    view,
    world,
  );
  const alongZ = projectTerrainCoordinate(
    { x: centerX, z: centerY + 1 },
    view,
    world,
  );
  const cameraA = alongX.x - origin.x;
  const cameraB = alongX.y - origin.y;
  const cameraC = alongZ.x - origin.x;
  const cameraD = alongZ.y - origin.y;
  const a = cameraA * model.scale_x;
  const b = cameraB * model.scale_x;
  const c = cameraC * model.scale_z;
  const d = cameraD * model.scale_z;
  const e =
    cameraA * model.translate_x + cameraC * model.translate_z + origin.x;
  const f =
    cameraB * model.translate_x + cameraD * model.translate_z + origin.y;
  return `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${f})`;
}

function verifyFrame(frame: FieldBounds, label: string) {
  if (
    !Number.isFinite(frame.x) ||
    !Number.isFinite(frame.y) ||
    !Number.isFinite(frame.width) ||
    !Number.isFinite(frame.height) ||
    frame.width <= 0 ||
    frame.height <= 0
  )
    throw new Error(`Atlas-to-Landscape ${label} frame is invalid`);
}

function smootherstep(value: number): number {
  const bounded = Math.max(0, Math.min(1, value));
  return bounded * bounded * bounded * (bounded * (bounded * 6 - 15) + 10);
}

function interpolate(left: number, right: number, progress: number): number {
  return left + (right - left) * progress;
}
