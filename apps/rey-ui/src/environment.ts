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
  groups: string[];
  purpose: string | null;
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
  schema: "rey.environment-operator-projection.v2";
  source_label: string;
  target_label: "WORKING";
  complete: boolean;
  mapping: {
    source_path: string;
    schema: string;
    graph_id: string;
  } | null;
  application_inventory: {
    head: EnvironmentApplicationInventoryCoordinate | null;
    index: EnvironmentApplicationInventoryCoordinate | null;
    working: EnvironmentApplicationInventoryCoordinate | null;
  };
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

export interface EnvironmentApplicationInventoryCoordinate {
  schema: "rey.environment-application-inventory.v2";
  source_path: string;
  inventory_id: string;
}

export interface EnvironmentStatus {
  schema: "rey.environment-status.v2";
  head_commit_id: string | null;
  head_sequence: number | null;
  head_snapshot_id: string | null;
  state: "unborn" | "clean" | "changed" | "staged" | "mixed" | "inconclusive";
  working_snapshot: {
    semantic_digest: string;
    complete: boolean;
    profile: string;
  };
  operator: EnvironmentOperatorProjection;
  staged_delta: { changes: unknown[] };
  unstaged_delta: { changes: unknown[] };
  ignored: ReyIgnoreProjection | null;
}

export interface ReyIgnoreProjection {
  schema: "rey.ignore.v1";
  source: string;
  source_digest: string;
  rules: { kind: string; pattern: string; source_line: number }[];
  omissions: {
    rule: { kind: string; pattern: string; source_line: number };
    matched: number;
  }[];
  ignored: number;
}

export interface EnvironmentDiffLine {
  key: string;
  kind: "context" | "inserted" | "deleted";
  text: string;
  admission: "clean" | "staged" | "working" | "mixed";
}

export interface EnvironmentApplicationDiffLine {
  key: string;
  kind: EnvironmentDiffLine["kind"];
  observation: EnvironmentApplicationObservation;
  admission: EnvironmentDiffLine["admission"];
}

interface EnvironmentObservationDiffLine<T> {
  objectId: string;
  kind: EnvironmentDiffLine["kind"];
  observation: T;
  admission: EnvironmentDiffLine["admission"];
}

export function environmentVariableDiff(
  variables: EnvironmentObjectStatus<EnvironmentVariableObservation>[],
): EnvironmentDiffLine[] {
  return environmentObservationDiff(variables).map((entry) =>
    line(entry.objectId, entry.kind, entry.observation, entry.admission),
  );
}

export function environmentApplicationDiff(
  applications: EnvironmentObjectStatus<EnvironmentApplicationObservation>[],
): EnvironmentApplicationDiffLine[] {
  return environmentObservationDiff(applications).map((entry) =>
    applicationLine(
      entry.objectId,
      entry.kind,
      entry.observation,
      entry.admission,
    ),
  );
}

function environmentObservationDiff<T>(
  objects: EnvironmentObjectStatus<T>[],
): EnvironmentObservationDiffLine<T>[] {
  return objects.flatMap((object) => {
    const admission = admissionState(object.changes);
    const entry = (
      kind: EnvironmentDiffLine["kind"],
      observation: T,
    ): EnvironmentObservationDiffLine<T> => ({
      objectId: object.object_id,
      kind,
      observation,
      admission,
    });

    switch (object.changes.head_to_working) {
      case "unchanged": {
        const observation = object.working ?? object.head;
        return observation ? [entry("context", observation)] : [];
      }
      case "inserted":
        return object.working ? [entry("inserted", object.working)] : [];
      case "deleted":
        return object.head ? [entry("deleted", object.head)] : [];
      case "modified":
        return [
          ...(object.head ? [entry("deleted", object.head)] : []),
          ...(object.working ? [entry("inserted", object.working)] : []),
        ];
    }
  });
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

function applicationLine(
  objectId: string,
  kind: EnvironmentApplicationDiffLine["kind"],
  observation: EnvironmentApplicationObservation,
  admission: EnvironmentApplicationDiffLine["admission"],
): EnvironmentApplicationDiffLine {
  return {
    key: `${objectId}:${kind}:${observation.name}:${observation.resolved_path ?? "unresolved"}:${observation.groups.join(",")}`,
    kind,
    observation,
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
        ? "<invalid:missing-value>"
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

function escapeValue(value: string): string {
  return JSON.stringify(value).slice(1, -1);
}

function shortValue(value: string): string {
  return value.length <= 22
    ? value
    : `${value.slice(0, 12)}…${value.slice(-6)}`;
}
