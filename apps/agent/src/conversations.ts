export type ConversationParticipantKind = "human" | "rey" | "agent";
export type ConversationTransportAvailability = "available" | "unavailable";
export type ConversationTranscriptCompleteness = "complete" | "truncated";

export interface ConversationParticipant {
  participant_id: string;
  kind: ConversationParticipantKind;
  label: string;
}

export interface ConversationSession {
  schema: "rey.conversation-session.v1";
  session_id: string;
  sequence: number;
  admitted_at_unix: number;
  source: { locator: string; content_digest: string };
  limits: ConversationLimits;
  proposal: {
    schema: "rey.conversation-session-proposal.v1";
    title: string;
    transport: {
      kind: "local_transcript";
      provider: string;
      provider_revision: string;
    };
    participants: ConversationParticipant[];
    writer_ids: string[];
    browser_writer_id: string | null;
  };
}

export interface ConversationMessage {
  schema: "rey.conversation-message.v1";
  message_id: string;
  sequence: number;
  admitted_at_unix: number;
  source: { locator: string; content_digest: string };
  delivery: "not_attempted";
  proposal: {
    schema: "rey.conversation-message-proposal.v1";
    session_id: string;
    author_id: string;
    body: string;
    reply_to: string | null;
  };
}

export interface ConversationLimits {
  max_sessions: number;
  max_messages: number;
  max_participants_per_session: number;
  max_writers_per_session: number;
  max_message_bytes: number;
  max_transcript_rows: number;
  max_state_bytes: number;
}

export interface ConversationTranscript {
  schema: "rey.conversation-transcript.v1";
  transcript_id: string;
  log_id: string;
  session: ConversationSession | null;
  availability: ConversationTransportAvailability;
  availability_detail: string;
  ordering: string;
  retention: string;
  read_authority: string;
  cli_write_authority: string;
  browser_write_authority: string;
  browser_write_enabled: boolean;
  effect_authority: string;
  failure_contract: string;
  completeness: ConversationTranscriptCompleteness;
  total_messages: number;
  omitted_messages: number;
  messages: ConversationMessage[];
  limits: ConversationLimits;
}

export interface ConversationMessageWrite {
  schema: "rey.ui-conversation-message-write.v1";
  expected_log_id: string;
  session_id: string;
  body: string;
  reply_to: string | null;
}

export interface ConversationMessageAdmission {
  schema: "rey.conversation-message-admission.v1";
  admitted: boolean;
  message: ConversationMessage;
  transcript: ConversationTranscript;
}

export function conversationBrowserWriter(
  transcript: ConversationTranscript,
): ConversationParticipant | null {
  const writerId = transcript.session?.proposal.browser_writer_id;
  if (!transcript.browser_write_enabled || !writerId) return null;
  return (
    transcript.session?.proposal.participants.find(
      (participant) =>
        participant.participant_id === writerId && participant.kind === "human",
    ) ?? null
  );
}

export function conversationParticipant(
  transcript: ConversationTranscript,
  participantId: string,
): ConversationParticipant | null {
  return (
    transcript.session?.proposal.participants.find(
      (participant) => participant.participant_id === participantId,
    ) ?? null
  );
}
