import { contextGlobeSamples } from "./globe-samples";
import type { ExplorerGlobe } from "./types";

export const SEMANTIC_GLOBE_MATERIAL_REVISION =
  "rey.semantic-globe.tsl-stippled-atmosphere@2";
export const GLOBE_RADIUS = 1.72;
export const GLOBE_SAMPLE_RADIUS = 0.0082;
export const GLOBE_SAMPLE_COUNT = 26_000;

export interface CompiledContextGlobe {
  globe: ExplorerGlobe;
  material_revision: string;
  sample_buckets: readonly {
    id: string;
    color: number;
    opacity: number;
    samples: ReturnType<typeof contextGlobeSamples>;
  }[];
  statistics: {
    field_sets: number;
    vertices: number;
    triangles: number;
    field_bytes: number;
    gpu_bytes: number;
    gpu_budget_bytes: number;
    parity_revision: string;
    parity_samples: number;
    geometry_compilation_ms: number;
  };
}

export function compileContextGlobe(
  globe: ExplorerGlobe,
): CompiledContextGlobe {
  const compilationStarted = measurementNow();
  const samples = contextGlobeSamples(
    globe.source_revision,
    GLOBE_SAMPLE_COUNT,
    globe.regions,
  );
  const bucketDefinitions = [
    { minimum: 0, color: 0x708079, opacity: 0.48 },
    { minimum: 0.58, color: 0x3e504c, opacity: 0.7 },
    { minimum: 0.76, color: 0x243b38, opacity: 0.88 },
  ];
  const sampleBuckets = bucketDefinitions.flatMap((bucket, index) => {
    const maximum =
      bucketDefinitions[index + 1]?.minimum ?? Number.POSITIVE_INFINITY;
    const members = samples.filter(
      (sample) =>
        sample.brightness >= bucket.minimum && sample.brightness < maximum,
    );
    return members.length === 0
      ? []
      : [
          Object.freeze({
            id: `context-globe-samples:${index}`,
            color: bucket.color,
            opacity: bucket.opacity,
            samples: Object.freeze(members),
          }),
        ];
  });
  const markerTriangles = globe.regions.length * 24 + globe.beacons.length * 96;
  const sampleTriangles = sampleBuckets.reduce(
    (total, bucket) => total + bucket.samples.length * 5,
    0,
  );
  const sphereVertices = (160 + 1) * (96 + 1);
  const sourceBytes = samples.length * 16;
  return Object.freeze({
    globe,
    material_revision: SEMANTIC_GLOBE_MATERIAL_REVISION,
    sample_buckets: Object.freeze(sampleBuckets),
    statistics: Object.freeze({
      field_sets: 1,
      vertices: sphereVertices + samples.length,
      triangles: 160 * 96 * 2 + sampleTriangles + markerTriangles,
      field_bytes: sourceBytes,
      gpu_bytes: sourceBytes,
      gpu_budget_bytes: sourceBytes,
      parity_revision: "unbound",
      parity_samples: 0,
      geometry_compilation_ms: measurementNow() - compilationStarted,
    }),
  });
}

function measurementNow(): number {
  return globalThis.performance?.now() ?? Date.now();
}
