#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CONVERSATION_SESSION_PROPOSAL_SCHEMA: &str = "rey.conversation-session-proposal.v1";
pub const CONVERSATION_SESSION_SCHEMA: &str = "rey.conversation-session.v1";
pub const CONVERSATION_MESSAGE_PROPOSAL_SCHEMA: &str = "rey.conversation-message-proposal.v1";
pub const CONVERSATION_MESSAGE_SCHEMA: &str = "rey.conversation-message.v1";
pub const CONVERSATION_LOG_SCHEMA: &str = "rey.conversation-log.v1";
pub const CONVERSATION_TRANSCRIPT_SCHEMA: &str = "rey.conversation-transcript.v1";
pub const CONVERSATION_SESSION_ADMISSION_SCHEMA: &str = "rey.conversation-session-admission.v1";
pub const CONVERSATION_MESSAGE_ADMISSION_SCHEMA: &str = "rey.conversation-message-admission.v1";
pub const LOCAL_TRANSCRIPT_PROVIDER: &str = "rey.local-transcript";
pub const LOCAL_TRANSCRIPT_PROVIDER_REVISION: &str = "v1";
pub const MAX_CONVERSATION_SESSION_INPUT_BYTES: u64 = 1024 * 1024;
pub const MAX_CONVERSATION_MESSAGE_INPUT_BYTES: u64 = 64 * 1024;
pub const MAX_CONVERSATION_STATE_BYTES: u64 = 4 * 1024 * 1024;
pub const DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT: usize = 100;

const STATE_FILE_NAME: &str = "conversations.json";
const LOCK_FILE_NAME: &str = "conversations.lock";
const MAX_SESSIONS: usize = 32;
const MAX_MESSAGES: usize = 2_048;
const MAX_PARTICIPANTS: usize = 16;
const MAX_WRITERS: usize = 16;
const MAX_MESSAGE_BYTES: usize = 16 * 1024;
const MAX_TITLE_BYTES: usize = 240;
const MAX_LABEL_BYTES: usize = 120;
const MAX_IDENTIFIER_CHARS: usize = 80;
const MAX_LOCATOR_BYTES: usize = 4_096;
const MAX_TRANSCRIPT_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationParticipantKind {
    Human,
    Rey,
    Agent,
}

impl ConversationParticipantKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Rey => "rey",
            Self::Agent => "agent",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationParticipant {
    pub participant_id: String,
    pub kind: ConversationParticipantKind,
    pub label: String,
}

