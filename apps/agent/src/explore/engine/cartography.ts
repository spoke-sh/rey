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
  feature: {
    geometry_kind: string;
    layer: string;
    cartographic_label?: { min_zoom: number; max_zoom: number };
  },
  regime: LensRegime,
  selected: boolean,
): boolean {
  if (selected) return true;
  if (feature.geometry_kind.toLowerCase() === "point") {
    const label = feature.cartographic_label;
    if (!label) return regime !== "landscape";
    const zoom = CARTOGRAPHIC_ZOOM_BY_LENS[regime];
    return zoom >= label.min_zoom && zoom <= label.max_zoom;
  }
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

const CARTOGRAPHIC_ZOOM_BY_LENS: Readonly<Record<LensRegime, number>> = {
  world: 0,
  atlas: 3,
  landscape: 6,
  neighborhoods: 9,
  objects: 13,
  evidence: 18,
};
