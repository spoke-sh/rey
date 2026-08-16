import { describe, expect, it } from "vitest";
import type { RegionalTerrainGrid } from "../../domain";
import {
  regionalTerrainGridCellAt,
  regionalTerrainGridCellIndexForRevision,
  regionalTerrainGridCells,
  regionalTerrainGridValueColumns,
  validRegionalTerrainGridTransport,
} from "./regional-grid-transport";

const compact = {
  schema: "rey.regional-terrain-grid.transport.v1",
  source_schema: "rey.regional-terrain-grid.v1",
  transport_id: "transport:1",
  dataset_id: "dataset:1",
  source_dataset_id: "source-grid",
  columns: 2,
  rows: 2,
  native_bounds: {
    west_microdegrees: -2,
    south_microdegrees: 4,
    east_microdegrees: 0,
    north_microdegrees: 6,
    crosses_antimeridian: false,
  },
  source_id: "terrain",
  source_path: "terrain.geojson",
  source_artifact_id: "artifact:1",
  cell_source_encoding: "geojson_point_features_v1",
  transport_authority:
    "lossless row-major transport of the exact admitted grid; coordinates and grid positions are reconstructed only from admitted bounds and dimensions",
  cell_ids: ["cell:0", "cell:1", "cell:2", "cell:3"],
  source_object_ids: ["point:0", "point:1", "point:2", "point:3"],
  source_object_revisions: [
    "revision:0",
    "revision:1",
    "revision:2",
    "revision:3",
  ],
  validity_hex: "01010001",
  elevation_micrometers: [1_000_000, 2_000_000, 0, 4_000_000],
  material_palette: ["granite", "soil"],
  material_indices_hex: "0001ff00",
  validity_semantics:
    "row-major source vertices are explicitly valid or no_data; no_data cuts triangle support",
  interpolation:
    "piecewise linear only within triangles whose three admitted source vertices are valid",
  authority:
    "qualified rectilinear height/material grid; validity ends at supported source triangles",
} satisfies RegionalTerrainGrid;

const packedDigest = (byte: number) =>
  `blake3:${byte.toString(16).padStart(2, "0").repeat(32)}`;

const packedDigestColumn = (bytes: number[]) =>
  btoa(
    String.fromCharCode(
      ...bytes.flatMap((byte) => Array.from({ length: 32 }, () => byte)),
    ),
  );

const packed = {
  schema: "rey.regional-terrain-grid.transport.v2",
  source_schema: "rey.regional-terrain-grid.v2",
  transport_id: "transport:2",
  dataset_id: compact.dataset_id,
  source_dataset_id: compact.source_dataset_id,
  columns: compact.columns,
  rows: compact.rows,
  native_bounds: compact.native_bounds,
  source_id: compact.source_id,
  source_path: compact.source_path,
  source_artifact_id: compact.source_artifact_id,
  cell_source_encoding: compact.cell_source_encoding,
  transport_authority: compact.transport_authority,
  digest_encoding: "base64-concatenated-blake3-256",
  cell_digests_base64: packedDigestColumn([0, 1, 2, 3]),
  source_object_id_prefix: "point:",
  source_object_id_suffixes: ["0", "1", "2", "3"],
  source_object_revision_digests_base64: packedDigestColumn([16, 17, 18, 19]),
  validity_hex: compact.validity_hex,
  elevation_micrometers: compact.elevation_micrometers,
  material_palette: compact.material_palette,
  material_indices_hex: compact.material_indices_hex,
  validity_semantics: compact.validity_semantics,
  interpolation: compact.interpolation,
  authority: compact.authority,
} satisfies RegionalTerrainGrid;

describe("regional terrain grid transport", () => {
  it("reconstructs exact row-major cell bindings without repeated geometry", () => {
    expect(validRegionalTerrainGridTransport(compact)).toBe(true);
    expect(regionalTerrainGridCells(compact)).toMatchObject([
      {
        grid_position: [0, 0],
        native_position: [-2, 6],
        elevation_micrometers: 1_000_000,
        material: "granite",
        validity: "valid",
      },
      {
        grid_position: [1, 0],
        native_position: [0, 6],
        elevation_micrometers: 2_000_000,
        material: "soil",
        validity: "valid",
      },
      {
        grid_position: [0, 1],
        native_position: [-2, 4],
        elevation_micrometers: null,
        material: null,
        validity: "no_data",
      },
      {
        grid_position: [1, 1],
        native_position: [0, 4],
        elevation_micrometers: 4_000_000,
        material: "granite",
        validity: "valid",
      },
    ]);
  });

  it("fails closed on malformed bytes, palette indices, and array bounds", () => {
    expect(
      validRegionalTerrainGridTransport({
        ...compact,
        validity_hex: "0101xz01",
      }),
    ).toBe(false);
    expect(
      validRegionalTerrainGridTransport({
        ...compact,
        material_indices_hex: "0002ff00",
      }),
    ).toBe(false);
    expect(
      regionalTerrainGridCellAt(compact, compact.columns * compact.rows),
    ).toBeUndefined();
  });

  it("decodes packed identity columns only for exact cell evidence", () => {
    expect(validRegionalTerrainGridTransport(packed)).toBe(true);
    expect(regionalTerrainGridValueColumns(packed)).toMatchObject({
      elevation_micrometers: compact.elevation_micrometers,
      material_palette: compact.material_palette,
    });
    expect(regionalTerrainGridCellAt(packed, 2)).toMatchObject({
      cell_id: packedDigest(2),
      source_object_id: "point:2",
      source_object_revision: packedDigest(18),
      native_position: [-2, 4],
      validity: "no_data",
    });
    expect(
      regionalTerrainGridCellIndexForRevision(packed, packedDigest(18)),
    ).toBe(2);
    expect(
      regionalTerrainGridCellIndexForRevision(packed, packedDigest(31)),
    ).toBeUndefined();
  });

  it("fails closed when a packed identity column changes length", () => {
    expect(
      validRegionalTerrainGridTransport({
        ...packed,
        cell_digests_base64: packedDigestColumn([0, 1, 2]),
      }),
    ).toBe(false);
  });
});
