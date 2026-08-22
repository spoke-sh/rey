import {
  verifyLandscapeReliefField,
  type LandscapeReliefField,
} from "./landscape-relief";
import type { TerrainFieldSetInput } from "./types";

export const LANDSCAPE_CARTOGRAPHIC_COLOR_REVISION =
  "rey.landscape.chromatic-relief@1" as const;

const WARM_DIRECT = Object.freeze([1.12, 0.98, 0.78] as const);
const COOL_SKY = Object.freeze([0.68, 0.82, 1.06] as const);

/**
 * Composes one linear-color terrain result. Relief owns all lighting; GPU and
 * reference renderers only project this retained array and never relight it.
 */
export function composeCartographicTerrainColor(
  field: TerrainFieldSetInput,
  relief: LandscapeReliefField,
): Float32Array {
  verifyLandscapeReliefField(field, relief);
  const color = new Float32Array(field.field_cells * 3);
  for (let index = 0; index < field.field_cells; index += 1) {
    if (field.validity.values[index] === 0) continue;
    const direct = smoothstep(0.64, 1.16, relief.mdow[index]!);
    const ambient =
      (0.3 + relief.sky_view_factor[index]! * 0.36) *
      (0.95 + relief.openness[index]! * 0.05) *
      (0.74 + field.material.occlusion[index]! * 0.26);
    const directStrength =
      (0.28 + direct * 0.62) * (0.88 + relief.local_contrast[index]! * 0.12);
    const targetLuminance =
      baseLuminance(field, index) * relief.hillshade[index]!;
    const candidate = [0, 0, 0];
    for (let component = 0; component < 3; component += 1) {
      const base = field.material.tint[index * 3 + component]!;
      candidate[component] =
        base *
        (WARM_DIRECT[component]! * directStrength +
          COOL_SKY[component]! * ambient * (1 - direct * 0.42));
    }
    const candidateLuminance =
      candidate[0]! * 0.2126 + candidate[1]! * 0.7152 + candidate[2]! * 0.0722;
    const luminanceScale =
      candidateLuminance <= 0 ? 0 : targetLuminance / candidateLuminance;
    const ridgeExposure = relief.salience[index]! * 0.025;
    for (let component = 0; component < 3; component += 1)
      color[index * 3 + component] = Math.fround(
        clamp(
          candidate[component]! * luminanceScale +
            WARM_DIRECT[component]! * ridgeExposure,
          0,
          1,
        ),
      );
  }
  return color;
}

export function linearTerrainColorToCss(color: readonly number[]): string {
  return `rgb(${color
    .map((component) => Math.round(linearToSrgb(clamp(component, 0, 1)) * 255))
    .join(" ")})`;
}

function baseLuminance(field: TerrainFieldSetInput, index: number): number {
  return (
    field.material.tint[index * 3]! * 0.2126 +
    field.material.tint[index * 3 + 1]! * 0.7152 +
    field.material.tint[index * 3 + 2]! * 0.0722
  );
}

function linearToSrgb(value: number): number {
  return value <= 0.003_130_8
    ? value * 12.92
    : 1.055 * value ** (1 / 2.4) - 0.055;
}

function smoothstep(minimum: number, maximum: number, value: number): number {
  const progress = clamp((value - minimum) / (maximum - minimum), 0, 1);
  return progress * progress * (3 - 2 * progress);
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}
