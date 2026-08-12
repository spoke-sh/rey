#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    journal::{
        JOURNAL_BROADSHEET_COLUMNS, JOURNAL_PROPOSAL_SCHEMA, JournalAuthor, JournalBinding,
        JournalBlock, JournalEntryProposal, JournalError, JournalLayout, JournalLayoutBand,
        JournalLayoutCell, JournalLayoutKind, JournalProseKind, JournalProseNode,
        MAX_JOURNAL_PROPOSAL_BYTES,
    },
    observations::{ObservationDetail, ObservationError, ObservationLog, ObservationState},
};

pub const JOURNAL_SEED_SCHEMA: &str = "rey.journal-seed.v1";
pub const MAX_JOURNAL_SEED_OBSERVATIONS: usize = 16;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalSeed {
    pub schema: String,
    pub seed_id: SemanticDigest,
    pub source_log_id: SemanticDigest,
    pub observation_ids: Vec<SemanticDigest>,
    pub proposal: JournalEntryProposal,
}

impl JournalSeed {
    pub fn from_log(
        log: &ObservationLog,
        observation_ids: &[String],
        author: JournalAuthor,
    ) -> Result<Self, JournalSeedError> {
        log.verify()?;
        if observation_ids.is_empty() {
            return Err(JournalSeedError::Empty);
        }
        if observation_ids.len() > MAX_JOURNAL_SEED_OBSERVATIONS {
            return Err(JournalSeedError::Limit {
                actual: observation_ids.len(),
                limit: MAX_JOURNAL_SEED_OBSERVATIONS,
            });
        }
        let mut selected = Vec::with_capacity(observation_ids.len());
        let mut unique = BTreeSet::new();
        for observation_id in observation_ids {
            if !unique.insert(observation_id.as_str()) {
                return Err(JournalSeedError::Duplicate(observation_id.clone()));
            }
            let detail = log.detail(observation_id)?;
            if detail.state != ObservationState::Unresolved {
                return Err(JournalSeedError::Closed {
                    observation_id: observation_id.clone(),
                    state: detail.state.label(),
                });
            }
            selected.push(detail);
        }
        selected.sort_by_key(|detail| detail.observation.sequence);
        let proposal = proposal_from_observations(&log.log_id, &selected, author);
        proposal.validate()?;
        let proposal_bytes = serde_json::to_vec(&proposal)?;
        if proposal_bytes.len() as u64 > MAX_JOURNAL_PROPOSAL_BYTES {
            return Err(JournalSeedError::ProposalByteLimit {
                actual: proposal_bytes.len() as u64,
                limit: MAX_JOURNAL_PROPOSAL_BYTES,
            });
        }
        let canonical_ids = selected
            .iter()
            .map(|detail| detail.observation.observation_id.clone())
            .collect::<Vec<_>>();
        let seed_id = seed_identity(&log.log_id, &canonical_ids, &proposal_bytes);
        Ok(Self {
            schema: JOURNAL_SEED_SCHEMA.to_owned(),
            seed_id,
            source_log_id: log.log_id.clone(),
            observation_ids: canonical_ids,
            proposal,
        })
    }

    pub fn verify_against(&self, log: &ObservationLog) -> Result<(), JournalSeedError> {
        if self.schema != JOURNAL_SEED_SCHEMA {
            return Err(JournalSeedError::Schema(self.schema.clone()));
        }
        if self.source_log_id != log.log_id {
            return Err(JournalSeedError::SourceLog);
        }
        let regenerated = Self::from_log(
            log,
            &self
                .observation_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            self.proposal.author.clone(),
        )?;
        if &regenerated != self {
            return Err(JournalSeedError::Identity);
        }
        Ok(())
    }
}

