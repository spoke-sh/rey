#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::DateTime;
use rey_core::{SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    journal::{
        JOURNAL_BROADSHEET_COLUMNS, JOURNAL_PROPOSAL_SCHEMA, JournalAuthor, JournalBlock,
        JournalEntry, JournalEntryProposal, JournalError, JournalFrameColumn, JournalLayoutBand,
        JournalLayoutCell, JournalLog, MAX_JOURNAL_BLOCKS,
    },
    observations::{ObservationError, ObservationFrontier, ObservationLog},
};

pub const JOURNAL_QUERY_ADMISSION_SCHEMA: &str = "rey.journal-query-admission.v1";
pub const JOURNAL_QUERY_ADMISSION_RESULT_SCHEMA: &str = "rey.journal-query-admission-result.v1";
pub const JOURNAL_QUERY_DELTA_SCHEMA: &str = "rey.journal-query-frame-delta.v1";
pub const JOURNAL_QUERY_EXECUTION_SCHEMA: &str = "rey.journal-query-execution.v1";
pub const JOURNAL_QUERY_EXECUTION_RESULT_SCHEMA: &str = "rey.journal-query-execution-result.v1";
pub const JOURNAL_QUERY_STATE_SCHEMA: &str = "rey.journal-query-state.v1";
pub const JOURNAL_QUERY_PROVIDER: &str = "rey.observations";
pub const JOURNAL_QUERY_LANGUAGE: &str = "rey";
pub const JOURNAL_QUERY_STATEMENT: &str = "frontier";
pub const DEFAULT_JOURNAL_QUERY_ROW_LIMIT: u64 = 64;
pub const MAX_JOURNAL_QUERY_ROW_LIMIT: u64 = 100;
pub const MAX_JOURNAL_QUERY_STATE_BYTES: u64 = 8 * 1_024 * 1_024;
const MAX_QUERY_ADMISSIONS: usize = 256;
const MAX_QUERY_EXECUTIONS: usize = 256;
const STATE_FILE_NAME: &str = "journal-queries.json";
const LOCK_FILE_NAME: &str = "journal-queries.lock";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryDeclaration {
    pub language: String,
    pub provider: String,
    pub mode: String,
    pub statement: String,
    pub parameters: BTreeMap<String, String>,
}

impl JournalQueryDeclaration {
    fn from_block(block: &JournalBlock) -> Result<Self, JournalQueryError> {
        let JournalBlock::Query {
            language,
            provider,
            mode,
            statement,
            parameters,
            ..
        } = block
        else {
            return Err(JournalQueryError::NotQuery(block.id().to_owned()));
        };
        let declaration = Self {
            language: language.clone(),
            provider: provider.clone(),
            mode: mode.clone(),
            statement: statement.clone(),
            parameters: parameters.clone(),
        };
        declaration.row_limit()?;
        Ok(declaration)
    }

