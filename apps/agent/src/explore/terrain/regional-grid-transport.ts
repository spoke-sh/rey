import type {
  RegionalTerrainGrid,
  RegionalTerrainGridCell,
} from "../../domain";

const compactBytes = new WeakMap<
  Extract<
    RegionalTerrainGrid,
    { schema: "rey.regional-terrain-grid.transport.v1" }
  >,
  { validity: Uint8Array; materials: Uint8Array }
>();

export function regionalTerrainGridCellCount(grid: RegionalTerrainGrid) {
  return grid.columns * grid.rows;
}

export function validRegionalTerrainGridTransport(
  grid: RegionalTerrainGrid,
): boolean {
  if (grid.schema === "rey.regional-terrain-grid.v1")
    return grid.cells.length === regionalTerrainGridCellCount(grid);
  const cells = regionalTerrainGridCellCount(grid);
  if (
    (grid.source_schema !== "rey.regional-terrain-grid.v1" &&
      grid.source_schema !== "rey.regional-terrain-grid.v2") ||
    !grid.transport_id ||
    !grid.source_id ||
    !grid.source_path ||
    !grid.source_artifact_id ||
    grid.transport_authority !==
      "lossless row-major transport of the exact admitted grid; coordinates and grid positions are reconstructed only from admitted bounds and dimensions" ||
    grid.cell_ids.length !== cells ||
    grid.source_object_ids.length !== cells ||
    grid.source_object_revisions.length !== cells ||
    grid.elevation_micrometers.length !== cells ||
    new Set(grid.cell_ids).size !== cells ||
    new Set(grid.source_object_ids).size !== cells ||
    grid.material_palette.length > 255 ||
    new Set(grid.material_palette).size !== grid.material_palette.length ||
    grid.material_palette.some((material) => !material)
  )
    return false;
  try {
    const { validity, materials } = transportBytes(grid);
    if (validity.length !== cells || materials.length !== cells) return false;
    for (let index = 0; index < cells; index += 1) {
      const valid = validity[index] === 1;
      const material = materials[index]!;
      if (
        (validity[index] !== 0 && validity[index] !== 1) ||
        !Number.isSafeInteger(grid.elevation_micrometers[index]) ||
        (valid && material >= grid.material_palette.length) ||
        (!valid && material !== 255)
      )
        return false;
    }
    return true;
  } catch {
    return false;
  }
}

export function regionalTerrainGridCellAt(
  grid: RegionalTerrainGrid,
  index: number,
): RegionalTerrainGridCell | undefined {
  const cells = regionalTerrainGridCellCount(grid);
  if (!Number.isInteger(index) || index < 0 || index >= cells) return undefined;
  if (grid.schema === "rey.regional-terrain-grid.v1") return grid.cells[index];
  const { validity, materials } = transportBytes(grid);
  const column = index % grid.columns;
  const row = Math.floor(index / grid.columns);
  const longitudeStep =
    (grid.native_bounds.east_microdegrees -
      grid.native_bounds.west_microdegrees) /
    (grid.columns - 1);
  const latitudeStep =
    (grid.native_bounds.north_microdegrees -
      grid.native_bounds.south_microdegrees) /
    (grid.rows - 1);
  const valid = validity[index] === 1;
  const materialIndex = materials[index]!;
  return {
    cell_id: grid.cell_ids[index]!,
    source_object_id: grid.source_object_ids[index]!,
    source_artifact_id: grid.source_artifact_id,
    source_object_revision: grid.source_object_revisions[index]!,
    grid_position: [column, row],
    native_position: [
      grid.native_bounds.west_microdegrees + column * longitudeStep,
      grid.native_bounds.north_microdegrees - row * latitudeStep,
    ],
    elevation_micrometers: valid ? grid.elevation_micrometers[index]! : null,
    material: valid ? grid.material_palette[materialIndex]! : null,
    validity: valid ? "valid" : "no_data",
    authority: valid
      ? "exact admitted Point altitude and material at one valid grid vertex"
      : "explicit source no-data vertex; geometry locates the hole but supplies no height or material",
  };
}

export function regionalTerrainGridCells(
  grid: RegionalTerrainGrid,
): RegionalTerrainGridCell[] {
  return Array.from(
    { length: regionalTerrainGridCellCount(grid) },
    (_, index) => regionalTerrainGridCellAt(grid, index)!,
  );
}

function transportBytes(
  grid: Extract<
    RegionalTerrainGrid,
    { schema: "rey.regional-terrain-grid.transport.v1" }
  >,
) {
  const retained = compactBytes.get(grid);
  if (retained) return retained;
  const decoded = {
    validity: decodeHex(grid.validity_hex),
    materials: decodeHex(grid.material_indices_hex),
  };
  compactBytes.set(grid, decoded);
  return decoded;
}

function decodeHex(value: string): Uint8Array {
  if (value.length % 2 !== 0 || !/^[0-9a-f]*$/.test(value))
    throw new Error(
      "regional terrain transport contains invalid hexadecimal bytes",
    );
  return Uint8Array.from({ length: value.length / 2 }, (_, index) =>
    Number.parseInt(value.slice(index * 2, index * 2 + 2), 16),
  );
}
