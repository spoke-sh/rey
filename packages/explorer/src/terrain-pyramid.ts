import { blake3 } from "@noble/hashes/blake3.js";
import {
  landscapeReliefFieldByteLength,
  type LandscapeReliefField,
} from "./landscape-relief";
import { summarizeTerrainFieldValidity } from "./terrain-validity";
import type { TerrainFieldSetInput } from "./types";

export const LANDSCAPE_HEIGHT_PYRAMID_SCHEMA =
  "rey.landscape-height-pyramid.v1" as const;
export const LANDSCAPE_RELIEF_PYRAMID_SCHEMA =
  "rey.landscape-relief-pyramid.v1" as const;
export const LANDSCAPE_PYRAMID_ENVELOPE_SCHEMA =
  "rey.landscape-pyramid-envelope.v1" as const;
export const LANDSCAPE_HEIGHT_PYRAMID_CONTRACT_REVISION =
  "rey.landscape-height-pyramid-contract@1" as const;
export const LANDSCAPE_RELIEF_PYRAMID_CONTRACT_REVISION =
  "rey.landscape-relief-pyramid-contract@1" as const;
export const LANDSCAPE_PYRAMID_ENVELOPE_REVISION =
  "rey.landscape-pyramid-envelope@1" as const;

export interface LandscapePyramidBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface LandscapePyramidLineage {
  kind: string;
  identity: string;
  revision: string;
}

export interface LandscapePyramidValidity {
  validity_id: string;
  valid_vertices: number;
  no_data_vertices: number;
  unsupported_vertices: number;
  policy: "conservative_support_only";
}

export interface LandscapeHeightPyramidLevel {
  level_id: string;
  height_id: string;
  level: number;
  implementation_revision: string;
  parent_level_id: string | null;
  child_level_id: string | null;
  sample_spacing_x_meters: number;
  sample_spacing_y_meters: number;
  columns: number;
  rows: number;
  bounds: LandscapePyramidBounds;
  validity: LandscapePyramidValidity;
  elevation_minimum_meters: number;
  elevation_maximum_meters: number;
  height_bytes: number;
  validity_bytes: number;
  source_lineage: readonly LandscapePyramidLineage[];
}

export interface LandscapeHeightPyramid {
  schema: typeof LANDSCAPE_HEIGHT_PYRAMID_SCHEMA;
  implementation_revision: string;
  pyramid_id: string;
  mosaic_id: string;
  coordinate_reference: string;
  vertical_reference: string;
  levels: readonly LandscapeHeightPyramidLevel[];
  byte_length: number;
  complete: boolean;
  omissions: readonly string[];
}

export interface LandscapeReliefOperatorSupport {
  support_id: string;
  operator_id: string;
  implementation_revision: string;
  target_radius_meters: number;
  support_radius_cells: number;
  support_radius_meters: number;
  gutter_radius_cells: number;
  supported: boolean;
  validity_policy: "complete_valid_window";
}

export interface LandscapeReliefPyramidLevel {
  level_id: string;
  level: number;
  implementation_revision: string;
  parent_level_id: string | null;
  child_level_id: string | null;
  source_height_level_id: string;
  sample_spacing_x_meters: number;
  sample_spacing_y_meters: number;
  columns: number;
  rows: number;
  bounds: LandscapePyramidBounds;
  validity: LandscapePyramidValidity;
  channel_ids: readonly string[];
  operator_support: readonly LandscapeReliefOperatorSupport[];
  relief_bytes: number;
  source_lineage: readonly LandscapePyramidLineage[];
}

export interface LandscapeReliefPyramid {
  schema: typeof LANDSCAPE_RELIEF_PYRAMID_SCHEMA;
  implementation_revision: string;
  pyramid_id: string;
  mosaic_id: string;
  source_height_pyramid_id: string;
  coordinate_reference: string;
  vertical_reference: string;
  levels: readonly LandscapeReliefPyramidLevel[];
  byte_length: number;
  complete: boolean;
  omissions: readonly string[];
}

