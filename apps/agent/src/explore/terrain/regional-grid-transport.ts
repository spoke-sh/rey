import type {
  RegionalTerrainGrid,
  RegionalTerrainGridCell,
} from "../../domain";

type TransportedRegionalTerrainGrid = Extract<
  RegionalTerrainGrid,
  {
    schema:
      | "rey.regional-terrain-grid.transport.v1"
      | "rey.regional-terrain-grid.transport.v2";
  }
>;

interface RegionalTerrainGridValueColumns {
  validity: Uint8Array;
  elevation_micrometers: readonly number[];
  material_indices: Uint8Array;
  material_palette: readonly string[];
}

interface TransportBytes {
  validity: Uint8Array;
  materials: Uint8Array;
}

interface TransportIdentityBytes {
  cellDigests: Uint8Array;
  sourceObjectRevisionDigests: Uint8Array;
}

const compactBytes = new WeakMap<
  TransportedRegionalTerrainGrid,
  TransportBytes
>();
const compactIdentityBytes = new WeakMap<
  Extract<
    RegionalTerrainGrid,
    { schema: "rey.regional-terrain-grid.transport.v2" }
  >,
  TransportIdentityBytes
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
  const commonValid =
    (grid.source_schema === "rey.regional-terrain-grid.v1" ||
      grid.source_schema === "rey.regional-terrain-grid.v2" ||
      grid.source_schema === "rey.regional-terrain-grid.v3") &&
    Boolean(grid.transport_id) &&
    Boolean(grid.source_id) &&
    Boolean(grid.source_path) &&
    Boolean(grid.source_artifact_id) &&
    (grid.cell_source_encoding === "geojson_point_features_v1" ||
      grid.cell_source_encoding === "geojson_packed_grid_v1") &&
    grid.transport_authority ===
      "lossless row-major transport of the exact admitted grid; coordinates and grid positions are reconstructed only from admitted bounds and dimensions" &&
    grid.elevation_micrometers.length === cells &&
    grid.material_palette.length <= 255 &&
    new Set(grid.material_palette).size === grid.material_palette.length &&
    grid.material_palette.every(Boolean);
  if (!commonValid) return false;
  if (
    grid.schema === "rey.regional-terrain-grid.transport.v1" &&
    (grid.cell_ids.length !== cells ||
      grid.source_object_ids.length !== cells ||
      grid.source_object_revisions.length !== cells ||
      new Set(grid.cell_ids).size !== cells ||
      new Set(grid.source_object_ids).size !== cells)
  )
    return false;
  if (
    grid.schema === "rey.regional-terrain-grid.transport.v2" &&
    (grid.digest_encoding !== "base64-concatenated-blake3-256" ||
      grid.source_object_id_suffixes.length !== cells ||
      new Set(grid.source_object_id_suffixes).size !== cells ||
      grid.source_object_id_suffixes.some(
        (suffix) => !`${grid.source_object_id_prefix}${suffix}`,
      ))
  )
    return false;
  try {
    const { validity, materials } = transportBytes(grid);
    if (
      validity.length !== cells ||
      materials.length !== cells ||
      (grid.schema === "rey.regional-terrain-grid.transport.v2" &&
        (!validBase64Bytes(grid.cell_digests_base64, cells * 32) ||
          !validBase64Bytes(
            grid.source_object_revision_digests_base64,
            cells * 32,
          )))
    )
      return false;
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

export function regionalTerrainGridValueColumns(
  grid: RegionalTerrainGrid,
): RegionalTerrainGridValueColumns {
  if (grid.schema === "rey.regional-terrain-grid.v1") {
    const validity = Uint8Array.from(
      grid.cells.map((cell) => (cell.validity === "valid" ? 1 : 0)),
    );
    const materialPalette = [
      ...new Set(
        grid.cells.flatMap((cell) =>
          cell.material === null ? [] : [cell.material],
        ),
      ),
    ];
    const materialLookup = new Map(
      materialPalette.map((material, index) => [material, index]),
    );
    return {
      validity,
      elevation_micrometers: grid.cells.map(
        (cell) => cell.elevation_micrometers ?? 0,
      ),
      material_indices: Uint8Array.from(
        grid.cells.map((cell) =>
          cell.material === null ? 255 : materialLookup.get(cell.material)!,
        ),
      ),
      material_palette: materialPalette,
    };
  }
  const { validity, materials } = transportBytes(grid);
  return {
    validity,
    elevation_micrometers: grid.elevation_micrometers,
    material_indices: materials,
    material_palette: grid.material_palette,
  };
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
  const packed = grid.schema === "rey.regional-terrain-grid.transport.v2";
  const identities = packed ? transportIdentityBytes(grid) : undefined;
  return {
    cell_id: packed
      ? digestAt(identities!.cellDigests, index)
      : grid.cell_ids[index]!,
    source_object_id: packed
      ? `${grid.source_object_id_prefix}${grid.source_object_id_suffixes[index]}`
      : grid.source_object_ids[index]!,
    source_artifact_id: grid.source_artifact_id,
    source_object_revision: packed
      ? digestAt(identities!.sourceObjectRevisionDigests, index)
      : grid.source_object_revisions[index]!,
    grid_position: [column, row],
    native_position: [
      grid.native_bounds.west_microdegrees + column * longitudeStep,
      grid.native_bounds.north_microdegrees - row * latitudeStep,
    ],
    elevation_micrometers: valid ? grid.elevation_micrometers[index]! : null,
    material: valid ? grid.material_palette[materialIndex]! : null,
    validity: valid ? "valid" : "no_data",
    authority:
      grid.cell_source_encoding === "geojson_packed_grid_v1"
        ? valid
          ? "exact packed source altitude and material at one valid grid vertex"
          : "explicit packed source no-data vertex; grid position locates the hole but supplies no height or material"
        : valid
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

export function regionalTerrainGridCellIndexForRevision(
  grid: RegionalTerrainGrid,
  revision: string,
): number | undefined {
  if (grid.schema === "rey.regional-terrain-grid.v1") {
    const index = grid.cells.findIndex(
      (cell) => cell.source_object_revision === revision,
    );
    return index >= 0 &&
      grid.cells.findLastIndex(
        (cell) => cell.source_object_revision === revision,
      ) === index
      ? index
      : undefined;
  }
  if (grid.schema === "rey.regional-terrain-grid.transport.v1") {
    const index = grid.source_object_revisions.indexOf(revision);
    return index >= 0 &&
      grid.source_object_revisions.lastIndexOf(revision) === index
      ? index
      : undefined;
  }
  const wanted = decodeBlake3Digest(revision);
  if (!wanted) return undefined;
  const revisions = transportIdentityBytes(grid).sourceObjectRevisionDigests;
  let matched: number | undefined;
  for (let index = 0; index < regionalTerrainGridCellCount(grid); index += 1) {
    const offset = index * 32;
    let equal = true;
    for (let byte = 0; byte < 32; byte += 1) {
      if (revisions[offset + byte] !== wanted[byte]) {
        equal = false;
        break;
      }
    }
    if (!equal) continue;
    if (matched !== undefined) return undefined;
    matched = index;
  }
  return matched;
}

function transportBytes(grid: TransportedRegionalTerrainGrid): TransportBytes {
  const retained = compactBytes.get(grid);
  if (retained) return retained;
  const decoded: TransportBytes = {
    validity: decodeHex(grid.validity_hex),
    materials: decodeHex(grid.material_indices_hex),
  };
  compactBytes.set(grid, decoded);
  return decoded;
}

function transportIdentityBytes(
  grid: Extract<
    RegionalTerrainGrid,
    { schema: "rey.regional-terrain-grid.transport.v2" }
  >,
): TransportIdentityBytes {
  const retained = compactIdentityBytes.get(grid);
  if (retained) return retained;
  const decoded = {
    cellDigests: decodeBase64(grid.cell_digests_base64),
    sourceObjectRevisionDigests: decodeBase64(
      grid.source_object_revision_digests_base64,
    ),
  };
  compactIdentityBytes.set(grid, decoded);
  return decoded;
}

function digestAt(bytes: Uint8Array, index: number): string {
  const start = index * 32;
  let digest = "blake3:";
  for (let offset = start; offset < start + 32; offset += 1)
    digest += bytes[offset]!.toString(16).padStart(2, "0");
  return digest;
}

function decodeBlake3Digest(value: string): Uint8Array | undefined {
  if (!/^blake3:[0-9a-f]{64}$/.test(value)) return undefined;
  return decodeHex(value.slice("blake3:".length));
}

function decodeBase64(value: string): Uint8Array {
  if (!validBase64Bytes(value))
    throw new Error("regional terrain transport contains invalid base64 bytes");
  const decoded = atob(value);
  const bytes = new Uint8Array(decoded.length);
  for (let index = 0; index < decoded.length; index += 1)
    bytes[index] = decoded.charCodeAt(index);
  return bytes;
}

function validBase64Bytes(value: string, expectedBytes?: number): boolean {
  if (value.length % 4 !== 0) return false;
  const padding = value.endsWith("==") ? 2 : value.endsWith("=") ? 1 : 0;
  const contentLength = value.length - padding;
  for (let index = 0; index < contentLength; index += 1) {
    const character = value.charCodeAt(index);
    if (
      !(
        (character >= 65 && character <= 90) ||
        (character >= 97 && character <= 122) ||
        (character >= 48 && character <= 57) ||
        character === 43 ||
        character === 47
      )
    )
      return false;
  }
  for (let index = contentLength; index < value.length; index += 1)
    if (value.charCodeAt(index) !== 61) return false;
  const bytes = (value.length / 4) * 3 - padding;
  return expectedBytes === undefined || bytes === expectedBytes;
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
