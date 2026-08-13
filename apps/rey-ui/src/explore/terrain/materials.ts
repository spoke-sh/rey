import {
  fieldCellCount,
  materialField,
  type MaskField2D,
  type MaterialField2D,
  type ScalarField2D,
  type VectorField2D,
} from "../engine/fields";
import { PROJECTED_SUPPORT } from "./elevation";
import { HYDROLOGY_ACCUMULATION_NORMALIZATION } from "./hydrology";

export function deriveTerrainMaterial(
  elevation: ScalarField2D,
  normal: VectorField2D,
  curvature: ScalarField2D,
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
    const normalZ = normal.values[index * 3 + 2]!;
    const slope = 1 - normalZ;
    const valley = Math.max(0, curvature.values[index]!);
    const ridge = Math.max(0, -curvature.values[index]!);
    const wetness = Math.min(
      1,
      flowAccumulation.values[index]! / HYDROLOGY_ACCUMULATION_NORMALIZATION,
    );
    const low = [0.27, 0.3, 0.3] as const;
    const middle = [0.45, 0.47, 0.43] as const;
    const high = [0.68, 0.67, 0.61] as const;
    const first = Math.min(1, height * 2);
    const second = Math.max(0, Math.min(1, height * 2 - 1));
    for (let component = 0; component < 3; component += 1) {
      const mid = mix(low[component]!, middle[component]!, first);
      const value = mix(mid, high[component]!, second);
      tint[index * 3 + component] = value * (1 - wetness * 0.12) + ridge * 0.16;
    }
    occlusion[index] = clamp(1 - valley * 2.2 - slope * 0.08, 0.56, 1);
    roughness[index] = clamp(0.72 + slope * 0.34 + wetness * 0.08, 0.68, 1);
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

function clamp(value: number, minimum: number, maximum: number) {
  return Math.max(minimum, Math.min(maximum, value));
}