export interface LandscapePyramidEnvelope {
  schema: typeof LANDSCAPE_PYRAMID_ENVELOPE_SCHEMA;
  implementation_revision: typeof LANDSCAPE_PYRAMID_ENVELOPE_REVISION;
  envelope_id: string;
  field_set_id: string;
  source_revision: string;
  height_pyramid: LandscapeHeightPyramid;
  relief_pyramid: LandscapeReliefPyramid;
}

export type LandscapeHeightPyramidLevelInput = Omit<
  LandscapeHeightPyramidLevel,
  "level_id" | "parent_level_id" | "child_level_id"
>;

export type LandscapeHeightPyramidInput = Omit<
  LandscapeHeightPyramid,
  "schema" | "pyramid_id" | "levels" | "byte_length"
> & {
  levels: readonly LandscapeHeightPyramidLevelInput[];
};

export type LandscapeReliefOperatorSupportInput = Omit<
  LandscapeReliefOperatorSupport,
  "support_id"
>;

export type LandscapeReliefPyramidLevelInput = Omit<
  LandscapeReliefPyramidLevel,
  "level_id" | "parent_level_id" | "child_level_id" | "operator_support"
> & {
  operator_support: readonly LandscapeReliefOperatorSupportInput[];
};

export type LandscapeReliefPyramidInput = Omit<
  LandscapeReliefPyramid,
  "schema" | "pyramid_id" | "levels" | "byte_length"
> & {
  levels: readonly LandscapeReliefPyramidLevelInput[];
};

export function finalizeLandscapePyramidEnvelope(
  fieldSetId: string,
  sourceRevision: string,
  heightPyramid: LandscapeHeightPyramid,
  reliefPyramid: LandscapeReliefPyramid,
): LandscapePyramidEnvelope {
  verifyLandscapeHeightPyramid(heightPyramid);
  verifyLandscapeReliefPyramid(reliefPyramid, heightPyramid);
  const identity = {
    schema: LANDSCAPE_PYRAMID_ENVELOPE_SCHEMA,
    implementation_revision: LANDSCAPE_PYRAMID_ENVELOPE_REVISION,
    field_set_id: fieldSetId,
    source_revision: sourceRevision,
    height_pyramid_id: heightPyramid.pyramid_id,
    relief_pyramid_id: reliefPyramid.pyramid_id,
  };
  const result = Object.freeze({
    schema: LANDSCAPE_PYRAMID_ENVELOPE_SCHEMA,
    implementation_revision: LANDSCAPE_PYRAMID_ENVELOPE_REVISION,
    envelope_id: semanticDigest(identity),
    field_set_id: fieldSetId,
    source_revision: sourceRevision,
    height_pyramid: heightPyramid,
    relief_pyramid: reliefPyramid,
  });
  verifyLandscapePyramidEnvelope(result);
  return result;
}

export function verifyLandscapePyramidEnvelope(
  envelope: LandscapePyramidEnvelope,
  field?: TerrainFieldSetInput,
  relief?: LandscapeReliefField,
): void {
  verifyLandscapeHeightPyramid(envelope.height_pyramid);
  verifyLandscapeReliefPyramid(
    envelope.relief_pyramid,
    envelope.height_pyramid,
  );
  const expected = semanticDigest({
    schema: envelope.schema,
    implementation_revision: envelope.implementation_revision,
    field_set_id: envelope.field_set_id,
    source_revision: envelope.source_revision,
    height_pyramid_id: envelope.height_pyramid.pyramid_id,
    relief_pyramid_id: envelope.relief_pyramid.pyramid_id,
  });
  if (
    envelope.schema !== LANDSCAPE_PYRAMID_ENVELOPE_SCHEMA ||
    envelope.implementation_revision !== LANDSCAPE_PYRAMID_ENVELOPE_REVISION ||
    !envelope.field_set_id ||
    !envelope.source_revision ||
    envelope.envelope_id !== expected
  )
    throw new Error("landscape pyramid envelope identity changed");
  if (!field && !relief) return;
  if (!field || !relief)
    throw new Error("landscape pyramid envelope binding is incomplete");
  verifyLandscapePyramidFieldBinding(envelope, field, relief);
}

