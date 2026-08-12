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

use crate::channels::{ChannelGraphSnapshot, ChannelObservationKind};

pub const OBSERVATION_PROPOSAL_SCHEMA: &str = "rey.observation.v1";
pub const OBSERVATION_ADMISSION_SCHEMA: &str = "rey.observation-admission.v1";
pub const OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA: &str = "rey.observation-resolution.v1";
pub const OBSERVATION_RESOLUTION_ADMISSION_SCHEMA: &str = "rey.observation-resolution-admission.v1";
pub const OBSERVATION_RESOLUTION_RESULT_SCHEMA: &str = "rey.observation-resolution-result.v1";
pub const CHANNEL_OBSERVATION_ADMISSION_SCHEMA: &str = "rey.channel-observation-admission.v1";
pub const OBSERVATION_BROADCAST_SCHEMA: &str = "rey.observation-broadcast.v1";
pub const OBSERVATION_ADMISSION_RESULT_SCHEMA: &str = "rey.observation-admission-result.v1";
pub const OBSERVATION_LOG_SCHEMA: &str = "rey.observation-log.v1";
pub const OBSERVATION_FRONTIER_SCHEMA: &str = "rey.observation-frontier.v1";
pub const OBSERVATION_DETAIL_SCHEMA: &str = "rey.observation-detail.v1";
pub const MAX_OBSERVATION_INPUT_BYTES: u64 = 1_024 * 1_024;
pub const MAX_OBSERVATION_STATE_BYTES: u64 = 4 * 1_024 * 1_024;
pub const DEFAULT_OBSERVATION_FRONTIER_LIMIT: usize = 64;

