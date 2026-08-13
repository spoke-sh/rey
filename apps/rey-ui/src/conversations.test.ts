import { describe, expect, it } from "vitest";
import {
  conversationBrowserWriter,
  conversationParticipant,
  type ConversationTranscript,
} from "./conversations";

describe("conversation contract projection", () => {
  it("selects only the exact declared human browser writer", () => {
    const transcript = fixtureTranscript();
    expect(conversationBrowserWriter(transcript)?.participant_id).toBe(
      "operator",
    );
    expect(conversationParticipant(transcript, "codex")?.kind).toBe("agent");

    transcript.browser_write_enabled = false;
    expect(conversationBrowserWriter(transcript)).toBeNull();
  });

  it("does not infer a writer missing from the exact participant relation", () => {
    const transcript = fixtureTranscript();
    transcript.session!.proposal.browser_writer_id = "missing";
    expect(conversationBrowserWriter(transcript)).toBeNull();
  });
});

function fixtureTranscript(): ConversationTranscript {
  const digest = `blake3:${"a".repeat(64)}`;
  const limits = {
    max_sessions: 32,
    max_messages: 2_048,
    max_participants_per_session: 16,
    max_writers_per_session: 16,
    max_message_bytes: 16_384,
    max_transcript_rows: 256,
    max_state_bytes: 4_194_304,
  };
  return {
    schema: "rey.conversation-transcript.v1",
    transcript_id: digest,
    log_id: digest,
    session: {
      schema: "rey.conversation-session.v1",
      session_id: digest,
      sequence: 1,
      admitted_at_unix: 1,
      source: { locator: "fixture:///session", content_digest: digest },
      limits,
      proposal: {
        schema: "rey.conversation-session-proposal.v1",
        title: "Fixture",
        transport: {
          kind: "local_transcript",
          provider: "rey.local-transcript",
          provider_revision: "v1",
        },
        participants: [
          { participant_id: "operator", kind: "human", label: "Operator" },
          { participant_id: "codex", kind: "agent", label: "Codex" },
        ],
        writer_ids: ["operator", "codex"],
        browser_writer_id: "operator",
      },
    },
    availability: "available",
    availability_detail: "available",
    ordering: "local_per_session_sequence",
    retention: "workspace_local_append_only",
    read_authority: "local",
    cli_write_authority: "declared writers",
    browser_write_authority: "operator",
    browser_write_enabled: true,
    effect_authority: "none",
    failure_contract: "fail closed",
    completeness: "complete",
    total_messages: 0,
    omitted_messages: 0,
    messages: [],
    limits,
  };
}