export function landscapePyramidContentId(
  channel: string,
  arrays: readonly (Float32Array | Int8Array | Uint8Array)[],
): string {
  if (!channel || arrays.length === 0)
    throw new Error("landscape pyramid content identity is incomplete");
  const header = new TextEncoder().encode(
    JSON.stringify({
      channel,
      byte_lengths: arrays.map(({ byteLength }) => byteLength),
    }),
  );
  const bytes = new Uint8Array(
    header.length +
      arrays.reduce((total, array) => total + array.byteLength, 0),
  );
  bytes.set(header);
  let offset = header.length;
  for (const array of arrays) {
    const content = new Uint8Array(
      array.buffer,
      array.byteOffset,
      array.byteLength,
    );
    bytes.set(content, offset);
    offset += content.length;
  }
  return `blake3:${hex(blake3(bytes))}`;
}

export function finalizeLandscapeHeightPyramid(
  input: LandscapeHeightPyramidInput,
): LandscapeHeightPyramid {
  const levels = [...input.levels]
    .sort((left, right) => left.level - right.level)
    .map((level) => normalizeHeightLevel(level));
  const omissions = canonicalStrings(input.omissions);
  const identityInput = {
    schema: LANDSCAPE_HEIGHT_PYRAMID_SCHEMA,
    implementation_revision: input.implementation_revision,
    mosaic_id: input.mosaic_id,
    coordinate_reference: input.coordinate_reference,
    vertical_reference: input.vertical_reference,
    levels,
    complete: input.complete,
    omissions,
  };
  const pyramidId = semanticDigest(identityInput);
  const linked = linkHeightLevels(pyramidId, levels);
  const result = Object.freeze({
    schema: LANDSCAPE_HEIGHT_PYRAMID_SCHEMA,
    implementation_revision: input.implementation_revision,
    pyramid_id: pyramidId,
    mosaic_id: input.mosaic_id,
    coordinate_reference: input.coordinate_reference,
    vertical_reference: input.vertical_reference,
    levels: linked,
    byte_length: linked.reduce(
      (total, level) => total + level.height_bytes + level.validity_bytes,
      0,
    ),
    complete: input.complete,
    omissions,
  });
  verifyLandscapeHeightPyramid(result);
  return result;
}

export function finalizeLandscapeReliefPyramid(
  input: LandscapeReliefPyramidInput,
  heightPyramid: LandscapeHeightPyramid,
): LandscapeReliefPyramid {
  verifyLandscapeHeightPyramid(heightPyramid);
  const levels = [...input.levels]
    .sort((left, right) => left.level - right.level)
    .map((level) => normalizeReliefLevel(level));
  const omissions = canonicalStrings(input.omissions);
  const identityInput = {
    schema: LANDSCAPE_RELIEF_PYRAMID_SCHEMA,
    implementation_revision: input.implementation_revision,
    mosaic_id: input.mosaic_id,
    source_height_pyramid_id: input.source_height_pyramid_id,
    coordinate_reference: input.coordinate_reference,
    vertical_reference: input.vertical_reference,
    levels,
    complete: input.complete,
    omissions,
  };
  const pyramidId = semanticDigest(identityInput);
  const linked = linkReliefLevels(pyramidId, levels);
  const result = Object.freeze({
    schema: LANDSCAPE_RELIEF_PYRAMID_SCHEMA,
    implementation_revision: input.implementation_revision,
    pyramid_id: pyramidId,
    mosaic_id: input.mosaic_id,
    source_height_pyramid_id: input.source_height_pyramid_id,
    coordinate_reference: input.coordinate_reference,
    vertical_reference: input.vertical_reference,
    levels: linked,
    byte_length: linked.reduce((total, level) => total + level.relief_bytes, 0),
    complete: input.complete,
    omissions,
  });
  verifyLandscapeReliefPyramid(result, heightPyramid);
  return result;
}

