import type {
  AdmittedRegionalScene,
  ContractIdentity,
  SceneAdmissionResult,
  WorkloadList,
  WorkloadSummary,
} from "../../domain";

export interface AdmittedRegionalProjection {
  workload: WorkloadSummary;
  result: SceneAdmissionResult;
  scene: AdmittedRegionalScene;
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
    const semanticPlacement = scene?.projection.transforms.find(
      (transform) =>
        transform.source_space === "native_crs84" &&
        transform.target_space === "synthetic_semantic",
    );
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
      scene.artifacts.terrain_program_id !==
        scene.projection.terrain_program_id ||
      scene.projection.coordinate_bindings.length !== coordinateSpaces.length ||
      scene.projection.coordinate_bindings.some(
        (binding, index) => binding.space !== coordinateSpaces[index],
      ) ||
      !semanticPlacement ||
      semanticPlacement.target_origin.length !== 2 ||
      !semanticPlacement.target_origin.every(Number.isFinite)
    )
      return [];
    return [{ workload, result, scene }];
  });
}

function sameContract(left: ContractIdentity, right: ContractIdentity) {
  return (
    left.id === right.id &&
    left.revision === right.revision &&
    left.semantic_digest === right.semantic_digest
  );
}
