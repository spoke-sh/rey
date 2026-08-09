use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DeltaAssessment;

pub const TEXT_DELTA_SCHEMA: &str = "rey.text-delta.v1";

#[must_use]
pub fn text_patch_projection() -> ContractIdentity {
    ContractIdentity::new(
        "rey.text-delta.terminal-patch",
        1,
        "render one directed ordered UTF-8 text delta as ANSI-independent line hunks with source and target coordinates; preserve the authoritative structured delta identity and omit no retained lines",
    )
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextDeltaInputs {
    pub source_artifact_id: SemanticDigest,
    pub target_artifact_id: SemanticDigest,
    pub source_label: String,
    pub target_label: String,
    pub comparator: ContractIdentity,
    pub encoding: String,
    pub segmentation: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextDeltaLimits {
    pub max_input_bytes: u64,
    pub max_lines: u64,
    pub max_alignment_cells: u64,
    pub max_changes: u64,
    pub max_string_bytes: u64,
}

impl Default for TextDeltaLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1_024,
            max_lines: 4_096,
            max_alignment_cells: 1_000_000,
            max_changes: 8_192,
            max_string_bytes: 512 * 1_024,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextLineKind {
    Context,
    Delete,
    Insert,
}

impl TextLineKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Delete => "delete",
            Self::Insert => "insert",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextLine {
    pub kind: TextLineKind,
    pub source_line: Option<u64>,
    pub target_line: Option<u64>,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextHunk {
    pub source_start_line: u64,
    pub source_line_count: u64,
    pub target_start_line: u64,
    pub target_line_count: u64,
    pub lines: Vec<TextLine>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub inputs: TextDeltaInputs,
    pub assessment: DeltaAssessment,
    pub source_line_count: u64,
    pub target_line_count: u64,
    pub source_final_newline: bool,
    pub target_final_newline: bool,
    pub hunks: Vec<TextHunk>,
    pub limits: TextDeltaLimits,
}

impl TextDelta {
    pub fn verify(&self, source: &str, target: &str) -> Result<(), TextDeltaError> {
        if self.schema != TEXT_DELTA_SCHEMA {
            return Err(TextDeltaError::UnsupportedSchema(self.schema.clone()));
        }
        let recomputed = compare_text(self.inputs.clone(), source, target, self.limits.clone())?;
        if self != &recomputed {
            return Err(TextDeltaError::ReplayMismatch);
        }
        Ok(())
    }
}

pub fn text_artifact_id(text: &str) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.utf8-text-artifact.v1");
    hasher.add_str("utf-8");
    hasher.add_bytes(text.as_bytes());
    hasher.finish()
}

pub fn compare_text(
    inputs: TextDeltaInputs,
    source: &str,
    target: &str,
    limits: TextDeltaLimits,
) -> Result<TextDelta, TextDeltaError> {
    validate_inputs(&inputs)?;
    validate_limits(&limits)?;
    if inputs.source_artifact_id != text_artifact_id(source)
        || inputs.target_artifact_id != text_artifact_id(target)
    {
        return Err(TextDeltaError::ArtifactBinding);
    }
    for (role, value) in [("source", source), ("target", target)] {
        if value.len() as u64 > limits.max_input_bytes {
            return Err(TextDeltaError::InputByteLimit {
                role,
                limit: limits.max_input_bytes,
                observed: value.len() as u64,
            });
        }
    }
    let source_lines = lines(source);
    let target_lines = lines(target);
    for (role, count) in [
        ("source", source_lines.len() as u64),
        ("target", target_lines.len() as u64),
    ] {
        if count > limits.max_lines {
            return Err(TextDeltaError::LineLimit {
                role,
                limit: limits.max_lines,
                observed: count,
            });
        }
    }
    let cells = (source_lines.len() as u64)
        .checked_add(1)
        .and_then(|left| {
            (target_lines.len() as u64)
                .checked_add(1)
                .and_then(|right| left.checked_mul(right))
        })
        .ok_or(TextDeltaError::CountOverflow)?;
    if cells > limits.max_alignment_cells {
        return Err(TextDeltaError::AlignmentLimit {
            limit: limits.max_alignment_cells,
            observed: cells,
        });
    }

    let aligned = align_lines(&source_lines, &target_lines)?;
    let change_count = aligned
        .iter()
        .filter(|line| line.kind != TextLineKind::Context)
        .count() as u64;
    if change_count > limits.max_changes {
        return Err(TextDeltaError::ChangeLimit {
            limit: limits.max_changes,
            observed: change_count,
        });
    }
    let hunks = if change_count == 0 {
        Vec::new()
    } else {
        vec![TextHunk {
            source_start_line: 1,
            source_line_count: source_lines.len() as u64,
            target_start_line: 1,
            target_line_count: target_lines.len() as u64,
            lines: aligned,
        }]
    };
    let string_bytes = inputs.source_label.len() as u64
        + inputs.target_label.len() as u64
        + inputs.encoding.len() as u64
        + inputs.segmentation.len() as u64
        + inputs.comparator.id.len() as u64
        + inputs.comparator.semantic_digest.as_str().len() as u64
        + hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .map(|line| line.text.len() as u64)
            .sum::<u64>();
    if string_bytes > limits.max_string_bytes {
        return Err(TextDeltaError::StringByteLimit {
            limit: limits.max_string_bytes,
            observed: string_bytes,
        });
    }

    let mut delta = TextDelta {
        schema: TEXT_DELTA_SCHEMA.to_owned(),
        delta_id: placeholder_digest(),
        inputs,
        assessment: if change_count == 0 {
            DeltaAssessment::Equal
        } else {
            DeltaAssessment::Different
        },
        source_line_count: source_lines.len() as u64,
        target_line_count: target_lines.len() as u64,
        source_final_newline: source.ends_with('\n'),
        target_final_newline: target.ends_with('\n'),
        hunks,
        limits,
    };
    delta.delta_id = delta_digest(&delta);
    Ok(delta)
}

fn lines(value: &str) -> Vec<&str> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split_inclusive('\n').collect()
    }
}

