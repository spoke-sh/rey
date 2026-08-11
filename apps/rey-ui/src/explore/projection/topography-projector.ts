import type {
  ProjectionPacket,
  TopographyPatch,
  WorkloadList,
  WorkloadSummary,
} from "../../domain";

export interface AdmittedTopography {
  workload: WorkloadSummary;
  patch: TopographyPatch;
  projection: ProjectionPacket;
}

export function admittedTopographies(
  portfolio: WorkloadList,
): AdmittedTopography[] {
  return portfolio.workloads.flatMap((workload) => {
    const patch = workload.topography_patch;
    const projection = workload.topography_projection;
    return patch &&
      projection &&
      projection.schema === "rey.projection-packet.v1" &&
      projection.source_patch_id === patch.patch_id &&
      projection.source_topography_revision === patch.topography_revision
      ? [{ workload, patch, projection }]
      : [];
  });
}
