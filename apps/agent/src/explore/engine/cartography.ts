import type { LensRegime } from "./camera";

export const LANDSCAPE_BASE_VECTOR_LAYERS = Object.freeze([
  "hydrology",
  "highway",
  "road",
  "railway",
  "connector",
] as const);

const LANDSCAPE_BASE_VECTOR_LAYER_SET = new Set<string>(
  LANDSCAPE_BASE_VECTOR_LAYERS,
);

export function featureVisibleAtLens(
  feature: { geometry_kind: string; layer: string },
  regime: LensRegime,
  selected: boolean,
): boolean {
  if (selected) return true;
  if (feature.layer === "terrain")
    return regime === "objects" || regime === "evidence";
  if (feature.layer === "terrain_control")
    return regime === "objects" || regime === "evidence";
  if (regime !== "landscape") return true;
  return (
    LANDSCAPE_BASE_VECTOR_LAYER_SET.has(feature.layer) &&
    feature.geometry_kind.toLowerCase() !== "point"
  );
}
