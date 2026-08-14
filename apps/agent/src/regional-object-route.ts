export function regionalObjectEvidenceRoute(
  workloadId: string,
  sceneId: string,
  objectRevision: string,
): string {
  return `/workloads/${encodeURIComponent(workloadId)}/scenes/${encodeURIComponent(sceneId)}/objects/${encodeURIComponent(objectRevision)}`;
}