fn align_lines(source: &[&str], target: &[&str]) -> Result<Vec<TextLine>, TextDeltaError> {
    let width = target
        .len()
        .checked_add(1)
        .ok_or(TextDeltaError::CountOverflow)?;
    let cells = source
        .len()
        .checked_add(1)
        .and_then(|height| height.checked_mul(width))
        .ok_or(TextDeltaError::CountOverflow)?;
    let mut lcs = vec![0_u32; cells];
    for source_index in (0..source.len()).rev() {
        for target_index in (0..target.len()).rev() {
            let index = source_index * width + target_index;
            lcs[index] = if source[source_index] == target[target_index] {
                lcs[(source_index + 1) * width + target_index + 1].saturating_add(1)
            } else {
                lcs[(source_index + 1) * width + target_index]
                    .max(lcs[source_index * width + target_index + 1])
            };
        }
    }

    let mut result = Vec::new();
    let (mut source_index, mut target_index) = (0_usize, 0_usize);
    while source_index < source.len() || target_index < target.len() {
        if source_index < source.len()
            && target_index < target.len()
            && source[source_index] == target[target_index]
        {
            result.push(TextLine {
                kind: TextLineKind::Context,
                source_line: Some(source_index as u64 + 1),
                target_line: Some(target_index as u64 + 1),
                text: source[source_index].to_owned(),
            });
            source_index += 1;
            target_index += 1;
        } else if target_index < target.len()
            && (source_index == source.len()
                || lcs[source_index * width + target_index + 1]
                    > lcs[(source_index + 1) * width + target_index])
        {
            result.push(TextLine {
                kind: TextLineKind::Insert,
                source_line: None,
                target_line: Some(target_index as u64 + 1),
                text: target[target_index].to_owned(),
            });
            target_index += 1;
        } else {
            result.push(TextLine {
                kind: TextLineKind::Delete,
                source_line: Some(source_index as u64 + 1),
                target_line: None,
                text: source[source_index].to_owned(),
            });
            source_index += 1;
        }
    }
    Ok(result)
}

