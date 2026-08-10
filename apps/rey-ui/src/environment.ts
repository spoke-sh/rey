export type EnvironmentAvailability = "available" | "unavailable" | "error";
export type EnvironmentCapture = "presence" | "digest" | "value";
export type EnvironmentObjectChange =
  "unchanged" | "inserted" | "deleted" | "modified";

export interface EnvironmentPlaneChanges {
  head_to_index: EnvironmentObjectChange;
  index_to_working: EnvironmentObjectChange;
  head_to_working: EnvironmentObjectChange;
}

export interface EnvironmentObjectStatus<T> {
  object_id: string;
  head: T | null;
  index: T | null;
  working: T | null;
  changes: EnvironmentPlaneChanges;
}

export interface EnvironmentVariableObservation {
  name: string;
  sensitive: boolean;
  capture: EnvironmentCapture;
  availability: EnvironmentAvailability;
  value: string | null;
  value_digest: string | null;
  error_code: string | null;
}

export interface EnvironmentApplicationObservation {
  name: string;
  required: boolean;
  availability: EnvironmentAvailability;
  resolved_path: string | null;
  content_digest: string | null;
  potential_capabilities: string[];
  searched_path_count: number;
  error_code: string | null;
}

export interface EnvironmentInputObservation {
  path: string;
  required: boolean;
  availability: EnvironmentAvailability;
  content_digest: string | null;
  byte_length: number | null;
  error_code: string | null;
}

export interface EnvironmentReferenceObservation {
  from: string;
  to: string;
  relation: string;
}

export interface EnvironmentOperatorProjection {
  schema: "rey.environment-operator-projection.v1";
  source_label: string;
  target_label: "WORKING";
  complete: boolean;
  mapping: {
    source_path: string;
    schema: string;
    graph_id: string;
  } | null;
  summary: {
    variables: number;
    changed_variables: number;
    applications_searched: number;
    applications_found: number;
    applications_not_found: number;
    application_errors: number;
    changed_applications: number;
    inputs: number;
    changed_inputs: number;
    references: number;
  };
  variables: EnvironmentObjectStatus<EnvironmentVariableObservation>[];
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[];
  inputs: EnvironmentObjectStatus<EnvironmentInputObservation>[];
  references: EnvironmentObjectStatus<EnvironmentReferenceObservation>[];
}

export interface EnvironmentStatus {
  schema: "rey.environment-status.v3";
  head_commit_id: string | null;
  head_sequence: number | null;
  head_snapshot_id: string | null;
  state: "unborn" | "clean" | "changed" | "staged" | "mixed" | "inconclusive";
  operator: EnvironmentOperatorProjection;
  staged_delta: { changes: unknown[] };
  unstaged_delta: { changes: unknown[] };
}

export interface EnvironmentDiffLine {
  key: string;
  kind: "context" | "inserted" | "deleted";
  text: string;
  admission: "clean" | "staged" | "working" | "mixed";
}

export function environmentVariableDiff(
  variables: EnvironmentObjectStatus<EnvironmentVariableObservation>[],
): EnvironmentDiffLine[] {
  return variables.flatMap((variable) => {
    const admission = admissionState(variable.changes);
    switch (variable.changes.head_to_working) {
      case "unchanged": {
        const observation = variable.working ?? variable.head;
        return observation
          ? [line(variable.object_id, "context", observation, admission)]
          : [];
      }
      case "inserted":
        return variable.working
          ? [line(variable.object_id, "inserted", variable.working, admission)]
          : [];
      case "deleted":
        return variable.head
          ? [line(variable.object_id, "deleted", variable.head, admission)]
          : [];
      case "modified":
        return [
          ...(variable.head
            ? [line(variable.object_id, "deleted", variable.head, admission)]
            : []),
          ...(variable.working
            ? [
                line(
                  variable.object_id,
                  "inserted",
                  variable.working,
                  admission,
                ),
              ]
            : []),
        ];
    }
  });
}

export function currentApplications(
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[],
  availability: EnvironmentAvailability,
): EnvironmentObjectStatus<EnvironmentApplicationObservation>[] {
  return applications.filter(
    (application) => application.working?.availability === availability,
  );
}

export function admissionState(
  changes: EnvironmentPlaneChanges,
): EnvironmentDiffLine["admission"] {
  const staged = changes.head_to_index !== "unchanged";
  const working = changes.index_to_working !== "unchanged";
  if (staged && working) return "mixed";
  if (staged) return "staged";
  if (working) return "working";
  return "clean";
}

function line(
  objectId: string,
  kind: EnvironmentDiffLine["kind"],
  observation: EnvironmentVariableObservation,
  admission: EnvironmentDiffLine["admission"],
): EnvironmentDiffLine {
  return {
    key: `${objectId}:${kind}:${formatVariable(observation)}`,
    kind,
    text: formatVariable(observation),
    admission,
  };
}

function formatVariable(observation: EnvironmentVariableObservation): string {
  if (observation.availability === "unavailable") {
    return `${observation.name}=<unset>`;
  }
  if (observation.availability === "error") {
    return `${observation.name}=<error:${observation.error_code ?? "observation_failed"}>`;
  }
  if (observation.capture === "value") {
    return `${observation.name}=${
      observation.value === null
        ? legacyValue(observation)
        : escapeValue(observation.value)
    }`;
  }
  if (observation.capture === "digest") {
    return `${observation.name}=<digest:${shortValue(observation.value_digest ?? "present")}>`;
  }
  return `${observation.name}=<${
    observation.sensitive ? "present:redacted" : "present"
  }>`;
}

function legacyValue(observation: EnvironmentVariableObservation): string {
  return observation.value_digest
    ? `<legacy-digest:${shortValue(observation.value_digest)}>`
    : "<present>";
}

function escapeValue(value: string): string {
  return JSON.stringify(value).slice(1, -1);
}

function shortValue(value: string): string {
  return value.length <= 22
    ? value
    : `${value.slice(0, 12)}…${value.slice(-6)}`;
}
