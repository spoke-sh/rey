#![forbid(unsafe_code)]

mod scenario;
mod source_matches;
mod text;

pub use scenario::{
    SCENARIO_OUTPUT_DELTA_SCHEMA, ScenarioDeltaError, ScenarioDeltaInputs, ScenarioDeltaLimits,
    ScenarioOutputDelta, ScenarioValueType, compare_scenario_utf8,
};
pub use source_matches::{
    ExpectedSourceMatch, ObservedSourceMatch, SOURCE_MATCH_DELTA_SCHEMA, SourceMatchChange,
    SourceMatchChangeKind, SourceMatchDelta, SourceMatchDeltaError, SourceMatchDeltaInputs,
    SourceMatchDeltaLimits, SourceMatchDeltaSummary, SourceMatchKey, compare_source_matches,
    source_match_comparator, source_match_table_projection,
};
pub use text::{
    TEXT_DELTA_SCHEMA, TextDelta, TextDeltaError, TextDeltaInputs, TextDeltaLimits, TextHunk,
    TextLine, TextLineKind, compare_text, text_artifact_id, text_patch_projection,
};

use std::collections::{BTreeMap, BTreeSet};

use csv::{Terminator, WriterBuilder};
use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameError, FrameMetadata};
use rey_environment::{Availability, CapabilityRecord, CapabilitySnapshot, TrustClass};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CAPABILITY_DELTA_SCHEMA: &str = "rey.capability-delta.v1";
pub const CAPABILITY_CHANGE_RELATION: &str = "rey.capability-changes";
pub const CAPABILITY_CHANGE_SCHEMA_VERSION: &str = "1";
pub const TABULAR_DIFF_MEDIA_TYPE: &str = "text/csv; charset=utf-8; profile=tabular-diff-0.8";

const COMPARATOR_ID: &str = "rey.capability-exact";
const COMPARATOR_REVISION: u64 = 1;
const COMPARATOR_DEFINITION: &str = "capability-v1 exact typed equality excluding observed_at and error_detail; composite key provider_id,provider_revision,capability_id";

const TABULAR_COLUMNS: &[&str] = &[
    "provider_id",
    "provider_revision",
    "provider_kind",
    "capability_id",
    "capability_kind",
    "resolved_location",
    "version",
    "content_digest",
    "provenance",
    "availability",
    "trust_class",
    "operations",
    "enforced_limits",
    "unsupported_limits",
    "error_code",
];

const NON_KEY_FIELDS: &[&str] = &[
    "provider_kind",
    "capability_kind",
    "resolved_location",
    "version",
    "content_digest",
    "provenance",
    "availability",
    "trust_class",
    "operations",
    "enforced_limits",
    "unsupported_limits",
    "error_code",
];

