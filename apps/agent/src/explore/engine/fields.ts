export const TERRAIN_FIELD_SCHEMA = "rey.terrain-fields.v1" as const;

export interface FieldBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface FieldGrid {
  columns: number;
  rows: number;
  bounds: FieldBounds;
}

interface FieldIdentity {
  channel: string;
  implementation_revision: string;
  grid: FieldGrid;
}

export interface ScalarField2D extends FieldIdentity {
  kind: "scalar";
  values: Float32Array;
  minimum: number;
  maximum: number;
}

export interface VectorField2D extends FieldIdentity {
  kind: "vector";
  components: 2 | 3;
  values: Float32Array | Int8Array;
}

export interface MaskField2D extends FieldIdentity {
  kind: "mask";
  values: Uint8Array;
}

export interface MaterialField2D extends FieldIdentity {
  kind: "material";
  tint: Float32Array;
  occlusion: Float32Array;
  roughness: Float32Array;
}

export function createFieldGrid(
  columns: number,
  rows: number,
  bounds: FieldBounds,
): FieldGrid {
  if (!Number.isInteger(columns) || columns < 2)
    throw new Error("field grid requires at least two columns");
  if (!Number.isInteger(rows) || rows < 2)
    throw new Error("field grid requires at least two rows");
  if (
    !Number.isFinite(bounds.x) ||
    !Number.isFinite(bounds.y) ||
    !Number.isFinite(bounds.width) ||
    !Number.isFinite(bounds.height) ||
    bounds.width <= 0 ||
    bounds.height <= 0
  )
    throw new Error("field grid requires finite positive bounds");
  return Object.freeze({
    columns,
    rows,
    bounds: Object.freeze({ ...bounds }),
  });
}

export function fieldCellCount(grid: FieldGrid): number {
  return grid.columns * grid.rows;
}

export function fieldIndex(
  grid: FieldGrid,
  column: number,
  row: number,
): number {
  if (
    !Number.isInteger(column) ||
    !Number.isInteger(row) ||
    column < 0 ||
    column >= grid.columns ||
    row < 0 ||
    row >= grid.rows
  )
    throw new Error("field coordinate is outside the bounded grid");
  return row * grid.columns + column;
}

export function fieldPoint(
  grid: FieldGrid,
  column: number,
  row: number,
): { x: number; y: number } {
  fieldIndex(grid, column, row);
  return {
    x:
      grid.bounds.x +
      (column / Math.max(1, grid.columns - 1)) * grid.bounds.width,
    y: grid.bounds.y + (row / Math.max(1, grid.rows - 1)) * grid.bounds.height,
  };
}

export function scalarField(
  channel: string,
  implementationRevision: string,
  grid: FieldGrid,
  values: Float32Array,
): ScalarField2D {
  requireLength(channel, values.length, fieldCellCount(grid));
  let minimum = Number.POSITIVE_INFINITY;
  let maximum = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    if (!Number.isFinite(value))
      throw new Error(`${channel} contains a non-finite scalar`);
    minimum = Math.min(minimum, value);
    maximum = Math.max(maximum, value);
  }
  return Object.freeze({
    kind: "scalar",
    channel,
    implementation_revision: implementationRevision,
    grid,
    values,
    minimum,
    maximum,
  });
}

export function vectorField(
  channel: string,
  implementationRevision: string,
  grid: FieldGrid,
  components: 2 | 3,
  values: Float32Array | Int8Array,
): VectorField2D {
  requireLength(channel, values.length, fieldCellCount(grid) * components);
  if (values instanceof Float32Array)
    for (const value of values)
      if (!Number.isFinite(value))
        throw new Error(`${channel} contains a non-finite vector component`);
  return Object.freeze({
    kind: "vector",
    channel,
    implementation_revision: implementationRevision,
    grid,
    components,
    values,
  });
}

export function maskField(
  channel: string,
  implementationRevision: string,
  grid: FieldGrid,
  values: Uint8Array,
): MaskField2D {
  requireLength(channel, values.length, fieldCellCount(grid));
  return Object.freeze({
    kind: "mask",
    channel,
    implementation_revision: implementationRevision,
    grid,
    values,
  });
}

export function materialField(
  channel: string,
  implementationRevision: string,
  grid: FieldGrid,
  tint: Float32Array,
  occlusion: Float32Array,
  roughness: Float32Array,
): MaterialField2D {
  const cells = fieldCellCount(grid);
  requireLength(`${channel} tint`, tint.length, cells * 3);
  requireLength(`${channel} occlusion`, occlusion.length, cells);
  requireLength(`${channel} roughness`, roughness.length, cells);
  return Object.freeze({
    kind: "material",
    channel,
    implementation_revision: implementationRevision,
    grid,
    tint,
    occlusion,
    roughness,
  });
}

export function fieldByteLength(
  field: ScalarField2D | VectorField2D | MaskField2D | MaterialField2D,
): number {
  if (field.kind === "material")
    return (
      field.tint.byteLength +
      field.occlusion.byteLength +
      field.roughness.byteLength
    );
  return field.values.byteLength;
}

function requireLength(channel: string, actual: number, expected: number) {
  if (actual !== expected)
    throw new Error(
      `${channel} field length ${actual} does not match ${expected}`,
    );
}
