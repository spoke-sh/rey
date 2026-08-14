export type ObservationKind =
  "finding" | "question" | "progress" | "blocker" | "handoff";

export type ObservationAuthorKind = "human" | "agent" | "rey";
export type ObservationCompleteness = "complete" | "partial";

export interface ObservationAuthor {
  kind: ObservationAuthorKind;
  id: string;
}

export interface ObservationEvidenceBinding {
  locator: string;
  source_revision: string;
  content_digest: string;
}

export interface ObservationProposal {
  schema: "rey.observation.v1";
  kind: ObservationKind;
  author: ObservationAuthor;
  subject_locator: string;
  body: string;
  desired_delta: string | null;
  completeness: ObservationCompleteness;
  omissions: string[];
  evidence: ObservationEvidenceBinding[];
  supersedes: string | null;
}

export interface RetainedObservation {
  schema: "rey.observation-admission.v1";
  observation_id: string;
  sequence: number;
  admitted_at_unix: number;
  source: {
    locator: string;
    content_digest: string;
  };
  limits: {
    max_body_bytes: number;
    max_evidence_bindings: number;
    max_omissions: number;
    max_broadcast_targets: number;
  };
  proposal: ObservationProposal;
}

export interface ObservationFrontierRow {
  observation: RetainedObservation;
  channel_ids: string[];
}

export interface ObservationFrontier {
  schema: "rey.observation-frontier.v1";
  frontier_id: string;
  source_log_id: string;
  ordering: "observation_sequence_ascending";
  limit: number;
  complete: boolean;
  omitted: number;
  summary: {
    observations: number;
    unresolved: number;
    superseded: number;
    resolved: number;
    withdrawn: number;
    unbroadcast: number;
  };
  rows: ObservationFrontierRow[];
}

export interface ObservationWrite {
  schema: "rey.ui-observation-write.v1";
  kind: ObservationKind;
  body: string;
}

export interface ObservationBroadcastTarget {
  channel_id: string;
  outcome:
    "admitted" | "already_admitted" | "unknown_channel" | "rejected_kind";
  admission: unknown | null;
  detail: string;
}

export interface ObservationBroadcast {
  schema: "rey.observation-admission-result.v1";
  observation_admitted: boolean;
  observation: RetainedObservation;
  broadcast: {
    schema: "rey.observation-broadcast.v1";
    broadcast_id: string;
    request_id: string;
    sequence: number;
    broadcast_at_unix: number;
    observation_id: string;
    channel_head_commit_id: string | null;
    channel_graph_id: string;
    selected_channel_ids: string[];
    targets: ObservationBroadcastTarget[];
  } | null;
  frontier: ObservationFrontier;
}

export function observationPosition(row: ObservationFrontierRow): string {
  return `O@${row.observation.sequence}`;
}
