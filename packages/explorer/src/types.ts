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
  field_cells: number;
  field_bytes: number;
  elevation_scale: number;
  grid: {
    columns: number;
    rows: number;
    bounds: { x: number; y: number; width: number; height: number };
  };
  validity: { values: Uint8Array };
  elevation: { values: Float32Array };
  normal: { values: Float32Array | Int8Array };
  curvature: { values: Float32Array };
  material: {
    tint: Float32Array;
    occlusion: Float32Array;
    roughness: Float32Array;
  };
}