impl ConversationParticipant {
    fn verify(&self) -> Result<(), ConversationError> {
        validate_identifier("conversation participant", &self.participant_id)?;
        validate_text(
            "conversation participant label",
            &self.label,
            MAX_LABEL_BYTES,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTransportKind {
    LocalTranscript,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTransportContract {
    pub kind: ConversationTransportKind,
    pub provider: String,
    pub provider_revision: String,
}

impl ConversationTransportContract {
    fn verify(&self) -> Result<(), ConversationError> {
        if self.kind != ConversationTransportKind::LocalTranscript
            || self.provider != LOCAL_TRANSCRIPT_PROVIDER
            || self.provider_revision != LOCAL_TRANSCRIPT_PROVIDER_REVISION
        {
            return Err(ConversationError::TransportContract);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSessionProposal {
    pub schema: String,
    pub title: String,
    pub transport: ConversationTransportContract,
    pub participants: Vec<ConversationParticipant>,
    pub writer_ids: Vec<String>,
    pub browser_writer_id: Option<String>,
}

impl ConversationSessionProposal {
    pub fn verify(&self) -> Result<(), ConversationError> {
        validate_schema(&self.schema, CONVERSATION_SESSION_PROPOSAL_SCHEMA)?;
        validate_text("conversation session title", &self.title, MAX_TITLE_BYTES)?;
        self.transport.verify()?;
        if self.participants.is_empty() || self.participants.len() > MAX_PARTICIPANTS {
            return Err(ConversationError::ParticipantLimit {
                actual: self.participants.len(),
                limit: MAX_PARTICIPANTS,
            });
        }
        let mut participants = BTreeMap::new();
        for participant in &self.participants {
            participant.verify()?;
            if participants
                .insert(participant.participant_id.as_str(), participant.kind)
                .is_some()
            {
                return Err(ConversationError::DuplicateParticipant(
                    participant.participant_id.clone(),
                ));
            }
        }
        if self.writer_ids.is_empty() || self.writer_ids.len() > MAX_WRITERS {
            return Err(ConversationError::WriterLimit {
                actual: self.writer_ids.len(),
                limit: MAX_WRITERS,
            });
        }
        let mut writers = BTreeSet::new();
        for writer_id in &self.writer_ids {
            validate_identifier("conversation writer", writer_id)?;
            if !participants.contains_key(writer_id.as_str()) {
                return Err(ConversationError::UnknownWriter(writer_id.clone()));
            }
            if !writers.insert(writer_id.as_str()) {
                return Err(ConversationError::DuplicateWriter(writer_id.clone()));
            }
        }
        if let Some(browser_writer_id) = &self.browser_writer_id {
            let Some(kind) = participants.get(browser_writer_id.as_str()) else {
                return Err(ConversationError::UnknownBrowserWriter(
                    browser_writer_id.clone(),
                ));
            };
            if !writers.contains(browser_writer_id.as_str()) {
                return Err(ConversationError::BrowserWriterCannotWrite(
                    browser_writer_id.clone(),
                ));
            }
            if *kind != ConversationParticipantKind::Human {
                return Err(ConversationError::BrowserWriterKind(
                    browser_writer_id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<SemanticDigest, ConversationError> {
        self.verify()?;
        semantic_identity(CONVERSATION_SESSION_PROPOSAL_SCHEMA, self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSource {
    pub locator: String,
    pub content_digest: SemanticDigest,
}

impl ConversationSource {
    #[must_use]
    pub fn from_bytes(locator: String, bytes: &[u8]) -> Self {
        let mut hasher = SemanticHasher::new("rey.conversation-source.v1");
        hasher.add_bytes(bytes);
        Self {
            locator,
            content_digest: hasher.finish(),
        }
    }

    fn verify(&self) -> Result<(), ConversationError> {
        validate_locator("conversation source", &self.locator)?;
        validate_digest("conversation source", &self.content_digest)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLimits {
    pub max_sessions: u64,
    pub max_messages: u64,
    pub max_participants_per_session: u64,
    pub max_writers_per_session: u64,
    pub max_message_bytes: u64,
    pub max_transcript_rows: u64,
    pub max_state_bytes: u64,
}

impl Default for ConversationLimits {
    fn default() -> Self {
        Self {
            max_sessions: MAX_SESSIONS as u64,
            max_messages: MAX_MESSAGES as u64,
            max_participants_per_session: MAX_PARTICIPANTS as u64,
            max_writers_per_session: MAX_WRITERS as u64,
            max_message_bytes: MAX_MESSAGE_BYTES as u64,
            max_transcript_rows: MAX_TRANSCRIPT_LIMIT as u64,
            max_state_bytes: MAX_CONVERSATION_STATE_BYTES,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSession {
    pub schema: String,
    pub session_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at_unix: i64,
    pub source: ConversationSource,
    pub limits: ConversationLimits,
    pub proposal: ConversationSessionProposal,
}

impl ConversationSession {
    fn verify(&self) -> Result<(), ConversationError> {
        validate_schema(&self.schema, CONVERSATION_SESSION_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ConversationError::Sequence {
                kind: "session",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.admitted_at_unix)?;
        self.source.verify()?;
        if self.limits != ConversationLimits::default() {
            return Err(ConversationError::LimitEnvelope);
        }
        if self.proposal.identity()? != self.session_id {
            return Err(ConversationError::Identity("session"));
        }
        Ok(())
    }

    #[must_use]
    pub fn participant(&self, participant_id: &str) -> Option<&ConversationParticipant> {
        self.proposal
            .participants
            .iter()
            .find(|participant| participant.participant_id == participant_id)
    }

    #[must_use]
    pub fn can_write(&self, participant_id: &str) -> bool {
        self.proposal
            .writer_ids
            .iter()
            .any(|writer_id| writer_id == participant_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessageProposal {
    pub schema: String,
    pub session_id: SemanticDigest,
    pub author_id: String,
    pub body: String,
    pub reply_to: Option<SemanticDigest>,
}

impl ConversationMessageProposal {
    pub fn verify(&self) -> Result<(), ConversationError> {
        validate_schema(&self.schema, CONVERSATION_MESSAGE_PROPOSAL_SCHEMA)?;
        validate_digest("conversation session", &self.session_id)?;
        validate_identifier("conversation message author", &self.author_id)?;
        validate_text("conversation message body", &self.body, MAX_MESSAGE_BYTES)?;
        if let Some(reply_to) = &self.reply_to {
            validate_digest("conversation reply", reply_to)?;
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<SemanticDigest, ConversationError> {
        self.verify()?;
        semantic_identity(CONVERSATION_MESSAGE_PROPOSAL_SCHEMA, self)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationDeliveryState {
    NotAttempted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessage {
    pub schema: String,
    pub message_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at_unix: i64,
    pub source: ConversationSource,
    pub delivery: ConversationDeliveryState,
    pub proposal: ConversationMessageProposal,
}

impl ConversationMessage {
    fn verify_intrinsic(&self) -> Result<(), ConversationError> {
        validate_schema(&self.schema, CONVERSATION_MESSAGE_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ConversationError::Sequence {
                kind: "message",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.admitted_at_unix)?;
        self.source.verify()?;
        if self.delivery != ConversationDeliveryState::NotAttempted {
            return Err(ConversationError::DeliveryState);
        }
        if self.proposal.identity()? != self.message_id {
            return Err(ConversationError::Identity("message"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationLog {
    pub schema: String,
    pub log_id: SemanticDigest,
    pub sessions: Vec<ConversationSession>,
    pub messages: Vec<ConversationMessage>,
}

impl Default for ConversationLog {
    fn default() -> Self {
        let mut log = Self {
            schema: CONVERSATION_LOG_SCHEMA.to_owned(),
            log_id: empty_log_identity(),
            sessions: Vec::new(),
            messages: Vec::new(),
        };
        log.refresh_identity()
            .expect("an empty conversation log is valid");
        log
    }
}

impl ConversationLog {
    pub fn verify(&self) -> Result<(), ConversationError> {
        validate_schema(&self.schema, CONVERSATION_LOG_SCHEMA)?;
        if self.sessions.len() > MAX_SESSIONS {
            return Err(ConversationError::RecordLimit {
                kind: "session",
                limit: MAX_SESSIONS,
            });
        }
        if self.messages.len() > MAX_MESSAGES {
            return Err(ConversationError::RecordLimit {
                kind: "message",
                limit: MAX_MESSAGES,
            });
        }
        let mut sessions = BTreeMap::new();
        for (position, session) in self.sessions.iter().enumerate() {
            session.verify()?;
            let expected = position as u64 + 1;
            if session.sequence != expected {
                return Err(ConversationError::Sequence {
                    kind: "session",
                    expected,
                    actual: session.sequence,
                });
            }
            if sessions
                .insert(session.session_id.clone(), session)
                .is_some()
            {
                return Err(ConversationError::DuplicateSession(
                    session.session_id.clone(),
                ));
            }
        }
        let mut message_ids = BTreeSet::new();
        let mut session_counts = BTreeMap::<SemanticDigest, u64>::new();
        let mut session_message_ids = BTreeMap::<SemanticDigest, BTreeSet<SemanticDigest>>::new();
        for message in &self.messages {
            message.verify_intrinsic()?;
            let Some(session) = sessions.get(&message.proposal.session_id) else {
                return Err(ConversationError::UnknownSession(
                    message.proposal.session_id.clone(),
                ));
            };
            if session.participant(&message.proposal.author_id).is_none() {
                return Err(ConversationError::UnknownAuthor(
                    message.proposal.author_id.clone(),
                ));
            }
            if !session.can_write(&message.proposal.author_id) {
                return Err(ConversationError::WriteDenied(
                    message.proposal.author_id.clone(),
                ));
            }
            let expected = session_counts
                .entry(message.proposal.session_id.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            if message.sequence != *expected {
                return Err(ConversationError::Sequence {
                    kind: "message",
                    expected: *expected,
                    actual: message.sequence,
                });
            }
            let prior = session_message_ids
                .entry(message.proposal.session_id.clone())
                .or_default();
            if let Some(reply_to) = &message.proposal.reply_to
                && !prior.contains(reply_to)
            {
                return Err(ConversationError::UnknownReply(reply_to.clone()));
            }
            prior.insert(message.message_id.clone());
            if !message_ids.insert(message.message_id.clone()) {
                return Err(ConversationError::DuplicateMessage(
                    message.message_id.clone(),
                ));
            }
        }
        if conversation_log_identity(&self.sessions, &self.messages)? != self.log_id {
            return Err(ConversationError::Identity("conversation log"));
        }
        Ok(())
    }

    fn refresh_identity(&mut self) -> Result<(), ConversationError> {
        self.log_id = conversation_log_identity(&self.sessions, &self.messages)?;
        Ok(())
    }

    pub fn session(&self, session_id: &str) -> Result<&ConversationSession, ConversationError> {
        self.sessions
            .iter()
            .find(|session| session.session_id.as_str() == session_id)
            .ok_or_else(|| ConversationError::UnknownSessionText(session_id.to_owned()))
    }

    pub fn transcript(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<ConversationTranscript, ConversationError> {
        self.verify()?;
        if !(1..=MAX_TRANSCRIPT_LIMIT).contains(&limit) {
            return Err(ConversationError::TranscriptLimit {
                actual: limit,
                limit: MAX_TRANSCRIPT_LIMIT,
            });
        }
        let session = match session_id {
            Some(session_id) => Some(self.session(session_id)?.clone()),
            None => self.sessions.last().cloned(),
        };
        let Some(session) = session else {
            return Ok(ConversationTranscript::unavailable(
                self.log_id.clone(),
                limit,
            ));
        };
        let all = self
            .messages
            .iter()
            .filter(|message| message.proposal.session_id == session.session_id)
            .cloned()
            .collect::<Vec<_>>();
        let omitted = all.len().saturating_sub(limit);
        let messages = all.into_iter().skip(omitted).collect::<Vec<_>>();
        let transcript_id =
            transcript_identity(&self.log_id, &session.session_id, limit, &messages)?;
        let browser_write_enabled = session.proposal.browser_writer_id.is_some();
        Ok(ConversationTranscript {
            schema: CONVERSATION_TRANSCRIPT_SCHEMA.to_owned(),
            transcript_id,
            log_id: self.log_id.clone(),
            session: Some(session.clone()),
            availability: ConversationTransportAvailability::Available,
            availability_detail: "the admitted workspace-local append-only transcript is available"
                .to_owned(),
            ordering: "local_per_session_sequence; no cross-session or provider clock"
                .to_owned(),
            retention: format!(
                "workspace_local_append_only; {} sessions, {} messages, {} bytes maximum",
                MAX_SESSIONS, MAX_MESSAGES, MAX_CONVERSATION_STATE_BYTES
            ),
            read_authority:
                "local CLI callers and clients of the configured UI listener; no authentication"
                    .to_owned(),
            cli_write_authority:
                "local workspace caller may self-assert one declared writer; admission only"
                    .to_owned(),
            browser_write_authority: if browser_write_enabled {
                format!(
                    "any UI listener client may act as self-asserted declared human writer {}; admission only",
                    session
                        .proposal
                        .browser_writer_id
                        .as_deref()
                        .unwrap_or("unknown")
                )
            } else {
                "none; this session declares no browser writer".to_owned()
            },
            browser_write_enabled,
            effect_authority:
                "none; transcript admission does not invoke an agent, deliver through Channel relay, create an observation or Journal entry, schedule work, mutate runtime state, or prove a claim"
                    .to_owned(),
            failure_contract:
                "validation, stale session, unknown author/reply, bounds, lock, persistence, or tamper failure rejects the append before publication and leaves the prior log authoritative"
                    .to_owned(),
            completeness: if omitted == 0 {
                ConversationTranscriptCompleteness::Complete
            } else {
                ConversationTranscriptCompleteness::Truncated
            },
            total_messages: (messages.len() + omitted) as u64,
            omitted_messages: omitted as u64,
            messages,
            limits: ConversationLimits::default(),
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTransportAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTranscriptCompleteness {
    Complete,
    Truncated,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTranscript {
    pub schema: String,
    pub transcript_id: SemanticDigest,
    pub log_id: SemanticDigest,
    pub session: Option<ConversationSession>,
    pub availability: ConversationTransportAvailability,
    pub availability_detail: String,
    pub ordering: String,
    pub retention: String,
    pub read_authority: String,
    pub cli_write_authority: String,
    pub browser_write_authority: String,
    pub browser_write_enabled: bool,
    pub effect_authority: String,
    pub failure_contract: String,
    pub completeness: ConversationTranscriptCompleteness,
    pub total_messages: u64,
    pub omitted_messages: u64,
    pub messages: Vec<ConversationMessage>,
    pub limits: ConversationLimits,
}

impl ConversationTranscript {
    fn unavailable(log_id: SemanticDigest, limit: usize) -> Self {
        let transcript_id = unavailable_transcript_identity(&log_id, limit);
        Self {
            schema: CONVERSATION_TRANSCRIPT_SCHEMA.to_owned(),
            transcript_id,
            log_id,
            session: None,
            availability: ConversationTransportAvailability::Unavailable,
            availability_detail: "no conversation session is admitted".to_owned(),
            ordering: "none; no session sequence exists".to_owned(),
            retention: "none; no transcript exists".to_owned(),
            read_authority:
                "local CLI callers and clients of the configured UI listener may inspect this unavailable boundary"
                    .to_owned(),
            cli_write_authority: "none; admit a session before appending messages".to_owned(),
            browser_write_authority: "none; no browser writer is bound".to_owned(),
            browser_write_enabled: false,
            effect_authority: "none".to_owned(),
            failure_contract:
                "message admission fails closed while no exact session and writer are bound"
                    .to_owned(),
            completeness: ConversationTranscriptCompleteness::Complete,
            total_messages: 0,
            omitted_messages: 0,
            messages: Vec::new(),
            limits: ConversationLimits::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationSessionAdmission {
    pub schema: String,
    pub admitted: bool,
    pub session: ConversationSession,
    pub transcript: ConversationTranscript,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationMessageAdmission {
    pub schema: String,
    pub admitted: bool,
    pub message: ConversationMessage,
    pub transcript: ConversationTranscript,
}

#[derive(Clone, Debug)]
pub struct LocalConversationStore {
    directory: PathBuf,
}

impl LocalConversationStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("conversations"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<ConversationLog, ConversationError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ConversationLog::default());
            }
            Err(source) => return Err(ConversationError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ConversationError::UnsafePath(path));
        }
        if metadata.len() > MAX_CONVERSATION_STATE_BYTES {
            return Err(ConversationError::ByteLimit(MAX_CONVERSATION_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| ConversationError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_CONVERSATION_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| ConversationError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_CONVERSATION_STATE_BYTES {
            return Err(ConversationError::ByteLimit(MAX_CONVERSATION_STATE_BYTES));
        }
        let log: ConversationLog = serde_json::from_slice(&bytes)?;
        log.verify()?;
        Ok(log)
    }

    pub fn transcript(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<ConversationTranscript, ConversationError> {
        self.load()?.transcript(session_id, limit)
    }

    pub fn admit_session(
        &self,
        proposal: ConversationSessionProposal,
        source: ConversationSource,
        admitted_at_unix: i64,
    ) -> Result<ConversationSessionAdmission, ConversationError> {
        proposal.verify()?;
        source.verify()?;
        validate_timestamp(admitted_at_unix)?;
        self.with_lock(|| {
            let mut log = self.load()?;
            let session_id = proposal.identity()?;
            let (session, admitted) = if let Some(session) = log
                .sessions
                .iter()
                .find(|session| session.session_id == session_id)
            {
                (session.clone(), false)
            } else {
                if log.sessions.len() >= MAX_SESSIONS {
                    return Err(ConversationError::RecordLimit {
                        kind: "session",
                        limit: MAX_SESSIONS,
                    });
                }
                let session = ConversationSession {
                    schema: CONVERSATION_SESSION_SCHEMA.to_owned(),
                    session_id,
                    sequence: log.sessions.len() as u64 + 1,
                    admitted_at_unix,
                    source,
                    limits: ConversationLimits::default(),
                    proposal,
                };
                session.verify()?;
                log.sessions.push(session.clone());
                log.refresh_identity()?;
                self.save(&log)?;
                (session, true)
            };
            let transcript = log.transcript(
                Some(session.session_id.as_str()),
                DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT,
            )?;
            Ok(ConversationSessionAdmission {
                schema: CONVERSATION_SESSION_ADMISSION_SCHEMA.to_owned(),
                admitted,
                session,
                transcript,
            })
        })
    }

    pub fn admit_message(
        &self,
        proposal: ConversationMessageProposal,
        source: ConversationSource,
        admitted_at_unix: i64,
    ) -> Result<ConversationMessageAdmission, ConversationError> {
        proposal.verify()?;
        source.verify()?;
        validate_timestamp(admitted_at_unix)?;
        self.with_lock(|| {
            let mut log = self.load()?;
            let session = log.session(proposal.session_id.as_str())?.clone();
            if session.participant(&proposal.author_id).is_none() {
                return Err(ConversationError::UnknownAuthor(proposal.author_id.clone()));
            }
            if !session.can_write(&proposal.author_id) {
                return Err(ConversationError::WriteDenied(proposal.author_id.clone()));
            }
            if let Some(reply_to) = &proposal.reply_to
                && !log.messages.iter().any(|message| {
                    message.message_id == *reply_to
                        && message.proposal.session_id == proposal.session_id
                })
            {
                return Err(ConversationError::UnknownReply(reply_to.clone()));
            }
            let message_id = proposal.identity()?;
            let (message, admitted) = if let Some(message) = log
                .messages
                .iter()
                .find(|message| message.message_id == message_id)
            {
                (message.clone(), false)
            } else {
                if log.messages.len() >= MAX_MESSAGES {
                    return Err(ConversationError::RecordLimit {
                        kind: "message",
                        limit: MAX_MESSAGES,
                    });
                }
                let sequence = log
                    .messages
                    .iter()
                    .filter(|message| message.proposal.session_id == proposal.session_id)
                    .count() as u64
                    + 1;
                let message = ConversationMessage {
                    schema: CONVERSATION_MESSAGE_SCHEMA.to_owned(),
                    message_id,
                    sequence,
                    admitted_at_unix,
                    source,
                    delivery: ConversationDeliveryState::NotAttempted,
                    proposal,
                };
                message.verify_intrinsic()?;
                log.messages.push(message.clone());
                log.refresh_identity()?;
                log.verify()?;
                self.save(&log)?;
                (message, true)
            };
            let transcript = log.transcript(
                Some(session.session_id.as_str()),
                DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT,
            )?;
            Ok(ConversationMessageAdmission {
                schema: CONVERSATION_MESSAGE_ADMISSION_SCHEMA.to_owned(),
                admitted,
                message,
                transcript,
            })
        })
    }

    fn save(&self, log: &ConversationLog) -> Result<(), ConversationError> {
        log.verify()?;
        let mut bytes = serde_json::to_vec_pretty(log)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_CONVERSATION_STATE_BYTES {
            return Err(ConversationError::ByteLimit(MAX_CONVERSATION_STATE_BYTES));
        }
        self.prepare_directory()?;
        let (temporary_path, mut temporary) = self.create_temporary()?;
        if let Err(source) = temporary
            .write_all(&bytes)
            .and_then(|()| temporary.flush())
            .and_then(|()| temporary.sync_all())
        {
            let _ = fs::remove_file(&temporary_path);
            return Err(ConversationError::Write {
                path: temporary_path,
                source,
            });
        }
        drop(temporary);
        if let Err(source) = fs::rename(&temporary_path, self.path()) {
            let _ = fs::remove_file(&temporary_path);
            return Err(ConversationError::Write {
                path: self.path(),
                source,
            });
        }
        Ok(())
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ConversationError>,
    ) -> Result<T, ConversationError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ConversationError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| ConversationError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| ConversationError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = operation();
        let unlock = File::unlock(&lock).map_err(|source| ConversationError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn prepare_directory(&self) -> Result<(), ConversationError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(ConversationError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| ConversationError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(ConversationError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), ConversationError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(ConversationError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(ConversationError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(ConversationError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), ConversationError> {
        for attempt in 0..32_u8 {
            let path = self.directory.join(format!(
                ".{STATE_FILE_NAME}.tmp-{}-{attempt}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(ConversationError::Write { path, source }),
            }
        }
        Err(ConversationError::TemporaryLimit(self.directory.clone()))
    }
}

fn semantic_identity(
    schema: &'static str,
    value: &impl Serialize,
) -> Result<SemanticDigest, ConversationError> {
    let mut hasher = SemanticHasher::new(schema);
    hasher.add_bytes(&serde_json::to_vec(value)?);
    Ok(hasher.finish())
}

fn empty_log_identity() -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CONVERSATION_LOG_SCHEMA);
    hasher.add_u64(0);
    hasher.add_u64(0);
    hasher.finish()
}

fn conversation_log_identity(
    sessions: &[ConversationSession],
    messages: &[ConversationMessage],
) -> Result<SemanticDigest, ConversationError> {
    let mut hasher = SemanticHasher::new(CONVERSATION_LOG_SCHEMA);
    hasher.add_u64(sessions.len() as u64);
    for session in sessions {
        hasher.add_str(session.session_id.as_str());
        hasher.add_u64(session.sequence);
        hasher.add_u64(session.admitted_at_unix as u64);
        hasher.add_bytes(&serde_json::to_vec(&session.source)?);
    }
    hasher.add_u64(messages.len() as u64);
    for message in messages {
        hasher.add_str(message.message_id.as_str());
        hasher.add_str(message.proposal.session_id.as_str());
        hasher.add_u64(message.sequence);
        hasher.add_u64(message.admitted_at_unix as u64);
        hasher.add_bytes(&serde_json::to_vec(&message.source)?);
    }
    Ok(hasher.finish())
}

fn transcript_identity(
    log_id: &SemanticDigest,
    session_id: &SemanticDigest,
    limit: usize,
    messages: &[ConversationMessage],
) -> Result<SemanticDigest, ConversationError> {
    let mut hasher = SemanticHasher::new(CONVERSATION_TRANSCRIPT_SCHEMA);
    hasher.add_str(log_id.as_str());
    hasher.add_str(session_id.as_str());
    hasher.add_u64(limit as u64);
    hasher.add_u64(messages.len() as u64);
    for message in messages {
        hasher.add_str(message.message_id.as_str());
    }
    Ok(hasher.finish())
}

fn unavailable_transcript_identity(log_id: &SemanticDigest, limit: usize) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CONVERSATION_TRANSCRIPT_SCHEMA);
    hasher.add_str(log_id.as_str());
    hasher.add_optional_str(None);
    hasher.add_u64(limit as u64);
    hasher.finish()
}

fn validate_schema(actual: &str, expected: &'static str) -> Result<(), ConversationError> {
    if actual != expected {
        return Err(ConversationError::Schema {
            expected,
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ConversationError> {
    let valid = !value.is_empty()
        && value.chars().count() <= MAX_IDENTIFIER_CHARS
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric);
    if !valid {
        return Err(ConversationError::Identifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str, limit: usize) -> Result<(), ConversationError> {
    if value.is_empty() || value.len() > limit || value.trim() != value || value.contains('\0') {
        return Err(ConversationError::Text { field, limit });
    }
    Ok(())
}

fn validate_locator(field: &'static str, value: &str) -> Result<(), ConversationError> {
    if value.is_empty()
        || value.len() > MAX_LOCATOR_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ConversationError::Locator {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_digest(field: &'static str, value: &SemanticDigest) -> Result<(), ConversationError> {
    let value = value.as_str();
    if value.len() != 71
        || !value.starts_with("blake3:")
        || !value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ConversationError::Digest {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_timestamp(value: i64) -> Result<(), ConversationError> {
    if value < 0 {
        return Err(ConversationError::Timestamp(value));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ConversationError {
    #[error("expected schema {expected}, found {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid {field} identifier {value:?}")]
    Identifier { field: &'static str, value: String },
    #[error("{field} is empty, non-canonical, or exceeds the {limit}-byte limit")]
    Text { field: &'static str, limit: usize },
    #[error("invalid bounded {field} locator {value:?}")]
    Locator { field: &'static str, value: String },
    #[error("invalid {field} digest {value:?}")]
    Digest { field: &'static str, value: String },
    #[error("conversation uses an unsupported transport contract")]
    TransportContract,
    #[error("conversation session has {actual} participants; supported range is 1..={limit}")]
    ParticipantLimit { actual: usize, limit: usize },
    #[error("conversation session has {actual} writers; supported range is 1..={limit}")]
    WriterLimit { actual: usize, limit: usize },
    #[error("conversation session repeats participant {0}")]
    DuplicateParticipant(String),
    #[error("conversation session repeats writer {0}")]
    DuplicateWriter(String),
    #[error("conversation writer {0} is not a declared participant")]
    UnknownWriter(String),
    #[error("browser writer {0} is not a declared participant")]
    UnknownBrowserWriter(String),
    #[error("browser writer {0} is not a declared writer")]
    BrowserWriterCannotWrite(String),
    #[error("browser writer {0} must be a human participant")]
    BrowserWriterKind(String),
    #[error("conversation participant {0} is not declared by the session")]
    UnknownAuthor(String),
    #[error("conversation participant {0} has no write authority")]
    WriteDenied(String),
    #[error("unknown conversation session {0}")]
    UnknownSession(SemanticDigest),
    #[error("unknown conversation session {0}")]
    UnknownSessionText(String),
    #[error("conversation reply references an unknown or non-prior message {0}")]
    UnknownReply(SemanticDigest),
    #[error("conversation transcript limit must be between 1 and {limit}, got {actual}")]
    TranscriptLimit { actual: usize, limit: usize },
    #[error("{kind} sequence must be {expected}, got {actual}")]
    Sequence {
        kind: &'static str,
        expected: u64,
        actual: u64,
    },
    #[error("{0} identity does not match its retained content")]
    Identity(&'static str),
    #[error("conversation timestamp {0} is outside the supported range")]
    Timestamp(i64),
    #[error("conversation uses an unsupported effective limit envelope")]
    LimitEnvelope,
    #[error("conversation message delivery state must remain not_attempted")]
    DeliveryState,
    #[error("conversation log repeats session {0}")]
    DuplicateSession(SemanticDigest),
    #[error("conversation log repeats message {0}")]
    DuplicateMessage(SemanticDigest),
    #[error("conversation log exceeds the {limit}-record {kind} limit")]
    RecordLimit { kind: &'static str, limit: usize },
    #[error("conversation state exceeds {0} bytes")]
    ByteLimit(u64),
    #[error("unsafe symlink or file type in conversation state path {0}")]
    UnsafePath(PathBuf),
    #[error("could not read conversation state {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not write conversation state {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not lock conversation state {path}: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not allocate a temporary conversation state file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        CONVERSATION_MESSAGE_PROPOSAL_SCHEMA, CONVERSATION_SESSION_PROPOSAL_SCHEMA,
        ConversationMessageProposal, ConversationParticipant, ConversationParticipantKind,
        ConversationSessionProposal, ConversationSource, ConversationTransportAvailability,
        ConversationTransportContract, ConversationTransportKind, LOCAL_TRANSCRIPT_PROVIDER,
        LOCAL_TRANSCRIPT_PROVIDER_REVISION, LocalConversationStore,
    };

    #[test]
    fn session_and_message_admission_are_idempotent_and_ordered() {
        let directory = TempDir::new().unwrap();
        let store = LocalConversationStore::new(directory.path().join("conversations"));
        let proposal = session_proposal(Some("operator"));
        let bytes = serde_json::to_vec(&proposal).unwrap();
        let first = store
            .admit_session(
                proposal.clone(),
                ConversationSource::from_bytes("worktree:///session.yaml".to_owned(), &bytes),
                10,
            )
            .unwrap();
        assert!(first.admitted);
        let replay = store
            .admit_session(
                proposal,
                ConversationSource::from_bytes("worktree:///session.yaml".to_owned(), &bytes),
                11,
            )
            .unwrap();
        assert!(!replay.admitted);
        assert_eq!(replay.session.session_id, first.session.session_id);

        let proposal =
            message_proposal(first.session.session_id.clone(), "operator", "Hello", None);
        let bytes = serde_json::to_vec(&proposal).unwrap();
        let message = store
            .admit_message(
                proposal.clone(),
                ConversationSource::from_bytes("worktree:///message.yaml".to_owned(), &bytes),
                12,
            )
            .unwrap();
        assert!(message.admitted);
        assert_eq!(message.message.sequence, 1);
        assert_eq!(message.transcript.messages.len(), 1);
        let replay = store
            .admit_message(
                proposal,
                ConversationSource::from_bytes("worktree:///message.yaml".to_owned(), &bytes),
                13,
            )
            .unwrap();
        assert!(!replay.admitted);
        assert_eq!(replay.message.message_id, message.message.message_id);
        assert_eq!(store.load().unwrap().messages.len(), 1);
    }

    #[test]
    fn availability_and_authority_follow_the_exact_session_contract() {
        let directory = TempDir::new().unwrap();
        let store = LocalConversationStore::new(directory.path().join("conversations"));
        let unavailable = store.transcript(None, 100).unwrap();
        assert_eq!(
            unavailable.availability,
            ConversationTransportAvailability::Unavailable
        );
        assert!(!unavailable.browser_write_enabled);

        let proposal = session_proposal(None);
        let bytes = serde_json::to_vec(&proposal).unwrap();
        let session = store
            .admit_session(
                proposal,
                ConversationSource::from_bytes("worktree:///session.yaml".to_owned(), &bytes),
                10,
            )
            .unwrap();
        assert_eq!(
            session.transcript.availability,
            ConversationTransportAvailability::Available
        );
        assert!(!session.transcript.browser_write_enabled);
        assert!(
            session
                .transcript
                .browser_write_authority
                .starts_with("none")
        );
    }

    #[test]
    fn undeclared_writers_and_cross_session_replies_fail_closed() {
        let directory = TempDir::new().unwrap();
        let store = LocalConversationStore::new(directory.path().join("conversations"));
        let first = admit_session(&store, session_proposal(Some("operator")), 10);
        let denied = message_proposal(first.session_id.clone(), "observer", "No write", None);
        let bytes = serde_json::to_vec(&denied).unwrap();
        assert!(
            store
                .admit_message(
                    denied,
                    ConversationSource::from_bytes("worktree:///denied.yaml".to_owned(), &bytes),
                    11,
                )
                .is_err()
        );

        let message = admit_message(
            &store,
            first.session_id.clone(),
            "operator",
            "First",
            None,
            12,
        );
        let second = admit_session(&store, session_proposal(Some("operator-two")), 13);
        let cross_reply = message_proposal(
            second.session_id,
            "operator-two",
            "Cross-session reply",
            Some(message.message_id),
        );
        let bytes = serde_json::to_vec(&cross_reply).unwrap();
        assert!(
            store
                .admit_message(
                    cross_reply,
                    ConversationSource::from_bytes("worktree:///reply.yaml".to_owned(), &bytes),
                    14,
                )
                .is_err()
        );
    }

    #[test]
    fn restart_tamper_and_symlink_boundaries_fail_closed() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("conversations");
        let store = LocalConversationStore::new(state.clone());
        let session = admit_session(&store, session_proposal(Some("operator")), 10);
        admit_message(&store, session.session_id, "operator", "Retained", None, 11);

        let restarted = LocalConversationStore::new(state.clone());
        assert_eq!(restarted.load().unwrap().messages.len(), 1);
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        document["messages"][0]["proposal"]["body"] = "tampered".into();
        fs::write(store.path(), serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        assert!(restarted.load().is_err());

        let unsafe_directory = TempDir::new().unwrap();
        let outside = unsafe_directory.path().join("outside.json");
        fs::write(&outside, b"{}\n").unwrap();
        let state = unsafe_directory.path().join("state");
        fs::create_dir(&state).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, state.join("conversations.json")).unwrap();
            assert!(LocalConversationStore::new(state).load().is_err());
        }
    }

    fn session_proposal(browser_writer_id: Option<&str>) -> ConversationSessionProposal {
        let operator_id = browser_writer_id.unwrap_or("operator");
        ConversationSessionProposal {
            schema: CONVERSATION_SESSION_PROPOSAL_SCHEMA.to_owned(),
            title: "Plan coordination".to_owned(),
            transport: ConversationTransportContract {
                kind: ConversationTransportKind::LocalTranscript,
                provider: LOCAL_TRANSCRIPT_PROVIDER.to_owned(),
                provider_revision: LOCAL_TRANSCRIPT_PROVIDER_REVISION.to_owned(),
            },
            participants: vec![
                ConversationParticipant {
                    participant_id: operator_id.to_owned(),
                    kind: ConversationParticipantKind::Human,
                    label: "Operator".to_owned(),
                },
                ConversationParticipant {
                    participant_id: "codex".to_owned(),
                    kind: ConversationParticipantKind::Agent,
                    label: "Codex".to_owned(),
                },
                ConversationParticipant {
                    participant_id: "observer".to_owned(),
                    kind: ConversationParticipantKind::Agent,
                    label: "Observer".to_owned(),
                },
            ],
            writer_ids: vec![operator_id.to_owned(), "codex".to_owned()],
            browser_writer_id: browser_writer_id.map(str::to_owned),
        }
    }

    fn message_proposal(
        session_id: rey_core::SemanticDigest,
        author_id: &str,
        body: &str,
        reply_to: Option<rey_core::SemanticDigest>,
    ) -> ConversationMessageProposal {
        ConversationMessageProposal {
            schema: CONVERSATION_MESSAGE_PROPOSAL_SCHEMA.to_owned(),
            session_id,
            author_id: author_id.to_owned(),
            body: body.to_owned(),
            reply_to,
        }
    }

    fn admit_session(
        store: &LocalConversationStore,
        proposal: ConversationSessionProposal,
        timestamp: i64,
    ) -> super::ConversationSession {
        let bytes = serde_json::to_vec(&proposal).unwrap();
        store
            .admit_session(
                proposal,
                ConversationSource::from_bytes(format!("fixture:///session-{timestamp}"), &bytes),
                timestamp,
            )
            .unwrap()
            .session
    }

    fn admit_message(
        store: &LocalConversationStore,
        session_id: rey_core::SemanticDigest,
        author_id: &str,
        body: &str,
        reply_to: Option<rey_core::SemanticDigest>,
        timestamp: i64,
    ) -> super::ConversationMessage {
        let proposal = message_proposal(session_id, author_id, body, reply_to);
        let bytes = serde_json::to_vec(&proposal).unwrap();
        store
            .admit_message(
                proposal,
                ConversationSource::from_bytes(format!("fixture:///message-{timestamp}"), &bytes),
                timestamp,
            )
            .unwrap()
            .message
    }
}
