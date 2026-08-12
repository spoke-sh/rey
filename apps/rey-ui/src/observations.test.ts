import { describe, expect, it } from "vitest";
import {
  operatorObservationMailboxRows,
  type ObservationFrontier,
} from "./observations";

describe("observation operator projection", () => {
  it("retains source order and identity without synthesizing mailbox authority", () => {
    const frontier: ObservationFrontier = {
      schema: "rey.observation-frontier.v1",
      frontier_id: "blake3:frontier",
      source_log_id: "blake3:log",
      ordering: "observation_sequence_ascending",
      limit: 64,
      complete: true,
      omitted: 0,
      summary: {
        observations: 1,
        unresolved: 1,
        superseded: 0,
        resolved: 0,
        withdrawn: 0,
        unbroadcast: 0,
      },
      rows: [
        {
          observation: {
            schema: "rey.observation-admission.v1",
            observation_id: "blake3:observation",
            sequence: 7,
            admitted_at_unix: 1,
            source: {
              locator: "workspace://observation.json",
              content_digest: "blake3:source",
            },
            limits: {
              max_body_bytes: 16_384,
              max_evidence_bindings: 32,
              max_omissions: 32,
              max_broadcast_targets: 32,
            },
            proposal: {
              schema: "rey.observation.v1",
              kind: "question",
              author: { kind: "human", id: "operator" },
              subject_locator: "rey+local://workload/alpha?revision=2",
              body: "Which exact delta remains?",
              desired_delta: null,
              completeness: "complete",
              omissions: [],
              evidence: [],
              supersedes: null,
            },
          },
          channel_ids: ["workspace"],
        },
      ],
    };

    const rows = operatorObservationMailboxRows(frontier);

    expect(rows).toEqual([
      {
        row_id: "observation:blake3:observation",
        position: "O@7",
        observation_id: "blake3:observation",
        author: { kind: "human", id: "operator" },
        kind: "question",
        subject_locator: "rey+local://workload/alpha?revision=2",
        body: "Which exact delta remains?",
        completeness: "complete",
        evidence_count: 0,
        omission_count: 0,
        channel_ids: ["workspace"],
      },
    ]);
    expect(rows[0]).not.toHaveProperty("unread");
    expect(rows[0]).not.toHaveProperty("priority");
    expect(rows[0]).not.toHaveProperty("assignment");
  });
});