fn validate_inputs(inputs: &TextDeltaInputs) -> Result<(), TextDeltaError> {
    for digest in [&inputs.source_artifact_id, &inputs.target_artifact_id] {
        validate_digest(digest)?;
    }
    for (role, value) in [
        ("source label", inputs.source_label.as_str()),
        ("target label", inputs.target_label.as_str()),
        ("encoding", inputs.encoding.as_str()),
        ("segmentation", inputs.segmentation.as_str()),
        ("comparator id", inputs.comparator.id.as_str()),
    ] {
        if value.is_empty() || value.contains('\0') {
            return Err(TextDeltaError::InvalidText(role));
        }
    }
    if inputs.comparator.revision == 0 {
        return Err(TextDeltaError::InvalidContract);
    }
    validate_digest(&inputs.comparator.semantic_digest)
}

fn validate_limits(limits: &TextDeltaLimits) -> Result<(), TextDeltaError> {
    if limits.max_input_bytes == 0
        || limits.max_lines == 0
        || limits.max_alignment_cells == 0
        || limits.max_changes == 0
        || limits.max_string_bytes == 0
    {
        return Err(TextDeltaError::InvalidLimit);
    }
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), TextDeltaError> {
    let value = digest.as_str();
    if value.len() != "blake3:".len() + 64
        || !value.starts_with("blake3:")
        || !value["blake3:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(TextDeltaError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn placeholder_digest() -> SemanticDigest {
    SemanticHasher::new("rey.text-delta.placeholder").finish()
}

fn delta_digest(delta: &TextDelta) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(TEXT_DELTA_SCHEMA);
    hasher.add_str(delta.inputs.source_artifact_id.as_str());
    hasher.add_str(delta.inputs.target_artifact_id.as_str());
    hasher.add_str(&delta.inputs.source_label);
    hasher.add_str(&delta.inputs.target_label);
    delta.inputs.comparator.add_semantics(&mut hasher);
    hasher.add_str(&delta.inputs.encoding);
    hasher.add_str(&delta.inputs.segmentation);
    hasher.add_str(delta.assessment.as_str());
    hasher.add_u64(delta.source_line_count);
    hasher.add_u64(delta.target_line_count);
    hasher.add_bool(delta.source_final_newline);
    hasher.add_bool(delta.target_final_newline);
    hasher.add_u64(delta.hunks.len() as u64);
    for hunk in &delta.hunks {
        hasher.add_u64(hunk.source_start_line);
        hasher.add_u64(hunk.source_line_count);
        hasher.add_u64(hunk.target_start_line);
        hasher.add_u64(hunk.target_line_count);
        hasher.add_u64(hunk.lines.len() as u64);
        for line in &hunk.lines {
            hasher.add_str(line.kind.as_str());
            hasher.add_bool(line.source_line.is_some());
            if let Some(value) = line.source_line {
                hasher.add_u64(value);
            }
            hasher.add_bool(line.target_line.is_some());
            if let Some(value) = line.target_line {
                hasher.add_u64(value);
            }
            hasher.add_str(&line.text);
        }
    }
    hasher.add_u64(delta.limits.max_input_bytes);
    hasher.add_u64(delta.limits.max_lines);
    hasher.add_u64(delta.limits.max_alignment_cells);
    hasher.add_u64(delta.limits.max_changes);
    hasher.add_u64(delta.limits.max_string_bytes);
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum TextDeltaError {
    #[error("text delta limits must be non-zero")]
    InvalidLimit,
    #[error("invalid text delta contract")]
    InvalidContract,
    #[error("invalid text delta {0}")]
    InvalidText(&'static str),
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("text inputs do not match their artifact identities")]
    ArtifactBinding,
    #[error("{role} text exceeds the {limit}-byte limit with {observed} bytes")]
    InputByteLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("{role} text exceeds the {limit}-line limit with {observed} lines")]
    LineLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("text alignment exceeds the {limit}-cell limit with {observed} cells")]
    AlignmentLimit { limit: u64, observed: u64 },
    #[error("text delta exceeds the {limit}-change limit with {observed} changes")]
    ChangeLimit { limit: u64, observed: u64 },
    #[error("text delta exceeds the {limit}-string-byte limit with {observed} bytes")]
    StringByteLimit { limit: u64, observed: u64 },
    #[error("text delta count overflowed")]
    CountOverflow,
    #[error("unsupported text delta schema {0}")]
    UnsupportedSchema(String),
    #[error("text delta replay did not reproduce the retained artifact")]
    ReplayMismatch,
}