export function verifyLandscapeHeightPyramid(
  pyramid: LandscapeHeightPyramid,
): void {
  if (
    pyramid.schema !== LANDSCAPE_HEIGHT_PYRAMID_SCHEMA ||
    !pyramid.implementation_revision ||
    !pyramid.mosaic_id ||
    !pyramid.coordinate_reference ||
    !pyramid.vertical_reference ||
    pyramid.levels.length === 0 ||
    (pyramid.complete && pyramid.omissions.length > 0) ||
    !canonicalStringOrder(pyramid.omissions)
  )
    throw new Error("landscape height pyramid contract is incomplete");
  verifyLevelSequence(pyramid.pyramid_id, pyramid.levels);
  for (const [index, level] of pyramid.levels.entries()) {
    verifyLevelGeometry(level);
    verifyValidity(level.validity, level.columns * level.rows);
    verifyLineage(level.source_lineage);
    if (
      !contentId(level.height_id) ||
      !level.implementation_revision ||
      !Number.isFinite(level.elevation_minimum_meters) ||
      !Number.isFinite(level.elevation_maximum_meters) ||
      level.elevation_minimum_meters > level.elevation_maximum_meters ||
      !validByteLength(level.height_bytes) ||
      !validByteLength(level.validity_bytes)
    )
      throw new Error("landscape height pyramid level is invalid");
    const previous = pyramid.levels[index - 1];
    if (previous) verifyRefinement(previous, level);
  }
  const byteLength = pyramid.levels.reduce(
    (total, level) => total + level.height_bytes + level.validity_bytes,
    0,
  );
  const expected = semanticDigest(heightIdentityInput(pyramid));
  if (pyramid.byte_length !== byteLength || pyramid.pyramid_id !== expected)
    throw new Error("landscape height pyramid identity changed");
}

export function verifyLandscapeReliefPyramid(
  pyramid: LandscapeReliefPyramid,
  heightPyramid: LandscapeHeightPyramid,
): void {
  verifyLandscapeHeightPyramid(heightPyramid);
  if (
    pyramid.schema !== LANDSCAPE_RELIEF_PYRAMID_SCHEMA ||
    !pyramid.implementation_revision ||
    pyramid.mosaic_id !== heightPyramid.mosaic_id ||
    pyramid.source_height_pyramid_id !== heightPyramid.pyramid_id ||
    pyramid.coordinate_reference !== heightPyramid.coordinate_reference ||
    pyramid.vertical_reference !== heightPyramid.vertical_reference ||
    pyramid.levels.length !== heightPyramid.levels.length ||
    (pyramid.complete && pyramid.omissions.length > 0) ||
    !canonicalStringOrder(pyramid.omissions)
  )
    throw new Error("landscape relief pyramid contract is incomplete");
  verifyLevelSequence(pyramid.pyramid_id, pyramid.levels);
  for (const [index, level] of pyramid.levels.entries()) {
    const heightLevel = heightPyramid.levels[index]!;
    verifyLevelGeometry(level);
    verifyValidity(level.validity, level.columns * level.rows);
    verifyLineage(level.source_lineage);
    if (
      !level.implementation_revision ||
      level.source_height_level_id !== heightLevel.level_id ||
      level.sample_spacing_x_meters !== heightLevel.sample_spacing_x_meters ||
      level.sample_spacing_y_meters !== heightLevel.sample_spacing_y_meters ||
      level.columns !== heightLevel.columns ||
      level.rows !== heightLevel.rows ||
      JSON.stringify(level.bounds) !== JSON.stringify(heightLevel.bounds) ||
      JSON.stringify(level.validity) !== JSON.stringify(heightLevel.validity) ||
      !validByteLength(level.relief_bytes) ||
      level.channel_ids.length === 0 ||
      !canonicalStringOrder(level.channel_ids) ||
      level.channel_ids.some((channelId) => !channelContentId(channelId)) ||
      level.operator_support.length === 0
    )
      throw new Error("landscape relief pyramid level is invalid");
    verifyOperatorSupport(level.operator_support);
  }
  const byteLength = pyramid.levels.reduce(
    (total, level) => total + level.relief_bytes,
    0,
  );
  const expected = semanticDigest(reliefIdentityInput(pyramid));
  if (pyramid.byte_length !== byteLength || pyramid.pyramid_id !== expected)
    throw new Error("landscape relief pyramid identity changed");
}

