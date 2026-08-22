import { blake3 } from "@noble/hashes/blake3.js";
import type { TerrainFieldSetInput } from "./types";

export const TERRAIN_VALIDITY_CLASSIFICATION_SCHEMA =
  "rey.terrain-validity-classification.v1" as const;
export const TERRAIN_VALIDITY_UNSUPPORTED = 0 as const;
export const TERRAIN_VALIDITY_VALID = 1 as const;
export const TERRAIN_VALIDITY_NO_DATA = 2 as const;

export type TerrainValidityClassification = NonNullable<
  TerrainFieldSetInput["validity_classification"]
>;

export interface TerrainValiditySummary {
  validity_id: string;
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
}

export function createTerrainValidityClassification(
  values: Uint8Array,
  implementationRevision: string,
): TerrainValidityClassification {
  const result = Object.freeze({
    schema: TERRAIN_VALIDITY_CLASSIFICATION_SCHEMA,
    implementation_revision: implementationRevision,
    values,
  });
  verifyTerrainValidityClassification(result);
  return result;
}

export function verifyTerrainFieldValidityClassification(
  field: TerrainFieldSetInput,
): TerrainValidityClassification {
  const classification = field.validity_classification;
  if (!classification)
    throw new Error("terrain validity classification is unbound");
  verifyTerrainValidityClassification(classification, field.field_cells);
  for (let index = 0; index < field.field_cells; index += 1) {
    const supported = field.validity.values[index] !== 0;
    const validityClass = classification.values[index]!;
    if (
      (supported && validityClass !== TERRAIN_VALIDITY_VALID) ||
      (!supported && validityClass === TERRAIN_VALIDITY_VALID)
    )
      throw new Error("terrain validity classification contradicts geometry");
  }
  return classification;
}

export function summarizeTerrainFieldValidity(
  field: TerrainFieldSetInput,
): TerrainValiditySummary {
  const classification = verifyTerrainFieldValidityClassification(field);
  return summarizeTerrainValidityClassification(classification);
}

export function summarizeTerrainValidityClassification(
  classification: TerrainValidityClassification,
): TerrainValiditySummary {
  verifyTerrainValidityClassification(classification);
  let validVertices = 0;
  let noDataVertices = 0;
  let unsupportedVertices = 0;
  for (const value of classification.values) {
    if (value === TERRAIN_VALIDITY_VALID) validVertices += 1;
    else if (value === TERRAIN_VALIDITY_NO_DATA) noDataVertices += 1;
    else unsupportedVertices += 1;
  }
  const header = new TextEncoder().encode(
    `${classification.schema}\u0000${classification.implementation_revision}\u0000`,
  );
  const identity = new Uint8Array(header.length + classification.values.length);
  identity.set(header);
  identity.set(classification.values, header.length);
  return Object.freeze({
    validity_id: `blake3:${hex(blake3(identity))}`,
    valid_vertices: validVertices,
    no_data_vertices: noDataVertices,
    unsupported_vertices: unsupportedVertices,
  });
}

function verifyTerrainValidityClassification(
  classification: TerrainValidityClassification,
  expectedCells?: number,
): void {
  if (
    classification.schema !== TERRAIN_VALIDITY_CLASSIFICATION_SCHEMA ||
    !classification.implementation_revision ||
    !(classification.values instanceof Uint8Array) ||
    (expectedCells !== undefined &&
      classification.values.length !== expectedCells) ||
    classification.values.some(
      (value) =>
        value !== TERRAIN_VALIDITY_UNSUPPORTED &&
        value !== TERRAIN_VALIDITY_VALID &&
        value !== TERRAIN_VALIDITY_NO_DATA,
    )
  )
    throw new Error("terrain validity classification is invalid");
}

function hex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}