#[cfg(test)]
mod tests {
    use rey_core::ContractIdentity;

    use super::{
        TextDeltaError, TextDeltaInputs, TextDeltaLimits, TextLineKind, compare_text,
        text_artifact_id,
    };
    use crate::DeltaAssessment;

    fn inputs(source: &str, target: &str) -> TextDeltaInputs {
        TextDeltaInputs {
            source_artifact_id: text_artifact_id(source),
            target_artifact_id: text_artifact_id(target),
            source_label: "EXPECTED".to_owned(),
            target_label: "OBSERVED".to_owned(),
            comparator: ContractIdentity::new("rey.text.lines-exact", 1, "exact UTF-8 lines"),
            encoding: "utf-8".to_owned(),
            segmentation: "lines-preserve-terminators".to_owned(),
        }
    }

    #[test]
    fn ordered_text_delta_preserves_direction_lines_and_final_newline() {
        let source = "alpha\nbeta\n";
        let target = "alpha\ngamma";
        let delta = compare_text(
            inputs(source, target),
            source,
            target,
            TextDeltaLimits::default(),
        )
        .unwrap();

        assert_eq!(delta.assessment, DeltaAssessment::Different);
        assert!(delta.source_final_newline);
        assert!(!delta.target_final_newline);
        assert_eq!(delta.hunks.len(), 1);
        assert!(
            delta.hunks[0]
                .lines
                .iter()
                .any(|line| line.kind == TextLineKind::Delete && line.text == "beta\n")
        );
        assert!(
            delta.hunks[0]
                .lines
                .iter()
                .any(|line| line.kind == TextLineKind::Insert && line.text == "gamma")
        );
        delta.verify(source, target).unwrap();
    }

    #[test]
    fn typed_empty_text_is_equal_and_replayable() {
        let delta = compare_text(inputs("", ""), "", "", TextDeltaLimits::default()).unwrap();
        assert_eq!(delta.assessment, DeltaAssessment::Equal);
        assert!(delta.hunks.is_empty());
        delta.verify("", "").unwrap();
    }

    #[test]
    fn unicode_and_long_lines_remain_exact_under_explicit_bounds() {
        let source = format!("café {}\n", "x".repeat(1_024));
        let target = format!("café {} Δ\n", "x".repeat(1_024));
        let limits = TextDeltaLimits {
            max_input_bytes: 4_096,
            max_string_bytes: 8_192,
            ..TextDeltaLimits::default()
        };
        let delta = compare_text(inputs(&source, &target), &source, &target, limits).unwrap();

        assert_eq!(delta.assessment, DeltaAssessment::Different);
        assert!(
            delta.hunks[0]
                .lines
                .iter()
                .any(|line| line.kind == TextLineKind::Insert && line.text.contains('Δ'))
        );
        delta.verify(&source, &target).unwrap();
    }

    #[test]
    fn text_bounds_fail_closed_before_alignment_is_accepted() {
        let source = "alpha\nbeta\n";
        let target = "alpha\ngamma\n";
        let byte_error = compare_text(
            inputs(source, target),
            source,
            target,
            TextDeltaLimits {
                max_input_bytes: 1,
                ..TextDeltaLimits::default()
            },
        )
        .unwrap_err();
        assert!(matches!(byte_error, TextDeltaError::InputByteLimit { .. }));

        let alignment_error = compare_text(
            inputs(source, target),
            source,
            target,
            TextDeltaLimits {
                max_alignment_cells: 1,
                ..TextDeltaLimits::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            alignment_error,
            TextDeltaError::AlignmentLimit { .. }
        ));
    }
}
