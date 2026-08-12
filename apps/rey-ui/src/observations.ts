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

export function observationPosition(row: ObservationFrontierRow): string {
  return `O@${row.observation.sequence}`;
}

export interface ObservationMailboxRow {
  row_id: string;
  position: string;
  observation_id: string;
  author: ObservationAuthor;
  kind: ObservationKind;
  subject_locator: string;
  body: string;
  completeness: ObservationCompleteness;
  evidence_count: number;
  omission_count: number;
  channel_ids: string[];
}

export function operatorObservationMailboxRows(
  frontier: ObservationFrontier,
): ObservationMailboxRow[] {
  return frontier.rows.map((row) => ({
    row_id: `observation:${row.observation.observation_id}`,
    position: observationPosition(row),
    observation_id: row.observation.observation_id,
    author: row.observation.proposal.author,
    kind: row.observation.proposal.kind,
    subject_locator: row.observation.proposal.subject_locator,
    body: row.observation.proposal.body,
    completeness: row.observation.proposal.completeness,
    evidence_count: row.observation.proposal.evidence.length,
    omission_count: row.observation.proposal.omissions.length,
    channel_ids: [...row.channel_ids],
  }));
}