    fn row_limit(&self) -> Result<u64, JournalQueryError> {
        if self.language != JOURNAL_QUERY_LANGUAGE
            || self.provider != JOURNAL_QUERY_PROVIDER
            || self.mode != "read_only"
            || self.statement != JOURNAL_QUERY_STATEMENT
        {
            return Err(JournalQueryError::UnsupportedQuery {
                language: self.language.clone(),
                provider: self.provider.clone(),
                mode: self.mode.clone(),
                statement: self.statement.clone(),
            });
        }
        if self.parameters.keys().any(|key| key != "limit") {
            return Err(JournalQueryError::UnsupportedParameter);
        }
        let limit = self.parameters.get("limit").map_or_else(
            || Ok(DEFAULT_JOURNAL_QUERY_ROW_LIMIT),
            |value| {
                let limit = value
                    .parse::<u64>()
                    .map_err(|_| JournalQueryError::InvalidRowLimit(value.clone()))?;
                if limit.to_string() != *value {
                    return Err(JournalQueryError::InvalidRowLimit(value.clone()));
                }
                Ok(limit)
            },
        )?;
        if !(1..=MAX_JOURNAL_QUERY_ROW_LIMIT).contains(&limit) {
            return Err(JournalQueryError::RowLimit {
                actual: limit,
                maximum: MAX_JOURNAL_QUERY_ROW_LIMIT,
            });
        }
        Ok(limit)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryLimits {
    pub max_rows: u64,
    pub max_frame_columns: u64,
    pub max_frame_cell_chars: u64,
    pub max_proposal_blocks: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryAdmission {
    pub schema: String,
    pub admission_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at: String,
    pub journal_log_id: SemanticDigest,
    pub entry_id: SemanticDigest,
    pub entry_sequence: u64,
    pub block_id: String,
    pub declaration: JournalQueryDeclaration,
    pub observation_log_id: SemanticDigest,
    pub observation_frontier_id: SemanticDigest,
    pub limits: JournalQueryLimits,
    pub authority: String,
}

impl JournalQueryAdmission {
    fn new(
        sequence: u64,
        admitted_at: &str,
        input: JournalQueryAdmissionInput<'_>,
    ) -> Result<Self, JournalQueryError> {
        validate_timestamp(admitted_at)?;
        let limits = JournalQueryLimits {
            max_rows: input.declaration.row_limit()?,
            max_frame_columns: 9,
            max_frame_cell_chars: 4_096,
            max_proposal_blocks: MAX_JOURNAL_BLOCKS as u64,
        };
        let mut admission = Self {
            schema: JOURNAL_QUERY_ADMISSION_SCHEMA.to_owned(),
            admission_id: placeholder_digest(JOURNAL_QUERY_ADMISSION_SCHEMA),
            sequence,
            admitted_at: admitted_at.to_owned(),
            journal_log_id: input.journal.log_id.clone(),
            entry_id: input.entry.entry_id.clone(),
            entry_sequence: input.entry.sequence,
            block_id: input.block_id.to_owned(),
            declaration: input.declaration,
            observation_log_id: input.observations.log_id.clone(),
            observation_frontier_id: input.frontier.frontier_id.clone(),
            limits,
            authority: "read_only_observation_frontier_query".to_owned(),
        };
        admission.admission_id = admission.identity()?;
        admission.verify()?;
        Ok(admission)
    }

    fn verify(&self) -> Result<(), JournalQueryError> {
        if self.schema != JOURNAL_QUERY_ADMISSION_SCHEMA {
            return Err(JournalQueryError::Schema {
                expected: JOURNAL_QUERY_ADMISSION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.sequence == 0 {
            return Err(JournalQueryError::Sequence);
        }
        validate_timestamp(&self.admitted_at)?;
        if self.entry_sequence == 0
            || self.block_id.is_empty()
            || self.limits.max_rows != self.declaration.row_limit()?
            || self.limits.max_frame_columns != 9
            || self.limits.max_frame_cell_chars != 4_096
            || self.limits.max_proposal_blocks != MAX_JOURNAL_BLOCKS as u64
            || self.authority != "read_only_observation_frontier_query"
        {
            return Err(JournalQueryError::AdmissionShape);
        }
        let actual = self.identity()?;
        if actual != self.admission_id {
            return Err(JournalQueryError::Identity {
                kind: "admission",
                declared: self.admission_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn identity(&self) -> Result<SemanticDigest, JournalQueryError> {
        let bytes = serde_json::to_vec(&JournalQueryAdmissionDigestInput {
            journal_log_id: &self.journal_log_id,
            entry_id: &self.entry_id,
            entry_sequence: self.entry_sequence,
            block_id: &self.block_id,
            declaration: &self.declaration,
            observation_log_id: &self.observation_log_id,
            observation_frontier_id: &self.observation_frontier_id,
            limits: &self.limits,
            authority: &self.authority,
        })?;
        let mut hasher = SemanticHasher::new(JOURNAL_QUERY_ADMISSION_SCHEMA);
        hasher.add_bytes(&bytes);
        Ok(hasher.finish())
    }
}

struct JournalQueryAdmissionInput<'a> {
    journal: &'a JournalLog,
    entry: &'a JournalEntry,
    block_id: &'a str,
    declaration: JournalQueryDeclaration,
    observations: &'a ObservationLog,
    frontier: &'a ObservationFrontier,
}

#[derive(Serialize)]
struct JournalQueryAdmissionDigestInput<'a> {
    journal_log_id: &'a SemanticDigest,
    entry_id: &'a SemanticDigest,
    entry_sequence: u64,
    block_id: &'a str,
    declaration: &'a JournalQueryDeclaration,
    observation_log_id: &'a SemanticDigest,
    observation_frontier_id: &'a SemanticDigest,
    limits: &'a JournalQueryLimits,
    authority: &'a str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryFrameDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub source_snapshot_id: SemanticDigest,
    pub target_snapshot_id: SemanticDigest,
    pub direction: String,
    pub assessment: String,
    pub inserted_rows: u64,
    pub deleted_rows: u64,
    pub modified_rows: u64,
    pub complete: bool,
    pub omitted_rows: u64,
}

impl JournalQueryFrameDelta {
    fn new(admission: &JournalQueryAdmission, frontier: &ObservationFrontier) -> Self {
        let source_snapshot_id = empty_frame_identity();
        let inserted_rows = frontier.rows.len() as u64;
        let assessment = if frontier.summary.unresolved == 0 {
            "equal"
        } else {
            "different"
        };
        let mut delta = Self {
            schema: JOURNAL_QUERY_DELTA_SCHEMA.to_owned(),
            delta_id: placeholder_digest(JOURNAL_QUERY_DELTA_SCHEMA),
            source_snapshot_id,
            target_snapshot_id: frontier.frontier_id.clone(),
            direction: "empty_to_observed".to_owned(),
            assessment: assessment.to_owned(),
            inserted_rows,
            deleted_rows: 0,
            modified_rows: 0,
            complete: frontier.complete,
            omitted_rows: frontier.omitted,
        };
        delta.delta_id = delta.identity(admission);
        delta
    }

    fn verify(&self, admission: &JournalQueryAdmission) -> Result<(), JournalQueryError> {
        if self.schema != JOURNAL_QUERY_DELTA_SCHEMA
            || self.source_snapshot_id != empty_frame_identity()
            || self.direction != "empty_to_observed"
            || self.deleted_rows != 0
            || self.modified_rows != 0
            || self.assessment
                != if self.inserted_rows == 0 && self.omitted_rows == 0 {
                    "equal"
                } else {
                    "different"
                }
            || self.complete == (self.omitted_rows > 0)
        {
            return Err(JournalQueryError::DeltaShape);
        }
        let actual = self.identity(admission);
        if actual != self.delta_id {
            return Err(JournalQueryError::Identity {
                kind: "delta",
                declared: self.delta_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn identity(&self, admission: &JournalQueryAdmission) -> SemanticDigest {
        let mut hasher = SemanticHasher::new(JOURNAL_QUERY_DELTA_SCHEMA);
        hasher.add_str(admission.admission_id.as_str());
        hasher.add_str(self.source_snapshot_id.as_str());
        hasher.add_str(self.target_snapshot_id.as_str());
        hasher.add_str(&self.direction);
        hasher.add_str(&self.assessment);
        hasher.add_u64(self.inserted_rows);
        hasher.add_u64(self.deleted_rows);
        hasher.add_u64(self.modified_rows);
        hasher.add_bool(self.complete);
        hasher.add_u64(self.omitted_rows);
        hasher.finish()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryExecution {
    pub schema: String,
    pub execution_id: SemanticDigest,
    pub sequence: u64,
    pub executed_at: String,
    pub admission_id: SemanticDigest,
    pub observation_frontier: ObservationFrontier,
    pub frame_block_id: String,
    pub delta_block_id: String,
    pub frame_snapshot_id: SemanticDigest,
    pub delta: JournalQueryFrameDelta,
    pub proposal: JournalEntryProposal,
    pub authority: String,
}

impl JournalQueryExecution {
    fn new(
        sequence: u64,
        executed_at: &str,
        admission: &JournalQueryAdmission,
        entry: &JournalEntry,
        frontier: ObservationFrontier,
        author: JournalAuthor,
    ) -> Result<Self, JournalQueryError> {
        validate_timestamp(executed_at)?;
        let delta = JournalQueryFrameDelta::new(admission, &frontier);
        let name = identity_suffix(&delta.delta_id);
        let frame_block_id = format!("query-frame-{name}");
        let delta_block_id = format!("query-delta-{name}");
        let proposal = result_proposal(
            admission,
            entry,
            &frontier,
            &delta,
            &frame_block_id,
            &delta_block_id,
            author,
        )?;
        let mut execution = Self {
            schema: JOURNAL_QUERY_EXECUTION_SCHEMA.to_owned(),
            execution_id: placeholder_digest(JOURNAL_QUERY_EXECUTION_SCHEMA),
            sequence,
            executed_at: executed_at.to_owned(),
            admission_id: admission.admission_id.clone(),
            frame_snapshot_id: frontier.frontier_id.clone(),
            observation_frontier: frontier,
            frame_block_id,
            delta_block_id,
            delta,
            proposal,
            authority: "retained_query_evidence; journal_proposal_unretained".to_owned(),
        };
        execution.execution_id = execution.identity()?;
        execution.verify(admission)?;
        Ok(execution)
    }

    fn verify(&self, admission: &JournalQueryAdmission) -> Result<(), JournalQueryError> {
        if self.schema != JOURNAL_QUERY_EXECUTION_SCHEMA {
            return Err(JournalQueryError::Schema {
                expected: JOURNAL_QUERY_EXECUTION_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.sequence == 0 || self.admission_id != admission.admission_id {
            return Err(JournalQueryError::ExecutionShape);
        }
        validate_timestamp(&self.executed_at)?;
        self.observation_frontier.verify_projection()?;
        self.delta.verify(admission)?;
        self.proposal.validate()?;
        if self.frame_snapshot_id != self.observation_frontier.frontier_id
            || self.delta.target_snapshot_id != self.frame_snapshot_id
            || self.proposal.supersedes.as_ref() != Some(&admission.entry_id)
            || self.authority != "retained_query_evidence; journal_proposal_unretained"
        {
            return Err(JournalQueryError::ExecutionShape);
        }
        verify_result_blocks(self, admission)?;
        let actual = self.identity()?;
        if actual != self.execution_id {
            return Err(JournalQueryError::Identity {
                kind: "execution",
                declared: self.execution_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn identity(&self) -> Result<SemanticDigest, JournalQueryError> {
        let bytes = serde_json::to_vec(&JournalQueryExecutionDigestInput {
            admission_id: &self.admission_id,
            observation_frontier: &self.observation_frontier,
            frame_block_id: &self.frame_block_id,
            delta_block_id: &self.delta_block_id,
            frame_snapshot_id: &self.frame_snapshot_id,
            delta: &self.delta,
            proposal: &self.proposal,
            authority: &self.authority,
        })?;
        let mut hasher = SemanticHasher::new(JOURNAL_QUERY_EXECUTION_SCHEMA);
        hasher.add_bytes(&bytes);
        Ok(hasher.finish())
    }
}

#[derive(Serialize)]
struct JournalQueryExecutionDigestInput<'a> {
    admission_id: &'a SemanticDigest,
    observation_frontier: &'a ObservationFrontier,
    frame_block_id: &'a str,
    delta_block_id: &'a str,
    frame_snapshot_id: &'a SemanticDigest,
    delta: &'a JournalQueryFrameDelta,
    proposal: &'a JournalEntryProposal,
    authority: &'a str,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryState {
    pub schema: String,
    pub state_id: SemanticDigest,
    pub admissions: Vec<JournalQueryAdmission>,
    pub executions: Vec<JournalQueryExecution>,
}

impl Default for JournalQueryState {
    fn default() -> Self {
        let mut state = Self {
            schema: JOURNAL_QUERY_STATE_SCHEMA.to_owned(),
            state_id: placeholder_digest(JOURNAL_QUERY_STATE_SCHEMA),
            admissions: Vec::new(),
            executions: Vec::new(),
        };
        state.state_id = state.identity();
        state
    }
}

impl JournalQueryState {
    pub fn verify(&self) -> Result<(), JournalQueryError> {
        if self.schema != JOURNAL_QUERY_STATE_SCHEMA {
            return Err(JournalQueryError::Schema {
                expected: JOURNAL_QUERY_STATE_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.admissions.len() > MAX_QUERY_ADMISSIONS
            || self.executions.len() > MAX_QUERY_EXECUTIONS
        {
            return Err(JournalQueryError::RecordLimit);
        }
        let mut admission_ids = BTreeSet::new();
        for (index, admission) in self.admissions.iter().enumerate() {
            admission.verify()?;
            if admission.sequence != index as u64 + 1
                || !admission_ids.insert(admission.admission_id.clone())
            {
                return Err(JournalQueryError::Sequence);
            }
        }
        let mut execution_ids = BTreeSet::new();
        for (index, execution) in self.executions.iter().enumerate() {
            let admission = self
                .admissions
                .iter()
                .find(|admission| admission.admission_id == execution.admission_id)
                .ok_or_else(|| {
                    JournalQueryError::UnknownAdmission(execution.admission_id.to_string())
                })?;
            execution.verify(admission)?;
            if execution.sequence != index as u64 + 1
                || !execution_ids.insert(execution.execution_id.clone())
            {
                return Err(JournalQueryError::Sequence);
            }
        }
        let actual = self.identity();
        if actual != self.state_id {
            return Err(JournalQueryError::Identity {
                kind: "state",
                declared: self.state_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    fn identity(&self) -> SemanticDigest {
        let mut hasher = SemanticHasher::new(JOURNAL_QUERY_STATE_SCHEMA);
        hasher.add_u64(self.admissions.len() as u64);
        for admission in &self.admissions {
            hasher.add_str(admission.admission_id.as_str());
            hasher.add_u64(admission.sequence);
            hasher.add_str(&admission.admitted_at);
        }
        hasher.add_u64(self.executions.len() as u64);
        for execution in &self.executions {
            hasher.add_str(execution.execution_id.as_str());
            hasher.add_u64(execution.sequence);
            hasher.add_str(&execution.executed_at);
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryAdmissionResult {
    pub schema: String,
    pub admitted: bool,
    pub admission: JournalQueryAdmission,
    pub state_id: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryExecutionResult {
    pub schema: String,
    pub executed: bool,
    pub execution: JournalQueryExecution,
    pub state_id: SemanticDigest,
}

#[derive(Clone, Debug)]
pub struct LocalJournalQueryStore {
    directory: PathBuf,
}

impl LocalJournalQueryStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("journal"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<JournalQueryState, JournalQueryError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(JournalQueryState::default());
            }
            Err(source) => return Err(JournalQueryError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(JournalQueryError::UnsafePath(path));
        }
        if metadata.len() > MAX_JOURNAL_QUERY_STATE_BYTES {
            return Err(JournalQueryError::ByteLimit);
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| JournalQueryError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_JOURNAL_QUERY_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| JournalQueryError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_JOURNAL_QUERY_STATE_BYTES {
            return Err(JournalQueryError::ByteLimit);
        }
        let state: JournalQueryState = serde_json::from_slice(&bytes)?;
        state.verify()?;
        Ok(state)
    }

    pub fn admit(
        &self,
        journal: &JournalLog,
        observations: &ObservationLog,
        entry_id: &str,
        block_id: &str,
        admitted_at: &str,
    ) -> Result<JournalQueryAdmissionResult, JournalQueryError> {
        journal.verify()?;
        observations.verify()?;
        let entry = current_entry(journal, entry_id)?;
        let block = entry
            .blocks
            .iter()
            .find(|block| block.id() == block_id)
            .ok_or_else(|| JournalQueryError::UnknownBlock(block_id.to_owned()))?;
        let declaration = JournalQueryDeclaration::from_block(block)?;
        let frontier = observations.frontier(declaration.row_limit()? as usize)?;
        self.with_locked_state(|state| {
            let candidate = JournalQueryAdmission::new(
                state.admissions.len() as u64 + 1,
                admitted_at,
                JournalQueryAdmissionInput {
                    journal,
                    entry,
                    block_id,
                    declaration,
                    observations,
                    frontier: &frontier,
                },
            )?;
            if let Some(existing) = state
                .admissions
                .iter()
                .find(|admission| admission.admission_id == candidate.admission_id)
            {
                return Ok((
                    JournalQueryAdmissionResult {
                        schema: JOURNAL_QUERY_ADMISSION_RESULT_SCHEMA.to_owned(),
                        admitted: false,
                        admission: existing.clone(),
                        state_id: state.state_id.clone(),
                    },
                    false,
                ));
            }
            if state.admissions.len() >= MAX_QUERY_ADMISSIONS {
                return Err(JournalQueryError::RecordLimit);
            }
            state.admissions.push(candidate.clone());
            state.state_id = state.identity();
            Ok((
                JournalQueryAdmissionResult {
                    schema: JOURNAL_QUERY_ADMISSION_RESULT_SCHEMA.to_owned(),
                    admitted: true,
                    admission: candidate,
                    state_id: state.state_id.clone(),
                },
                true,
            ))
        })
    }

    pub fn execute(
        &self,
        journal: &JournalLog,
        observations: &ObservationLog,
        admission_id: &str,
        author: JournalAuthor,
        executed_at: &str,
    ) -> Result<JournalQueryExecutionResult, JournalQueryError> {
        self.with_locked_state(|state| {
            let admission = state
                .admissions
                .iter()
                .find(|admission| admission.admission_id.as_str() == admission_id)
                .cloned()
                .ok_or_else(|| JournalQueryError::UnknownAdmission(admission_id.to_owned()))?;
            if let Some(existing) = state.executions.iter().find(|execution| {
                execution.admission_id == admission.admission_id
                    && execution.proposal.author == author
            }) {
                return Ok((
                    JournalQueryExecutionResult {
                        schema: JOURNAL_QUERY_EXECUTION_RESULT_SCHEMA.to_owned(),
                        executed: false,
                        execution: existing.clone(),
                        state_id: state.state_id.clone(),
                    },
                    false,
                ));
            }
            journal.verify()?;
            observations.verify()?;
            if journal.log_id != admission.journal_log_id {
                return Err(JournalQueryError::StaleJournal {
                    admitted: admission.journal_log_id.clone(),
                    current: journal.log_id.clone(),
                });
            }
            if observations.log_id != admission.observation_log_id {
                return Err(JournalQueryError::StaleObservations {
                    admitted: admission.observation_log_id.clone(),
                    current: observations.log_id.clone(),
                });
            }
            let entry = current_entry(journal, admission.entry_id.as_str())?;
            let block = entry
                .blocks
                .iter()
                .find(|block| block.id() == admission.block_id)
                .ok_or_else(|| JournalQueryError::UnknownBlock(admission.block_id.clone()))?;
            if JournalQueryDeclaration::from_block(block)? != admission.declaration {
                return Err(JournalQueryError::AdmissionShape);
            }
            let frontier = observations.frontier(admission.limits.max_rows as usize)?;
            if frontier.frontier_id != admission.observation_frontier_id {
                return Err(JournalQueryError::StaleFrontier);
            }
            if state.executions.len() >= MAX_QUERY_EXECUTIONS {
                return Err(JournalQueryError::RecordLimit);
            }
            let execution = JournalQueryExecution::new(
                state.executions.len() as u64 + 1,
                executed_at,
                &admission,
                entry,
                frontier,
                author,
            )?;
            state.executions.push(execution.clone());
            state.state_id = state.identity();
            Ok((
                JournalQueryExecutionResult {
                    schema: JOURNAL_QUERY_EXECUTION_RESULT_SCHEMA.to_owned(),
                    executed: true,
                    execution,
                    state_id: state.state_id.clone(),
                },
                true,
            ))
        })
    }

    fn with_locked_state<T>(
        &self,
        change: impl FnOnce(&mut JournalQueryState) -> Result<(T, bool), JournalQueryError>,
    ) -> Result<T, JournalQueryError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(JournalQueryError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| JournalQueryError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| JournalQueryError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = (|| {
            let mut state = self.load()?;
            let (result, changed) = change(&mut state)?;
            state.verify()?;
            if changed {
                self.save(&state)?;
            }
            Ok(result)
        })();
        let unlock = File::unlock(&lock).map_err(|source| JournalQueryError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn save(&self, state: &JournalQueryState) -> Result<(), JournalQueryError> {
        state.verify()?;
        let mut bytes = serde_json::to_vec_pretty(state)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_JOURNAL_QUERY_STATE_BYTES {
            return Err(JournalQueryError::ByteLimit);
        }
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(JournalQueryError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = file
            .write_all(&bytes)
            .and_then(|()| file.flush())
            .and_then(|()| {
                drop(file);
                fs::rename(&temporary, &target)
            });
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(JournalQueryError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), JournalQueryError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(JournalQueryError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| JournalQueryError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(JournalQueryError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), JournalQueryError> {
        for attempt in 0..32_u8 {
            let path = self.directory.join(format!(
                ".{STATE_FILE_NAME}.tmp-{}-{attempt}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(JournalQueryError::Write { path, source }),
            }
        }
        Err(JournalQueryError::TemporaryLimit(self.directory.clone()))
    }

    fn verify_directory_boundary(&self) -> Result<(), JournalQueryError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(JournalQueryError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(JournalQueryError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(JournalQueryError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }
}

fn current_entry<'a>(
    journal: &'a JournalLog,
    entry_id: &str,
) -> Result<&'a JournalEntry, JournalQueryError> {
    let entry = journal
        .entries
        .iter()
        .find(|entry| entry.entry_id.as_str() == entry_id)
        .ok_or_else(|| JournalQueryError::UnknownEntry(entry_id.to_owned()))?;
    if journal.entries.iter().any(|candidate| {
        candidate
            .supersedes
            .as_ref()
            .is_some_and(|supersedes| supersedes == &entry.entry_id)
    }) {
        return Err(JournalQueryError::SupersededEntry(entry.entry_id.clone()));
    }
    Ok(entry)
}

fn result_proposal(
    admission: &JournalQueryAdmission,
    entry: &JournalEntry,
    frontier: &ObservationFrontier,
    delta: &JournalQueryFrameDelta,
    frame_block_id: &str,
    delta_block_id: &str,
    author: JournalAuthor,
) -> Result<JournalEntryProposal, JournalQueryError> {
    if entry.blocks.len().saturating_add(2) > MAX_JOURNAL_BLOCKS {
        return Err(JournalQueryError::ProposalBlockLimit);
    }
    let mut blocks = entry.blocks.clone();
    blocks.push(JournalBlock::Frame {
        id: frame_block_id.to_owned(),
        source_block_id: admission.block_id.clone(),
        snapshot_id: frontier.frontier_id.to_string(),
        columns: observation_frame_columns(),
        preview_rows: observation_frame_rows(frontier)?,
        row_count: frontier.summary.unresolved,
        truncated: !frontier.complete,
    });
    blocks.push(JournalBlock::Diff {
        id: delta_block_id.to_owned(),
        source: format!(
            "rey-journal-query-delta://{}/source/{}",
            delta.delta_id, delta.source_snapshot_id
        ),
        target: format!(
            "rey-journal-query-delta://{}/target/{}",
            delta.delta_id, delta.target_snapshot_id
        ),
        direction: delta.direction.clone(),
        assessment: delta.assessment.clone(),
        summary: format!(
            "{} unresolved observation rows projected; {} omitted by the exact query bound.",
            delta.inserted_rows, delta.omitted_rows
        ),
    });
    let mut layout = entry.layout.clone();
    layout.bands.extend([
        JournalLayoutBand {
            id: format!("band-{frame_block_id}"),
            cells: vec![JournalLayoutCell {
                block_id: frame_block_id.to_owned(),
                span: JOURNAL_BROADSHEET_COLUMNS,
            }],
        },
        JournalLayoutBand {
            id: format!("band-{delta_block_id}"),
            cells: vec![JournalLayoutCell {
                block_id: delta_block_id.to_owned(),
                span: JOURNAL_BROADSHEET_COLUMNS,
            }],
        },
    ]);
    let proposal = JournalEntryProposal {
        schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
        title: entry.title.clone(),
        author,
        binding: entry.binding.clone(),
        supersedes: Some(entry.entry_id.clone()),
        layout,
        blocks,
    };
    proposal.validate()?;
    Ok(proposal)
}

fn observation_frame_columns() -> Vec<JournalFrameColumn> {
    [
        ("observation_id", "utf8"),
        ("sequence", "u64"),
        ("kind", "utf8"),
        ("author", "utf8"),
        ("subject_locator", "utf8"),
        ("desired_delta", "utf8"),
        ("completeness", "utf8"),
        ("evidence_count", "u64"),
        ("channel_count", "u64"),
    ]
    .into_iter()
    .map(|(name, data_type)| JournalFrameColumn {
        name: name.to_owned(),
        data_type: data_type.to_owned(),
    })
    .collect()
}

fn observation_frame_rows(
    frontier: &ObservationFrontier,
) -> Result<Vec<BTreeMap<String, Option<String>>>, JournalQueryError> {
    frontier
        .rows
        .iter()
        .map(|row| {
            let observation = &row.observation;
            Ok(BTreeMap::from([
                (
                    "observation_id".to_owned(),
                    Some(observation.observation_id.to_string()),
                ),
                (
                    "sequence".to_owned(),
                    Some(observation.sequence.to_string()),
                ),
                (
                    "kind".to_owned(),
                    Some(observation.proposal.kind.label().to_owned()),
                ),
                (
                    "author".to_owned(),
                    Some(format!(
                        "{}:{}",
                        serde_json::to_value(observation.proposal.author.kind)?
                            .as_str()
                            .ok_or(JournalQueryError::ExecutionShape)?,
                        observation.proposal.author.id
                    )),
                ),
                (
                    "subject_locator".to_owned(),
                    Some(observation.proposal.subject_locator.clone()),
                ),
                (
                    "desired_delta".to_owned(),
                    observation.proposal.desired_delta.clone(),
                ),
                (
                    "completeness".to_owned(),
                    Some(
                        serde_json::to_value(observation.proposal.completeness)?
                            .as_str()
                            .ok_or(JournalQueryError::ExecutionShape)?
                            .to_owned(),
                    ),
                ),
                (
                    "evidence_count".to_owned(),
                    Some(observation.proposal.evidence.len().to_string()),
                ),
                (
                    "channel_count".to_owned(),
                    Some(row.channel_ids.len().to_string()),
                ),
            ]))
        })
        .collect()
}

fn verify_result_blocks(
    execution: &JournalQueryExecution,
    admission: &JournalQueryAdmission,
) -> Result<(), JournalQueryError> {
    let frame = execution
        .proposal
        .blocks
        .iter()
        .find(|block| block.id() == execution.frame_block_id)
        .ok_or(JournalQueryError::ExecutionShape)?;
    let delta = execution
        .proposal
        .blocks
        .iter()
        .find(|block| block.id() == execution.delta_block_id)
        .ok_or(JournalQueryError::ExecutionShape)?;
    let JournalBlock::Frame {
        source_block_id,
        snapshot_id,
        columns,
        preview_rows,
        row_count,
        truncated,
        ..
    } = frame
    else {
        return Err(JournalQueryError::ExecutionShape);
    };
    if source_block_id != &admission.block_id
        || snapshot_id != execution.frame_snapshot_id.as_str()
        || columns != &observation_frame_columns()
        || preview_rows != &observation_frame_rows(&execution.observation_frontier)?
        || *row_count != execution.observation_frontier.summary.unresolved
        || *truncated == execution.observation_frontier.complete
    {
        return Err(JournalQueryError::ExecutionShape);
    }
    let JournalBlock::Diff {
        source,
        target,
        direction,
        assessment,
        summary,
        ..
    } = delta
    else {
        return Err(JournalQueryError::ExecutionShape);
    };
    if !source.contains(execution.delta.delta_id.as_str())
        || !target.contains(execution.delta.delta_id.as_str())
        || direction != &execution.delta.direction
        || assessment != &execution.delta.assessment
        || !summary.contains(&execution.delta.inserted_rows.to_string())
    {
        return Err(JournalQueryError::ExecutionShape);
    }
    Ok(())
}

fn validate_timestamp(value: &str) -> Result<(), JournalQueryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| JournalQueryError::Timestamp(value.to_owned()))
}

fn identity_suffix(identity: &SemanticDigest) -> &str {
    let value = identity
        .as_str()
        .strip_prefix("blake3:")
        .unwrap_or(identity.as_str());
    &value[..16.min(value.len())]
}

fn empty_frame_identity() -> SemanticDigest {
    SemanticHasher::new("rey.journal-query-empty-frame.v1").finish()
}

fn placeholder_digest(domain: &str) -> SemanticDigest {
    SemanticHasher::new(&format!("{domain}.placeholder")).finish()
}

trait VerifyObservationFrontier {
    fn verify_projection(&self) -> Result<(), ObservationError>;
}

impl VerifyObservationFrontier for ObservationFrontier {
    fn verify_projection(&self) -> Result<(), ObservationError> {
        if self.schema != crate::observations::OBSERVATION_FRONTIER_SCHEMA
            || self.ordering != "observation_sequence_ascending"
            || self.limit == 0
            || self.rows.len() as u64 > self.limit
            || self.complete != (self.omitted == 0)
            || self.summary.unresolved != self.rows.len() as u64 + self.omitted
            || self.summary.observations
                != self
                    .summary
                    .unresolved
                    .saturating_add(self.summary.superseded)
                    .saturating_add(self.summary.resolved)
                    .saturating_add(self.summary.withdrawn)
            || self.summary.unbroadcast > self.summary.unresolved
            || self
                .rows
                .windows(2)
                .any(|rows| rows[0].observation.sequence >= rows[1].observation.sequence)
        {
            return Err(ObservationError::Identity("observation frontier"));
        }
        let mut hasher = SemanticHasher::new(crate::observations::OBSERVATION_FRONTIER_SCHEMA);
        hasher.add_str(self.source_log_id.as_str());
        hasher.add_u64(self.limit);
        hasher.add_bool(self.complete);
        hasher.add_u64(self.omitted);
        for row in &self.rows {
            hasher.add_str(row.observation.observation_id.as_str());
            for channel_id in &row.channel_ids {
                hasher.add_str(channel_id);
            }
        }
        if hasher.finish() != self.frontier_id {
            return Err(ObservationError::Identity("observation frontier"));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum JournalQueryError {
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Observation(#[from] ObservationError),
    #[error("journal query schema must be {expected}, got {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("journal entry {0} is unknown")]
    UnknownEntry(String),
    #[error("journal entry {0} has already been superseded")]
    SupersededEntry(SemanticDigest),
    #[error("journal block {0} is unknown")]
    UnknownBlock(String),
    #[error("journal block {0} is not a query")]
    NotQuery(String),
    #[error(
        "unsupported journal query {provider}/{language} {mode} {statement}; this slice accepts only rey.observations/rey read_only frontier"
    )]
    UnsupportedQuery {
        language: String,
        provider: String,
        mode: String,
        statement: String,
    },
    #[error("the observation frontier query accepts only the optional limit parameter")]
    UnsupportedParameter,
    #[error("journal query row limit is not a canonical integer: {0}")]
    InvalidRowLimit(String),
    #[error("journal query row limit {actual} is outside 1..={maximum}")]
    RowLimit { actual: u64, maximum: u64 },
    #[error("journal query admission is malformed")]
    AdmissionShape,
    #[error("journal query execution is malformed")]
    ExecutionShape,
    #[error("journal query delta is malformed")]
    DeltaShape,
    #[error("journal query sequence is malformed")]
    Sequence,
    #[error("journal query record limit reached")]
    RecordLimit,
    #[error("journal query result would exceed the Journal block limit")]
    ProposalBlockLimit,
    #[error("unknown journal query admission {0}")]
    UnknownAdmission(String),
    #[error("journal query admission is stale against Journal log {admitted} -> {current}")]
    StaleJournal {
        admitted: SemanticDigest,
        current: SemanticDigest,
    },
    #[error("journal query admission is stale against observation log {admitted} -> {current}")]
    StaleObservations {
        admitted: SemanticDigest,
        current: SemanticDigest,
    },
    #[error("journal query admission frontier is stale")]
    StaleFrontier,
    #[error("invalid journal query timestamp {0}")]
    Timestamp(String),
    #[error("journal query {kind} identity mismatch: declared {declared}, actual {actual}")]
    Identity {
        kind: &'static str,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("unsafe journal query state path {0}")]
    UnsafePath(PathBuf),
    #[error("journal query state exceeds its byte limit")]
    ByteLimit,
    #[error("journal query state {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("journal query state {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("journal query state lock {path} failed: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("journal query state could not allocate a temporary file in {0}")]
    TemporaryLimit(PathBuf),
    #[error("journal query JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::{
        channels::{ChannelObservationKind, LocalChannelStore},
        journal::{
            JOURNAL_PROPOSAL_SCHEMA, JournalAuthorKind, JournalBinding, JournalLayout,
            JournalLayoutKind,
        },
        observations::{
            LocalObservationStore, OBSERVATION_PROPOSAL_SCHEMA, ObservationAuthor,
            ObservationAuthorKind, ObservationCompleteness, ObservationProposal, ObservationSource,
        },
    };

    use super::*;

    fn journal_with_query(provider: &str) -> JournalLog {
        let blocks = vec![JournalBlock::Query {
            id: "open-observations".to_owned(),
            language: JOURNAL_QUERY_LANGUAGE.to_owned(),
            provider: provider.to_owned(),
            mode: "read_only".to_owned(),
            statement: JOURNAL_QUERY_STATEMENT.to_owned(),
            parameters: BTreeMap::from([("limit".to_owned(), "2".to_owned())]),
        }];
        let mut log = JournalLog::default();
        log.admit(
            JournalEntryProposal {
                schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
                title: "Inspect open observations".to_owned(),
                author: JournalAuthor {
                    kind: JournalAuthorKind::Agent,
                    id: "codex".to_owned(),
                },
                binding: JournalBinding {
                    coordinate: "rey+local://document/observations?revision=blake3%3Asource"
                        .to_owned(),
                    scale: 1.0,
                    source_revision: "blake3:source".to_owned(),
                },
                supersedes: None,
                layout: JournalLayout {
                    kind: JournalLayoutKind::Broadsheet,
                    columns: JOURNAL_BROADSHEET_COLUMNS,
                    bands: vec![JournalLayoutBand {
                        id: "query".to_owned(),
                        cells: vec![JournalLayoutCell {
                            block_id: "open-observations".to_owned(),
                            span: JOURNAL_BROADSHEET_COLUMNS,
                        }],
                    }],
                },
                blocks,
            },
            "2026-08-12T20:00:00Z",
        )
        .unwrap();
        log
    }

    fn observation_log(workspace: &TempDir, body: &str, timestamp: i64) -> ObservationLog {
        let channel_store = LocalChannelStore::default_for_workspace(workspace.path());
        let store = LocalObservationStore::default_for_workspace(workspace.path());
        let proposal = ObservationProposal {
            schema: OBSERVATION_PROPOSAL_SCHEMA.to_owned(),
            kind: ChannelObservationKind::Finding,
            author: ObservationAuthor {
                kind: ObservationAuthorKind::Agent,
                id: "codex".to_owned(),
            },
            subject_locator: "rey+local://workload/survey?revision=1".to_owned(),
            body: body.to_owned(),
            desired_delta: Some("Close the exact gap.".to_owned()),
            completeness: ObservationCompleteness::Complete,
            omissions: Vec::new(),
            evidence: Vec::new(),
            supersedes: None,
        };
        store
            .admit_and_broadcast(
                proposal,
                ObservationSource::workspace_file(
                    format!("workspace://observation-{timestamp}.json"),
                    body.as_bytes(),
                ),
                Vec::new(),
                None,
                &channel_store.status().unwrap().working,
                timestamp,
            )
            .unwrap();
        store.load().unwrap()
    }

    #[test]
    fn admission_and_execution_are_separate_and_produce_a_superseding_proposal() {
        let workspace = TempDir::new().unwrap();
        let journal = journal_with_query(JOURNAL_QUERY_PROVIDER);
        let observations = observation_log(&workspace, "One open finding.", 1);
        let store = LocalJournalQueryStore::default_for_workspace(workspace.path());
        let entry = &journal.entries[0];

        let admission = store
            .admit(
                &journal,
                &observations,
                entry.entry_id.as_str(),
                "open-observations",
                "2026-08-12T20:01:00Z",
            )
            .unwrap();
        assert!(admission.admitted);
        assert_eq!(store.load().unwrap().executions.len(), 0);

        let result = store
            .execute(
                &journal,
                &observations,
                admission.admission.admission_id.as_str(),
                JournalAuthor {
                    kind: JournalAuthorKind::Agent,
                    id: "codex".to_owned(),
                },
                "2026-08-12T20:02:00Z",
            )
            .unwrap();
        assert!(result.executed);
        assert_eq!(result.execution.observation_frontier.rows.len(), 1);
        assert_eq!(result.execution.delta.inserted_rows, 1);
        assert_eq!(result.execution.delta.assessment, "different");
        assert_eq!(
            result.execution.proposal.supersedes.as_ref(),
            Some(&entry.entry_id)
        );
        assert!(matches!(
            result
                .execution
                .proposal
                .blocks
                .get(result.execution.proposal.blocks.len() - 2),
            Some(JournalBlock::Frame { .. })
        ));
        assert!(matches!(
            result.execution.proposal.blocks.last(),
            Some(JournalBlock::Diff { .. })
        ));
        assert_eq!(journal.entries.len(), 1);

        let replay = store
            .execute(
                &JournalLog::default(),
                &ObservationLog::default(),
                admission.admission.admission_id.as_str(),
                JournalAuthor {
                    kind: JournalAuthorKind::Agent,
                    id: "codex".to_owned(),
                },
                "2026-08-12T20:03:00Z",
            )
            .unwrap();
        assert!(!replay.executed);
        assert_eq!(replay.execution.execution_id, result.execution.execution_id);
    }

    #[test]
    fn changed_inputs_unsupported_queries_and_tampered_state_fail_closed() {
        let workspace = TempDir::new().unwrap();
        let journal = journal_with_query(JOURNAL_QUERY_PROVIDER);
        let observations = observation_log(&workspace, "One open finding.", 1);
        let store = LocalJournalQueryStore::default_for_workspace(workspace.path());
        let admission = store
            .admit(
                &journal,
                &observations,
                journal.entries[0].entry_id.as_str(),
                "open-observations",
                "2026-08-12T20:01:00Z",
            )
            .unwrap();
        let changed = observation_log(&workspace, "A second finding.", 2);
        assert!(matches!(
            store.execute(
                &journal,
                &changed,
                admission.admission.admission_id.as_str(),
                JournalAuthor {
                    kind: JournalAuthorKind::Agent,
                    id: "codex".to_owned(),
                },
                "2026-08-12T20:02:00Z",
            ),
            Err(JournalQueryError::StaleObservations { .. })
        ));

        let unsupported = journal_with_query("database.example");
        assert!(matches!(
            store.admit(
                &unsupported,
                &changed,
                unsupported.entries[0].entry_id.as_str(),
                "open-observations",
                "2026-08-12T20:03:00Z",
            ),
            Err(JournalQueryError::UnsupportedQuery { .. })
        ));

        let mut state: serde_json::Value =
            serde_json::from_slice(&fs::read(store.path()).unwrap()).unwrap();
        state["admissions"][0]["authority"] = serde_json::json!("execute");
        fs::write(store.path(), serde_json::to_vec_pretty(&state).unwrap()).unwrap();
        assert!(store.load().is_err());
    }
}