fn proposal_from_observations(
    source_log_id: &SemanticDigest,
    selected: &[ObservationDetail],
    author: JournalAuthor,
) -> JournalEntryProposal {
    let mut blocks = Vec::with_capacity(selected.len() * 2);
    let mut bands = Vec::with_capacity(selected.len());
    for detail in selected {
        let observation = &detail.observation;
        let context_id = format!("observation-{}-context", observation.sequence);
        let source_id = format!("observation-{}-source", observation.sequence);
        let proposal = &observation.proposal;
        let mut document = vec![
            JournalProseNode {
                kind: JournalProseKind::Heading,
                text: format!(
                    "{} observation O@{}",
                    observation_kind(proposal.kind),
                    observation.sequence
                ),
            },
            JournalProseNode {
                kind: JournalProseKind::Paragraph,
                text: proposal.body.clone(),
            },
            JournalProseNode {
                kind: JournalProseKind::Paragraph,
                text: format!("Subject: {}", proposal.subject_locator),
            },
        ];
        if let Some(desired_delta) = &proposal.desired_delta {
            document.push(JournalProseNode {
                kind: JournalProseKind::Paragraph,
                text: format!("Desired delta: {desired_delta}"),
            });
        }
        document.push(JournalProseNode {
            kind: JournalProseKind::Paragraph,
            text: format!(
                "Coverage: {} · {} evidence bindings · {} omissions",
                completeness(proposal.completeness),
                proposal.evidence.len(),
                proposal.omissions.len()
            ),
        });
        document.extend(proposal.omissions.iter().map(|omission| JournalProseNode {
            kind: JournalProseKind::Bullet,
            text: format!("Omission: {omission}"),
        }));
        document.extend(proposal.evidence.iter().map(|evidence| JournalProseNode {
            kind: JournalProseKind::Code,
            text: format!(
                "Evidence: {}\nSource revision: {}\nContent digest: {}",
                evidence.locator, evidence.source_revision, evidence.content_digest
            ),
        }));
        let mut channel_ids = detail
            .channel_admissions
            .iter()
            .map(|admission| admission.channel_id.as_str())
            .collect::<Vec<_>>();
        channel_ids.sort_unstable();
        channel_ids.dedup();
        document.push(JournalProseNode {
            kind: JournalProseKind::Code,
            text: format!(
                "Observation: {}\nObservation log: {}\nSource: {}\nSource digest: {}\nChannels: {}",
                observation.observation_id,
                source_log_id,
                observation.source.locator,
                observation.source.content_digest,
                if channel_ids.is_empty() {
                    "unbroadcast".to_owned()
                } else {
                    channel_ids.join(", ")
                }
            ),
        });
        blocks.push(JournalBlock::Prose {
            id: context_id.clone(),
            document,
        });
        blocks.push(JournalBlock::Explore {
            id: source_id.clone(),
            coordinate: document_coordinate(
                &format!("observation-{}", observation.observation_id),
                observation.observation_id.as_str(),
            ),
            scale: 1.0,
            source_revision: observation.observation_id.to_string(),
            caption: Some(format!(
                "Exact {} observation O@{}",
                observation_kind(proposal.kind).to_lowercase(),
                observation.sequence
            )),
        });
        bands.push(JournalLayoutBand {
            id: format!("observation-{}", observation.sequence),
            cells: vec![
                JournalLayoutCell {
                    block_id: context_id,
                    span: 8,
                },
                JournalLayoutCell {
                    block_id: source_id,
                    span: 4,
                },
            ],
        });
    }
    JournalEntryProposal {
        schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
        title: if selected.len() == 1 {
            "Catch up on 1 unresolved observation".to_owned()
        } else {
            format!("Catch up on {} unresolved observations", selected.len())
        },
        author,
        binding: JournalBinding {
            coordinate: document_coordinate("observation-frontier", source_log_id.as_str()),
            scale: 1.0,
            source_revision: source_log_id.to_string(),
        },
        supersedes: None,
        layout: JournalLayout {
            kind: JournalLayoutKind::Broadsheet,
            columns: JOURNAL_BROADSHEET_COLUMNS,
            bands,
        },
        blocks,
    }
}

fn seed_identity(
    source_log_id: &SemanticDigest,
    observation_ids: &[SemanticDigest],
    proposal_bytes: &[u8],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(JOURNAL_SEED_SCHEMA);
    hasher.add_str(source_log_id.as_str());
    hasher.add_u64(observation_ids.len() as u64);
    for observation_id in observation_ids {
        hasher.add_str(observation_id.as_str());
    }
    hasher.add_bytes(proposal_bytes);
    hasher.finish()
}

fn document_coordinate(identity: &str, revision: &str) -> String {
    format!(
        "rey+local://document/{}?revision={}",
        percent_encode(identity),
        percent_encode(revision)
    )
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn observation_kind(kind: crate::channels::ChannelObservationKind) -> &'static str {
    use crate::channels::ChannelObservationKind;
    match kind {
        ChannelObservationKind::Finding => "Finding",
        ChannelObservationKind::Question => "Question",
        ChannelObservationKind::Progress => "Progress",
        ChannelObservationKind::Blocker => "Blocker",
        ChannelObservationKind::Handoff => "Handoff",
    }
}

