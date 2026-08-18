export interface GlobeSample {
  longitude_degrees: number;
  latitude_degrees: number;
  brightness: number;
}

export interface GlobeSampleEmphasis {
  longitude_degrees: number;
  latitude_degrees: number;
  angular_radius_degrees: number;
}

export type GlobePole = "north" | "south";

export interface GlobePolePattern {
  id: string;
  pole: GlobePole;
  samples: readonly GlobeSample[];
}

export const CONTEXT_GLOBE_POLE_PATTERN_REVISION =
  "rey.context-globe-pole-pattern@1";

const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

/**
 * Builds deterministic presentation fabric over a sphere. The base field is
 * deliberately coordinate scaffolding, not inferred terrain. Admitted
 * semantic coordinates may brighten the fabric but never create samples
 * outside the same frozen field.
 */
export function contextGlobeSamples(
  sourceRevision: string,
  candidateCount = 26_000,
  emphasis: readonly GlobeSampleEmphasis[] = [],
): readonly GlobeSample[] {
  const seed = revisionSeed(sourceRevision);
  const phase = ((seed % 65_521) / 65_521) * Math.PI * 2;
  const samples: GlobeSample[] = [];

  for (let index = 0; index < candidateCount; index += 1) {
    const y = 1 - ((index + 0.5) * 2) / candidateCount;
    const latitude = Math.asin(y);
    const latitudeRadius = Math.sqrt(Math.max(0, 1 - y * y));
    const azimuth = index * GOLDEN_ANGLE + phase;
    const x = Math.sin(azimuth) * latitudeRadius;
    const z = Math.cos(azimuth) * latitudeRadius;
    const longitude = Math.atan2(x, z);
    const fabric = projectionFabric(x, y, z, seed);
    const emphasized =
      emphasis.length === 0
        ? 0
        : emphasisStrength(longitude, latitude, emphasis);

    // The sparse intervals keep the globe legible while the coherent terms
    // reveal its curvature. Emphasis only raises admitted regions; it does not
    // turn unknown space into a semantic landmass.
    const selection = pseudoRandom(index, seed);
    const density = Math.min(0.98, Math.max(0.44, 0.69 + fabric * 0.2));
    if (selection > density && emphasized < 0.12) continue;
    samples.push({
      longitude_degrees: (longitude * 180) / Math.PI,
      latitude_degrees: (latitude * 180) / Math.PI,
      brightness: Math.min(
        1,
        Math.max(0.28, 0.54 + fabric * 0.27 + emphasized * 0.46),
      ),
    });
  }
  return Object.freeze(samples.map((sample) => Object.freeze(sample)));
}

/**
 * Builds two deterministic dotted polar caps in the same coordinate fabric as
 * the globe samples. They identify geometric ±90° only and carry no evidence,
 * coverage, or native-CRS claim.
 */
export function contextGlobePolePatterns(): readonly GlobePolePattern[] {
  const sampleCount = 34;
  const capRadiusDegrees = 15;
  return Object.freeze(
    (["north", "south"] as const).map((pole) => {
      const sign = pole === "north" ? 1 : -1;
      return Object.freeze({
        id: `${CONTEXT_GLOBE_POLE_PATTERN_REVISION}:${pole}`,
        pole,
        samples: Object.freeze(
          Array.from({ length: sampleCount }, (_, sampleIndex) => {
            const angularRadius =
              capRadiusDegrees *
              Math.sqrt(sampleIndex / Math.max(1, sampleCount - 1));
            const azimuth =
              sampleIndex === 0
                ? 0
                : ((sampleIndex * GOLDEN_ANGLE * sign * 180) / Math.PI) % 360;
            return Object.freeze({
              longitude_degrees: azimuth,
              latitude_degrees: sign * (90 - angularRadius),
              brightness: 0.58 + (1 - angularRadius / capRadiusDegrees) * 0.12,
            });
          }),
        ),
      });
    }),
  );
}

export interface PlanarPresentationSample {
  u: number;
  v: number;
  brightness: number;
}

export const PLANAR_PRESENTATION_FABRIC_REVISION =
  "rey.planar-presentation-fabric@1";