function verifyLandscapePyramidFieldBinding(
  envelope: LandscapePyramidEnvelope,
  field: TerrainFieldSetInput,
  relief: LandscapeReliefField,
): void {
  const heightLevel = envelope.height_pyramid.levels.at(-1)!;
  const reliefLevel = envelope.relief_pyramid.levels.at(-1)!;
  const validity = summarizeTerrainFieldValidity(field);
  const expectedReliefChannels = canonicalStrings([
    `hillshade:${landscapePyramidContentId("hillshade", [relief.hillshade])}`,
    `salience:${landscapePyramidContentId("salience", [relief.salience])}`,
    `tangent:${landscapePyramidContentId("tangent", [relief.tangent])}`,
  ]);
  if (
    envelope.field_set_id !== field.field_set_id ||
    envelope.source_revision !== field.source_revision ||
    relief.field_set_id !== field.field_set_id ||
    relief.source_field_set_id !== field.field_set_id ||
    heightLevel.columns !== field.grid.columns ||
    heightLevel.rows !== field.grid.rows ||
    JSON.stringify(heightLevel.bounds) !== JSON.stringify(field.grid.bounds) ||
    JSON.stringify(heightLevel.validity) !==
      JSON.stringify({ ...validity, policy: "conservative_support_only" }) ||
    heightLevel.height_id !==
      landscapePyramidContentId("height", [field.elevation.values]) ||
    heightLevel.height_bytes !== field.elevation.values.byteLength ||
    heightLevel.validity_bytes !==
      field.validity_classification?.values.byteLength ||
    reliefLevel.relief_bytes !== landscapeReliefFieldByteLength(relief) ||
    JSON.stringify(reliefLevel.channel_ids) !==
      JSON.stringify(expectedReliefChannels)
  )
    throw new Error("landscape pyramid envelope diverges from its field");
}

function normalizeHeightLevel(
  level: LandscapeHeightPyramidLevelInput,
): LandscapeHeightPyramidLevelInput {
  return Object.freeze({
    ...level,
    bounds: Object.freeze({ ...level.bounds }),
    validity: Object.freeze({ ...level.validity }),
    source_lineage: canonicalLineage(level.source_lineage),
  });
}

function normalizeReliefLevel(
  level: LandscapeReliefPyramidLevelInput,
): LandscapeReliefPyramidLevelInput & {
  operator_support: readonly LandscapeReliefOperatorSupport[];
} {
  const operatorSupport = [...level.operator_support]
    .sort((left, right) => left.operator_id.localeCompare(right.operator_id))
    .map((support) =>
      Object.freeze({
        ...support,
        support_id: semanticDigest(support),
      }),
    );
  return Object.freeze({
    ...level,
    bounds: Object.freeze({ ...level.bounds }),
    validity: Object.freeze({ ...level.validity }),
    channel_ids: canonicalStrings(level.channel_ids),
    operator_support: Object.freeze(operatorSupport),
    source_lineage: canonicalLineage(level.source_lineage),
  });
}

function linkHeightLevels(
  pyramidId: string,
  levels: readonly LandscapeHeightPyramidLevelInput[],
): readonly LandscapeHeightPyramidLevel[] {
  const ids = levels.map(({ level }) => `${pyramidId}:level:${level}`);
  return Object.freeze(
    levels.map((level, index) =>
      Object.freeze({
        ...level,
        level_id: ids[index]!,
        parent_level_id: ids[index - 1] ?? null,
        child_level_id: ids[index + 1] ?? null,
      }),
    ),
  );
}

