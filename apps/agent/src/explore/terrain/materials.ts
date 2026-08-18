import {
  fieldCellCount,
  materialField,
  type MaskField2D,
  type MaterialField2D,
  type ScalarField2D,
} from "../engine/fields";
import { PROJECTED_SUPPORT } from "./elevation";
import { HYDROLOGY_ACCUMULATION_NORMALIZATION } from "./hydrology";

const TERRAIN_MATERIAL_ELEVATION_BAND_COUNT = 4;

/**
 * Flat, banded elevation tint instead of a continuous height/slope/curvature/
 * wetness-driven gradient — quantizing height into a handful of steps, and
 * flattening occlusion/roughness to near-constants, is what keeps this
 * reading as an abstract map rather than a lit 3D relief render. Wetness
 * still nudges hue slightly so water-adjacent ground stays legible.
 */
export function deriveTerrainMaterial(
  elevation: ScalarField2D,
  flowAccumulation: ScalarField2D,
  validity: MaskField2D,
  revision: string,
): MaterialField2D {
  const cells = fieldCellCount(elevation.grid);
  const tint = new Float32Array(cells * 3);
  const occlusion = new Float32Array(cells);
  const roughness = new Float32Array(cells);
  for (let index = 0; index < cells; index += 1) {
    if (validity.values[index] !== PROJECTED_SUPPORT) continue;
    const height = elevation.values[index]!;
    const wetness = Math.min(
      1,
      flowAccumulation.values[index]! / HYDROLOGY_ACCUMULATION_NORMALIZATION,
    );
    const low = [0.27, 0.3, 0.3] as const;
    const middle = [0.45, 0.47, 0.43] as const;
    const high = [0.68, 0.67, 0.61] as const;
    const bandedHeight =
      Math.round(
        Math.max(0, Math.min(1, height)) *
          (TERRAIN_MATERIAL_ELEVATION_BAND_COUNT - 1),
      ) /
      (TERRAIN_MATERIAL_ELEVATION_BAND_COUNT - 1);
    const first = Math.min(1, bandedHeight * 2);
    const second = Math.max(0, Math.min(1, bandedHeight * 2 - 1));
    for (let component = 0; component < 3; component += 1) {
      const mid = mix(low[component]!, middle[component]!, first);
      const value = mix(mid, high[component]!, second);
      tint[index * 3 + component] = value * (1 - wetness * 0.12);
    }
    occlusion[index] = 0.92;
    roughness[index] = 0.88;
  }
  return materialField(
    "material",
    revision,
    elevation.grid,
    tint,
    occlusion,
    roughness,
  );
}

function mix(left: number, right: number, amount: number) {
  return left + (right - left) * amount;
}