#[must_use]
pub fn capability_comparator() -> ContractIdentity {
    ContractIdentity::new(COMPARATOR_ID, COMPARATOR_REVISION, COMPARATOR_DEFINITION)
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CapabilityKey {
    pub provider_id: String,
    pub provider_revision: u64,
    pub capability_id: String,
}

impl From<&CapabilityRecord> for CapabilityKey {
    fn from(record: &CapabilityRecord) -> Self {
        Self {
            provider_id: record.provider_id.clone(),
            provider_revision: record.provider_revision,
            capability_id: record.capability_id.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilitySemanticRecord {
    pub provider_id: String,
    pub provider_revision: u64,
    pub provider_kind: String,
    pub capability_id: String,
    pub capability_kind: String,
    pub resolved_location: Option<String>,
    pub version: Option<String>,
    pub content_digest: Option<String>,
    pub provenance: Option<String>,
    pub availability: Availability,
    pub trust_class: TrustClass,
    pub operations: Vec<String>,
    pub enforced_limits: Vec<String>,
    pub unsupported_limits: Vec<String>,
    pub error_code: Option<String>,
}

impl From<&CapabilityRecord> for CapabilitySemanticRecord {
    fn from(record: &CapabilityRecord) -> Self {
        Self {
            provider_id: record.provider_id.clone(),
            provider_revision: record.provider_revision,
            provider_kind: record.provider_kind.clone(),
            capability_id: record.capability_id.clone(),
            capability_kind: record.capability_kind.clone(),
            resolved_location: record.resolved_location.clone(),
            version: record.version.clone(),
            content_digest: record.content_digest.clone(),
            provenance: record.provenance.clone(),
            availability: record.availability,
            trust_class: record.trust_class,
            operations: record.operations.clone(),
            enforced_limits: record.enforced_limits.clone(),
            unsupported_limits: record.unsupported_limits.clone(),
            error_code: record.error_code.clone(),
        }
    }
}

impl CapabilitySemanticRecord {
    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(&self.provider_id);
        hasher.add_u64(self.provider_revision);
        hasher.add_str(&self.provider_kind);
        hasher.add_str(&self.capability_id);
        hasher.add_str(&self.capability_kind);
        hasher.add_optional_str(self.resolved_location.as_deref());
        hasher.add_optional_str(self.version.as_deref());
        hasher.add_optional_str(self.content_digest.as_deref());
        hasher.add_optional_str(self.provenance.as_deref());
        hasher.add_str(self.availability.as_str());
        hasher.add_str(self.trust_class.as_str());
        add_strings(hasher, &self.operations);
        add_strings(hasher, &self.enforced_limits);
        add_strings(hasher, &self.unsupported_limits);
        hasher.add_optional_str(self.error_code.as_deref());
    }

    fn tabular_cells(&self) -> Result<Vec<String>, DeltaError> {
        Ok(vec![
            encode_string(&self.provider_id),
            self.provider_revision.to_string(),
            encode_string(&self.provider_kind),
            encode_string(&self.capability_id),
            encode_string(&self.capability_kind),
            encode_optional(self.resolved_location.as_deref()),
            encode_optional(self.version.as_deref()),
            encode_optional(self.content_digest.as_deref()),
            encode_optional(self.provenance.as_deref()),
            self.availability.as_str().to_owned(),
            self.trust_class.as_str().to_owned(),
            encode_string(&serde_json::to_string(&self.operations)?),
            encode_string(&serde_json::to_string(&self.enforced_limits)?),
            encode_string(&serde_json::to_string(&self.unsupported_limits)?),
            encode_optional(self.error_code.as_deref()),
        ])
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityChangeKind {
    Inserted,
    Deleted,
    Modified,
}

impl CapabilityChangeKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Inserted => "inserted",
            Self::Deleted => "deleted",
            Self::Modified => "modified",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityChange {
    pub kind: CapabilityChangeKind,
    pub key: CapabilityKey,
    pub before: Option<CapabilitySemanticRecord>,
    pub after: Option<CapabilitySemanticRecord>,
    pub changed_fields: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaAssessment {
    Equal,
    Different,
    Inconclusive,
}

impl DeltaAssessment {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "equal",
            Self::Different => "different",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeltaLimits {
    pub max_changes: u64,
}

impl Default for DeltaLimits {
    fn default() -> Self {
        Self { max_changes: 4_096 }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityDeltaSummary {
    pub assessment: DeltaAssessment,
    pub inserted: u64,
    pub deleted: u64,
    pub modified: u64,
    pub unchanged: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub source_snapshot: SemanticDigest,
    pub target_snapshot: SemanticDigest,
    pub source_label: String,
    pub target_label: String,
    pub comparator: ContractIdentity,
    pub limits: DeltaLimits,
    pub summary: CapabilityDeltaSummary,
    pub changes: Vec<CapabilityChange>,
}

#[derive(Clone, Debug)]
pub struct DeltaOptions {
    pub source_label: String,
    pub target_label: String,
    pub limits: DeltaLimits,
    pub comparator: ContractIdentity,
}

impl Default for DeltaOptions {
    fn default() -> Self {
        Self {
            source_label: "SOURCE".to_owned(),
            target_label: "TARGET".to_owned(),
            limits: DeltaLimits::default(),
            comparator: capability_comparator(),
        }
    }
}

pub fn compare_capabilities(
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    options: DeltaOptions,
) -> Result<CapabilityDelta, DeltaError> {
    source.verify()?;
    target.verify()?;
    validate_label("source", &options.source_label)?;
    validate_label("target", &options.target_label)?;
    if options.limits.max_changes == 0 {
        return Err(DeltaError::ZeroChangeLimit);
    }

    let source_rows = source
        .capabilities
        .iter()
        .map(|row| (CapabilityKey::from(row), row))
        .collect::<BTreeMap<_, _>>();
    let target_rows = target
        .capabilities
        .iter()
        .map(|row| (CapabilityKey::from(row), row))
        .collect::<BTreeMap<_, _>>();
    let keys = source_rows
        .keys()
        .chain(target_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut changes = Vec::new();
    let mut unchanged = 0_u64;
    let mut inserted = 0_u64;
    let mut deleted = 0_u64;
    let mut modified = 0_u64;
    for key in keys {
        let before = source_rows
            .get(&key)
            .map(|row| CapabilitySemanticRecord::from(*row));
        let after = target_rows
            .get(&key)
            .map(|row| CapabilitySemanticRecord::from(*row));
        let change = match (&before, &after) {
            (None, Some(_)) => {
                inserted += 1;
                Some(CapabilityChange {
                    kind: CapabilityChangeKind::Inserted,
                    key,
                    before,
                    after,
                    changed_fields: Vec::new(),
                })
            }
            (Some(_), None) => {
                deleted += 1;
                Some(CapabilityChange {
                    kind: CapabilityChangeKind::Deleted,
                    key,
                    before,
                    after,
                    changed_fields: Vec::new(),
                })
            }
            (Some(left), Some(right)) => {
                let changed_fields = changed_fields(left, right);
                if changed_fields.is_empty() {
                    unchanged += 1;
                    None
                } else {
                    modified += 1;
                    Some(CapabilityChange {
                        kind: CapabilityChangeKind::Modified,
                        key,
                        before,
                        after,
                        changed_fields,
                    })
                }
            }
            (None, None) => unreachable!("a union key exists in at least one snapshot"),
        };
        if let Some(change) = change {
            changes.push(change);
            if changes.len() as u64 > options.limits.max_changes {
                return Err(DeltaError::ChangeLimit {
                    limit: options.limits.max_changes,
                    observed: changes.len() as u64,
                });
            }
        }
    }

    let assessment = if !source.complete || !target.complete {
        DeltaAssessment::Inconclusive
    } else if changes.is_empty() {
        DeltaAssessment::Equal
    } else {
        DeltaAssessment::Different
    };
    let summary = CapabilityDeltaSummary {
        assessment,
        inserted,
        deleted,
        modified,
        unchanged,
    };
    let delta_id = delta_digest(
        source,
        target,
        &options.source_label,
        &options.target_label,
        &options.comparator,
        &options.limits,
        &summary,
        &changes,
    );
    Ok(CapabilityDelta {
        schema: CAPABILITY_DELTA_SCHEMA.to_owned(),
        delta_id,
        source_snapshot: source.semantic_digest.clone(),
        target_snapshot: target.semantic_digest.clone(),
        source_label: options.source_label,
        target_label: options.target_label,
        comparator: options.comparator,
        limits: options.limits,
        summary,
        changes,
    })
}

impl CapabilityDelta {
    pub fn to_frame(&self) -> Result<Frame, DeltaError> {
        let rows = &self.changes;
        let changed_fields = rows
            .iter()
            .map(|row| serde_json::to_string(&row.changed_fields))
            .collect::<Result<Vec<_>, _>>()?;
        let before_operations = json_array_column(rows, Side::Before, |row| &row.operations)?;
        let after_operations = json_array_column(rows, Side::After, |row| &row.operations)?;
        let before_enforced = json_array_column(rows, Side::Before, |row| &row.enforced_limits)?;
        let after_enforced = json_array_column(rows, Side::After, |row| &row.enforced_limits)?;
        let before_unsupported =
            json_array_column(rows, Side::Before, |row| &row.unsupported_limits)?;
        let after_unsupported =
            json_array_column(rows, Side::After, |row| &row.unsupported_limits)?;
        let dataframe = df!(
            "change_kind" => rows.iter().map(|row| row.kind.as_str()).collect::<Vec<_>>(),
            "provider_id" => rows.iter().map(|row| row.key.provider_id.as_str()).collect::<Vec<_>>(),
            "provider_revision" => rows.iter().map(|row| row.key.provider_revision).collect::<Vec<_>>(),
            "capability_id" => rows.iter().map(|row| row.key.capability_id.as_str()).collect::<Vec<_>>(),
            "changed_fields" => changed_fields,
            "before_provider_kind" => optional_string_column(rows, Side::Before, |row| Some(row.provider_kind.as_str())),
            "after_provider_kind" => optional_string_column(rows, Side::After, |row| Some(row.provider_kind.as_str())),
            "before_capability_kind" => optional_string_column(rows, Side::Before, |row| Some(row.capability_kind.as_str())),
            "after_capability_kind" => optional_string_column(rows, Side::After, |row| Some(row.capability_kind.as_str())),
            "before_resolved_location" => optional_string_column(rows, Side::Before, |row| row.resolved_location.as_deref()),
            "after_resolved_location" => optional_string_column(rows, Side::After, |row| row.resolved_location.as_deref()),
            "before_version" => optional_string_column(rows, Side::Before, |row| row.version.as_deref()),
            "after_version" => optional_string_column(rows, Side::After, |row| row.version.as_deref()),
            "before_content_digest" => optional_string_column(rows, Side::Before, |row| row.content_digest.as_deref()),
            "after_content_digest" => optional_string_column(rows, Side::After, |row| row.content_digest.as_deref()),
            "before_provenance" => optional_string_column(rows, Side::Before, |row| row.provenance.as_deref()),
            "after_provenance" => optional_string_column(rows, Side::After, |row| row.provenance.as_deref()),
            "before_availability" => optional_string_column(rows, Side::Before, |row| Some(row.availability.as_str())),
            "after_availability" => optional_string_column(rows, Side::After, |row| Some(row.availability.as_str())),
            "before_trust_class" => optional_string_column(rows, Side::Before, |row| Some(row.trust_class.as_str())),
            "after_trust_class" => optional_string_column(rows, Side::After, |row| Some(row.trust_class.as_str())),
            "before_operations" => before_operations,
            "after_operations" => after_operations,
            "before_enforced_limits" => before_enforced,
            "after_enforced_limits" => after_enforced,
            "before_unsupported_limits" => before_unsupported,
            "after_unsupported_limits" => after_unsupported,
            "before_error_code" => optional_string_column(rows, Side::Before, |row| row.error_code.as_deref()),
            "after_error_code" => optional_string_column(rows, Side::After, |row| row.error_code.as_deref()),
        )?;
        let attributes = BTreeMap::from([
            ("rey.delta-schema".to_owned(), self.schema.clone()),
            ("rey.delta-id".to_owned(), self.delta_id.to_string()),
            (
                "rey.source-snapshot".to_owned(),
                self.source_snapshot.to_string(),
            ),
            (
                "rey.target-snapshot".to_owned(),
                self.target_snapshot.to_string(),
            ),
            ("rey.source-label".to_owned(), self.source_label.clone()),
            ("rey.target-label".to_owned(), self.target_label.clone()),
            ("rey.comparator-id".to_owned(), self.comparator.id.clone()),
            (
                "rey.comparator-revision".to_owned(),
                self.comparator.revision.to_string(),
            ),
            (
                "rey.comparator-digest".to_owned(),
                self.comparator.semantic_digest.to_string(),
            ),
            (
                "rey.max-changes".to_owned(),
                self.limits.max_changes.to_string(),
            ),
            (
                "rey.assessment".to_owned(),
                self.summary.assessment.as_str().to_owned(),
            ),
            ("rey.inserted".to_owned(), self.summary.inserted.to_string()),
            ("rey.deleted".to_owned(), self.summary.deleted.to_string()),
            ("rey.modified".to_owned(), self.summary.modified.to_string()),
            (
                "rey.unchanged".to_owned(),
                self.summary.unchanged.to_string(),
            ),
        ]);
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: CAPABILITY_CHANGE_RELATION.to_owned(),
                schema_version: CAPABILITY_CHANGE_SCHEMA_VERSION.to_owned(),
                semantic_digest: self.delta_id.to_string(),
                row_count: rows.len() as u64,
                complete: self.summary.assessment != DeltaAssessment::Inconclusive,
                key_columns: vec![
                    "provider_id".to_owned(),
                    "provider_revision".to_owned(),
                    "capability_id".to_owned(),
                ],
                attributes,
            },
        )?)
    }

    pub fn to_tabular_diff(&self) -> Result<Vec<u8>, DeltaError> {
        let mut output = Vec::new();
        {
            let mut writer = WriterBuilder::new()
                .terminator(Terminator::Any(b'\n'))
                .from_writer(&mut output);
            let mut header = vec!["@@"];
            header.extend_from_slice(TABULAR_COLUMNS);
            writer.write_record(header)?;
            if self.summary.unchanged > 0 {
                let mut omitted = vec!["..."; TABULAR_COLUMNS.len() + 1];
                omitted[0] = "...";
                writer.write_record(omitted)?;
            }
            for change in &self.changes {
                let row = match (&change.before, &change.after) {
                    (None, Some(after)) => {
                        let mut row = vec!["+++".to_owned()];
                        row.extend(after.tabular_cells()?);
                        row
                    }
                    (Some(before), None) => {
                        let mut row = vec!["---".to_owned()];
                        row.extend(before.tabular_cells()?);
                        row
                    }
                    (Some(before), Some(after)) => {
                        let before = before.tabular_cells()?;
                        let after = after.tabular_cells()?;
                        let separator = collision_safe_separator(&before, &after);
                        let changed = change
                            .changed_fields
                            .iter()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>();
                        let mut row = vec![separator.clone()];
                        row.extend(TABULAR_COLUMNS.iter().enumerate().map(|(index, field)| {
                            if changed.contains(field) {
                                format!("{}{separator}{}", before[index], after[index])
                            } else {
                                after[index].clone()
                            }
                        }));
                        row
                    }
                    (None, None) => unreachable!("a change has at least one side"),
                };
                writer.write_record(row)?;
            }
            writer.flush()?;
        }
        Ok(output)
    }
}

#[derive(Clone, Copy)]
enum Side {
    Before,
    After,
}

fn side(change: &CapabilityChange, side: Side) -> Option<&CapabilitySemanticRecord> {
    match side {
        Side::Before => change.before.as_ref(),
        Side::After => change.after.as_ref(),
    }
}

fn optional_string_column<'a>(
    rows: &'a [CapabilityChange],
    selected: Side,
    field: impl Fn(&'a CapabilitySemanticRecord) -> Option<&'a str>,
) -> Vec<Option<&'a str>> {
    rows.iter()
        .map(|change| side(change, selected).and_then(&field))
        .collect()
}

fn json_array_column(
    rows: &[CapabilityChange],
    selected: Side,
    field: impl Fn(&CapabilitySemanticRecord) -> &Vec<String>,
) -> Result<Vec<Option<String>>, serde_json::Error> {
    rows.iter()
        .map(|change| {
            side(change, selected)
                .map(&field)
                .map(serde_json::to_string)
                .transpose()
        })
        .collect()
}

fn changed_fields(
    left: &CapabilitySemanticRecord,
    right: &CapabilitySemanticRecord,
) -> Vec<String> {
    let mut fields = Vec::new();
    macro_rules! changed {
        ($name:literal, $field:ident) => {
            if left.$field != right.$field {
                fields.push($name.to_owned());
            }
        };
    }
    changed!("provider_kind", provider_kind);
    changed!("capability_kind", capability_kind);
    changed!("resolved_location", resolved_location);
    changed!("version", version);
    changed!("content_digest", content_digest);
    changed!("provenance", provenance);
    changed!("availability", availability);
    changed!("trust_class", trust_class);
    changed!("operations", operations);
    changed!("enforced_limits", enforced_limits);
    changed!("unsupported_limits", unsupported_limits);
    changed!("error_code", error_code);
    debug_assert!(
        fields
            .iter()
            .all(|field| NON_KEY_FIELDS.contains(&field.as_str()))
    );
    fields
}

#[allow(clippy::too_many_arguments)]
fn delta_digest(
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    source_label: &str,
    target_label: &str,
    comparator: &ContractIdentity,
    limits: &DeltaLimits,
    summary: &CapabilityDeltaSummary,
    changes: &[CapabilityChange],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(CAPABILITY_DELTA_SCHEMA);
    hasher.add_str(source.semantic_digest.as_str());
    hasher.add_str(target.semantic_digest.as_str());
    hasher.add_str(source_label);
    hasher.add_str(target_label);
    comparator.add_semantics(&mut hasher);
    hasher.add_u64(limits.max_changes);
    hasher.add_str(summary.assessment.as_str());
    hasher.add_u64(summary.inserted);
    hasher.add_u64(summary.deleted);
    hasher.add_u64(summary.modified);
    hasher.add_u64(summary.unchanged);
    hasher.add_u64(changes.len() as u64);
    for change in changes {
        hasher.add_str(change.kind.as_str());
        hasher.add_str(&change.key.provider_id);
        hasher.add_u64(change.key.provider_revision);
        hasher.add_str(&change.key.capability_id);
        add_optional_record(&mut hasher, change.before.as_ref());
        add_optional_record(&mut hasher, change.after.as_ref());
        add_strings(&mut hasher, &change.changed_fields);
    }
    hasher.finish()
}

fn add_optional_record(hasher: &mut SemanticHasher, record: Option<&CapabilitySemanticRecord>) {
    hasher.add_bool(record.is_some());
    if let Some(record) = record {
        record.add_semantics(hasher);
    }
}

fn add_strings(hasher: &mut SemanticHasher, values: &[String]) {
    hasher.add_u64(values.len() as u64);
    for value in values {
        hasher.add_str(value);
    }
}

fn validate_label(kind: &'static str, label: &str) -> Result<(), DeltaError> {
    if label.is_empty() || label.chars().count() > 64 || label.chars().any(char::is_control) {
        return Err(DeltaError::InvalidLabel(kind));
    }
    Ok(())
}

fn encode_optional(value: Option<&str>) -> String {
    value.map_or_else(|| "NULL".to_owned(), encode_string)
}

fn encode_string(value: &str) -> String {
    if value.trim_start_matches('_') == "NULL" {
        format!("_{value}")
    } else {
        value.to_owned()
    }
}

fn collision_safe_separator(before: &[String], after: &[String]) -> String {
    let mut separator = "->".to_owned();
    while before
        .iter()
        .chain(after)
        .any(|cell| cell.contains(&separator))
    {
        separator.insert(0, '-');
    }
    separator
}

#[derive(Debug, Error)]
pub enum DeltaError {
    #[error("{0} label must contain 1-64 non-control characters")]
    InvalidLabel(&'static str),
    #[error("maximum changes must be greater than zero")]
    ZeroChangeLimit,
    #[error("capability delta contains {observed} changes, exceeding limit {limit}")]
    ChangeLimit { limit: u64, observed: u64 },
    #[error(transparent)]
    Snapshot(#[from] rey_environment::DiscoveryError),
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("capability dataframe failed: {0}")]
    Polars(#[from] polars::error::PolarsError),
    #[error("delta JSON encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Tabular Diff encoding failed: {0}")]
    Csv(#[from] csv::Error),
    #[error("Tabular Diff output failed: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use rey_dataframe::Frame;
    use rey_environment::{DiscoveryLimits, LOCAL_PROVIDER_REVISION};

    use super::*;

    fn record(id: &str, version: Option<&str>) -> CapabilityRecord {
        CapabilityRecord {
            provider_id: "fixture".to_owned(),
            provider_revision: LOCAL_PROVIDER_REVISION,
            provider_kind: "tool".to_owned(),
            capability_id: id.to_owned(),
            capability_kind: "identity".to_owned(),
            resolved_location: None,
            version: version.map(str::to_owned),
            content_digest: None,
            provenance: Some("fixture".to_owned()),
            availability: Availability::Available,
            trust_class: TrustClass::BuiltIn,
            operations: vec!["inspect".to_owned()],
            enforced_limits: Vec::new(),
            unsupported_limits: Vec::new(),
            observed_at: None,
            error_code: None,
            error_detail: None,
        }
    }

    fn snapshot(rows: Vec<CapabilityRecord>) -> CapabilitySnapshot {
        let limits = DiscoveryLimits {
            max_capabilities: 16,
            ..DiscoveryLimits::default()
        };
        CapabilitySnapshot::new("fixture", limits, rows).unwrap()
    }

    #[test]
    fn capability_delta_is_typed_ordered_and_ignores_observation_detail() {
        let mut unchanged = record("same", Some("1"));
        unchanged.observed_at = Some("source-time".to_owned());
        unchanged.error_detail = Some("source-detail".to_owned());
        let source = snapshot(vec![
            record("deleted", Some("1")),
            record("modified", Some("a->b")),
            unchanged.clone(),
        ]);
        unchanged.observed_at = Some("target-time".to_owned());
        unchanged.error_detail = Some("target-detail".to_owned());
        let target = snapshot(vec![
            record("inserted", Some("NULL")),
            record("modified", Some("2")),
            unchanged,
        ]);

        let delta = compare_capabilities(&source, &target, DeltaOptions::default()).unwrap();
        assert_eq!(delta.summary.inserted, 1);
        assert_eq!(delta.summary.deleted, 1);
        assert_eq!(delta.summary.modified, 1);
        assert_eq!(delta.summary.unchanged, 1);
        let modified = delta
            .changes
            .iter()
            .find(|change| change.kind == CapabilityChangeKind::Modified)
            .unwrap();
        assert_eq!(modified.changed_fields, ["version"]);
        assert_eq!(
            modified.before.as_ref().unwrap().version.as_deref(),
            Some("a->b")
        );

        let frame = delta.to_frame().unwrap();
        assert_eq!(frame.dataframe().height(), 3);
        let decoded = Frame::from_arrow_stream(&frame.to_arrow_stream().unwrap()).unwrap();
        assert_eq!(decoded.metadata(), frame.metadata());
        assert_eq!(
            decoded.metadata().attributes.get("rey.source-snapshot"),
            Some(&source.semantic_digest.to_string())
        );

        let csv = String::from_utf8(delta.to_tabular_diff().unwrap()).unwrap();
        assert!(csv.lines().next().unwrap().starts_with("@@,provider_id"));
        assert!(csv.lines().any(|line| line.starts_with("-->")), "{csv}");
        assert!(csv.contains("a->b-->2"), "{csv}");
        assert!(csv.contains("_NULL"), "{csv}");
        assert!(csv.lines().any(|line| line.starts_with("...")));
    }

    #[test]
    fn empty_delta_arrow_retains_lineage() {
        let snapshot = snapshot(vec![record("same", Some("1"))]);
        let delta = compare_capabilities(&snapshot, &snapshot, DeltaOptions::default()).unwrap();
        let frame = delta.to_frame().unwrap();

        assert_eq!(frame.dataframe().height(), 0);
        assert_eq!(delta.summary.assessment, DeltaAssessment::Equal);
        assert_eq!(
            frame.metadata().attributes.get("rey.delta-id"),
            Some(&delta.delta_id.to_string())
        );
    }

    #[test]
    fn change_limit_fails_closed() {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record("a", None), record("b", None)]);
        let error = compare_capabilities(
            &source,
            &target,
            DeltaOptions {
                limits: DeltaLimits { max_changes: 1 },
                ..DeltaOptions::default()
            },
        )
        .unwrap_err();

        assert!(matches!(error, DeltaError::ChangeLimit { .. }));
    }
}
