export interface GlobeCameraView {
  yaw_degrees: number;
  pitch_degrees: number;
  projection_morph_progress?: number;
}

export interface TerrainCameraView {
  world_width: number;
  world_height: number;
  viewport_width: number;
  viewport_height: number;
  rendered_scale: number;
  pan_x: number;
  pan_y: number;
  pitch_degrees?: number;
  yaw_degrees?: number;
  model_transform?: {
    scale_x: number;
    scale_z: number;
    translate_x: number;
    translate_z: number;
    elevation_scale: number;
  };
}

export interface ExplorerGlobeRegion {
  id: string;
  cluster_id: string;
  focus_id: string;
  workload_id: string;
  label: string;
  detail: string;
  longitude_degrees: number;
  latitude_degrees: number;
  angular_radius_degrees: number;
  tone: string;
}

export interface ExplorerGlobeBeacon {
  id: string;
  focus_id: string;
  workload_id: string;
  label: string;
  detail: string;
  source: string;
  source_revision: string;
  producer: string;
  state: string;
  mapping_role: string;
  next_step: string;
  longitude_degrees: number;
  latitude_degrees: number;
  tone: string;
}

export interface ExplorerGlobeSector {
  id: string;
  label: string;
  west_degrees: number;
  south_degrees: number;
  east_degrees: number;
  north_degrees: number;
  crosses_antimeridian: boolean;
  tone: string;
}

export interface ExplorerGlobe {
  schema: string;
  posture: string;
  globe_id: string;
  source_revision: string;
  compiler_revision: string;
  coordinate_authority: string;
  clusters: readonly unknown[];
  regions: readonly ExplorerGlobeRegion[];
  beacons: readonly ExplorerGlobeBeacon[];
  sectors?: readonly ExplorerGlobeSector[];
}

export interface TerrainFieldSetInput {
  field_set_id: string;
  source_revision?: string;
  field_cells: number;
  field_bytes: number;
  elevation_scale: number;
  grid: {
    columns: number;
    rows: number;
    bounds: { x: number; y: number; width: number; height: number };
  };
  validity: { values: Uint8Array };
  /**
   * Preserves why a vertex is invalid. The binary validity mask remains the
   * geometry authority; this channel prevents source no-data from being
   * conflated with space for which no terrain source was admitted.
   */
  validity_classification?: {
    schema: "rey.terrain-validity-classification.v1";
    implementation_revision: string;
    values: Uint8Array;
  };
  elevation: { values: Float32Array };
  normal: { values: Float32Array | Int8Array };
  curvature: { values: Float32Array };
  material: {
    tint: Float32Array;
    occlusion: Float32Array;
    roughness: Float32Array;
  };
  relief_metrics?: {
    schema: "rey.terrain-relief-metrics.v1";
    sample_spacing_x_meters: number;
    sample_spacing_y_meters: number;
    elevation_range_meters: number;
    authority: string;
  };
  landscape_reference?: {
    schema: "rey.landscape-spatial-reference.v1";
    reference_id: string;
    coordinate_reference: string;
    vertical_reference: string;
  };
  landscape_mosaic?: {
    schema: "rey.landscape-mosaic-binding.v1";
    mosaic_id: string;
    composition_revision: string;
    primary_patch_id: string;
    patch_ids: readonly string[];
    overlap_pairs: readonly (readonly [string, string])[];
    bounds: { x: number; y: number; width: number; height: number };
    coordinate_reference: string;
    vertical_reference: string;
    overlap_policy:
      | "qualified_shared_samples_must_match_before_derivation"
      | "validity_authority_resolution_then_stable_identity";
    source_contribution_id?: string;
    conflict_id?: string;
    conflict_vertices?: number;
    gap_policy: "unsupported_remains_transparent";
  };
}

export type TerrainExecutablePassId =
  | "validity_background"
  | "base_terrain"
  | "height_normals_hillshade"
  | "ambient_valley_occlusion"
  | "contours"
  | "water_weather_boundary"
  | "features_labels_selection";

export interface TerrainExecutablePass {
  id: TerrainExecutablePassId;
  implementation_revision: string;
  input_revision: string;
  authority: "evidence" | "derived" | "presentation" | "interface";
}

export interface TerrainLineFeatureInput {
  id: string;
  pass_id: "contours" | "water_weather_boundary" | "features_labels_selection";
  kind: string;
  source_revision: string;
  authority: string;
  /** Non-indexed independent segment endpoint pairs in x/up/y order. */
  positions: Float32Array;
  color: number;
  opacity: number;
}

export interface TerrainAreaFeatureInput {
  id: string;
  pass_id: "water_weather_boundary";
  kind: string;
  source_revision: string;
  authority: string;
  /** Non-indexed terrain-draped triangle positions in x/up/y order. */
  positions: Float32Array;
  color: number;
  opacity: number;
}

export interface TerrainPointFeatureInput {
  id: string;
  pass_id: "features_labels_selection";
  kind: string;
  source_revision: string;
  authority: string;
  position: readonly [number, number, number];
  color: number;
  radius: number;
}

export interface TerrainRenderPassSetInput {
  schema: "rey.terrain-render-pass-set.v1";
  pass_set_id: string;
  bounds: { x: number; y: number; width: number; height: number };
  passes: readonly TerrainExecutablePass[];
  areas: readonly TerrainAreaFeatureInput[];
  lines: readonly TerrainLineFeatureInput[];
  points: readonly TerrainPointFeatureInput[];
  omissions: readonly string[];
}
