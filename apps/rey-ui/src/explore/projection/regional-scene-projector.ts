import type {
  AdmittedRegionalScene,
  ContractIdentity,
  SemanticAtlas,
  SemanticAtlasRegionalRegion,
  SceneAdmissionResult,
  WorkloadList,
  WorkloadSummary,
} from "../../domain";
import { compileCountyFrame, type CountyFrame } from "./county-frame";

export interface AdmittedRegionalProjection {
  workload: WorkloadSummary;
  result: SceneAdmissionResult;
  scene: AdmittedRegionalScene;
  atlas_region: SemanticAtlasRegionalRegion;
  atlas_sector: SemanticAtlas["sectors"][number];
  county_frame: CountyFrame;
}

const coordinateSpaces = [
  "native_crs84",
  "synthetic_semantic",
  "semantic_mercator",
  "county_local",
  "camera",
] as const;

export function admittedRegionalScenes(
  portfolio: WorkloadList,
): AdmittedRegionalProjection[] {
  return portfolio.workloads.flatMap((workload) => {
    const result = workload.latest_scene_admission;
    const scene = result?.scene;
    const atlas = portfolio.semantic_atlas;
    const retainedAtlas = portfolio.semantic_atlas_history.at(-1);
    const retainedDelta = portfolio.semantic_atlas_deltas.at(-1);
    const semanticPlacement = scene?.projection.transforms.find(
      (transform) =>
        transform.source_space === "native_crs84" &&
        transform.target_space === "synthetic_semantic",
    );
    const atlasSource = atlas?.regional_sources.find(
      (source) =>
        source.workload_id === workload.workload.id &&
        source.scene_region_id === scene?.region_id &&
        source.source_scene_id === scene?.scene_id &&
        source.source_admission_id === scene?.admission.admission_id &&
        source.source_package_id === scene?.admission.package_id &&
        source.source_package_revision ===
          scene?.admission.package_snapshot_revision &&
        source.projection_packet_id === scene?.projection.packet_id,
    );
    const atlasRegion = atlas?.regional_regions.find(
      (region) =>
        region.region_id === atlasSource?.region_id &&
        region.workload_id === atlasSource.workload_id &&
        region.scene_region_id === atlasSource.scene_region_id &&
        region.source_scene_id === atlasSource.source_scene_id &&
        region.source_admission_id === atlasSource.source_admission_id &&
        region.source_package_id === atlasSource.source_package_id &&
        region.source_package_revision ===
          atlasSource.source_package_revision &&
        region.projection_packet_id === atlasSource.projection_packet_id,
    );
    const atlasSector = atlas?.sectors.find(
      (sector) =>
        sector.sector_id === atlasRegion?.sector_id &&
        sector.member_region_ids.includes(atlasRegion.region_id),
    );
    const countyFrame = scene ? safeCountyFrame(scene) : null;
    if (
      !result ||
      result.schema !== "rey.scene-admission-result.v1" ||
      result.status !== "accepted" ||
      result.scenario !== null ||
      !scene ||
      scene.schema !== "rey.admitted-regional-scene.v1" ||
      scene.projection.schema !== "rey.regional-projection-packet.v1" ||
      !sameContract(result.workload, workload.workload) ||
      !sameContract(result.graph, workload.candidate_graph) ||
      !sameContract(scene.admission.workload, result.workload) ||
      !sameContract(scene.admission.graph, result.graph) ||
      scene.admission.capability_snapshot_id !==
        result.capability_snapshot_id ||
      scene.admission.package_id !== scene.projection.source_package_id ||
      scene.admission.package_snapshot_revision !==
        scene.projection.source_snapshot_revision ||
      scene.artifacts.projection_packet_id !== scene.projection.packet_id ||
      scene.artifacts.admitted_atlas_revision !== atlas?.atlas_revision ||
      scene.artifacts.terrain_program_id !==
        scene.projection.terrain_program_id ||
      scene.projection.coordinate_bindings.length !== coordinateSpaces.length ||
      scene.projection.coordinate_bindings.some(
        (binding, index) => binding.space !== coordinateSpaces[index],
      ) ||
      !semanticPlacement ||
      semanticPlacement.target_origin.length !== 2 ||
      !semanticPlacement.target_origin.every(Number.isFinite) ||
      !atlas ||
      retainedAtlas?.atlas_revision !== atlas.atlas_revision ||
      retainedDelta?.target_revision !== atlas.atlas_revision ||
      portfolio.semantic_atlas_history.length !==
        portfolio.semantic_atlas_deltas.length ||
      !atlasSource ||
      !atlasRegion ||
      !atlasSector ||
      !countyFrame ||
      atlasRegion.semantic_longitude_microdegrees !==
        semanticPlacement.target_origin[0] ||
      atlasRegion.semantic_latitude_microdegrees !==
        semanticPlacement.target_origin[1]
    )
      return [];
    return [
      {
        workload,
        result,
        scene,
        atlas_region: atlasRegion,
        atlas_sector: atlasSector,
        county_frame: countyFrame,
      },
    ];
  });
}

function safeCountyFrame(scene: AdmittedRegionalScene) {
  try {
    return compileCountyFrame(scene);
  } catch {
    return null;
  }
}

function sameContract(left: ContractIdentity, right: ContractIdentity) {
  return (
    left.id === right.id &&
    left.revision === right.revision &&
    left.semantic_digest === right.semantic_digest
  );
}