function linkReliefLevels(
  pyramidId: string,
  levels: readonly ReturnType<typeof normalizeReliefLevel>[],
): readonly LandscapeReliefPyramidLevel[] {
  const ids = levels.map(({ level }) => `${pyramidId}:level:${level}`);
  return Object.freeze(
    levels.map((level, index) =>
      Object.freeze({
        ...level,
        level_id: ids[index]!,
        parent_level_id: ids[index - 1] ?? null,
        child_level_id: ids[index + 1] ?? null,
      }),
    ),
  );
}

function verifyLevelSequence(
  pyramidId: string,
  levels: readonly {
    level_id: string;
    level: number;
    parent_level_id: string | null;
    child_level_id: string | null;
  }[],
): void {
  for (const [index, level] of levels.entries()) {
    const expectedId = `${pyramidId}:level:${index}`;
    if (
      level.level !== index ||
      level.level_id !== expectedId ||
      level.parent_level_id !==
        (index === 0 ? null : `${pyramidId}:level:${index - 1}`) ||
      level.child_level_id !==
        (index === levels.length - 1 ? null : `${pyramidId}:level:${index + 1}`)
    )
      throw new Error("landscape pyramid parent/child identity changed");
  }
}

function verifyLevelGeometry(level: {
  sample_spacing_x_meters: number;
  sample_spacing_y_meters: number;
  columns: number;
  rows: number;
  bounds: LandscapePyramidBounds;
}): void {
  if (
    !Number.isFinite(level.sample_spacing_x_meters) ||
    level.sample_spacing_x_meters <= 0 ||
    !Number.isFinite(level.sample_spacing_y_meters) ||
    level.sample_spacing_y_meters <= 0 ||
    !Number.isSafeInteger(level.columns) ||
    level.columns < 2 ||
    !Number.isSafeInteger(level.rows) ||
    level.rows < 2 ||
    !Number.isFinite(level.bounds.x) ||
    !Number.isFinite(level.bounds.y) ||
    !Number.isFinite(level.bounds.width) ||
    level.bounds.width <= 0 ||
    !Number.isFinite(level.bounds.height) ||
    level.bounds.height <= 0
  )
    throw new Error("landscape pyramid metric geometry is invalid");
}

function verifyRefinement(
  parent: LandscapeHeightPyramidLevel,
  child: LandscapeHeightPyramidLevel,
): void {
  if (
    child.sample_spacing_x_meters > parent.sample_spacing_x_meters ||
    child.sample_spacing_y_meters > parent.sample_spacing_y_meters ||
    child.columns < parent.columns ||
    child.rows < parent.rows ||
    JSON.stringify(child.bounds) !== JSON.stringify(parent.bounds)
  )
    throw new Error("landscape height pyramid refinement is invalid");
}

function verifyValidity(
  validity: LandscapePyramidValidity,
  vertices: number,
): void {
  if (
    !contentId(validity.validity_id) ||
    validity.policy !== "conservative_support_only" ||
    !validCount(validity.valid_vertices) ||
    !validCount(validity.no_data_vertices) ||
    !validCount(validity.unsupported_vertices) ||
    validity.valid_vertices +
      validity.no_data_vertices +
      validity.unsupported_vertices !==
      vertices
  )
    throw new Error("landscape pyramid validity contract is invalid");
}

function verifyLineage(lineage: readonly LandscapePyramidLineage[]): void {
  if (
    lineage.length === 0 ||
    lineage.some(({ kind, identity, revision }) =>
      [kind, identity, revision].some((value) => !value),
    ) ||
    JSON.stringify(lineage) !== JSON.stringify(canonicalLineage(lineage))
  )
    throw new Error("landscape pyramid lineage is invalid");
}