fn completeness(value: crate::observations::ObservationCompleteness) -> &'static str {
    use crate::observations::ObservationCompleteness;
    match value {
        ObservationCompleteness::Complete => "complete",
        ObservationCompleteness::Partial => "partial",
    }
}

#[derive(Debug, Error)]
pub enum JournalSeedError {
    #[error("journal seed schema must be {JOURNAL_SEED_SCHEMA}, got {0}")]
    Schema(String),
    #[error("a journal seed requires at least one exact observation identity")]
    Empty,
    #[error("journal seed selected {actual} observations; limit is {limit}")]
    Limit { actual: usize, limit: usize },
    #[error("journal seed proposal is {actual} bytes; limit is {limit}")]
    ProposalByteLimit { actual: u64, limit: u64 },
    #[error("journal seed repeats observation {0}")]
    Duplicate(String),
    #[error("observation {observation_id} is {state}, not unresolved")]
    Closed {
        observation_id: String,
        state: &'static str,
    },
    #[error("journal seed source log does not match current observation state")]
    SourceLog,
    #[error("journal seed identity or deterministic proposal does not match its source")]
    Identity,
    #[error(transparent)]
    Observation(#[from] ObservationError),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::JournalSeed;
    use crate::{
        channels::{ChannelObservationKind, LocalChannelStore},
        journal::{JournalAuthor, JournalAuthorKind, LocalJournalStore},
        observations::{
            LocalObservationStore, OBSERVATION_PROPOSAL_SCHEMA, ObservationAuthor,
            ObservationAuthorKind, ObservationCompleteness, ObservationProposal, ObservationSource,
        },
    };

    #[test]
    fn seed_is_deterministic_unretained_and_requires_normal_journal_admission() {
        let workspace = TempDir::new().unwrap();
        let channel_directory = workspace.path().join(".rey/channels");
        let channel_store = LocalChannelStore::new(channel_directory.clone());
        let graph = channel_store.status().unwrap().working;
        let observation_store = LocalObservationStore::new(channel_directory);
        let first = observation_store
            .admit_and_broadcast(
                proposal("First finding"),
                ObservationSource::workspace_file(
                    "workspace://observations/first.yaml".to_owned(),
                    b"first",
                ),
                Vec::new(),
                None,
                &graph,
                1,
            )
            .unwrap()
            .observation;
        let second = observation_store
            .admit_and_broadcast(
                proposal("Second finding"),
                ObservationSource::workspace_file(
                    "workspace://observations/second.yaml".to_owned(),
                    b"second",
                ),
                Vec::new(),
                None,
                &graph,
                2,
            )
            .unwrap()
            .observation;
        let log = observation_store.load().unwrap();
        let author = JournalAuthor {
            kind: JournalAuthorKind::Agent,
            id: "codex".to_owned(),
        };
        let seed = JournalSeed::from_log(
            &log,
            &[
                second.observation_id.to_string(),
                first.observation_id.to_string(),
            ],
            author.clone(),
        )
        .unwrap();
        let replay = JournalSeed::from_log(
            &log,
            &[
                first.observation_id.to_string(),
                second.observation_id.to_string(),
            ],
            author,
        )
        .unwrap();

        assert_eq!(seed, replay);
        assert_eq!(seed.observation_ids[0], first.observation_id);
        assert_eq!(seed.observation_ids[1], second.observation_id);
        assert_eq!(seed.proposal.blocks.len(), 4);
        seed.verify_against(&log).unwrap();
        let mut tampered = seed.clone();
        tampered.proposal.title.push_str(" tampered");
        assert!(tampered.verify_against(&log).is_err());

        let journal_store = LocalJournalStore::default_for_workspace(workspace.path());
        assert!(!journal_store.path().exists());
        let admission = journal_store
            .admit(seed.proposal, "2026-08-12T00:00:00Z")
            .unwrap();
        assert!(admission.admitted);
        assert_eq!(journal_store.load().unwrap().entries.len(), 1);
    }

    fn proposal(body: &str) -> ObservationProposal {
        ObservationProposal {
            schema: OBSERVATION_PROPOSAL_SCHEMA.to_owned(),
            kind: ChannelObservationKind::Finding,
            author: ObservationAuthor {
                kind: ObservationAuthorKind::Agent,
                id: "codex".to_owned(),
            },
            subject_locator: "rey+local://workload/alpha?revision=1".to_owned(),
            body: body.to_owned(),
            desired_delta: Some("Close the exact directed delta".to_owned()),
            completeness: ObservationCompleteness::Complete,
            omissions: Vec::new(),
            evidence: Vec::new(),
            supersedes: None,
        }
    }
}
