use std::collections::{BTreeMap, BTreeSet};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_environment::{SourcePathEncoding, SourcePathIdentity, SourceSearchEvidence};
use rey_mining::MiningCompleteness;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DeltaAssessment;

pub const SOURCE_MATCH_DELTA_SCHEMA: &str = "rey.source-match-delta.v1";

#[must_use]
pub fn source_match_table_projection() -> ContractIdentity {
    ContractIdentity::new(
        "rey.source-matches.terminal-table",
        1,
        "render one typed expected-to-observed source-match relation as ANSI-independent assessment counts, changed rows, canonical observed matches, native context, omissions, effective limits, and exact deep bindings",
    )
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourceMatchKey {
    pub path_encoding: SourcePathEncoding,
    pub path_identity: String,
    pub start_byte: u64,
    pub end_byte: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedSourceMatch {
    pub path: SourcePathIdentity,
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub start_byte_in_line: u64,
    pub end_line: u64,
    pub end_byte_in_line: u64,
    pub matched_text: String,
    pub context_text: String,
}

impl ExpectedSourceMatch {
    #[must_use]
    pub fn key(&self) -> SourceMatchKey {
        SourceMatchKey {
            path_encoding: self.path.encoding,
            path_identity: self.path.encoded.clone(),
            start_byte: self.start_byte,
            end_byte: self.end_byte,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedSourceMatch {
    pub key: SourceMatchKey,
    pub path_display: String,
    pub source_artifact_id: SemanticDigest,
    pub match_id: SemanticDigest,
    pub start_line: u64,
    pub start_byte_in_line: u64,
    pub end_line: u64,
    pub end_byte_in_line: u64,
    pub matched_text: String,
    pub context_artifact_id: SemanticDigest,
    pub context_text: String,
    pub context_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMatchChangeKind {
    Inserted,
    Deleted,
    Modified,
}

impl SourceMatchChangeKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Inserted => "inserted",
            Self::Deleted => "deleted",
            Self::Modified => "modified",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatchChange {
    pub kind: SourceMatchChangeKind,
    pub key: SourceMatchKey,
    pub expected: Option<ExpectedSourceMatch>,
    pub observed: Option<ObservedSourceMatch>,
    pub changed_fields: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatchDeltaInputs {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: ContractIdentity,
    pub comparator: ContractIdentity,
    pub binding_id: SemanticDigest,
    pub mining_request_id: SemanticDigest,
    pub mining_result_id: SemanticDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatchDeltaLimits {
    pub max_expected_rows: u64,
    pub max_observed_rows: u64,
    pub max_changes: u64,
    pub max_string_bytes: u64,
}

impl Default for SourceMatchDeltaLimits {
    fn default() -> Self {
        Self {
            max_expected_rows: 4_096,
            max_observed_rows: 4_096,
            max_changes: 8_192,
            max_string_bytes: 2 * 1_024 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatchDeltaSummary {
    pub expected_rows: u64,
    pub observed_rows: u64,
    pub equal_rows: u64,
    pub inserted: u64,
    pub deleted: u64,
    pub modified: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatchDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub inputs: SourceMatchDeltaInputs,
    pub expected_relation_id: SemanticDigest,
    pub observed_relation_id: Option<SemanticDigest>,
    pub completeness: MiningCompleteness,
    pub assessment: DeltaAssessment,
    pub summary: SourceMatchDeltaSummary,
    pub expected: Vec<ExpectedSourceMatch>,
    pub observed: Vec<ObservedSourceMatch>,
    pub changes: Vec<SourceMatchChange>,
    pub limits: SourceMatchDeltaLimits,
}

impl SourceMatchDelta {
    pub fn verify(&self, evidence: &SourceSearchEvidence) -> Result<(), SourceMatchDeltaError> {
        if self.schema != SOURCE_MATCH_DELTA_SCHEMA {
            return Err(SourceMatchDeltaError::UnsupportedSchema(
                self.schema.clone(),
            ));
        }
        let recomputed = compare_source_matches(
            self.inputs.clone(),
            self.expected.clone(),
            evidence,
            self.limits.clone(),
        )?;
        if self != &recomputed {
            return Err(SourceMatchDeltaError::ReplayMismatch);
        }
        Ok(())
    }
}

pub fn source_match_comparator() -> ContractIdentity {
    ContractIdentity::new(
        "rey.source-matches.expected-observed",
        1,
        "compare expected to observed source matches by reversible path identity and byte span; preserve typed insertions, deletions, modifications, context, native source ids, and incomplete mining status",
    )
}

pub fn compare_source_matches(
    inputs: SourceMatchDeltaInputs,
    mut expected: Vec<ExpectedSourceMatch>,
    evidence: &SourceSearchEvidence,
    limits: SourceMatchDeltaLimits,
) -> Result<SourceMatchDelta, SourceMatchDeltaError> {
    validate_inputs(&inputs)?;
    validate_limits(&limits)?;
    if inputs.comparator != source_match_comparator()
        || inputs.binding_id != evidence.binding_id
        || inputs.mining_request_id != evidence.result.request_id
        || inputs.mining_result_id != evidence.result.result_id
    {
        return Err(SourceMatchDeltaError::EvidenceBinding);
    }
    if expected.len() as u64 > limits.max_expected_rows {
        return Err(SourceMatchDeltaError::RowLimit {
            role: "expected",
            limit: limits.max_expected_rows,
            observed: expected.len() as u64,
        });
    }
    if evidence.matches.len() as u64 > limits.max_observed_rows {
        return Err(SourceMatchDeltaError::RowLimit {
            role: "observed",
            limit: limits.max_observed_rows,
            observed: evidence.matches.len() as u64,
        });
    }
    for row in &expected {
        validate_expected(row)?;
    }
    expected.sort_by_key(ExpectedSourceMatch::key);
    let expected_by_key = expected
        .iter()
        .map(|row| (row.key(), row))
        .collect::<BTreeMap<_, _>>();
    if expected_by_key.len() != expected.len() {
        return Err(SourceMatchDeltaError::DuplicateKey("expected"));
    }

    let mut observed = Vec::with_capacity(evidence.matches.len());
    for row in &evidence.matches {
        let context = evidence
            .contexts
            .iter()
            .find(|context| context.artifact_id == row.context_artifact_id)
            .ok_or(SourceMatchDeltaError::MissingContext)?;
        observed.push(ObservedSourceMatch {
            key: SourceMatchKey {
                path_encoding: row.path.encoding,
                path_identity: row.path.encoded.clone(),
                start_byte: row.start_byte,
                end_byte: row.end_byte,
            },
            path_display: row.path.display.clone(),
            source_artifact_id: row.source_artifact_id.clone(),
            match_id: row.match_id.clone(),
            start_line: row.start_line,
            start_byte_in_line: row.start_byte_in_line,
            end_line: row.end_line,
            end_byte_in_line: row.end_byte_in_line,
            matched_text: row.matched_text.clone(),
            context_artifact_id: row.context_artifact_id.clone(),
            context_text: context.text.clone(),
            context_ref: row.context_ref.clone(),
        });
    }
    observed.sort_by(|left, right| left.key.cmp(&right.key));
    let observed_by_key = observed
        .iter()
        .map(|row| (row.key.clone(), row))
        .collect::<BTreeMap<_, _>>();
    if observed_by_key.len() != observed.len() {
        return Err(SourceMatchDeltaError::DuplicateKey("observed"));
    }

    let mut changes = Vec::new();
    let mut equal_rows = 0_u64;
    let keys = expected_by_key
        .keys()
        .chain(observed_by_key.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in &keys {
        match (expected_by_key.get(key), observed_by_key.get(key)) {
            (Some(expected), Some(observed)) => {
                let changed_fields = changed_fields(expected, observed);
                if changed_fields.is_empty() {
                    equal_rows = equal_rows.saturating_add(1);
                } else {
                    changes.push(SourceMatchChange {
                        kind: SourceMatchChangeKind::Modified,
                        key: key.clone(),
                        expected: Some((*expected).clone()),
                        observed: Some((*observed).clone()),
                        changed_fields,
                    });
                }
            }
            (Some(expected), None) => changes.push(SourceMatchChange {
                kind: SourceMatchChangeKind::Deleted,
                key: key.clone(),
                expected: Some((*expected).clone()),
                observed: None,
                changed_fields: Vec::new(),
            }),
            (None, Some(observed)) => changes.push(SourceMatchChange {
                kind: SourceMatchChangeKind::Inserted,
                key: key.clone(),
                expected: None,
                observed: Some((*observed).clone()),
                changed_fields: Vec::new(),
            }),
            (None, None) => unreachable!("key comes from one side"),
        }
    }
    changes.sort_by(|left, right| left.key.cmp(&right.key));
    if changes.len() as u64 > limits.max_changes {
        return Err(SourceMatchDeltaError::ChangeLimit {
            limit: limits.max_changes,
            observed: changes.len() as u64,
        });
    }
    let string_bytes = expected
        .iter()
        .map(expected_string_bytes)
        .sum::<u64>()
        .saturating_add(observed.iter().map(observed_string_bytes).sum::<u64>())
        .saturating_add(
            changes
                .iter()
                .flat_map(|change| &change.changed_fields)
                .map(|field| field.len() as u64)
                .sum::<u64>(),
        );
    if string_bytes > limits.max_string_bytes {
        return Err(SourceMatchDeltaError::StringByteLimit {
            limit: limits.max_string_bytes,
            observed: string_bytes,
        });
    }

    let summary = SourceMatchDeltaSummary {
        expected_rows: expected.len() as u64,
        observed_rows: observed.len() as u64,
        equal_rows,
        inserted: changes
            .iter()
            .filter(|change| change.kind == SourceMatchChangeKind::Inserted)
            .count() as u64,
        deleted: changes
            .iter()
            .filter(|change| change.kind == SourceMatchChangeKind::Deleted)
            .count() as u64,
        modified: changes
            .iter()
            .filter(|change| change.kind == SourceMatchChangeKind::Modified)
            .count() as u64,
    };
    let expected_relation_id = expected_relation_id(&inputs.scenario, &expected);
    let observed_relation_id = evidence
        .match_artifact
        .as_ref()
        .map(|artifact| artifact.artifact_id.clone());
    let assessment = if evidence.result.completeness != MiningCompleteness::Complete {
        DeltaAssessment::Inconclusive
    } else if changes.is_empty() {
        DeltaAssessment::Equal
    } else {
        DeltaAssessment::Different
    };
    let mut delta = SourceMatchDelta {
        schema: SOURCE_MATCH_DELTA_SCHEMA.to_owned(),
        delta_id: placeholder_digest(),
        inputs,
        expected_relation_id,
        observed_relation_id,
        completeness: evidence.result.completeness,
        assessment,
        summary,
        expected,
        observed,
        changes,
        limits,
    };
    delta.delta_id = delta_digest(&delta);
    Ok(delta)
}

fn changed_fields(expected: &ExpectedSourceMatch, observed: &ObservedSourceMatch) -> Vec<String> {
    let mut fields = Vec::new();
    if expected.path.display != observed.path_display {
        fields.push("path_display".to_owned());
    }
    if expected.start_line != observed.start_line {
        fields.push("start_line".to_owned());
    }
    if expected.start_byte_in_line != observed.start_byte_in_line {
        fields.push("start_byte_in_line".to_owned());
    }
    if expected.end_line != observed.end_line {
        fields.push("end_line".to_owned());
    }
    if expected.end_byte_in_line != observed.end_byte_in_line {
        fields.push("end_byte_in_line".to_owned());
    }
    if expected.matched_text != observed.matched_text {
        fields.push("matched_text".to_owned());
    }
    if expected.context_text != observed.context_text {
        fields.push("context_text".to_owned());
    }
    fields
}

fn validate_inputs(inputs: &SourceMatchDeltaInputs) -> Result<(), SourceMatchDeltaError> {
    for contract in [
        &inputs.workload,
        &inputs.graph,
        &inputs.scenario,
        &inputs.comparator,
    ] {
        if contract.id.is_empty() || contract.revision == 0 {
            return Err(SourceMatchDeltaError::InvalidContract);
        }
        validate_digest(&contract.semantic_digest)?;
    }
    for digest in [
        &inputs.binding_id,
        &inputs.mining_request_id,
        &inputs.mining_result_id,
    ] {
        validate_digest(digest)?;
    }
    Ok(())
}

fn validate_expected(row: &ExpectedSourceMatch) -> Result<(), SourceMatchDeltaError> {
    let valid_line_span = row.start_line > 0
        && row.end_line >= row.start_line
        && (row.end_line != row.start_line || row.end_byte_in_line > row.start_byte_in_line);
    if row.path.encoded.is_empty()
        || row.path.display.is_empty()
        || row.path.encoded.contains('\0')
        || row.path.display.contains('\0')
        || row.start_byte >= row.end_byte
        || !valid_line_span
        || row.matched_text.is_empty()
        || row.matched_text.contains('\0')
        || row.context_text.contains('\0')
        || row.end_byte.saturating_sub(row.start_byte) != row.matched_text.len() as u64
    {
        return Err(SourceMatchDeltaError::InvalidExpected);
    }
    Ok(())
}

fn validate_limits(limits: &SourceMatchDeltaLimits) -> Result<(), SourceMatchDeltaError> {
    if limits.max_expected_rows == 0
        || limits.max_observed_rows == 0
        || limits.max_changes == 0
        || limits.max_string_bytes == 0
    {
        return Err(SourceMatchDeltaError::InvalidLimit);
    }
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), SourceMatchDeltaError> {
    let value = digest.as_str();
    if value.len() != "blake3:".len() + 64
        || !value.starts_with("blake3:")
        || !value["blake3:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(SourceMatchDeltaError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn expected_string_bytes(row: &ExpectedSourceMatch) -> u64 {
    row.path.encoded.len() as u64
        + row.path.display.len() as u64
        + row.matched_text.len() as u64
        + row.context_text.len() as u64
}

fn observed_string_bytes(row: &ObservedSourceMatch) -> u64 {
    row.key.path_identity.len() as u64
        + row.path_display.len() as u64
        + row.matched_text.len() as u64
        + row.context_text.len() as u64
        + row.context_ref.len() as u64
}

fn expected_relation_id(
    scenario: &ContractIdentity,
    expected: &[ExpectedSourceMatch],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.expected-source-matches.v1");
    scenario.add_semantics(&mut hasher);
    hasher.add_u64(expected.len() as u64);
    for row in expected {
        add_expected(&mut hasher, row);
    }
    hasher.finish()
}

fn add_key(hasher: &mut SemanticHasher, key: &SourceMatchKey) {
    hasher.add_str(key.path_encoding.as_str());
    hasher.add_str(&key.path_identity);
    hasher.add_u64(key.start_byte);
    hasher.add_u64(key.end_byte);
}

fn add_expected(hasher: &mut SemanticHasher, row: &ExpectedSourceMatch) {
    add_key(hasher, &row.key());
    hasher.add_str(&row.path.display);
    hasher.add_u64(row.start_line);
    hasher.add_u64(row.start_byte_in_line);
    hasher.add_u64(row.end_line);
    hasher.add_u64(row.end_byte_in_line);
    hasher.add_str(&row.matched_text);
    hasher.add_str(&row.context_text);
}

fn add_observed(hasher: &mut SemanticHasher, row: &ObservedSourceMatch) {
    add_key(hasher, &row.key);
    hasher.add_str(&row.path_display);
    hasher.add_str(row.source_artifact_id.as_str());
    hasher.add_str(row.match_id.as_str());
    hasher.add_u64(row.start_line);
    hasher.add_u64(row.start_byte_in_line);
    hasher.add_u64(row.end_line);
    hasher.add_u64(row.end_byte_in_line);
    hasher.add_str(&row.matched_text);
    hasher.add_str(row.context_artifact_id.as_str());
    hasher.add_str(&row.context_text);
    hasher.add_str(&row.context_ref);
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.source-match-delta.placeholder").finish()
}

fn delta_digest(delta: &SourceMatchDelta) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SOURCE_MATCH_DELTA_SCHEMA);
    for contract in [
        &delta.inputs.workload,
        &delta.inputs.graph,
        &delta.inputs.scenario,
        &delta.inputs.comparator,
    ] {
        contract.add_semantics(&mut hasher);
    }
    hasher.add_str(delta.inputs.binding_id.as_str());
    hasher.add_str(delta.inputs.mining_request_id.as_str());
    hasher.add_str(delta.inputs.mining_result_id.as_str());
    hasher.add_str(delta.expected_relation_id.as_str());
    hasher.add_bool(delta.observed_relation_id.is_some());
    if let Some(id) = &delta.observed_relation_id {
        hasher.add_str(id.as_str());
    }
    hasher.add_str(delta.completeness.as_str());
    hasher.add_str(delta.assessment.as_str());
    hasher.add_u64(delta.summary.expected_rows);
    hasher.add_u64(delta.summary.observed_rows);
    hasher.add_u64(delta.summary.equal_rows);
    hasher.add_u64(delta.summary.inserted);
    hasher.add_u64(delta.summary.deleted);
    hasher.add_u64(delta.summary.modified);
    hasher.add_u64(delta.expected.len() as u64);
    for row in &delta.expected {
        add_expected(&mut hasher, row);
    }
    hasher.add_u64(delta.observed.len() as u64);
    for row in &delta.observed {
        add_observed(&mut hasher, row);
    }
    hasher.add_u64(delta.changes.len() as u64);
    for change in &delta.changes {
        hasher.add_str(change.kind.as_str());
        add_key(&mut hasher, &change.key);
        hasher.add_bool(change.expected.is_some());
        if let Some(row) = &change.expected {
            add_expected(&mut hasher, row);
        }
        hasher.add_bool(change.observed.is_some());
        if let Some(row) = &change.observed {
            add_observed(&mut hasher, row);
        }
        hasher.add_u64(change.changed_fields.len() as u64);
        for field in &change.changed_fields {
            hasher.add_str(field);
        }
    }
    hasher.add_u64(delta.limits.max_expected_rows);
    hasher.add_u64(delta.limits.max_observed_rows);
    hasher.add_u64(delta.limits.max_changes);
    hasher.add_u64(delta.limits.max_string_bytes);
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum SourceMatchDeltaError {
    #[error("source match delta limits must be non-zero")]
    InvalidLimit,
    #[error("invalid source match delta contract")]
    InvalidContract,
    #[error("invalid expected source match row")]
    InvalidExpected,
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("source match delta is not bound to its mining evidence")]
    EvidenceBinding,
    #[error("{role} match relation exceeds the {limit}-row limit with {observed} rows")]
    RowLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("{0} match relation has duplicate keys")]
    DuplicateKey(&'static str),
    #[error("source match evidence is missing a referenced native context")]
    MissingContext,
    #[error("source match delta exceeds the {limit}-change limit with {observed} changes")]
    ChangeLimit { limit: u64, observed: u64 },
    #[error("source match delta exceeds the {limit}-string-byte limit with {observed} bytes")]
    StringByteLimit { limit: u64, observed: u64 },
    #[error("unsupported source match delta schema {0}")]
    UnsupportedSchema(String),
    #[error("source match delta replay did not reproduce the retained artifact")]
    ReplayMismatch,
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
    use rey_environment::{
        LocalSourceCorpus, SourceBindingLimits, SourceSearchEvidence,
        builtin_source_search_operation, local_source_provider,
    };
    use rey_mining::{
        MiningCompleteness, MiningLimits, MiningParameterValue, MiningRationaleKind, MiningRequest,
        MiningRequestContext,
    };

    use super::{
        ExpectedSourceMatch, SourceMatchChangeKind, SourceMatchDeltaError, SourceMatchDeltaInputs,
        SourceMatchDeltaLimits, compare_source_matches, source_match_comparator,
    };
    use crate::DeltaAssessment;

    fn digest(label: &str) -> SemanticDigest {
        let mut hasher = SemanticHasher::new("rey.source-match-delta.test");
        hasher.add_str(label);
        hasher.finish()
    }

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, &format!("{id} fixture contract"))
    }

    fn evidence(pattern: &str, limits: MiningLimits) -> SourceSearchEvidence {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../rey-environment/tests/fixtures/source-corpus");
        let corpus = LocalSourceCorpus::bind(
            root,
            [PathBuf::from("alpha.txt"), PathBuf::from("nested/beta.rs")],
            SourceBindingLimits::default(),
        )
        .unwrap();
        let operation = builtin_source_search_operation();
        let request = MiningRequest::new(
            MiningRequestContext {
                workload: contract("workload.source-search"),
                graph: contract("graph.source-search"),
                scenario: Some(contract("scenario.source-search")),
                campaign_id: Some(digest("campaign")),
                space: contract("space.local-source"),
                active_transition_id: None,
                graph_node_id: "search".to_owned(),
                rationale: MiningRationaleKind::WorkloadGraph,
                frontier_row_ids: Vec::new(),
                delta_ids: Vec::new(),
            },
            &operation,
            local_source_provider(),
            digest("capability-snapshot"),
            vec![corpus.binding().artifact_ref()],
            BTreeMap::from([
                (
                    "pattern".to_owned(),
                    MiningParameterValue::Utf8(pattern.to_owned()),
                ),
                ("context_before".to_owned(), MiningParameterValue::U64(0)),
                ("context_after".to_owned(), MiningParameterValue::U64(0)),
            ]),
            MiningLimits::default(),
            limits,
        )
        .unwrap();
        corpus.search(&request).unwrap()
    }

    fn inputs(evidence: &SourceSearchEvidence) -> SourceMatchDeltaInputs {
        SourceMatchDeltaInputs {
            workload: contract("workload.source-search"),
            graph: contract("graph.source-search"),
            scenario: contract("scenario.source-search"),
            comparator: source_match_comparator(),
            binding_id: evidence.binding_id.clone(),
            mining_request_id: evidence.result.request_id.clone(),
            mining_result_id: evidence.result.result_id.clone(),
        }
    }

    fn expected(evidence: &SourceSearchEvidence) -> Vec<ExpectedSourceMatch> {
        evidence
            .matches
            .iter()
            .map(|row| ExpectedSourceMatch {
                path: row.path.clone(),
                start_byte: row.start_byte,
                end_byte: row.end_byte,
                start_line: row.start_line,
                start_byte_in_line: row.start_byte_in_line,
                end_line: row.end_line,
                end_byte_in_line: row.end_byte_in_line,
                matched_text: row.matched_text.clone(),
                context_text: evidence
                    .contexts
                    .iter()
                    .find(|context| context.artifact_id == row.context_artifact_id)
                    .unwrap()
                    .text
                    .clone(),
            })
            .collect()
    }

    #[test]
    fn typed_relation_preserves_insert_delete_modify_and_replays() {
        let evidence = evidence("delta", MiningLimits::default());
        let mut expected = expected(&evidence);
        expected[0].matched_text = "DELTA".to_owned();
        expected.remove(1);
        let mut deleted = expected[1].clone();
        deleted.start_byte = 10_000;
        deleted.end_byte = 10_005;
        deleted.start_line = 999;
        deleted.end_line = 999;
        expected.push(deleted);

        let delta = compare_source_matches(
            inputs(&evidence),
            expected,
            &evidence,
            SourceMatchDeltaLimits::default(),
        )
        .unwrap();

        assert_eq!(delta.assessment, DeltaAssessment::Different);
        assert_eq!(delta.summary.equal_rows, 2);
        assert_eq!(delta.summary.inserted, 1);
        assert_eq!(delta.summary.deleted, 1);
        assert_eq!(delta.summary.modified, 1);
        assert!(delta.changes.iter().any(|change| {
            change.kind == SourceMatchChangeKind::Modified
                && change.changed_fields == ["matched_text"]
        }));
        delta.verify(&evidence).unwrap();
    }

    #[test]
    fn incomplete_mining_keeps_the_relation_inconclusive() {
        let complete = evidence("delta", MiningLimits::default());
        let expected = expected(&complete);
        let truncated = evidence(
            "delta",
            MiningLimits {
                max_matches: 1,
                max_rows: 1,
                ..MiningLimits::default()
            },
        );
        let delta = compare_source_matches(
            inputs(&truncated),
            expected,
            &truncated,
            SourceMatchDeltaLimits::default(),
        )
        .unwrap();

        assert_eq!(delta.completeness, MiningCompleteness::Truncated);
        assert_eq!(delta.assessment, DeltaAssessment::Inconclusive);
        assert_eq!(delta.summary.equal_rows, 1);
        assert_eq!(delta.summary.deleted, 3);
        delta.verify(&truncated).unwrap();
    }

    #[test]
    fn malformed_reviewed_rows_fail_before_alignment() {
        let evidence = evidence("delta", MiningLimits::default());
        let mut expected = expected(&evidence);
        expected[0].end_byte = expected[0].start_byte;

        assert!(matches!(
            compare_source_matches(
                inputs(&evidence),
                expected,
                &evidence,
                SourceMatchDeltaLimits::default(),
            ),
            Err(SourceMatchDeltaError::InvalidExpected)
        ));
    }
}