const STATE_FILE_NAME: &str = "observations.json";
const LOCK_FILE_NAME: &str = "observations.lock";
const MAX_OBSERVATIONS: usize = 1_024;
const MAX_CHANNEL_ADMISSIONS: usize = 4_096;
const MAX_RESOLUTIONS: usize = 1_024;
const MAX_BROADCASTS: usize = 4_096;
const MAX_BROADCAST_TARGETS: usize = 32;
const MAX_EVIDENCE_BINDINGS: usize = 32;
const MAX_BODY_BYTES: usize = 16 * 1_024;
const MAX_REASON_BYTES: usize = 4 * 1_024;
const MAX_OMISSIONS: usize = 32;
const MAX_LOCATOR_BYTES: usize = 4_096;
const MAX_REVISION_BYTES: usize = 512;
const MAX_IDENTIFIER_CHARS: usize = 80;
const MAX_FRONTIER_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationAuthorKind {
    Human,
    Agent,
    Rey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationAuthor {
    pub kind: ObservationAuthorKind,
    pub id: String,
}

impl ObservationAuthor {
    fn verify(&self) -> Result<(), ObservationError> {
        validate_identifier("observation author", &self.id)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationCompleteness {
    Complete,
    Partial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLimits {
    pub max_body_bytes: u64,
    pub max_evidence_bindings: u64,
    pub max_omissions: u64,
    pub max_broadcast_targets: u64,
}

impl Default for ObservationLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: MAX_BODY_BYTES as u64,
            max_evidence_bindings: MAX_EVIDENCE_BINDINGS as u64,
            max_omissions: MAX_OMISSIONS as u64,
            max_broadcast_targets: MAX_BROADCAST_TARGETS as u64,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEvidenceBinding {
    pub locator: String,
    pub source_revision: String,
    pub content_digest: SemanticDigest,
}

impl ObservationEvidenceBinding {
    fn verify(&self) -> Result<(), ObservationError> {
        validate_locator("evidence", &self.locator)?;
        validate_revision("evidence source", &self.source_revision)?;
        validate_digest("evidence content", &self.content_digest)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationProposal {
    pub schema: String,
    pub kind: ChannelObservationKind,
    pub author: ObservationAuthor,
    pub subject_locator: String,
    pub body: String,
    pub desired_delta: Option<String>,
    pub completeness: ObservationCompleteness,
    #[serde(default)]
    pub omissions: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<ObservationEvidenceBinding>,
    pub supersedes: Option<SemanticDigest>,
}

impl ObservationProposal {
    pub fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_PROPOSAL_SCHEMA)?;
        self.author.verify()?;
        validate_locator("observation subject", &self.subject_locator)?;
        validate_text("observation body", &self.body, MAX_BODY_BYTES)?;
        if let Some(desired_delta) = &self.desired_delta {
            validate_text("observation desired delta", desired_delta, MAX_REASON_BYTES)?;
        }
        if self.omissions.len() > MAX_OMISSIONS {
            return Err(ObservationError::OmissionLimit {
                actual: self.omissions.len(),
                limit: MAX_OMISSIONS,
            });
        }
        let mut omissions = BTreeSet::new();
        for omission in &self.omissions {
            validate_text("observation omission", omission, MAX_REVISION_BYTES)?;
            if !omissions.insert(omission.as_str()) {
                return Err(ObservationError::DuplicateOmission(omission.clone()));
            }
        }
        match self.completeness {
            ObservationCompleteness::Complete if !self.omissions.is_empty() => {
                return Err(ObservationError::Completeness);
            }
            ObservationCompleteness::Partial if self.omissions.is_empty() => {
                return Err(ObservationError::Completeness);
            }
            ObservationCompleteness::Complete | ObservationCompleteness::Partial => {}
        }
        validate_evidence(&self.evidence)?;
        if let Some(supersedes) = &self.supersedes {
            validate_digest("superseded observation", supersedes)?;
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<SemanticDigest, ObservationError> {
        self.verify()?;
        let mut hasher = SemanticHasher::new(OBSERVATION_PROPOSAL_SCHEMA);
        hasher.add_bytes(&serde_json::to_vec(self)?);
        Ok(hasher.finish())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSource {
    pub locator: String,
    pub content_digest: SemanticDigest,
}

impl ObservationSource {
    #[must_use]
    pub fn workspace_file(locator: String, bytes: &[u8]) -> Self {
        let mut hasher = SemanticHasher::new("rey.observation-source.v1");
        hasher.add_bytes(bytes);
        Self {
            locator,
            content_digest: hasher.finish(),
        }
    }

    fn verify(&self) -> Result<(), ObservationError> {
        validate_locator("observation source", &self.locator)?;
        validate_digest("observation source", &self.content_digest)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub observation_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at_unix: i64,
    pub source: ObservationSource,
    pub limits: ObservationLimits,
    pub proposal: ObservationProposal,
}

impl Observation {
    fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_ADMISSION_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ObservationError::Sequence {
                kind: "observation",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.admitted_at_unix)?;
        self.source.verify()?;
        if self.limits != ObservationLimits::default() {
            return Err(ObservationError::LimitEnvelope);
        }
        if self.proposal.identity()? != self.observation_id {
            return Err(ObservationError::Identity("observation"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationResolutionKind {
    Resolved,
    Withdrawn,
}

impl ObservationResolutionKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationResolutionProposal {
    pub schema: String,
    pub observation_id: SemanticDigest,
    pub author: ObservationAuthor,
    pub kind: ObservationResolutionKind,
    pub reason: String,
    #[serde(default)]
    pub evidence: Vec<ObservationEvidenceBinding>,
}

impl ObservationResolutionProposal {
    pub fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA)?;
        validate_digest("resolved observation", &self.observation_id)?;
        self.author.verify()?;
        validate_text("resolution reason", &self.reason, MAX_REASON_BYTES)?;
        validate_evidence(&self.evidence)
    }

    fn identity(&self) -> Result<SemanticDigest, ObservationError> {
        self.verify()?;
        let mut hasher = SemanticHasher::new(OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA);
        hasher.add_bytes(&serde_json::to_vec(self)?);
        Ok(hasher.finish())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationResolution {
    pub schema: String,
    pub resolution_id: SemanticDigest,
    pub sequence: u64,
    pub resolved_at_unix: i64,
    pub source: ObservationSource,
    pub proposal: ObservationResolutionProposal,
}

impl ObservationResolution {
    fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_RESOLUTION_ADMISSION_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ObservationError::Sequence {
                kind: "resolution",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.resolved_at_unix)?;
        self.source.verify()?;
        if self.proposal.identity()? != self.resolution_id {
            return Err(ObservationError::Identity("resolution"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelObservationAdmission {
    pub schema: String,
    pub admission_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at_unix: i64,
    pub observation_id: SemanticDigest,
    pub channel_id: String,
    pub channel_head_commit_id: Option<SemanticDigest>,
    pub channel_graph_id: SemanticDigest,
}

impl ChannelObservationAdmission {
    fn new(
        sequence: u64,
        admitted_at_unix: i64,
        observation_id: SemanticDigest,
        channel_id: String,
        channel_head_commit_id: Option<SemanticDigest>,
        channel_graph_id: SemanticDigest,
    ) -> Result<Self, ObservationError> {
        let admission_id = channel_admission_identity(
            &observation_id,
            &channel_id,
            channel_head_commit_id.as_ref(),
            &channel_graph_id,
        );
        let admission = Self {
            schema: CHANNEL_OBSERVATION_ADMISSION_SCHEMA.to_owned(),
            admission_id,
            sequence,
            admitted_at_unix,
            observation_id,
            channel_id,
            channel_head_commit_id,
            channel_graph_id,
        };
        admission.verify()?;
        Ok(admission)
    }

    fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, CHANNEL_OBSERVATION_ADMISSION_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ObservationError::Sequence {
                kind: "channel admission",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.admitted_at_unix)?;
        validate_digest("admitted observation", &self.observation_id)?;
        validate_identifier("admission channel", &self.channel_id)?;
        if let Some(commit_id) = &self.channel_head_commit_id {
            validate_digest("channel HEAD commit", commit_id)?;
        }
        validate_digest("channel graph", &self.channel_graph_id)?;
        let actual = channel_admission_identity(
            &self.observation_id,
            &self.channel_id,
            self.channel_head_commit_id.as_ref(),
            &self.channel_graph_id,
        );
        if actual != self.admission_id {
            return Err(ObservationError::Identity("channel admission"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationBroadcastOutcome {
    Admitted,
    AlreadyAdmitted,
    UnknownChannel,
    RejectedKind,
}

impl ObservationBroadcastOutcome {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::AlreadyAdmitted => "already admitted",
            Self::UnknownChannel => "unknown channel",
            Self::RejectedKind => "rejected kind",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationBroadcastTarget {
    pub channel_id: String,
    pub outcome: ObservationBroadcastOutcome,
    pub admission: Option<ChannelObservationAdmission>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationBroadcastReceipt {
    pub schema: String,
    pub broadcast_id: SemanticDigest,
    pub request_id: SemanticDigest,
    pub sequence: u64,
    pub broadcast_at_unix: i64,
    pub observation_id: SemanticDigest,
    pub channel_head_commit_id: Option<SemanticDigest>,
    pub channel_graph_id: SemanticDigest,
    pub selected_channel_ids: Vec<String>,
    pub targets: Vec<ObservationBroadcastTarget>,
}

impl ObservationBroadcastReceipt {
    fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_BROADCAST_SCHEMA)?;
        if self.sequence == 0 {
            return Err(ObservationError::Sequence {
                kind: "broadcast",
                expected: 1,
                actual: 0,
            });
        }
        validate_timestamp(self.broadcast_at_unix)?;
        validate_digest("broadcast observation", &self.observation_id)?;
        validate_digest("broadcast request", &self.request_id)?;
        if let Some(commit_id) = &self.channel_head_commit_id {
            validate_digest("broadcast Channel HEAD", commit_id)?;
        }
        validate_digest("broadcast Channel graph", &self.channel_graph_id)?;
        if self.selected_channel_ids.is_empty()
            || canonical_targets(self.selected_channel_ids.clone())? != self.selected_channel_ids
            || self.targets.len() != self.selected_channel_ids.len()
            || self
                .targets
                .iter()
                .zip(&self.selected_channel_ids)
                .any(|(target, channel_id)| {
                    target.channel_id != *channel_id
                        || target.detail.is_empty()
                        || target.detail.len() > MAX_REASON_BYTES
                        || target.detail.trim() != target.detail
                        || match target.outcome {
                            ObservationBroadcastOutcome::Admitted
                            | ObservationBroadcastOutcome::AlreadyAdmitted => {
                                target.admission.as_ref().is_none_or(|admission| {
                                    admission.verify().is_err()
                                        || admission.channel_id != target.channel_id
                                        || admission.observation_id != self.observation_id
                                })
                            }
                            ObservationBroadcastOutcome::UnknownChannel
                            | ObservationBroadcastOutcome::RejectedKind => {
                                target.admission.is_some()
                            }
                        }
                })
        {
            return Err(ObservationError::BroadcastOutcome);
        }
        let actual_request = broadcast_request_identity(
            &self.observation_id,
            &self.selected_channel_ids,
            self.channel_head_commit_id.as_ref(),
            &self.channel_graph_id,
        );
        if actual_request != self.request_id
            || broadcast_identity(&self.request_id, &self.targets)? != self.broadcast_id
        {
            return Err(ObservationError::Identity("observation broadcast"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationBroadcast {
    pub schema: String,
    pub observation_admitted: bool,
    pub observation: Observation,
    pub broadcast: Option<ObservationBroadcastReceipt>,
    pub frontier: ObservationFrontier,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationState {
    Unresolved,
    Superseded,
    Resolved,
    Withdrawn,
}

impl ObservationState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unresolved => "unresolved",
            Self::Superseded => "superseded",
            Self::Resolved => "resolved",
            Self::Withdrawn => "withdrawn",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontierRow {
    pub observation: Observation,
    pub channel_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontierSummary {
    pub observations: u64,
    pub unresolved: u64,
    pub superseded: u64,
    pub resolved: u64,
    pub withdrawn: u64,
    pub unbroadcast: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrontier {
    pub schema: String,
    pub frontier_id: SemanticDigest,
    pub source_log_id: SemanticDigest,
    pub ordering: String,
    pub limit: u64,
    pub complete: bool,
    pub omitted: u64,
    pub summary: ObservationFrontierSummary,
    pub rows: Vec<ObservationFrontierRow>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationDetail {
    pub schema: String,
    pub state: ObservationState,
    pub observation: Observation,
    pub superseded_by: Option<SemanticDigest>,
    pub resolution: Option<ObservationResolution>,
    pub channel_admissions: Vec<ChannelObservationAdmission>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationResolutionAdmission {
    pub schema: String,
    pub admitted: bool,
    pub resolution: ObservationResolution,
    pub detail: ObservationDetail,
    pub frontier: ObservationFrontier,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLog {
    pub schema: String,
    pub log_id: SemanticDigest,
    pub observations: Vec<Observation>,
    pub channel_admissions: Vec<ChannelObservationAdmission>,
    pub broadcasts: Vec<ObservationBroadcastReceipt>,
    pub resolutions: Vec<ObservationResolution>,
}

impl Default for ObservationLog {
    fn default() -> Self {
        let mut log = Self {
            schema: OBSERVATION_LOG_SCHEMA.to_owned(),
            log_id: SemanticHasher::new(OBSERVATION_LOG_SCHEMA).finish(),
            observations: Vec::new(),
            channel_admissions: Vec::new(),
            broadcasts: Vec::new(),
            resolutions: Vec::new(),
        };
        log.log_id = log.identity().expect("empty observation log serializes");
        log
    }
}

impl ObservationLog {
    pub fn verify(&self) -> Result<(), ObservationError> {
        validate_schema(&self.schema, OBSERVATION_LOG_SCHEMA)?;
        if self.observations.len() > MAX_OBSERVATIONS {
            return Err(ObservationError::RecordLimit {
                kind: "observation",
                limit: MAX_OBSERVATIONS,
            });
        }
        if self.channel_admissions.len() > MAX_CHANNEL_ADMISSIONS {
            return Err(ObservationError::RecordLimit {
                kind: "channel admission",
                limit: MAX_CHANNEL_ADMISSIONS,
            });
        }
        if self.resolutions.len() > MAX_RESOLUTIONS {
            return Err(ObservationError::RecordLimit {
                kind: "resolution",
                limit: MAX_RESOLUTIONS,
            });
        }
        if self.broadcasts.len() > MAX_BROADCASTS {
            return Err(ObservationError::RecordLimit {
                kind: "broadcast",
                limit: MAX_BROADCASTS,
            });
        }

        let mut observation_ids = BTreeSet::new();
        let mut closed = BTreeSet::new();
        for (index, observation) in self.observations.iter().enumerate() {
            observation.verify()?;
            let expected = index as u64 + 1;
            if observation.sequence != expected {
                return Err(ObservationError::Sequence {
                    kind: "observation",
                    expected,
                    actual: observation.sequence,
                });
            }
            if !observation_ids.insert(observation.observation_id.clone()) {
                return Err(ObservationError::DuplicateObservation(
                    observation.observation_id.clone(),
                ));
            }
            if let Some(target) = &observation.proposal.supersedes {
                if !observation_ids.contains(target) {
                    return Err(ObservationError::UnknownObservation(target.clone()));
                }
                if !closed.insert(target.clone()) {
                    return Err(ObservationError::ObservationClosed(target.clone()));
                }
            }
        }

        let mut admission_ids = BTreeSet::new();
        let mut admitted_pairs = BTreeSet::new();
        for (index, admission) in self.channel_admissions.iter().enumerate() {
            admission.verify()?;
            let expected = index as u64 + 1;
            if admission.sequence != expected {
                return Err(ObservationError::Sequence {
                    kind: "channel admission",
                    expected,
                    actual: admission.sequence,
                });
            }
            if !observation_ids.contains(&admission.observation_id) {
                return Err(ObservationError::UnknownObservation(
                    admission.observation_id.clone(),
                ));
            }
            if !admission_ids.insert(admission.admission_id.clone())
                || !admitted_pairs.insert((
                    admission.observation_id.clone(),
                    admission.channel_id.clone(),
                ))
            {
                return Err(ObservationError::DuplicateChannelAdmission {
                    observation_id: admission.observation_id.clone(),
                    channel_id: admission.channel_id.clone(),
                });
            }
        }

        let mut resolution_ids = BTreeSet::new();
        for (index, resolution) in self.resolutions.iter().enumerate() {
            resolution.verify()?;
            let expected = index as u64 + 1;
            if resolution.sequence != expected {
                return Err(ObservationError::Sequence {
                    kind: "resolution",
                    expected,
                    actual: resolution.sequence,
                });
            }
            if !observation_ids.contains(&resolution.proposal.observation_id) {
                return Err(ObservationError::UnknownObservation(
                    resolution.proposal.observation_id.clone(),
                ));
            }
            if !resolution_ids.insert(resolution.resolution_id.clone()) {
                return Err(ObservationError::DuplicateResolution(
                    resolution.resolution_id.clone(),
                ));
            }
            if !closed.insert(resolution.proposal.observation_id.clone()) {
                return Err(ObservationError::ObservationClosed(
                    resolution.proposal.observation_id.clone(),
                ));
            }
        }
        let admission_ids = self
            .channel_admissions
            .iter()
            .map(|admission| admission.admission_id.clone())
            .collect::<BTreeSet<_>>();
        let mut broadcast_ids = BTreeSet::new();
        for (index, broadcast) in self.broadcasts.iter().enumerate() {
            broadcast.verify()?;
            let expected = index as u64 + 1;
            if broadcast.sequence != expected {
                return Err(ObservationError::Sequence {
                    kind: "broadcast",
                    expected,
                    actual: broadcast.sequence,
                });
            }
            if !observation_ids.contains(&broadcast.observation_id) {
                return Err(ObservationError::UnknownObservation(
                    broadcast.observation_id.clone(),
                ));
            }
            if !broadcast_ids.insert(broadcast.broadcast_id.clone()) {
                return Err(ObservationError::DuplicateBroadcast(
                    broadcast.broadcast_id.clone(),
                ));
            }
            for admission in broadcast
                .targets
                .iter()
                .filter_map(|target| target.admission.as_ref())
            {
                if !admission_ids.contains(&admission.admission_id) {
                    return Err(ObservationError::UnknownChannelAdmission(
                        admission.admission_id.clone(),
                    ));
                }
            }
        }
        if self.identity()? != self.log_id {
            return Err(ObservationError::Identity("observation log"));
        }
        Ok(())
    }

    pub fn frontier(&self, limit: usize) -> Result<ObservationFrontier, ObservationError> {
        self.verify()?;
        if limit == 0 || limit > MAX_FRONTIER_LIMIT {
            return Err(ObservationError::FrontierLimit {
                actual: limit,
                limit: MAX_FRONTIER_LIMIT,
            });
        }
        let superseded = self
            .observations
            .iter()
            .filter_map(|observation| {
                observation
                    .proposal
                    .supersedes
                    .as_ref()
                    .map(|target| (target.clone(), observation.observation_id.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let resolutions = self
            .resolutions
            .iter()
            .map(|resolution| {
                (
                    resolution.proposal.observation_id.clone(),
                    resolution.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut summary = ObservationFrontierSummary {
            observations: self.observations.len() as u64,
            ..ObservationFrontierSummary::default()
        };
        let mut unresolved = Vec::new();
        for observation in &self.observations {
            if superseded.contains_key(&observation.observation_id) {
                summary.superseded += 1;
                continue;
            }
            if let Some(resolution) = resolutions.get(&observation.observation_id) {
                match resolution.proposal.kind {
                    ObservationResolutionKind::Resolved => summary.resolved += 1,
                    ObservationResolutionKind::Withdrawn => summary.withdrawn += 1,
                }
                continue;
            }
            summary.unresolved += 1;
            let channel_ids = self
                .channel_admissions
                .iter()
                .filter(|admission| admission.observation_id == observation.observation_id)
                .map(|admission| admission.channel_id.clone())
                .collect::<Vec<_>>();
            if channel_ids.is_empty() {
                summary.unbroadcast += 1;
            }
            unresolved.push(ObservationFrontierRow {
                observation: observation.clone(),
                channel_ids,
            });
        }
        let omitted = unresolved.len().saturating_sub(limit) as u64;
        let rows = unresolved.into_iter().take(limit).collect::<Vec<_>>();
        let complete = omitted == 0;
        let mut hasher = SemanticHasher::new(OBSERVATION_FRONTIER_SCHEMA);
        hasher.add_str(self.log_id.as_str());
        hasher.add_u64(limit as u64);
        hasher.add_bool(complete);
        hasher.add_u64(omitted);
        for row in &rows {
            hasher.add_str(row.observation.observation_id.as_str());
            for channel_id in &row.channel_ids {
                hasher.add_str(channel_id);
            }
        }
        Ok(ObservationFrontier {
            schema: OBSERVATION_FRONTIER_SCHEMA.to_owned(),
            frontier_id: hasher.finish(),
            source_log_id: self.log_id.clone(),
            ordering: "observation_sequence_ascending".to_owned(),
            limit: limit as u64,
            complete,
            omitted,
            summary,
            rows,
        })
    }

    pub fn detail(&self, observation_id: &str) -> Result<ObservationDetail, ObservationError> {
        self.verify()?;
        let observation = self
            .observations
            .iter()
            .find(|observation| observation.observation_id.as_str() == observation_id)
            .cloned()
            .ok_or_else(|| ObservationError::UnknownObservationText(observation_id.to_owned()))?;
        let superseded_by = self
            .observations
            .iter()
            .find(|candidate| {
                candidate.proposal.supersedes.as_ref() == Some(&observation.observation_id)
            })
            .map(|candidate| candidate.observation_id.clone());
        let resolution = self
            .resolutions
            .iter()
            .find(|resolution| resolution.proposal.observation_id == observation.observation_id)
            .cloned();
        let state = match (&superseded_by, &resolution) {
            (Some(_), None) => ObservationState::Superseded,
            (None, Some(resolution)) => match resolution.proposal.kind {
                ObservationResolutionKind::Resolved => ObservationState::Resolved,
                ObservationResolutionKind::Withdrawn => ObservationState::Withdrawn,
            },
            (None, None) => ObservationState::Unresolved,
            (Some(_), Some(_)) => {
                return Err(ObservationError::ObservationClosed(
                    observation.observation_id,
                ));
            }
        };
        let channel_admissions = self
            .channel_admissions
            .iter()
            .filter(|admission| admission.observation_id == observation.observation_id)
            .cloned()
            .collect();
        Ok(ObservationDetail {
            schema: OBSERVATION_DETAIL_SCHEMA.to_owned(),
            state,
            observation,
            superseded_by,
            resolution,
            channel_admissions,
        })
    }

    fn identity(&self) -> Result<SemanticDigest, ObservationError> {
        let mut hasher = SemanticHasher::new(OBSERVATION_LOG_SCHEMA);
        hasher.add_bytes(&serde_json::to_vec(&self.observations)?);
        hasher.add_bytes(&serde_json::to_vec(&self.channel_admissions)?);
        hasher.add_bytes(&serde_json::to_vec(&self.broadcasts)?);
        hasher.add_bytes(&serde_json::to_vec(&self.resolutions)?);
        Ok(hasher.finish())
    }

    fn refresh_identity(&mut self) -> Result<(), ObservationError> {
        self.log_id = self.identity()?;
        self.verify()
    }
}

#[derive(Clone, Debug)]
pub struct LocalObservationStore {
    directory: PathBuf,
}

impl LocalObservationStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("channels"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<ObservationLog, ObservationError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ObservationLog::default());
            }
            Err(source) => return Err(ObservationError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ObservationError::UnsafePath(path));
        }
        if metadata.len() > MAX_OBSERVATION_STATE_BYTES {
            return Err(ObservationError::ByteLimit(MAX_OBSERVATION_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| ObservationError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_OBSERVATION_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| ObservationError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_OBSERVATION_STATE_BYTES {
            return Err(ObservationError::ByteLimit(MAX_OBSERVATION_STATE_BYTES));
        }
        let log: ObservationLog = serde_json::from_slice(&bytes)?;
        log.verify()?;
        Ok(log)
    }

    pub fn admit_and_broadcast(
        &self,
        proposal: ObservationProposal,
        source: ObservationSource,
        channel_ids: Vec<String>,
        channel_head_commit_id: Option<SemanticDigest>,
        channel_graph: &ChannelGraphSnapshot,
        admitted_at_unix: i64,
    ) -> Result<ObservationBroadcast, ObservationError> {
        proposal.verify()?;
        source.verify()?;
        channel_graph
            .verify()
            .map_err(|error| ObservationError::ChannelGraph(error.to_string()))?;
        if let Some(commit_id) = &channel_head_commit_id {
            validate_digest("channel HEAD commit", commit_id)?;
        }
        validate_timestamp(admitted_at_unix)?;
        let selected_channel_ids = canonical_targets(channel_ids)?;
        self.with_lock(|| {
            let mut log = self.load()?;
            let observation_id = proposal.identity()?;
            let (observation, observation_admitted) = if let Some(observation) = log
                .observations
                .iter()
                .find(|observation| observation.observation_id == observation_id)
            {
                (observation.clone(), false)
            } else {
                if log.observations.len() >= MAX_OBSERVATIONS {
                    return Err(ObservationError::RecordLimit {
                        kind: "observation",
                        limit: MAX_OBSERVATIONS,
                    });
                }
                if let Some(supersedes) = &proposal.supersedes {
                    log.detail(supersedes.as_str())?;
                    ensure_open(&log, supersedes)?;
                }
                let observation = Observation {
                    schema: OBSERVATION_ADMISSION_SCHEMA.to_owned(),
                    observation_id,
                    sequence: log.observations.len() as u64 + 1,
                        admitted_at_unix,
                        source,
                        limits: ObservationLimits::default(),
                        proposal,
                };
                observation.verify()?;
                log.observations.push(observation.clone());
                (observation, true)
            };

            let mut changed = observation_admitted;
            let broadcast = if selected_channel_ids.is_empty() {
                None
            } else {
                let request_id = broadcast_request_identity(
                    &observation.observation_id,
                    &selected_channel_ids,
                    channel_head_commit_id.as_ref(),
                    &channel_graph.graph_id,
                );
                if let Some(existing) = log
                    .broadcasts
                    .iter()
                    .find(|broadcast| broadcast.request_id == request_id)
                {
                    Some(existing.clone())
                } else {
                    if log.broadcasts.len() >= MAX_BROADCASTS {
                        return Err(ObservationError::RecordLimit {
                            kind: "broadcast",
                            limit: MAX_BROADCASTS,
                        });
                    }
                    let mut targets = Vec::with_capacity(selected_channel_ids.len());
                    for channel_id in &selected_channel_ids {
                        if let Some(existing) =
                            log.channel_admissions.iter().find(|admission| {
                                admission.observation_id == observation.observation_id
                                    && admission.channel_id == *channel_id
                            })
                        {
                            targets.push(ObservationBroadcastTarget {
                                channel_id: channel_id.clone(),
                                outcome: ObservationBroadcastOutcome::AlreadyAdmitted,
                                admission: Some(existing.clone()),
                                detail: "the exact observation identity is already admitted to this channel"
                                    .to_owned(),
                            });
                            continue;
                        }
                        let Some(channel) = channel_graph
                            .graph
                            .channels
                            .iter()
                            .find(|channel| channel.id == *channel_id)
                        else {
                            targets.push(ObservationBroadcastTarget {
                                channel_id: channel_id.clone(),
                                outcome: ObservationBroadcastOutcome::UnknownChannel,
                                admission: None,
                                detail: format!(
                                    "channel is absent from graph {}",
                                    channel_graph.graph_id
                                ),
                            });
                            continue;
                        };
                        if !channel
                            .accepted_observation_kinds
                            .contains(&observation.proposal.kind)
                        {
                            targets.push(ObservationBroadcastTarget {
                                channel_id: channel_id.clone(),
                                outcome: ObservationBroadcastOutcome::RejectedKind,
                                admission: None,
                                detail: format!(
                                    "channel does not accept {} observations",
                                    observation.proposal.kind.label()
                                ),
                            });
                            continue;
                        }
                        if log.channel_admissions.len() >= MAX_CHANNEL_ADMISSIONS {
                            return Err(ObservationError::RecordLimit {
                                kind: "channel admission",
                                limit: MAX_CHANNEL_ADMISSIONS,
                            });
                        }
                        let admission = ChannelObservationAdmission::new(
                            log.channel_admissions.len() as u64 + 1,
                            admitted_at_unix,
                            observation.observation_id.clone(),
                            channel_id.clone(),
                            channel_head_commit_id.clone(),
                            channel_graph.graph_id.clone(),
                        )?;
                        log.channel_admissions.push(admission.clone());
                        targets.push(ObservationBroadcastTarget {
                            channel_id: channel_id.clone(),
                            outcome: ObservationBroadcastOutcome::Admitted,
                            admission: Some(admission),
                            detail: "admitted locally; no relay or execution authority granted"
                                .to_owned(),
                        });
                    }
                    let receipt = ObservationBroadcastReceipt {
                        schema: OBSERVATION_BROADCAST_SCHEMA.to_owned(),
                        broadcast_id: broadcast_identity(&request_id, &targets)?,
                        request_id,
                        sequence: log.broadcasts.len() as u64 + 1,
                        broadcast_at_unix: admitted_at_unix,
                        observation_id: observation.observation_id.clone(),
                        channel_head_commit_id: channel_head_commit_id.clone(),
                        channel_graph_id: channel_graph.graph_id.clone(),
                        selected_channel_ids: selected_channel_ids.clone(),
                        targets,
                    };
                    receipt.verify()?;
                    log.broadcasts.push(receipt.clone());
                    changed = true;
                    Some(receipt)
                }
            };
            if changed {
                log.refresh_identity()?;
                self.save(&log)?;
            }
            Ok(ObservationBroadcast {
                schema: OBSERVATION_ADMISSION_RESULT_SCHEMA.to_owned(),
                observation_admitted,
                observation,
                broadcast,
                frontier: log.frontier(DEFAULT_OBSERVATION_FRONTIER_LIMIT)?,
            })
        })
    }

    pub fn resolve(
        &self,
        proposal: ObservationResolutionProposal,
        source: ObservationSource,
        resolved_at_unix: i64,
    ) -> Result<ObservationResolutionAdmission, ObservationError> {
        proposal.verify()?;
        source.verify()?;
        validate_timestamp(resolved_at_unix)?;
        self.with_lock(|| {
            let mut log = self.load()?;
            log.detail(proposal.observation_id.as_str())?;
            let resolution_id = proposal.identity()?;
            let (resolution, admitted) = if let Some(resolution) = log
                .resolutions
                .iter()
                .find(|resolution| resolution.resolution_id == resolution_id)
            {
                (resolution.clone(), false)
            } else {
                ensure_open(&log, &proposal.observation_id)?;
                if log.resolutions.len() >= MAX_RESOLUTIONS {
                    return Err(ObservationError::RecordLimit {
                        kind: "resolution",
                        limit: MAX_RESOLUTIONS,
                    });
                }
                let resolution = ObservationResolution {
                    schema: OBSERVATION_RESOLUTION_ADMISSION_SCHEMA.to_owned(),
                    resolution_id,
                    sequence: log.resolutions.len() as u64 + 1,
                    resolved_at_unix,
                    source,
                    proposal,
                };
                resolution.verify()?;
                log.resolutions.push(resolution.clone());
                log.refresh_identity()?;
                self.save(&log)?;
                (resolution, true)
            };
            Ok(ObservationResolutionAdmission {
                schema: OBSERVATION_RESOLUTION_RESULT_SCHEMA.to_owned(),
                admitted,
                detail: log.detail(resolution.proposal.observation_id.as_str())?,
                resolution,
                frontier: log.frontier(DEFAULT_OBSERVATION_FRONTIER_LIMIT)?,
            })
        })
    }

    fn save(&self, log: &ObservationLog) -> Result<(), ObservationError> {
        log.verify()?;
        let bytes = serde_json::to_vec_pretty(log)?;
        if bytes.len().saturating_add(1) as u64 > MAX_OBSERVATION_STATE_BYTES {
            return Err(ObservationError::ByteLimit(MAX_OBSERVATION_STATE_BYTES));
        }
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ObservationError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(ObservationError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ObservationError>,
    ) -> Result<T, ObservationError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(ObservationError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| ObservationError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| ObservationError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = operation();
        let unlock = File::unlock(&lock).map_err(|source| ObservationError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn prepare_directory(&self) -> Result<(), ObservationError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(ObservationError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| ObservationError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(ObservationError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), ObservationError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(ObservationError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(ObservationError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(ObservationError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), ObservationError> {
        for attempt in 0..32_u8 {
            let path = self.directory.join(format!(
                ".{STATE_FILE_NAME}.tmp-{}-{attempt}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(ObservationError::Write { path, source }),
            }
        }
        Err(ObservationError::TemporaryLimit(self.directory.clone()))
    }
}

fn ensure_open(
    log: &ObservationLog,
    observation_id: &SemanticDigest,
) -> Result<(), ObservationError> {
    let detail = log.detail(observation_id.as_str())?;
    if detail.state != ObservationState::Unresolved {
        return Err(ObservationError::ObservationClosed(observation_id.clone()));
    }
    Ok(())
}

fn canonical_targets(mut channel_ids: Vec<String>) -> Result<Vec<String>, ObservationError> {
    if channel_ids.len() > MAX_BROADCAST_TARGETS {
        return Err(ObservationError::BroadcastTargetLimit {
            actual: channel_ids.len(),
            limit: MAX_BROADCAST_TARGETS,
        });
    }
    for channel_id in &channel_ids {
        validate_identifier("broadcast channel", channel_id)?;
    }
    channel_ids.sort();
    if channel_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ObservationError::DuplicateBroadcastTarget);
    }
    Ok(channel_ids)
}

fn validate_evidence(evidence: &[ObservationEvidenceBinding]) -> Result<(), ObservationError> {
    if evidence.len() > MAX_EVIDENCE_BINDINGS {
        return Err(ObservationError::EvidenceLimit {
            actual: evidence.len(),
            limit: MAX_EVIDENCE_BINDINGS,
        });
    }
    let mut bindings = BTreeSet::new();
    for binding in evidence {
        binding.verify()?;
        let key = (
            binding.locator.as_str(),
            binding.source_revision.as_str(),
            binding.content_digest.as_str(),
        );
        if !bindings.insert(key) {
            return Err(ObservationError::DuplicateEvidence(binding.locator.clone()));
        }
    }
    Ok(())
}

fn channel_admission_identity(
    observation_id: &SemanticDigest,
    channel_id: &str,
    channel_head_commit_id: Option<&SemanticDigest>,
    channel_graph_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CHANNEL_OBSERVATION_ADMISSION_SCHEMA);
    hasher.add_str(observation_id.as_str());
    hasher.add_str(channel_id);
    hasher.add_optional_str(channel_head_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(channel_graph_id.as_str());
    hasher.finish()
}

fn broadcast_request_identity(
    observation_id: &SemanticDigest,
    channel_ids: &[String],
    channel_head_commit_id: Option<&SemanticDigest>,
    channel_graph_id: &SemanticDigest,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(OBSERVATION_BROADCAST_SCHEMA);
    hasher.add_str(observation_id.as_str());
    hasher.add_u64(channel_ids.len() as u64);
    for channel_id in channel_ids {
        hasher.add_str(channel_id);
    }
    hasher.add_optional_str(channel_head_commit_id.map(SemanticDigest::as_str));
    hasher.add_str(channel_graph_id.as_str());
    hasher.finish()
}

fn broadcast_identity(
    request_id: &SemanticDigest,
    targets: &[ObservationBroadcastTarget],
) -> Result<SemanticDigest, ObservationError> {
    let mut hasher = SemanticHasher::new("rey.observation-broadcast-receipt.v1");
    hasher.add_str(request_id.as_str());
    hasher.add_bytes(&serde_json::to_vec(targets)?);
    Ok(hasher.finish())
}

fn validate_schema(actual: &str, expected: &'static str) -> Result<(), ObservationError> {
    if actual != expected {
        return Err(ObservationError::Schema {
            expected,
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), ObservationError> {
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
        return Err(ObservationError::Identifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_locator(field: &'static str, value: &str) -> Result<(), ObservationError> {
    if value.is_empty()
        || value.len() > MAX_LOCATOR_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ObservationError::Locator {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_revision(field: &'static str, value: &str) -> Result<(), ObservationError> {
    if value.is_empty()
        || value.len() > MAX_REVISION_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ObservationError::Revision {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_text(field: &'static str, value: &str, limit: usize) -> Result<(), ObservationError> {
    if value.is_empty() || value.len() > limit || value.trim() != value || value.contains('\0') {
        return Err(ObservationError::Text { field, limit });
    }
    Ok(())
}

fn validate_digest(field: &'static str, value: &SemanticDigest) -> Result<(), ObservationError> {
    let value = value.as_str();
    if value.len() != 71
        || !value.starts_with("blake3:")
        || !value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ObservationError::Digest {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_timestamp(value: i64) -> Result<(), ObservationError> {
    if value < 0 {
        return Err(ObservationError::Timestamp(value));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ObservationError {
    #[error("expected schema {expected}, found {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid {field} identifier {value:?}")]
    Identifier { field: &'static str, value: String },
    #[error("invalid bounded {field} locator {value:?}")]
    Locator { field: &'static str, value: String },
    #[error("invalid bounded {field} revision {value:?}")]
    Revision { field: &'static str, value: String },
    #[error("invalid {field} digest {value:?}")]
    Digest { field: &'static str, value: String },
    #[error("{field} is empty, non-canonical, or exceeds the {limit}-byte limit")]
    Text { field: &'static str, limit: usize },
    #[error("observation evidence has {actual} bindings; limit is {limit}")]
    EvidenceLimit { actual: usize, limit: usize },
    #[error("observation has {actual} omissions; limit is {limit}")]
    OmissionLimit { actual: usize, limit: usize },
    #[error("observation repeats evidence binding {0}")]
    DuplicateEvidence(String),
    #[error("observation repeats omission {0}")]
    DuplicateOmission(String),
    #[error("complete observations cannot retain omissions and partial observations require one")]
    Completeness,
    #[error("observation uses an unsupported effective limit envelope")]
    LimitEnvelope,
    #[error("broadcast target set has {actual} channels; limit is {limit}")]
    BroadcastTargetLimit { actual: usize, limit: usize },
    #[error("broadcast target set repeats a channel")]
    DuplicateBroadcastTarget,
    #[error("{kind} sequence must be {expected}, got {actual}")]
    Sequence {
        kind: &'static str,
        expected: u64,
        actual: u64,
    },
    #[error("{0} identity does not match its retained content")]
    Identity(&'static str),
    #[error("observation timestamp {0} is outside the supported range")]
    Timestamp(i64),
    #[error("observation log exceeds the {limit}-record {kind} limit")]
    RecordLimit { kind: &'static str, limit: usize },
    #[error("duplicate observation {0}")]
    DuplicateObservation(SemanticDigest),
    #[error("duplicate resolution {0}")]
    DuplicateResolution(SemanticDigest),
    #[error("duplicate observation broadcast {0}")]
    DuplicateBroadcast(SemanticDigest),
    #[error("observation broadcast target outcomes are incomplete or inconsistent")]
    BroadcastOutcome,
    #[error("observation broadcast references unknown channel admission {0}")]
    UnknownChannelAdmission(SemanticDigest),
    #[error("observation {observation_id} is already admitted to channel {channel_id}")]
    DuplicateChannelAdmission {
        observation_id: SemanticDigest,
        channel_id: String,
    },
    #[error("unknown observation {0}")]
    UnknownObservation(SemanticDigest),
    #[error("unknown observation {0}")]
    UnknownObservationText(String),
    #[error("observation {0} is already resolved or superseded")]
    ObservationClosed(SemanticDigest),
    #[error("frontier limit must be between 1 and {limit}, got {actual}")]
    FrontierLimit { actual: usize, limit: usize },
    #[error("invalid Channel graph for observation broadcast: {0}")]
    ChannelGraph(String),
    #[error("observation state exceeds {0} bytes")]
    ByteLimit(u64),
    #[error("unsafe symlink or file type in observation state path {0}")]
    UnsafePath(PathBuf),
    #[error("could not read observation state {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not write observation state {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not lock observation state {path}: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not allocate a temporary observation state file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_core::SemanticHasher;
    use tempfile::TempDir;

    use super::{
        LocalObservationStore, OBSERVATION_PROPOSAL_SCHEMA, OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA,
        ObservationAuthor, ObservationAuthorKind, ObservationBroadcastOutcome,
        ObservationCompleteness, ObservationEvidenceBinding, ObservationProposal,
        ObservationResolutionKind, ObservationResolutionProposal, ObservationSource,
        ObservationState,
    };
    use crate::channels::{
        ChannelDefinition, ChannelGraph, ChannelGraphSnapshot, ChannelGraphSource,
        ChannelObservationKind, ChannelScope,
    };

    #[test]
    fn broadcast_is_idempotent_and_retains_typed_partial_target_outcomes() {
        let directory = TempDir::new().unwrap();
        let store = LocalObservationStore::new(directory.path().join("channels"));
        let graph = broadcast_graph();
        let proposal = proposal(ChannelObservationKind::Blocker, None);
        let bytes = serde_json::to_vec(&proposal).unwrap();
        let source = ObservationSource::workspace_file("observation.yaml".to_owned(), &bytes);

        let first = store
            .admit_and_broadcast(
                proposal.clone(),
                source.clone(),
                vec![
                    "workspace".to_owned(),
                    "review".to_owned(),
                    "missing".to_owned(),
                ],
                None,
                &graph,
                1_000,
            )
            .unwrap();
        assert!(first.observation_admitted);
        let first_broadcast = first.broadcast.as_ref().unwrap();
        assert_eq!(
            first_broadcast
                .targets
                .iter()
                .map(|target| (target.channel_id.as_str(), target.outcome))
                .collect::<Vec<_>>(),
            [
                ("missing", ObservationBroadcastOutcome::UnknownChannel),
                ("review", ObservationBroadcastOutcome::RejectedKind),
                ("workspace", ObservationBroadcastOutcome::Admitted),
            ]
        );
        assert_eq!(first.frontier.summary.unresolved, 1);
        assert_eq!(first.frontier.rows[0].channel_ids, ["workspace"]);

        let replay = store
            .admit_and_broadcast(
                proposal,
                source,
                vec![
                    "workspace".to_owned(),
                    "review".to_owned(),
                    "missing".to_owned(),
                ],
                None,
                &graph,
                1_001,
            )
            .unwrap();
        assert!(!replay.observation_admitted);
        assert_eq!(
            replay.broadcast.as_ref().unwrap().broadcast_id,
            first_broadcast.broadcast_id
        );
        assert_eq!(
            replay.broadcast.as_ref().unwrap().targets[2].outcome,
            ObservationBroadcastOutcome::Admitted
        );
        assert_eq!(store.load().unwrap().observations.len(), 1);
        assert_eq!(store.load().unwrap().channel_admissions.len(), 1);
        assert_eq!(store.load().unwrap().broadcasts.len(), 1);
    }

    #[test]
    fn supersession_and_resolution_close_the_bounded_frontier_once() {
        let directory = TempDir::new().unwrap();
        let store = LocalObservationStore::new(directory.path().join("channels"));
        let graph = ChannelGraphSnapshot::built_in().unwrap();
        let first = admit(
            &store,
            &graph,
            proposal(ChannelObservationKind::Question, None),
            10,
        );
        let correction = admit(
            &store,
            &graph,
            proposal(
                ChannelObservationKind::Finding,
                Some(first.observation.observation_id.clone()),
            ),
            11,
        );
        let source = ObservationSource::workspace_file("resolution.yaml".to_owned(), b"resolution");
        let resolution = store
            .resolve(
                ObservationResolutionProposal {
                    schema: OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA.to_owned(),
                    observation_id: correction.observation.observation_id.clone(),
                    author: fixture_author(),
                    kind: ObservationResolutionKind::Resolved,
                    reason: "The corrected evidence was reviewed.".to_owned(),
                    evidence: Vec::new(),
                },
                source,
                12,
            )
            .unwrap();

        assert_eq!(resolution.detail.state, ObservationState::Resolved);
        assert_eq!(resolution.frontier.summary.superseded, 1);
        assert_eq!(resolution.frontier.summary.resolved, 1);
        assert_eq!(resolution.frontier.summary.unresolved, 0);
        assert!(resolution.frontier.rows.is_empty());
        assert!(
            store
                .resolve(
                    ObservationResolutionProposal {
                        schema: OBSERVATION_RESOLUTION_PROPOSAL_SCHEMA.to_owned(),
                        observation_id: first.observation.observation_id,
                        author: fixture_author(),
                        kind: ObservationResolutionKind::Withdrawn,
                        reason: "Cannot close a superseded observation twice.".to_owned(),
                        evidence: Vec::new(),
                    },
                    ObservationSource::workspace_file("other.yaml".to_owned(), b"other"),
                    13,
                )
                .is_err()
        );
    }

    #[test]
    fn restart_tamper_and_symlink_boundaries_fail_closed() {
        let directory = TempDir::new().unwrap();
        let state = directory.path().join("channels");
        let store = LocalObservationStore::new(state.clone());
        let graph = ChannelGraphSnapshot::built_in().unwrap();
        let admitted = admit(
            &store,
            &graph,
            proposal(ChannelObservationKind::Progress, None),
            20,
        );
        let restarted = LocalObservationStore::new(state.clone());
        assert_eq!(
            restarted
                .load()
                .unwrap()
                .detail(admitted.observation.observation_id.as_str())
                .unwrap()
                .state,
            ObservationState::Unresolved
        );

        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        document["observations"][0]["proposal"]["body"] = "tampered".into();
        fs::write(store.path(), serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        assert!(restarted.load().is_err());

        let unsafe_directory = TempDir::new().unwrap();
        let outside = unsafe_directory.path().join("outside.json");
        fs::write(&outside, b"{}\n").unwrap();
        let state = unsafe_directory.path().join("state");
        fs::create_dir(&state).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, state.join("observations.json")).unwrap();
            assert!(LocalObservationStore::new(state).load().is_err());
        }
    }

    fn admit(
        store: &LocalObservationStore,
        graph: &ChannelGraphSnapshot,
        proposal: ObservationProposal,
        timestamp: i64,
    ) -> super::ObservationBroadcast {
        let bytes = serde_json::to_vec(&proposal).unwrap();
        store
            .admit_and_broadcast(
                proposal,
                ObservationSource::workspace_file(format!("observation-{timestamp}.yaml"), &bytes),
                Vec::new(),
                None,
                graph,
                timestamp,
            )
            .unwrap()
    }

    fn proposal(
        kind: ChannelObservationKind,
        supersedes: Option<rey_core::SemanticDigest>,
    ) -> ObservationProposal {
        let mut hasher = SemanticHasher::new("fixture.evidence.v1");
        hasher.add_str(kind.label());
        ObservationProposal {
            schema: OBSERVATION_PROPOSAL_SCHEMA.to_owned(),
            kind,
            author: fixture_author(),
            subject_locator: "rey+local://portfolio/current?revision=fixture%401".to_owned(),
            body: format!("Bounded {} observation.", kind.label()),
            desired_delta: Some("Resolve the exact collaboration frontier relation.".to_owned()),
            completeness: ObservationCompleteness::Complete,
            omissions: Vec::new(),
            evidence: vec![ObservationEvidenceBinding {
                locator: "rey+local://evidence/example".to_owned(),
                source_revision: "fixture@1".to_owned(),
                content_digest: hasher.finish(),
            }],
            supersedes,
        }
    }

    fn fixture_author() -> ObservationAuthor {
        ObservationAuthor {
            kind: ObservationAuthorKind::Agent,
            id: "fixture-agent".to_owned(),
        }
    }

    fn broadcast_graph() -> ChannelGraphSnapshot {
        let mut graph = ChannelGraph::built_in().unwrap();
        graph.channels.push(ChannelDefinition {
            id: "review".to_owned(),
            revision: 1,
            name: "Review".to_owned(),
            scope: ChannelScope::WorkspaceLocal,
            accepted_observation_kinds: vec![ChannelObservationKind::Finding],
            broadcast_default: false,
        });
        let graph = graph.canonicalize().unwrap();
        let bytes = serde_json::to_vec(&graph).unwrap();
        ChannelGraphSnapshot::new(
            graph,
            ChannelGraphSource::worktree("fixture://channels".to_owned(), &bytes),
        )
        .unwrap()
    }
}
