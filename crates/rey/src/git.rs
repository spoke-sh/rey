use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chrono::Utc;
use rey_core::{SemanticDigest, SemanticHasher};
use rey_git::{
    GitActivationProposal, GitActivationTrigger, GitError, GitPollCursor, GitPollTransition,
    GitSnapshot, derive_activation_proposals,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LOCAL_GIT_STATE_SCHEMA: &str = "rey.local-git-state.v1";
pub const GIT_POLL_RECORD_SCHEMA: &str = "rey.git-poll-record.v1";
pub const GIT_OPERATOR_STATUS_SCHEMA: &str = "rey.git-operator-status.v1";
pub const GIT_POLL_OUTCOME_SCHEMA: &str = "rey.git-poll-outcome.v1";
pub const GIT_ACKNOWLEDGEMENT_SCHEMA: &str = "rey.git-acknowledgement.v1";
pub const GIT_CADENCE_TICK_SCHEMA: &str = "rey.git-cadence-tick.v1";
pub const GIT_WATCH_OUTCOME_SCHEMA: &str = "rey.git-watch-outcome.v1";
pub const GIT_WATCH_RECEIPT_SCHEMA: &str = "rey.git-watch-receipt.v1";
pub const MAX_GIT_WATCH_ITERATIONS: u64 = 1_024;
pub const MAX_GIT_WATCH_INTERVAL_MS: u64 = 60_000;
pub const MAX_GIT_WATCH_ELAPSED_MS: u64 = 86_400_000;
const STATE_FILE_NAME: &str = "state.json";
const MAX_GIT_STATE_BYTES: u64 = 16 * 1_024 * 1_024;
const MAX_RETAINED_GIT_TRANSITIONS: usize = 1_024;
const MAX_RETAINED_GIT_CADENCE_TICKS: usize = 4_096;
const MAX_RETAINED_GIT_WATCH_RECEIPTS: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitOperatorStatus {
    pub schema: String,
    pub observed_snapshot: GitSnapshot,
    pub state: LocalGitState,
    pub changed_since_cursor: Option<bool>,
    pub repository_authority: String,
    pub next: String,
}

impl GitOperatorStatus {
    pub fn new(
        observed_snapshot: GitSnapshot,
        state: LocalGitState,
    ) -> Result<Self, LocalGitStateError> {
        observed_snapshot.verify()?;
        state.verify()?;
        let changed_since_cursor = state
            .cursor
            .as_ref()
            .map(|cursor| cursor.snapshot_id != observed_snapshot.snapshot_id);
        Ok(Self {
            schema: GIT_OPERATOR_STATUS_SCHEMA.to_owned(),
            observed_snapshot,
            changed_since_cursor,
            state,
            repository_authority: "read_only_observation; no Git mutation or workload execution"
                .to_owned(),
            next: "Initialize a retained cursor, poll/watch transitions, or acknowledge exact retained transition evidence"
                .to_owned(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollOutcome {
    pub schema: String,
    pub changed: bool,
    pub retained: bool,
    pub record: GitPollRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitAcknowledgement {
    pub schema: String,
    pub acknowledged_transition_id: SemanticDigest,
    pub cursor: GitPollCursor,
    pub retained_transition_count: u64,
    pub authority: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitCadenceTick {
    pub schema: String,
    pub tick_id: SemanticDigest,
    pub sequence: u64,
    pub observed_at_unix_ms: i64,
    pub source_cursor_id: SemanticDigest,
    pub source_snapshot_id: SemanticDigest,
    pub observed_snapshot_id: SemanticDigest,
    pub changed: bool,
    pub retained_transition_id: Option<SemanticDigest>,
    pub activation_ids: Vec<SemanticDigest>,
    pub interval_ms: u64,
    pub complete: bool,
    pub omissions: Vec<String>,
    pub authority: String,
}

impl GitCadenceTick {
    fn new(
        sequence: u64,
        cursor: &GitPollCursor,
        record: &GitPollRecord,
        changed: bool,
        interval_ms: u64,
    ) -> Result<Self, LocalGitStateError> {
        let mut omissions = record.transition.omissions.clone();
        if !record.target_snapshot.complete && omissions.is_empty() {
            omissions.push("Git snapshot is incomplete under provider semantics".to_owned());
        }
        omissions.sort();
        omissions.dedup();
        let mut activation_ids = record
            .proposals
            .iter()
            .map(|proposal| proposal.activation_id.clone())
            .collect::<Vec<_>>();
        activation_ids.sort();
        let mut tick = Self {
            schema: GIT_CADENCE_TICK_SCHEMA.to_owned(),
            tick_id: SemanticHasher::new("rey.git-cadence-tick.pending.v1").finish(),
            sequence,
            observed_at_unix_ms: Utc::now().timestamp_millis(),
            source_cursor_id: cursor.cursor_id.clone(),
            source_snapshot_id: cursor.snapshot_id.clone(),
            observed_snapshot_id: record.target_snapshot.snapshot_id.clone(),
            changed,
            retained_transition_id: changed.then(|| record.transition.transition_id.clone()),
            activation_ids,
            interval_ms,
            complete: omissions.is_empty(),
            omissions,
            authority: "cadence_observation_only; no Git mutation or workload execution".to_owned(),
        };
        tick.tick_id = git_cadence_tick_digest(&tick);
        tick.verify_against(record)?;
        Ok(tick)
    }

    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        if self.schema != GIT_CADENCE_TICK_SCHEMA
            || !is_semantic_digest(&self.tick_id)
            || self.sequence == 0
            || self.observed_at_unix_ms < 0
            || !is_semantic_digest(&self.source_cursor_id)
            || !is_semantic_digest(&self.source_snapshot_id)
            || !is_semantic_digest(&self.observed_snapshot_id)
            || self.interval_ms == 0
            || self.interval_ms > MAX_GIT_WATCH_INTERVAL_MS
            || self.complete != self.omissions.is_empty()
            || !is_canonical(&self.activation_ids)
            || !is_canonical(&self.omissions)
            || self
                .retained_transition_id
                .as_ref()
                .is_some_and(|transition| !is_semantic_digest(transition))
            || if self.changed {
                self.retained_transition_id.is_none()
            } else {
                self.retained_transition_id.is_some()
                    || !self.activation_ids.is_empty()
                    || self.source_snapshot_id != self.observed_snapshot_id
            }
            || self.authority != "cadence_observation_only; no Git mutation or workload execution"
            || self.tick_id != git_cadence_tick_digest(self)
        {
            return Err(LocalGitStateError::InvalidCadenceTick);
        }
        Ok(())
    }

    fn verify_against(&self, record: &GitPollRecord) -> Result<(), LocalGitStateError> {
        self.verify()?;
        record.verify()?;
        let changed = record.transition.source_snapshot_id != record.transition.target_snapshot_id
            || !record.transition.events.is_empty();
        let mut activation_ids = record
            .proposals
            .iter()
            .map(|proposal| proposal.activation_id.clone())
            .collect::<Vec<_>>();
        activation_ids.sort();
        let mut omissions = record.transition.omissions.clone();
        if !record.target_snapshot.complete && omissions.is_empty() {
            omissions.push("Git snapshot is incomplete under provider semantics".to_owned());
        }
        omissions.sort();
        omissions.dedup();
        if self.source_cursor_id != record.transition.source_cursor_id
            || self.source_snapshot_id != record.transition.source_snapshot_id
            || self.observed_snapshot_id != record.transition.target_snapshot_id
            || self.changed != changed
            || self.retained_transition_id.as_ref()
                != changed.then_some(&record.transition.transition_id)
            || self.activation_ids != activation_ids
            || self.omissions != omissions
            || self.complete != omissions.is_empty()
        {
            return Err(LocalGitStateError::InvalidCadenceTick);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GitWatchStopReason {
    PendingTransition,
    IterationLimit,
    TimeLimit,
}

impl GitWatchStopReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PendingTransition => "pending_transition",
            Self::IterationLimit => "iteration_limit",
            Self::TimeLimit => "time_limit",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitWatchOutcome {
    pub schema: String,
    pub watch_id: SemanticDigest,
    pub max_iterations: u64,
    pub interval_ms: u64,
    pub max_elapsed_ms: u64,
    pub elapsed_ms: u64,
    pub ticks: Vec<GitCadenceTick>,
    pub stop_reason: GitWatchStopReason,
    pub pending_transition_id: Option<SemanticDigest>,
    pub authority: String,
}

impl GitWatchOutcome {
    pub fn new(
        max_iterations: u64,
        interval_ms: u64,
        max_elapsed_ms: u64,
        elapsed_ms: u64,
        ticks: Vec<GitCadenceTick>,
        stop_reason: GitWatchStopReason,
    ) -> Result<Self, LocalGitStateError> {
        let pending_transition_id = ticks
            .last()
            .and_then(|tick| tick.retained_transition_id.clone());
        let mut outcome = Self {
            schema: GIT_WATCH_OUTCOME_SCHEMA.to_owned(),
            watch_id: SemanticHasher::new("rey.git-watch-outcome.pending.v1").finish(),
            max_iterations,
            interval_ms,
            max_elapsed_ms,
            elapsed_ms,
            ticks,
            stop_reason,
            pending_transition_id,
            authority:
                "bounded_cadence_observation; pending evidence requires explicit acknowledgement"
                    .to_owned(),
        };
        outcome.watch_id = git_watch_outcome_digest(&outcome);
        outcome.verify()?;
        Ok(outcome)
    }

    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        for tick in &self.ticks {
            tick.verify()?;
        }
        let canonical_sequence = self
            .ticks
            .windows(2)
            .all(|window| window[0].sequence + 1 == window[1].sequence);
        if self.schema != GIT_WATCH_OUTCOME_SCHEMA
            || !is_semantic_digest(&self.watch_id)
            || self.max_iterations == 0
            || self.max_iterations > MAX_GIT_WATCH_ITERATIONS
            || self.interval_ms == 0
            || self.interval_ms > MAX_GIT_WATCH_INTERVAL_MS
            || self.max_elapsed_ms == 0
            || self.max_elapsed_ms > MAX_GIT_WATCH_ELAPSED_MS
            || self.ticks.is_empty()
            || self.ticks.len() as u64 > self.max_iterations
            || self
                .ticks
                .iter()
                .any(|tick| tick.interval_ms != self.interval_ms)
            || !canonical_sequence
            || self.pending_transition_id
                != self
                    .ticks
                    .last()
                    .and_then(|tick| tick.retained_transition_id.clone())
            || ((self.stop_reason == GitWatchStopReason::PendingTransition)
                != self.pending_transition_id.is_some())
            || (self.stop_reason == GitWatchStopReason::IterationLimit
                && self.ticks.len() as u64 != self.max_iterations)
            || (self.stop_reason == GitWatchStopReason::TimeLimit
                && self.elapsed_ms < self.max_elapsed_ms
                && self.elapsed_ms.saturating_add(self.interval_ms) <= self.max_elapsed_ms)
            || self
                .ticks
                .iter()
                .take(self.ticks.len().saturating_sub(1))
                .any(|tick| tick.changed)
            || self.authority
                != "bounded_cadence_observation; pending evidence requires explicit acknowledgement"
            || self.watch_id != git_watch_outcome_digest(self)
        {
            return Err(LocalGitStateError::InvalidWatchOutcome);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitWatchReceipt {
    pub schema: String,
    pub watch_id: SemanticDigest,
    pub max_iterations: u64,
    pub interval_ms: u64,
    pub max_elapsed_ms: u64,
    pub elapsed_ms: u64,
    pub start_sequence: u64,
    pub end_sequence: u64,
    pub tick_ids: Vec<SemanticDigest>,
    pub stop_reason: GitWatchStopReason,
    pub pending_transition_id: Option<SemanticDigest>,
    pub authority: String,
}

impl GitWatchReceipt {
    fn from_outcome(outcome: &GitWatchOutcome) -> Result<Self, LocalGitStateError> {
        outcome.verify()?;
        let first = outcome
            .ticks
            .first()
            .ok_or(LocalGitStateError::InvalidWatchOutcome)?;
        let last = outcome
            .ticks
            .last()
            .ok_or(LocalGitStateError::InvalidWatchOutcome)?;
        let receipt = Self {
            schema: GIT_WATCH_RECEIPT_SCHEMA.to_owned(),
            watch_id: outcome.watch_id.clone(),
            max_iterations: outcome.max_iterations,
            interval_ms: outcome.interval_ms,
            max_elapsed_ms: outcome.max_elapsed_ms,
            elapsed_ms: outcome.elapsed_ms,
            start_sequence: first.sequence,
            end_sequence: last.sequence,
            tick_ids: outcome
                .ticks
                .iter()
                .map(|tick| tick.tick_id.clone())
                .collect(),
            stop_reason: outcome.stop_reason,
            pending_transition_id: outcome.pending_transition_id.clone(),
            authority: outcome.authority.clone(),
        };
        receipt.verify()?;
        Ok(receipt)
    }

    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        let unique_ticks = self.tick_ids.iter().collect::<BTreeSet<_>>();
        if self.schema != GIT_WATCH_RECEIPT_SCHEMA
            || !is_semantic_digest(&self.watch_id)
            || self.max_iterations == 0
            || self.max_iterations > MAX_GIT_WATCH_ITERATIONS
            || self.interval_ms == 0
            || self.interval_ms > MAX_GIT_WATCH_INTERVAL_MS
            || self.max_elapsed_ms == 0
            || self.max_elapsed_ms > MAX_GIT_WATCH_ELAPSED_MS
            || self.start_sequence == 0
            || self.end_sequence < self.start_sequence
            || self.end_sequence - self.start_sequence + 1 != self.tick_ids.len() as u64
            || self.tick_ids.is_empty()
            || self.tick_ids.len() as u64 > self.max_iterations
            || unique_ticks.len() != self.tick_ids.len()
            || self.tick_ids.iter().any(|tick| !is_semantic_digest(tick))
            || ((self.stop_reason == GitWatchStopReason::PendingTransition)
                != self.pending_transition_id.is_some())
            || (self.stop_reason == GitWatchStopReason::IterationLimit
                && self.tick_ids.len() as u64 != self.max_iterations)
            || (self.stop_reason == GitWatchStopReason::TimeLimit
                && self.elapsed_ms < self.max_elapsed_ms
                && self.elapsed_ms.saturating_add(self.interval_ms) <= self.max_elapsed_ms)
            || self.authority
                != "bounded_cadence_observation; pending evidence requires explicit acknowledgement"
            || self.watch_id != git_watch_receipt_digest(self)
        {
            return Err(LocalGitStateError::InvalidWatchReceipt);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GitPollRecord {
    pub schema: String,
    pub target_snapshot: GitSnapshot,
    pub transition: GitPollTransition,
    pub triggers: Vec<GitActivationTrigger>,
    pub proposals: Vec<GitActivationProposal>,
}

impl GitPollRecord {
    pub fn new(
        target_snapshot: GitSnapshot,
        transition: GitPollTransition,
        triggers: Vec<GitActivationTrigger>,
    ) -> Result<Self, LocalGitStateError> {
        let proposals = derive_activation_proposals(&transition, &triggers)?;
        let record = Self {
            schema: GIT_POLL_RECORD_SCHEMA.to_owned(),
            target_snapshot,
            transition,
            triggers,
            proposals,
        };
        record.verify()?;
        Ok(record)
    }

    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        self.target_snapshot.verify()?;
        self.transition.verify()?;
        for trigger in &self.triggers {
            trigger.verify()?;
        }
        for proposal in &self.proposals {
            proposal.verify()?;
        }
        if self.schema != GIT_POLL_RECORD_SCHEMA
            || self.transition.target_snapshot_id != self.target_snapshot.snapshot_id
            || self.transition.repository_id != self.target_snapshot.repository_id
            || self.transition.worktree_id != self.target_snapshot.worktree_id
            || self.transition.object_format != self.target_snapshot.object_format
            || derive_activation_proposals(&self.transition, &self.triggers)? != self.proposals
        {
            return Err(LocalGitStateError::InvalidState);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalGitState {
    pub schema: String,
    pub cursor_snapshot: Option<GitSnapshot>,
    pub cursor: Option<GitPollCursor>,
    pub pending: Option<GitPollRecord>,
    pub retained_polls: Vec<GitPollRecord>,
    #[serde(default)]
    pub cadence_ticks: Vec<GitCadenceTick>,
    #[serde(default)]
    pub watch_receipts: Vec<GitWatchReceipt>,
}

impl Default for LocalGitState {
    fn default() -> Self {
        Self {
            schema: LOCAL_GIT_STATE_SCHEMA.to_owned(),
            cursor_snapshot: None,
            cursor: None,
            pending: None,
            retained_polls: Vec::new(),
            cadence_ticks: Vec::new(),
            watch_receipts: Vec::new(),
        }
    }
}

impl LocalGitState {
    pub fn verify(&self) -> Result<(), LocalGitStateError> {
        if self.schema != LOCAL_GIT_STATE_SCHEMA
            || self.retained_polls.len() > MAX_RETAINED_GIT_TRANSITIONS
            || self.cadence_ticks.len() > MAX_RETAINED_GIT_CADENCE_TICKS
            || self.watch_receipts.len() > MAX_RETAINED_GIT_WATCH_RECEIPTS
            || self.cursor.is_some() != self.cursor_snapshot.is_some()
        {
            return Err(LocalGitStateError::InvalidState);
        }
        let (Some(cursor), Some(snapshot)) = (&self.cursor, &self.cursor_snapshot) else {
            if self.pending.is_some()
                || !self.retained_polls.is_empty()
                || !self.cadence_ticks.is_empty()
                || !self.watch_receipts.is_empty()
            {
                return Err(LocalGitStateError::InvalidState);
            }
            return Ok(());
        };
        cursor.verify()?;
        snapshot.verify()?;
        if cursor.snapshot_id != snapshot.snapshot_id
            || cursor.repository_id != snapshot.repository_id
            || cursor.worktree_id != snapshot.worktree_id
            || cursor.object_format != snapshot.object_format
            || cursor.watched_refs != snapshot.watched_refs
        {
            return Err(LocalGitStateError::InvalidState);
        }
        let mut replayed_cursor = self.retained_polls.first().map(|poll| {
            let transition = &poll.transition;
            GitPollCursor {
                schema: rey_git::GIT_POLL_CURSOR_SCHEMA.to_owned(),
                cursor_id: transition.source_cursor_id.clone(),
                repository_id: transition.repository_id.clone(),
                worktree_id: transition.worktree_id.clone(),
                snapshot_id: transition.source_snapshot_id.clone(),
                object_format: transition.object_format.clone(),
                shallow: transition.source_shallow,
                head: transition.source_head.clone(),
                watched_refs: transition.source_watched_refs.clone(),
                index_digest: transition.source_index_digest.clone(),
                index_complete: transition.source_index_complete,
                index_conflicted: transition.source_index_conflicted,
                provider_revision: rey_environment::LOCAL_PROVIDER_REVISION,
                retained_evidence_id: transition.source_snapshot_id.clone(),
            }
        });
        for poll in &self.retained_polls {
            poll.verify()?;
            let transition = &poll.transition;
            transition.verify()?;
            let expected = replayed_cursor
                .as_ref()
                .ok_or(LocalGitStateError::InvalidState)?;
            expected.verify()?;
            if expected.cursor_id != transition.source_cursor_id
                || expected.snapshot_id != transition.source_snapshot_id
            {
                return Err(LocalGitStateError::InvalidState);
            }
            replayed_cursor = Some(expected.advance(transition, transition.transition_id.clone())?);
        }
        if let Some(replayed) = replayed_cursor {
            if &replayed != cursor {
                return Err(LocalGitStateError::InvalidState);
            }
        } else if cursor.retained_evidence_id != snapshot.snapshot_id {
            return Err(LocalGitStateError::InvalidState);
        }
        if let Some(pending) = &self.pending {
            pending.verify()?;
            if pending.transition.source_cursor_id != cursor.cursor_id
                || pending.transition.source_snapshot_id != cursor.snapshot_id
            {
                return Err(LocalGitStateError::InvalidState);
            }
        }
        for (index, tick) in self.cadence_ticks.iter().enumerate() {
            tick.verify()?;
            if tick.sequence != index as u64 + 1 {
                return Err(LocalGitStateError::InvalidState);
            }
            if let Some(transition_id) = &tick.retained_transition_id {
                let record = self
                    .pending
                    .iter()
                    .chain(self.retained_polls.iter())
                    .find(|record| &record.transition.transition_id == transition_id)
                    .ok_or(LocalGitStateError::InvalidState)?;
                tick.verify_against(record)?;
            } else if !cursor_boundary_exists(self, tick) {
                return Err(LocalGitStateError::InvalidState);
            }
        }
        for (index, receipt) in self.watch_receipts.iter().enumerate() {
            receipt.verify()?;
            if index > 0 && self.watch_receipts[index - 1].end_sequence >= receipt.start_sequence {
                return Err(LocalGitStateError::InvalidState);
            }
            let start = usize::try_from(receipt.start_sequence - 1)
                .map_err(|_| LocalGitStateError::InvalidState)?;
            let end = usize::try_from(receipt.end_sequence)
                .map_err(|_| LocalGitStateError::InvalidState)?;
            let ticks = self
                .cadence_ticks
                .get(start..end)
                .ok_or(LocalGitStateError::InvalidState)?;
            if ticks
                .iter()
                .map(|tick| &tick.tick_id)
                .ne(receipt.tick_ids.iter())
                || ticks
                    .iter()
                    .any(|tick| tick.interval_ms != receipt.interval_ms)
                || receipt.pending_transition_id
                    != ticks
                        .last()
                        .and_then(|tick| tick.retained_transition_id.clone())
                || ticks
                    .iter()
                    .take(ticks.len().saturating_sub(1))
                    .any(|tick| tick.changed)
                || ((receipt.stop_reason == GitWatchStopReason::PendingTransition)
                    != ticks.last().is_some_and(|tick| tick.changed))
            {
                return Err(LocalGitStateError::InvalidState);
            }
        }
        Ok(())
    }

    pub fn acknowledged_activation(
        &self,
        activation_id: &str,
    ) -> Result<GitActivationProposal, LocalGitStateError> {
        self.verify()?;
        if let Some(proposal) = self
            .retained_polls
            .iter()
            .flat_map(|poll| &poll.proposals)
            .find(|proposal| proposal.activation_id.as_str() == activation_id)
        {
            return Ok(proposal.clone());
        }
        if self.pending.as_ref().is_some_and(|pending| {
            pending
                .proposals
                .iter()
                .any(|proposal| proposal.activation_id.as_str() == activation_id)
        }) {
            return Err(LocalGitStateError::ActivationNotAcknowledged(
                activation_id.to_owned(),
            ));
        }
        Err(LocalGitStateError::UnknownActivation(
            activation_id.to_owned(),
        ))
    }
}

fn cursor_boundary_exists(state: &LocalGitState, tick: &GitCadenceTick) -> bool {
    state.cursor.as_ref().is_some_and(|cursor| {
        cursor.cursor_id == tick.source_cursor_id && cursor.snapshot_id == tick.source_snapshot_id
    }) || state.retained_polls.iter().any(|record| {
        record.transition.source_cursor_id == tick.source_cursor_id
            && record.transition.source_snapshot_id == tick.source_snapshot_id
    })
}

fn git_cadence_tick_digest(tick: &GitCadenceTick) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_CADENCE_TICK_SCHEMA);
    hasher.add_u64(tick.sequence);
    hasher.add_u64(tick.observed_at_unix_ms as u64);
    hasher.add_str(tick.source_cursor_id.as_str());
    hasher.add_str(tick.source_snapshot_id.as_str());
    hasher.add_str(tick.observed_snapshot_id.as_str());
    hasher.add_bool(tick.changed);
    hasher.add_optional_str(
        tick.retained_transition_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_u64(tick.activation_ids.len() as u64);
    for activation_id in &tick.activation_ids {
        hasher.add_str(activation_id.as_str());
    }
    hasher.add_u64(tick.interval_ms);
    hasher.add_bool(tick.complete);
    hasher.add_u64(tick.omissions.len() as u64);
    for omission in &tick.omissions {
        hasher.add_str(omission);
    }
    hasher.add_str(&tick.authority);
    hasher.finish()
}

fn git_watch_outcome_digest(outcome: &GitWatchOutcome) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_WATCH_OUTCOME_SCHEMA);
    hasher.add_u64(outcome.max_iterations);
    hasher.add_u64(outcome.interval_ms);
    hasher.add_u64(outcome.max_elapsed_ms);
    hasher.add_u64(outcome.elapsed_ms);
    hasher.add_u64(outcome.ticks.len() as u64);
    for tick in &outcome.ticks {
        hasher.add_str(tick.tick_id.as_str());
    }
    hasher.add_str(outcome.stop_reason.as_str());
    hasher.add_optional_str(
        outcome
            .pending_transition_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_str(&outcome.authority);
    hasher.finish()
}

fn git_watch_receipt_digest(receipt: &GitWatchReceipt) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(GIT_WATCH_OUTCOME_SCHEMA);
    hasher.add_u64(receipt.max_iterations);
    hasher.add_u64(receipt.interval_ms);
    hasher.add_u64(receipt.max_elapsed_ms);
    hasher.add_u64(receipt.elapsed_ms);
    hasher.add_u64(receipt.tick_ids.len() as u64);
    for tick_id in &receipt.tick_ids {
        hasher.add_str(tick_id.as_str());
    }
    hasher.add_str(receipt.stop_reason.as_str());
    hasher.add_optional_str(
        receipt
            .pending_transition_id
            .as_ref()
            .map(SemanticDigest::as_str),
    );
    hasher.add_str(&receipt.authority);
    hasher.finish()
}

fn is_semantic_digest(digest: &SemanticDigest) -> bool {
    let Some(value) = digest.as_str().strip_prefix("blake3:") else {
        return false;
    };
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_canonical<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

#[derive(Clone, Debug)]
pub struct LocalGitStore {
    directory: PathBuf,
}

impl LocalGitStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("git"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<LocalGitState, LocalGitStateError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalGitState::default());
            }
            Err(source) => return Err(LocalGitStateError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalGitStateError::UnsafePath(path));
        }
        if metadata.len() > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalGitStateError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_GIT_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalGitStateError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        let state = serde_json::from_slice::<LocalGitState>(&bytes).map_err(|source| {
            LocalGitStateError::Json {
                path: path.clone(),
                source,
            }
        })?;
        state.verify()?;
        Ok(state)
    }

    pub fn initialize(&self, snapshot: GitSnapshot) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        if state.cursor.is_some() {
            return Err(LocalGitStateError::AlreadyInitialized);
        }
        let cursor =
            GitPollCursor::from_retained_snapshot(&snapshot, snapshot.snapshot_id.clone())?;
        state.cursor_snapshot = Some(snapshot);
        state.cursor = Some(cursor);
        self.save(&state)?;
        Ok(state)
    }

    pub fn retain_poll(&self, record: GitPollRecord) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        if record.transition.source_cursor_id != cursor.cursor_id {
            return Err(LocalGitStateError::StalePoll);
        }
        if let Some(pending) = &state.pending {
            if pending == &record {
                return Ok(state);
            }
            return Err(LocalGitStateError::PendingPoll(
                pending.transition.transition_id.clone(),
            ));
        }
        state.pending = Some(record);
        self.save(&state)?;
        Ok(state)
    }

    pub fn retain_cadence_poll(
        &self,
        record: GitPollRecord,
        interval_ms: u64,
    ) -> Result<(LocalGitState, GitCadenceTick), LocalGitStateError> {
        let mut state = self.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        if record.transition.source_cursor_id != cursor.cursor_id
            || record.transition.source_snapshot_id != cursor.snapshot_id
        {
            return Err(LocalGitStateError::StalePoll);
        }
        if let Some(pending) = &state.pending {
            return Err(LocalGitStateError::PendingPoll(
                pending.transition.transition_id.clone(),
            ));
        }
        if state.cadence_ticks.len() >= MAX_RETAINED_GIT_CADENCE_TICKS {
            return Err(LocalGitStateError::CadenceTickLimit(
                MAX_RETAINED_GIT_CADENCE_TICKS,
            ));
        }
        let changed = record.transition.source_snapshot_id != record.transition.target_snapshot_id
            || !record.transition.events.is_empty();
        let tick = GitCadenceTick::new(
            state.cadence_ticks.len() as u64 + 1,
            cursor,
            &record,
            changed,
            interval_ms,
        )?;
        if changed {
            state.pending = Some(record);
        }
        state.cadence_ticks.push(tick.clone());
        self.save(&state)?;
        Ok((state, tick))
    }

    pub fn retain_watch_outcome(
        &self,
        outcome: &GitWatchOutcome,
    ) -> Result<LocalGitState, LocalGitStateError> {
        let receipt = GitWatchReceipt::from_outcome(outcome)?;
        let mut state = self.load()?;
        if let Some(existing) = state
            .watch_receipts
            .iter()
            .find(|existing| existing.watch_id == receipt.watch_id)
        {
            if existing == &receipt {
                return Ok(state);
            }
            return Err(LocalGitStateError::InvalidWatchReceipt);
        }
        if state.watch_receipts.len() >= MAX_RETAINED_GIT_WATCH_RECEIPTS {
            return Err(LocalGitStateError::WatchReceiptLimit(
                MAX_RETAINED_GIT_WATCH_RECEIPTS,
            ));
        }
        state.watch_receipts.push(receipt);
        self.save(&state)?;
        Ok(state)
    }

    pub fn acknowledge(
        &self,
        expected_transition_id: &str,
    ) -> Result<LocalGitState, LocalGitStateError> {
        let mut state = self.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        let pending = state
            .pending
            .take()
            .ok_or(LocalGitStateError::NoPendingPoll)?;
        if pending.transition.transition_id.as_str() != expected_transition_id {
            return Err(LocalGitStateError::StaleAcknowledgement {
                expected: pending.transition.transition_id,
                actual: expected_transition_id.to_owned(),
            });
        }
        if state.retained_polls.len() >= MAX_RETAINED_GIT_TRANSITIONS {
            return Err(LocalGitStateError::TransitionLimit(
                MAX_RETAINED_GIT_TRANSITIONS,
            ));
        }
        let advanced = cursor.advance(
            &pending.transition,
            pending.transition.transition_id.clone(),
        )?;
        state.cursor_snapshot = Some(pending.target_snapshot.clone());
        state.retained_polls.push(pending);
        state.cursor = Some(advanced);
        self.save(&state)?;
        Ok(state)
    }

    pub fn save(&self, state: &LocalGitState) -> Result<(), LocalGitStateError> {
        state.verify()?;
        let mut bytes =
            serde_json::to_vec_pretty(state).map_err(|source| LocalGitStateError::Json {
                path: self.path(),
                source,
            })?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_GIT_STATE_BYTES {
            return Err(LocalGitStateError::ByteLimit(MAX_GIT_STATE_BYTES));
        }
        self.prepare_directory()?;
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalGitStateError::UnsafePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = (|| {
            file.write_all(&bytes).and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(LocalGitStateError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), LocalGitStateError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(LocalGitStateError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| LocalGitStateError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(LocalGitStateError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), LocalGitStateError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(LocalGitStateError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(LocalGitStateError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(LocalGitStateError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), LocalGitStateError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".state.json.tmp-{}-{attempt}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalGitStateError::Write { path, source }),
            }
        }
        Err(LocalGitStateError::TemporaryLimit(self.directory.clone()))
    }
}

#[derive(Debug, Error)]
pub enum LocalGitStateError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("local Git state is invalid or semantically tampered")]
    InvalidState,
    #[error("local Git state path is symlinked or has the wrong file type: {0}")]
    UnsafePath(PathBuf),
    #[error("local Git state at {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local Git state at {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local Git state JSON at {path} is invalid: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("local Git state exceeds the {0}-byte limit")]
    ByteLimit(u64),
    #[error("local Git state temporary-file attempts were exhausted in {0}")]
    TemporaryLimit(PathBuf),
    #[error("Git polling is already initialized")]
    AlreadyInitialized,
    #[error("Git polling has no retained cursor; run `rey git init` first")]
    Uninitialized,
    #[error("Git poll was derived from a stale cursor")]
    StalePoll,
    #[error("Git transition {0} is already pending acknowledgement")]
    PendingPoll(SemanticDigest),
    #[error("there is no retained Git transition awaiting acknowledgement")]
    NoPendingPoll,
    #[error("Git acknowledgement expected {expected}, not {actual}")]
    StaleAcknowledgement {
        expected: SemanticDigest,
        actual: String,
    },
    #[error("retained Git transition history exceeds {0}")]
    TransitionLimit(usize),
    #[error("retained Git cadence history exceeds {0} ticks")]
    CadenceTickLimit(usize),
    #[error("retained Git watch history exceeds {0} receipts")]
    WatchReceiptLimit(usize),
    #[error("retained Git cadence tick is invalid or semantically tampered")]
    InvalidCadenceTick,
    #[error("Git watch outcome is invalid or semantically tampered")]
    InvalidWatchOutcome,
    #[error("retained Git watch receipt is invalid or semantically tampered")]
    InvalidWatchReceipt,
    #[error("Git activation {0} is pending and must be acknowledged before workload admission")]
    ActivationNotAcknowledged(String),
    #[error("unknown acknowledged Git activation {0}")]
    UnknownActivation(String),
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command};

    use rey_environment::resolve_executable;
    use rey_git::{GitInspector, GitLimits};
    use tempfile::TempDir;

    use super::{GitPollRecord, LocalGitStateError, LocalGitStore};

    fn git(directory: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-C", directory.to_str().unwrap()])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(status.success(), "git fixture command failed: {args:?}");
    }

    fn repository() -> TempDir {
        let directory = TempDir::new().unwrap();
        git(directory.path(), &["init", "-q"]);
        git(directory.path(), &["config", "user.name", "Rey Test"]);
        git(
            directory.path(),
            &["config", "user.email", "rey@example.invalid"],
        );
        fs::write(directory.path().join("tracked"), "one\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "initial"]);
        directory
    }

    fn inspector(directory: &Path) -> GitInspector {
        let paths = std::env::split_paths(&std::env::var_os("PATH").unwrap()).collect::<Vec<_>>();
        GitInspector {
            git_program: resolve_executable("git", &paths).unwrap(),
            workspace: directory.to_owned(),
            limits: GitLimits::default(),
        }
    }

    #[test]
    fn local_store_retains_pending_evidence_before_advancing_the_cursor() {
        let directory = repository();
        let inspect = inspector(directory.path());
        let store = LocalGitStore::default_for_workspace(directory.path());
        let initial = inspect.inspect().unwrap().unwrap();
        let state = store.initialize(initial).unwrap();
        let initial_cursor = state.cursor.unwrap();

        fs::write(directory.path().join("tracked"), "two\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        git(directory.path(), &["commit", "-q", "-m", "second"]);
        let (target, transition) = inspect
            .inspect_transition(&initial_cursor)
            .unwrap()
            .unwrap();
        let record = GitPollRecord::new(target, transition.clone(), Vec::new()).unwrap();
        let retained = store.retain_poll(record.clone()).unwrap();
        assert_eq!(retained.pending, Some(record.clone()));
        assert_eq!(retained.cursor, Some(initial_cursor));
        assert_eq!(store.retain_poll(record).unwrap(), retained);

        assert!(matches!(
            store.acknowledge(
                "blake3:0000000000000000000000000000000000000000000000000000000000000000"
            ),
            Err(LocalGitStateError::StaleAcknowledgement { .. })
        ));
        assert!(store.load().unwrap().pending.is_some());

        let advanced = store
            .acknowledge(transition.transition_id.as_str())
            .unwrap();
        assert!(advanced.pending.is_none());
        assert_eq!(advanced.retained_polls.len(), 1);
        assert_eq!(advanced.retained_polls[0].transition, transition);
        assert_eq!(store.load().unwrap(), advanced);

        fs::write(directory.path().join("tracked"), "staged\n").unwrap();
        git(directory.path(), &["add", "tracked"]);
        let cursor = advanced.cursor.as_ref().unwrap();
        let (target, transition) = inspect.inspect_transition(cursor).unwrap().unwrap();
        store
            .retain_poll(GitPollRecord::new(target, transition.clone(), Vec::new()).unwrap())
            .unwrap();
        let advanced = store
            .acknowledge(transition.transition_id.as_str())
            .unwrap();
        assert_eq!(advanced.retained_polls.len(), 2);
        advanced.verify().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_local_git_state_ancestor_fails_closed() {
        use std::os::unix::fs::symlink;

        let directory = repository();
        let outside = TempDir::new().unwrap();
        symlink(outside.path(), directory.path().join(".rey")).unwrap();
        let store = LocalGitStore::default_for_workspace(directory.path());
        assert!(matches!(
            store.load(),
            Err(LocalGitStateError::UnsafePath(_))
        ));
    }
}