function verifyOperatorSupport(
  supports: readonly LandscapeReliefOperatorSupport[],
): void {
  if (
    supports.some(
      (support, index) =>
        !support.operator_id ||
        !support.implementation_revision ||
        support.validity_policy !== "complete_valid_window" ||
        !Number.isFinite(support.target_radius_meters) ||
        support.target_radius_meters <= 0 ||
        !Number.isSafeInteger(support.support_radius_cells) ||
        support.support_radius_cells < 1 ||
        !Number.isFinite(support.support_radius_meters) ||
        support.support_radius_meters <= 0 ||
        !Number.isSafeInteger(support.gutter_radius_cells) ||
        support.gutter_radius_cells < 0 ||
        (support.supported &&
          support.gutter_radius_cells < support.support_radius_cells) ||
        support.support_id !== semanticDigest(operatorIdentityInput(support)) ||
        (index > 0 && supports[index - 1]!.operator_id >= support.operator_id),
    )
  )
    throw new Error("landscape relief operator support is invalid");
}

function heightIdentityInput(pyramid: LandscapeHeightPyramid) {
  return {
    schema: pyramid.schema,
    implementation_revision: pyramid.implementation_revision,
    mosaic_id: pyramid.mosaic_id,
    coordinate_reference: pyramid.coordinate_reference,
    vertical_reference: pyramid.vertical_reference,
    levels: pyramid.levels.map(stripLevelLinks),
    complete: pyramid.complete,
    omissions: pyramid.omissions,
  };
}

function reliefIdentityInput(pyramid: LandscapeReliefPyramid) {
  return {
    schema: pyramid.schema,
    implementation_revision: pyramid.implementation_revision,
    mosaic_id: pyramid.mosaic_id,
    source_height_pyramid_id: pyramid.source_height_pyramid_id,
    coordinate_reference: pyramid.coordinate_reference,
    vertical_reference: pyramid.vertical_reference,
    levels: pyramid.levels.map(stripLevelLinks),
    complete: pyramid.complete,
    omissions: pyramid.omissions,
  };
}

function stripLevelLinks<
  Level extends {
    level_id: string;
    parent_level_id: string | null;
    child_level_id: string | null;
  },
>(
  level: Level,
): Omit<Level, "level_id" | "parent_level_id" | "child_level_id"> {
  const {
    level_id: _levelId,
    parent_level_id: _parentLevelId,
    child_level_id: _childLevelId,
    ...identity
  } = level;
  return identity;
}

function operatorIdentityInput(support: LandscapeReliefOperatorSupport) {
  const { support_id: _supportId, ...identity } = support;
  return identity;
}

function canonicalLineage(
  lineage: readonly LandscapePyramidLineage[],
): readonly LandscapePyramidLineage[] {
  return Object.freeze(
    lineage
      .map((entry) => Object.freeze({ ...entry }))
      .sort(
        (left, right) =>
          left.kind.localeCompare(right.kind) ||
          left.identity.localeCompare(right.identity) ||
          left.revision.localeCompare(right.revision),
      ),
  );
}

function canonicalStrings(values: readonly string[]): readonly string[] {
  return Object.freeze(
    [...new Set(values)].sort((left, right) => left.localeCompare(right)),
  );
}

function canonicalStringOrder(values: readonly string[]): boolean {
  return JSON.stringify(values) === JSON.stringify(canonicalStrings(values));
}

function validCount(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0;
}

function validByteLength(value: number): boolean {
  return Number.isSafeInteger(value) && value > 0;
}

function contentId(value: string): boolean {
  return /^blake3:[0-9a-f]{64}$/.test(value);
}

function channelContentId(value: string): boolean {
  const separator = value.indexOf(":blake3:");
  return separator > 0 && contentId(value.slice(separator + 1));
}

function semanticDigest(value: unknown): string {
  return `blake3:${hex(blake3(new TextEncoder().encode(JSON.stringify(value))))}`;
}

function hex(bytes: Uint8Array): string {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}
