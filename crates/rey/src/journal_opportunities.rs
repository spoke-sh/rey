#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::journal::{
    JournalAuthor, JournalBinding, JournalBlock, JournalError, JournalLog, MAX_JOURNAL_BLOCKS,
    MAX_JOURNAL_ENTRIES,
};

pub const JOURNAL_OPPORTUNITY_SCHEMA: &str = "rey.journal-opportunity.v1";
pub const JOURNAL_OPPORTUNITY_SURFACE_SCHEMA: &str = "rey.journal-opportunity-surface.v1";
pub const DEFAULT_JOURNAL_OPPORTUNITY_LIMIT: u64 = 128;
pub const MAX_JOURNAL_OPPORTUNITY_LIMIT: u64 = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalOpportunityCompleteness {
    Complete,
    Truncated,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOpportunityLimits {
    pub max_rows: u64,
    pub max_log_entries: u64,
    pub max_blocks_per_entry: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOpportunityOmission {
    pub kind: String,
    pub omitted_count: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOpportunity {
    pub schema: String,
    pub opportunity_id: SemanticDigest,
    pub entry_id: SemanticDigest,
    pub entry_sequence: u64,
    pub document_path: String,
    pub block_id: String,
    pub fragment: String,
    pub author: JournalAuthor,
    pub binding: JournalBinding,
    pub operation: String,
    pub desired_delta: String,
    pub evidence_ids: Vec<String>,
    pub dependency_ids: Vec<String>,
    pub readiness: String,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOpportunitySummary {
    pub current_entries: u64,
    pub authored_actions: u64,
    pub projected: u64,
    pub omitted: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalOpportunitySurface {
    pub schema: String,
    pub surface_id: SemanticDigest,
    pub source_log_id: SemanticDigest,
    pub ordering: String,
    pub completeness: JournalOpportunityCompleteness,
    pub limits: JournalOpportunityLimits,
    pub summary: JournalOpportunitySummary,
    pub rows: Vec<JournalOpportunity>,
    pub omissions: Vec<JournalOpportunityOmission>,
    pub runtime_boundary: String,
}

impl JournalOpportunitySurface {
    pub fn derive(log: &JournalLog, max_rows: u64) -> Result<Self, JournalOpportunityError> {
        let surface = Self::derive_without_replay(log, max_rows)?;
        surface.verify_against(log)?;
        Ok(surface)
    }

    pub fn verify_against(&self, log: &JournalLog) -> Result<(), JournalOpportunityError> {
        if self.schema != JOURNAL_OPPORTUNITY_SURFACE_SCHEMA {
            return Err(JournalOpportunityError::Schema(self.schema.clone()));
        }
        if self.source_log_id != log.log_id {
            return Err(JournalOpportunityError::SourceLog {
                declared: self.source_log_id.clone(),
                actual: log.log_id.clone(),
            });
        }
        let expected = Self::derive_unverified(log, self.limits.max_rows)?;
        if *self != expected {
            return Err(JournalOpportunityError::ProjectionMismatch);
        }
        Ok(())
    }

    fn derive_unverified(log: &JournalLog, max_rows: u64) -> Result<Self, JournalOpportunityError> {
        let surface = Self::derive_without_replay(log, max_rows)?;
        let actual = surface.identity()?;
        if surface.surface_id != actual {
            return Err(JournalOpportunityError::Identity {
                declared: surface.surface_id.clone(),
                actual,
            });
        }
        Ok(surface)
    }

    fn derive_without_replay(
        log: &JournalLog,
        max_rows: u64,
    ) -> Result<Self, JournalOpportunityError> {
        // Keep replay construction in one place without recursively verifying it.
        let mut surface = derive_surface_fields(log, max_rows)?;
        surface.surface_id = surface.identity()?;
        Ok(surface)
    }

    fn identity(&self) -> Result<SemanticDigest, JournalOpportunityError> {
        let bytes = serde_json::to_vec(&JournalOpportunitySurfaceDigestInput {
            source_log_id: &self.source_log_id,
            ordering: &self.ordering,
            completeness: self.completeness,
            limits: &self.limits,
            summary: &self.summary,
            rows: &self.rows,
            omissions: &self.omissions,
            runtime_boundary: &self.runtime_boundary,
        })?;
        let mut hasher = SemanticHasher::new(JOURNAL_OPPORTUNITY_SURFACE_SCHEMA);
        hasher.add_bytes(&bytes);
        Ok(hasher.finish())
    }
}

#[derive(Serialize)]
struct JournalOpportunitySurfaceDigestInput<'a> {
    source_log_id: &'a SemanticDigest,
    ordering: &'a str,
    completeness: JournalOpportunityCompleteness,
    limits: &'a JournalOpportunityLimits,
    summary: &'a JournalOpportunitySummary,
    rows: &'a [JournalOpportunity],
    omissions: &'a [JournalOpportunityOmission],
    runtime_boundary: &'a str,
}

fn derive_surface_fields(
    log: &JournalLog,
    max_rows: u64,
) -> Result<JournalOpportunitySurface, JournalOpportunityError> {
    log.verify()?;
    validate_limit(max_rows)?;
    let superseded = log
        .entries
        .iter()
        .filter_map(|entry| entry.supersedes.as_ref().map(SemanticDigest::as_str))
        .collect::<BTreeSet<_>>();
    let current_entries = log
        .entries
        .iter()
        .filter(|entry| !superseded.contains(entry.entry_id.as_str()))
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for entry in &current_entries {
        for block in &entry.blocks {
            let JournalBlock::Action {
                id,
                operation,
                desired_delta,
                evidence_ids,
                dependency_ids,
            } = block
            else {
                continue;
            };
            let mut hasher = SemanticHasher::new(JOURNAL_OPPORTUNITY_SCHEMA);
            hasher.add_str(entry.entry_id.as_str());
            hasher.add_str(id);
            rows.push(JournalOpportunity {
                schema: JOURNAL_OPPORTUNITY_SCHEMA.to_owned(),
                opportunity_id: hasher.finish(),
                entry_id: entry.entry_id.clone(),
                entry_sequence: entry.sequence,
                document_path: format!("/journal/{}", entry.slug()),
                block_id: id.clone(),
                fragment: format!("block-{id}"),
                author: entry.author.clone(),
                binding: entry.binding.clone(),
                operation: operation.clone(),
                desired_delta: desired_delta.clone(),
                evidence_ids: evidence_ids.clone(),
                dependency_ids: dependency_ids.clone(),
                readiness: "authored_only".to_owned(),
                authority: "none".to_owned(),
            });
        }
    }
    let authored_actions =
        u64::try_from(rows.len()).map_err(|_| JournalOpportunityError::CountOverflow)?;
    let omitted = authored_actions.saturating_sub(max_rows);
    if omitted > 0 {
        rows.drain(..usize::try_from(omitted).map_err(|_| JournalOpportunityError::CountOverflow)?);
    }
    let completeness = if omitted == 0 {
        JournalOpportunityCompleteness::Complete
    } else {
        JournalOpportunityCompleteness::Truncated
    };
    Ok(JournalOpportunitySurface {
        schema: JOURNAL_OPPORTUNITY_SURFACE_SCHEMA.to_owned(),
        surface_id: SemanticHasher::new(JOURNAL_OPPORTUNITY_SURFACE_SCHEMA).finish(),
        source_log_id: log.log_id.clone(),
        ordering: "journal_sequence_then_block_order".to_owned(),
        completeness,
        limits: JournalOpportunityLimits {
            max_rows,
            max_log_entries: MAX_JOURNAL_ENTRIES as u64,
            max_blocks_per_entry: MAX_JOURNAL_BLOCKS as u64,
        },
        summary: JournalOpportunitySummary {
            current_entries: u64::try_from(current_entries.len())
                .map_err(|_| JournalOpportunityError::CountOverflow)?,
            authored_actions,
            projected: u64::try_from(rows.len())
                .map_err(|_| JournalOpportunityError::CountOverflow)?,
            omitted,
        },
        rows,
        omissions: if omitted == 0 {
            Vec::new()
        } else {
            vec![JournalOpportunityOmission {
                kind: "row_limit".to_owned(),
                omitted_count: omitted,
                reason: "oldest current authored action cells omitted by the effective row limit"
                    .to_owned(),
            }]
        },
        runtime_boundary:
            "requires_verified_selected_ready_create_attention_row_and_workload_admission"
                .to_owned(),
    })
}

fn validate_limit(max_rows: u64) -> Result<(), JournalOpportunityError> {
    if !(1..=MAX_JOURNAL_OPPORTUNITY_LIMIT).contains(&max_rows) {
        return Err(JournalOpportunityError::RowLimit {
            actual: max_rows,
            maximum: MAX_JOURNAL_OPPORTUNITY_LIMIT,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum JournalOpportunityError {
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(
        "journal opportunity surface schema must be {JOURNAL_OPPORTUNITY_SURFACE_SCHEMA}, got {0}"
    )]
    Schema(String),
    #[error("journal opportunity row limit {actual} is outside 1..={maximum}")]
    RowLimit { actual: u64, maximum: u64 },
    #[error("journal opportunity source log mismatch: declared {declared}, actual {actual}")]
    SourceLog {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("journal opportunity surface identity mismatch: declared {declared}, actual {actual}")]
    Identity {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("journal opportunity surface does not replay from its exact Journal log")]
    ProjectionMismatch,
    #[error("journal opportunity count overflowed")]
    CountOverflow,
    #[error("journal opportunity projection serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use crate::journal::{
        JOURNAL_PROPOSAL_SCHEMA, JournalAuthorKind, JournalEntryProposal, JournalLayout,
        JournalLayoutBand, JournalLayoutCell, JournalLayoutKind,
    };

    use super::*;

    fn proposal(title: &str, action_ids: &[&str]) -> JournalEntryProposal {
        let blocks = action_ids
            .iter()
            .map(|id| JournalBlock::Action {
                id: (*id).to_owned(),
                operation: "refine".to_owned(),
                desired_delta: format!("Resolve {id}."),
                evidence_ids: vec![format!("evidence:{id}")],
                dependency_ids: Vec::new(),
            })
            .collect::<Vec<_>>();
        JournalEntryProposal {
            schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
            title: title.to_owned(),
            author: JournalAuthor {
                kind: JournalAuthorKind::Agent,
                id: "codex".to_owned(),
            },
            binding: JournalBinding {
                coordinate: "rey+local://document/current?revision=blake3%3Asource".to_owned(),
                scale: 1.0,
                source_revision: "blake3:source".to_owned(),
            },
            supersedes: None,
            layout: JournalLayout {
                kind: JournalLayoutKind::Broadsheet,
                columns: 12,
                bands: blocks
                    .iter()
                    .map(|block| JournalLayoutBand {
                        id: format!("band-{}", block.id()),
                        cells: vec![JournalLayoutCell {
                            block_id: block.id().to_owned(),
                            span: 12,
                        }],
                    })
                    .collect(),
            },
            blocks,
        }
    }

    #[test]
    fn projects_only_action_cells_on_current_revision_leaves() {
        let mut log = JournalLog::default();
        let (first, _) = log
            .admit(proposal("First", &["old"]), "2026-08-12T10:00:00Z")
            .unwrap();
        let mut revision = proposal("Revision", &["current"]);
        revision.supersedes = Some(first.entry_id);
        log.admit(revision, "2026-08-12T10:01:00Z").unwrap();
        log.admit(
            proposal("Independent", &["branch-a", "branch-b"]),
            "2026-08-12T10:02:00Z",
        )
        .unwrap();

        let surface = JournalOpportunitySurface::derive(&log, 128).unwrap();

        assert_eq!(surface.summary.current_entries, 2);
        assert_eq!(surface.summary.authored_actions, 3);
        assert_eq!(
            surface
                .rows
                .iter()
                .map(|row| row.block_id.as_str())
                .collect::<Vec<_>>(),
            ["current", "branch-a", "branch-b"]
        );
        assert!(surface.rows.iter().all(|row| {
            row.readiness == "authored_only"
                && row.authority == "none"
                && row.document_path.starts_with("/journal/")
                && row.fragment == format!("block-{}", row.block_id)
        }));
        surface.verify_against(&log).unwrap();
    }

    #[test]
    fn truncates_oldest_rows_and_rejects_tampering_or_log_drift() {
        let mut log = JournalLog::default();
        log.admit(
            proposal("Actions", &["first", "second", "third"]),
            "2026-08-12T10:00:00Z",
        )
        .unwrap();
        let surface = JournalOpportunitySurface::derive(&log, 2).unwrap();
        assert_eq!(
            surface
                .rows
                .iter()
                .map(|row| row.block_id.as_str())
                .collect::<Vec<_>>(),
            ["second", "third"]
        );
        assert_eq!(
            surface.completeness,
            JournalOpportunityCompleteness::Truncated
        );
        assert_eq!(surface.summary.omitted, 1);

        let mut tampered = surface.clone();
        tampered.rows[0].authority = "execute".to_owned();
        assert!(matches!(
            tampered.verify_against(&log),
            Err(JournalOpportunityError::ProjectionMismatch)
        ));

        let mut changed = log.clone();
        changed
            .admit(proposal("Later", &["later"]), "2026-08-12T10:01:00Z")
            .unwrap();
        assert!(matches!(
            surface.verify_against(&changed),
            Err(JournalOpportunityError::SourceLog { .. })
        ));
    }
}
