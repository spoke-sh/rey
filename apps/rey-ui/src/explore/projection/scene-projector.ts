import type { SceneAdmission, WorkloadList } from "../../domain";

export function admittedScenes(portfolio: WorkloadList): SceneAdmission[] {
  return portfolio.scene_admissions.filter(
    (admission) =>
      admission.schema === "rey.scene-admission.v1" &&
      admission.admitted &&
      admission.status === "admitted" &&
      admission.validation.complete &&
      admission.validation.validator === "rey.scene-admission.validate@1" &&
      admission.validation.workload.id === "rey.scene-admission" &&
      admission.projection.complete &&
      admission.package_id === admission.validation.package_id &&
      admission.package_id === admission.projection.package_id &&
      admission.validation.snapshot_revision ===
        admission.projection.snapshot_revision,
  );
}