/**
 * The flat-map sibling of contextGlobeSamples: same deterministic,
 * revision-seeded presentation fabric, laid out over a unit square instead
 * of a sphere so it can dress a rectangular terrain frame with the same
 * visual language as the globe's stipple. Point placement uses the R2
 * (plastic-number) low-discrepancy sequence — the natural 2D generalization
 * of a golden-angle spiral for a square domain rather than a sphere, so it
 * fills the whole rectangle evenly instead of fading out toward the corners
 * the way a disk-packed spiral would.
 */
export function planarPresentationSamples(
  sourceRevision: string,
  candidateCount = 6_000,
): readonly PlanarPresentationSample[] {
  const seed = revisionSeed(sourceRevision);
  const phase = ((seed % 65_521) / 65_521) * Math.PI * 2;
  const samples: PlanarPresentationSample[] = [];
  for (let index = 0; index < candidateCount; index += 1) {
    const u = fractional(0.5 + PLASTIC_INVERSE * index + phase / (Math.PI * 2));
    const v = fractional(0.5 + PLASTIC_INVERSE_SQUARE * index);
    const fabric = planarFabric(u * 2 - 1, v * 2 - 1, seed);
    const selection = pseudoRandom(index, seed);
    const density = Math.min(0.98, Math.max(0.44, 0.69 + fabric * 0.2));
    if (selection > density) continue;
    samples.push({
      u,
      v,
      brightness: Math.min(1, Math.max(0.28, 0.54 + fabric * 0.27)),
    });
  }
  return Object.freeze(samples.map((sample) => Object.freeze(sample)));
}

const PLASTIC_NUMBER = 1.324_717_957_244_746;
const PLASTIC_INVERSE = 1 / PLASTIC_NUMBER;
const PLASTIC_INVERSE_SQUARE = 1 / (PLASTIC_NUMBER * PLASTIC_NUMBER);

function fractional(value: number): number {
  return value - Math.floor(value);
}

function planarFabric(x: number, z: number, seed: number): number {
  const a = ((seed >>> 3) % 997) / 997;
  const b = ((seed >>> 13) % 991) / 991;
  const broad = Math.sin(x * 3.1 + z * 2.4 + a * 6.1) * 0.46;
  const folded = Math.sin(z * 5.7 - x * 3.1 + b * 5.4) * 0.31;
  const grain = Math.cos((x - z) * 9.3 + a * 3.2) * 0.15;
  return broad + folded + grain;
}

function pseudoRandom(index: number, seed: number) {
  let value = (index + 1) ^ seed;
  value = Math.imul(value ^ (value >>> 16), 0x7feb352d);
  value = Math.imul(value ^ (value >>> 15), 0x846ca68b);
  return ((value ^ (value >>> 16)) >>> 0) / 4_294_967_295;
}

function projectionFabric(x: number, y: number, z: number, seed: number) {
  const a = ((seed >>> 3) % 997) / 997;
  const b = ((seed >>> 13) % 991) / 991;
  const broad = Math.sin(x * 4.2 + y * 2.7 + a * 6.1) * 0.46;
  const folded = Math.sin(y * 7.3 - z * 3.8 + b * 5.4) * 0.31;
  const grain = Math.cos((x - z) * 13.1 + y * 5.6 + a * 3.2) * 0.15;
  const polar = Math.cos(y * Math.PI * 2.3 + b) * 0.08;
  return broad + folded + grain + polar;
}

function emphasisStrength(
  longitude: number,
  latitude: number,
  emphasis: readonly GlobeSampleEmphasis[],
) {
  let maximum = 0;
  for (const region of emphasis) {
    const regionLongitude = (region.longitude_degrees * Math.PI) / 180;
    const regionLatitude = (region.latitude_degrees * Math.PI) / 180;
    const cosine =
      Math.sin(latitude) * Math.sin(regionLatitude) +
      Math.cos(latitude) *
        Math.cos(regionLatitude) *
        Math.cos(longitude - regionLongitude);
    const distance = Math.acos(Math.min(1, Math.max(-1, cosine)));
    const radius = Math.max(
      (region.angular_radius_degrees * Math.PI) / 180,
      0.04,
    );
    maximum = Math.max(
      maximum,
      Math.exp(-(distance * distance) / (radius * radius * 4)),
    );
  }
  return maximum;
}

function revisionSeed(revision: string) {
  let hash = 2_166_136_261;
  for (let index = 0; index < revision.length; index += 1) {
    hash ^= revision.charCodeAt(index);
    hash = Math.imul(hash, 16_777_619);
  }
  return hash >>> 0;
}
