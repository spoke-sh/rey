use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{self, Read},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use polars::df;
use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_dataframe::{Frame, FrameError, FrameMetadata};
use rey_mining::{
    MiningArtifactContract, MiningArtifactKind, MiningArtifactRef, MiningCompleteness,
    MiningConsumption, MiningDependencyEdge, MiningDependencyKind, MiningDeterminism, MiningError,
    MiningExecutionClass, MiningFamily, MiningInvalidation, MiningLimits, MiningLineage,
    MiningLineageKind, MiningOmission, MiningOmissionKind, MiningOperation, MiningOperationKind,
    MiningParameterContract, MiningParameterType, MiningParameterValue, MiningRequest,
    MiningResult,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SOURCE_CORPUS_SCHEMA: &str = "rey.source-corpus.v1";
pub const SOURCE_MATCH_RELATION: &str = "rey.source-matches";
pub const SOURCE_MATCH_SCHEMA_VERSION: &str = "1";

const SOURCE_PROVIDER_ID: &str = "rey.local-source.builtin";
const SOURCE_SEARCH_OPERATION_ID: &str = "rey.source-search.literal-utf8";
const SOURCE_SEARCH_CAPABILITY_ID: &str = "source.search.literal-utf8";
const SOURCE_SEARCH_DEFINITION: &str = "search an explicitly bound canonical local corpus for a non-empty UTF-8 literal; files ordered by reversible path identity and matches ordered by non-overlapping start byte; byte columns and context spans are zero-based while line numbers are one-based; binary and invalid UTF-8 files remain explicit omissions";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePathEncoding {
    UnixBytesBase64Url,
    WindowsUtf16LeBase64Url,
}

impl SourcePathEncoding {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnixBytesBase64Url => "unix_bytes_base64url",
            Self::WindowsUtf16LeBase64Url => "windows_utf16le_base64url",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SourcePathIdentity {
    pub encoding: SourcePathEncoding,
    pub encoded: String,
    pub display: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceContentClass {
    Utf8Text,
    Binary,
    InvalidUtf8,
}

impl SourceContentClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8Text => "utf8_text",
            Self::Binary => "binary",
            Self::InvalidUtf8 => "invalid_utf8",
        }
    }

    const fn media_type(self) -> &'static str {
        match self {
            Self::Utf8Text => "text/plain; charset=utf-8",
            Self::Binary | Self::InvalidUtf8 => "application/octet-stream",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceBindingLimits {
    pub max_files: u64,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_lines_per_file: u64,
    pub max_path_bytes: u64,
}

impl Default for SourceBindingLimits {
    fn default() -> Self {
        Self {
            max_files: 1_024,
            max_file_bytes: 8 * 1_024 * 1_024,
            max_total_bytes: 64 * 1_024 * 1_024,
            max_lines_per_file: 1_000_000,
            max_path_bytes: 4_096,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceFileBinding {
    pub path: SourcePathIdentity,
    pub artifact_id: SemanticDigest,
    pub byte_len: u64,
    pub line_count: u64,
    pub content_class: SourceContentClass,
    pub media_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceCorpusBinding {
    pub schema: String,
    pub binding_id: SemanticDigest,
    pub provider: ContractIdentity,
    pub root_id: SemanticDigest,
    pub limits: SourceBindingLimits,
    pub total_bytes: u64,
    pub files: Vec<SourceFileBinding>,
}

impl SourceCorpusBinding {
    fn new(
        root_id: SemanticDigest,
        limits: SourceBindingLimits,
        mut files: Vec<SourceFileBinding>,
    ) -> Result<Self, SourceMiningError> {
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let total_bytes = validate_binding_shape(&root_id, &limits, &files)?;
        let mut binding = Self {
            schema: SOURCE_CORPUS_SCHEMA.to_owned(),
            binding_id: placeholder_digest("rey.source-corpus.placeholder"),
            provider: local_source_provider(),
            root_id,
            limits,
            total_bytes,
            files,
        };
        binding.binding_id = corpus_digest(&binding);
        Ok(binding)
    }

    pub fn verify(&self) -> Result<(), SourceMiningError> {
        if self.schema != SOURCE_CORPUS_SCHEMA {
            return Err(SourceMiningError::UnsupportedSchema {
                expected: SOURCE_CORPUS_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.provider != local_source_provider() {
            return Err(SourceMiningError::BindingProvider);
        }
        validate_digest(&self.binding_id)?;
        let total_bytes = validate_binding_shape(&self.root_id, &self.limits, &self.files)?;
        if total_bytes != self.total_bytes {
            return Err(SourceMiningError::BindingShape);
        }
        let actual = corpus_digest(self);
        if actual != self.binding_id {
            return Err(SourceMiningError::BindingDigest {
                declared: self.binding_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn artifact_ref(&self) -> MiningArtifactRef {
        MiningArtifactRef {
            port_id: "corpus".to_owned(),
            artifact_id: self.binding_id.clone(),
            kind: MiningArtifactKind::Native,
            schema: Some(source_corpus_contract()),
            media_type: "application/vnd.rey.source-corpus+json".to_owned(),
            provider: self.provider.clone(),
            source_id: format!("rey-local-corpus://{}", self.binding_id),
            source_revision: self.binding_id.to_string(),
            logical_bytes: self.total_bytes,
        }
    }
}

#[derive(Clone, Debug)]
struct FrozenSourceFile {
    relative_path: PathBuf,
    binding: SourceFileBinding,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct LocalSourceCorpus {
    canonical_root: PathBuf,
    binding: SourceCorpusBinding,
    files: Vec<FrozenSourceFile>,
}

impl LocalSourceCorpus {
    pub fn bind(
        root: impl AsRef<Path>,
        relative_paths: impl IntoIterator<Item = PathBuf>,
        limits: SourceBindingLimits,
    ) -> Result<Self, SourceMiningError> {
        validate_binding_limits(&limits)?;
        let root = root.as_ref();
        let canonical_root = fs::canonicalize(root).map_err(|source| SourceMiningError::Root {
            path: root.to_owned(),
            source,
        })?;
        if !canonical_root.is_dir() {
            return Err(SourceMiningError::RootNotDirectory(canonical_root));
        }
        let root_id = root_digest(&canonical_root);
        let paths = relative_paths.into_iter().collect::<Vec<_>>();
        enforce_limit("source file", paths.len() as u64, limits.max_files)?;
        if paths.is_empty() {
            return Err(SourceMiningError::EmptyCorpus);
        }

        let mut files = Vec::with_capacity(paths.len());
        let mut seen = BTreeSet::new();
        let mut total_bytes = 0_u64;
        for relative_path in paths {
            validate_relative_path(&relative_path)?;
            let path_identity = path_identity(&relative_path);
            let path_bytes = path_identity_bytes(&path_identity)?;
            if path_bytes > limits.max_path_bytes {
                return Err(SourceMiningError::Limit {
                    kind: "source path byte",
                    limit: limits.max_path_bytes,
                    observed: path_bytes,
                });
            }
            if !seen.insert((path_identity.encoding, path_identity.encoded.clone())) {
                return Err(SourceMiningError::DuplicatePath(path_identity.encoded));
            }
            let first = read_bound_file(&canonical_root, &relative_path, limits.max_file_bytes)?;
            let second = read_bound_file(&canonical_root, &relative_path, limits.max_file_bytes)?;
            if first != second {
                return Err(SourceMiningError::SourceChanged(path_identity.encoded));
            }
            let byte_len = first.len() as u64;
            total_bytes = total_bytes
                .checked_add(byte_len)
                .ok_or(SourceMiningError::CountOverflow)?;
            enforce_limit("source total byte", total_bytes, limits.max_total_bytes)?;
            let line_count = line_count(&first)?;
            enforce_limit("source line", line_count, limits.max_lines_per_file)?;
            let content_class = classify_content(&first);
            let artifact_id = source_artifact_digest(&root_id, &path_identity, &first);
            let binding = SourceFileBinding {
                path: path_identity,
                artifact_id,
                byte_len,
                line_count,
                content_class,
                media_type: content_class.media_type().to_owned(),
            };
            files.push(FrozenSourceFile {
                relative_path,
                binding,
                bytes: first,
            });
        }
        files.sort_by(|left, right| left.binding.path.cmp(&right.binding.path));
        let binding = SourceCorpusBinding::new(
            root_id,
            limits,
            files.iter().map(|file| file.binding.clone()).collect(),
        )?;
        let corpus = Self {
            canonical_root,
            binding,
            files,
        };
        corpus.verify_frozen()?;
        Ok(corpus)
    }

    #[must_use]
    pub const fn binding(&self) -> &SourceCorpusBinding {
        &self.binding
    }

    #[must_use]
    pub fn file_bytes(&self, artifact_id: &SemanticDigest) -> Option<&[u8]> {
        self.files
            .iter()
            .find(|file| &file.binding.artifact_id == artifact_id)
            .map(|file| file.bytes.as_slice())
    }

    pub fn verify_current(&self) -> Result<(), SourceMiningError> {
        self.verify_frozen()?;
        for file in &self.files {
            let current = read_bound_file(
                &self.canonical_root,
                &file.relative_path,
                self.binding.limits.max_file_bytes,
            )?;
            if current != file.bytes {
                return Err(SourceMiningError::SourceChanged(
                    file.binding.path.encoded.clone(),
                ));
            }
        }
        Ok(())
    }

    fn verify_frozen(&self) -> Result<(), SourceMiningError> {
        self.binding.verify()?;
        if self.files.len() != self.binding.files.len() {
            return Err(SourceMiningError::BindingShape);
        }
        for (frozen, declared) in self.files.iter().zip(&self.binding.files) {
            if &frozen.binding != declared
                || source_artifact_digest(
                    &self.binding.root_id,
                    &frozen.binding.path,
                    &frozen.bytes,
                ) != frozen.binding.artifact_id
            {
                return Err(SourceMiningError::BindingShape);
            }
        }
        Ok(())
    }

    pub fn search(
        &self,
        request: &MiningRequest,
    ) -> Result<SourceSearchEvidence, SourceMiningError> {
        let operation = builtin_source_search_operation();
        request.verify_against(&operation)?;
        if request.provider != local_source_provider() {
            return Err(SourceMiningError::RequestProvider);
        }
        if request.inputs != [self.binding.artifact_ref()] {
            return Err(SourceMiningError::RequestInput);
        }
        if let Err(error) = self.verify_current() {
            let omission = MiningOmission {
                kind: MiningOmissionKind::SourceDrift,
                subject_id: None,
                omitted_count: 1,
                reason: error.to_string(),
            };
            return SourceSearchEvidence::terminal(
                self,
                request,
                &operation,
                MiningCompleteness::Failed,
                omission,
            );
        }

        let pattern = parameter_utf8(request, "pattern")?;
        if pattern.is_empty() {
            return SourceSearchEvidence::terminal(
                self,
                request,
                &operation,
                MiningCompleteness::Failed,
                MiningOmission {
                    kind: MiningOmissionKind::MalformedInput,
                    subject_id: Some("pattern".to_owned()),
                    omitted_count: 1,
                    reason: "literal search pattern must be non-empty".to_owned(),
                },
            );
        }
        let context_before = parameter_u64(request, "context_before")?;
        let context_after = parameter_u64(request, "context_after")?;
        let context_depth = context_before
            .checked_add(context_after)
            .ok_or(SourceMiningError::CountOverflow)?;
        if context_depth > request.effective_limits.max_depth {
            return SourceSearchEvidence::terminal(
                self,
                request,
                &operation,
                MiningCompleteness::Failed,
                MiningOmission {
                    kind: MiningOmissionKind::MalformedInput,
                    subject_id: Some("context".to_owned()),
                    omitted_count: 1,
                    reason: format!(
                        "requested context depth {context_depth} exceeds effective depth {}",
                        request.effective_limits.max_depth
                    ),
                },
            );
        }

        let evidence =
            self.search_literal(request, &operation, pattern, context_before, context_after)?;
        if let Err(error) = self.verify_current() {
            return SourceSearchEvidence::terminal(
                self,
                request,
                &operation,
                MiningCompleteness::Failed,
                MiningOmission {
                    kind: MiningOmissionKind::SourceDrift,
                    subject_id: None,
                    omitted_count: 1,
                    reason: error.to_string(),
                },
            );
        }
        Ok(evidence)
    }

    fn search_literal(
        &self,
        request: &MiningRequest,
        operation: &MiningOperation,
        pattern: &str,
        context_before: u64,
        context_after: u64,
    ) -> Result<SourceSearchEvidence, SourceMiningError> {
        let started = Instant::now();
        let deadline = Duration::from_millis(request.effective_limits.max_time_ms);
        let pattern_id = literal_pattern_digest(pattern);
        let mut matches = Vec::new();
        let mut contexts = Vec::new();
        let mut omissions = Vec::new();
        let mut files_consumed = 0_u64;
        let mut bytes_read = 0_u64;
        let mut bytes_written = 0_u64;
        let mut string_bytes = 0_u64;
        let row_limit = request
            .effective_limits
            .max_matches
            .min(request.effective_limits.max_rows);
        let file_limit = usize::try_from(request.effective_limits.max_files)
            .unwrap_or(usize::MAX)
            .min(self.files.len());
        if file_limit < self.files.len() {
            omissions.push(MiningOmission {
                kind: MiningOmissionKind::FileLimit,
                subject_id: None,
                omitted_count: (self.files.len() - file_limit) as u64,
                reason: "effective file limit omitted canonically trailing bound files".to_owned(),
            });
        }

        'files: for file in self.files.iter().take(file_limit) {
            if started.elapsed() >= deadline {
                omissions.push(time_omission());
                break;
            }
            let next_bytes_read = bytes_read
                .checked_add(file.binding.byte_len)
                .ok_or(SourceMiningError::CountOverflow)?;
            let next_total_bytes = next_bytes_read
                .checked_add(bytes_written)
                .ok_or(SourceMiningError::CountOverflow)?;
            if next_total_bytes > request.effective_limits.max_bytes {
                omissions.push(MiningOmission {
                    kind: MiningOmissionKind::ByteLimit,
                    subject_id: Some(file.binding.path.encoded.clone()),
                    omitted_count: 1,
                    reason: "effective read/write byte limit reached before the next file"
                        .to_owned(),
                });
                break;
            }
            files_consumed = files_consumed
                .checked_add(1)
                .ok_or(SourceMiningError::CountOverflow)?;
            bytes_read = next_bytes_read;
            if file.binding.content_class != SourceContentClass::Utf8Text {
                omissions.push(MiningOmission {
                    kind: MiningOmissionKind::Unsupported,
                    subject_id: Some(file.binding.path.encoded.clone()),
                    omitted_count: 1,
                    reason: format!(
                        "{} source is not searchable by the UTF-8 literal baseline",
                        file.binding.content_class.as_str()
                    ),
                });
                continue;
            }
            let text = std::str::from_utf8(&file.bytes)
                .map_err(|_| SourceMiningError::FrozenClassification)?;
            let line_starts = line_starts(&file.bytes)?;
            for (start_byte, matched) in text.match_indices(pattern) {
                if started.elapsed() >= deadline {
                    omissions.push(time_omission());
                    break 'files;
                }
                if matches.len() as u64 >= row_limit {
                    let kind = if request.effective_limits.max_matches
                        <= request.effective_limits.max_rows
                    {
                        MiningOmissionKind::MatchLimit
                    } else {
                        MiningOmissionKind::RowLimit
                    };
                    omissions.push(MiningOmission {
                        kind,
                        subject_id: None,
                        omitted_count: 1,
                        reason: "effective match relation limit reached".to_owned(),
                    });
                    break 'files;
                }
                let end_byte = start_byte
                    .checked_add(matched.len())
                    .ok_or(SourceMiningError::CountOverflow)?;
                let span = match_span(
                    &file.bytes,
                    &line_starts,
                    start_byte,
                    end_byte,
                    context_before,
                    context_after,
                )?;
                let context_text = text
                    .get(span.context_start_byte..span.context_end_byte)
                    .ok_or(SourceMiningError::InvalidTextBoundary)?
                    .to_owned();
                let context_artifact_id = context_digest(
                    &file.binding.artifact_id,
                    span.context_start_byte,
                    span.context_end_byte,
                    context_text.as_bytes(),
                );
                let context_ref = format!(
                    "rey-local-source://{}#bytes={}-{}",
                    file.binding.artifact_id, span.context_start_byte, span.context_end_byte
                );
                let match_id = source_match_digest(
                    &request.request_id,
                    &file.binding.artifact_id,
                    &pattern_id,
                    start_byte as u64,
                    end_byte as u64,
                );
                let source_match = SourceMatch {
                    match_id,
                    source_artifact_id: file.binding.artifact_id.clone(),
                    path: file.binding.path.clone(),
                    pattern_id: pattern_id.clone(),
                    start_byte: start_byte as u64,
                    end_byte: end_byte as u64,
                    start_line: span.start_line,
                    start_byte_in_line: span.start_byte_in_line,
                    end_line: span.end_line,
                    end_byte_in_line: span.end_byte_in_line,
                    matched_text: matched.to_owned(),
                    context_artifact_id: context_artifact_id.clone(),
                    context_start_byte: span.context_start_byte as u64,
                    context_end_byte: span.context_end_byte as u64,
                    context_start_line: span.context_start_line,
                    context_end_line: span.context_end_line,
                    context_ref,
                };
                let context = SourceContextArtifact {
                    artifact_id: context_artifact_id,
                    source_artifact_id: file.binding.artifact_id.clone(),
                    start_byte: span.context_start_byte as u64,
                    end_byte: span.context_end_byte as u64,
                    text: context_text,
                };
                let next_string_bytes = string_bytes
                    .checked_add(match_string_bytes(&source_match, &context)?)
                    .ok_or(SourceMiningError::CountOverflow)?;
                if next_string_bytes > request.effective_limits.max_string_bytes {
                    omissions.push(MiningOmission {
                        kind: MiningOmissionKind::ByteLimit,
                        subject_id: None,
                        omitted_count: 1,
                        reason: "effective string-byte limit reached".to_owned(),
                    });
                    break 'files;
                }
                let next_bytes_written = bytes_written
                    .checked_add(match_relation_row_bytes(&source_match)?)
                    .and_then(|value| value.checked_add(context.text.len() as u64))
                    .ok_or(SourceMiningError::CountOverflow)?;
                let next_total_bytes = bytes_read
                    .checked_add(next_bytes_written)
                    .ok_or(SourceMiningError::CountOverflow)?;
                if next_total_bytes > request.effective_limits.max_bytes {
                    omissions.push(MiningOmission {
                        kind: MiningOmissionKind::ByteLimit,
                        subject_id: None,
                        omitted_count: 1,
                        reason: "effective read/write byte limit reached".to_owned(),
                    });
                    break 'files;
                }
                string_bytes = next_string_bytes;
                bytes_written = next_bytes_written;
                matches.push(source_match);
                contexts.push(context);
            }
        }

        matches.sort_by(source_match_order);
        contexts.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));
        contexts.dedup_by(|left, right| left.artifact_id == right.artifact_id);
        omissions.sort();
        omissions.dedup();
        let has_limit = omissions.iter().any(|omission| {
            matches!(
                omission.kind,
                MiningOmissionKind::FileLimit
                    | MiningOmissionKind::RowLimit
                    | MiningOmissionKind::MatchLimit
                    | MiningOmissionKind::ByteLimit
                    | MiningOmissionKind::TimeLimit
            )
        });
        let has_unsupported = omissions
            .iter()
            .any(|omission| omission.kind == MiningOmissionKind::Unsupported);
        let searched_text = self
            .files
            .iter()
            .take(file_limit)
            .any(|file| file.binding.content_class == SourceContentClass::Utf8Text);
        let completeness = if has_limit {
            MiningCompleteness::Truncated
        } else if has_unsupported && searched_text {
            MiningCompleteness::Partial
        } else if has_unsupported {
            MiningCompleteness::Unsupported
        } else {
            MiningCompleteness::Complete
        };

        let consumption = MiningConsumption {
            files: files_consumed,
            rows: matches.len() as u64,
            matches: matches.len() as u64,
            nodes: 0,
            edges: 0,
            depth: context_before.saturating_add(context_after),
            bytes_read,
            bytes_written,
            observed_time_ms: None,
        };
        SourceSearchEvidence::from_matches(
            self,
            request,
            operation,
            completeness,
            matches,
            contexts,
            omissions,
            consumption,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceMatch {
    pub match_id: SemanticDigest,
    pub source_artifact_id: SemanticDigest,
    pub path: SourcePathIdentity,
    pub pattern_id: SemanticDigest,
    pub start_byte: u64,
    pub end_byte: u64,
    pub start_line: u64,
    pub start_byte_in_line: u64,
    pub end_line: u64,
    pub end_byte_in_line: u64,
    pub matched_text: String,
    pub context_artifact_id: SemanticDigest,
    pub context_start_byte: u64,
    pub context_end_byte: u64,
    pub context_start_line: u64,
    pub context_end_line: u64,
    pub context_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceContextArtifact {
    pub artifact_id: SemanticDigest,
    pub source_artifact_id: SemanticDigest,
    pub start_byte: u64,
    pub end_byte: u64,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceSearchEvidence {
    pub binding_id: SemanticDigest,
    pub match_artifact: Option<MiningArtifactRef>,
    pub result: MiningResult,
    pub matches: Vec<SourceMatch>,
    pub contexts: Vec<SourceContextArtifact>,
}

impl SourceSearchEvidence {
    fn terminal(
        corpus: &LocalSourceCorpus,
        request: &MiningRequest,
        operation: &MiningOperation,
        completeness: MiningCompleteness,
        omission: MiningOmission,
    ) -> Result<Self, SourceMiningError> {
        let result = MiningResult::new(
            request,
            operation,
            completeness,
            Vec::new(),
            search_lineage(operation),
            Vec::new(),
            vec![omission],
            MiningConsumption {
                files: 0,
                rows: 0,
                matches: 0,
                nodes: 0,
                edges: 0,
                depth: 0,
                bytes_read: 0,
                bytes_written: 0,
                observed_time_ms: None,
            },
        )?;
        Ok(Self {
            binding_id: corpus.binding.binding_id.clone(),
            match_artifact: None,
            result,
            matches: Vec::new(),
            contexts: Vec::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn from_matches(
        corpus: &LocalSourceCorpus,
        request: &MiningRequest,
        operation: &MiningOperation,
        completeness: MiningCompleteness,
        matches: Vec<SourceMatch>,
        contexts: Vec<SourceContextArtifact>,
        omissions: Vec<MiningOmission>,
        consumption: MiningConsumption,
    ) -> Result<Self, SourceMiningError> {
        if completeness == MiningCompleteness::Unsupported {
            let result = MiningResult::new(
                request,
                operation,
                completeness,
                Vec::new(),
                search_lineage(operation),
                Vec::new(),
                omissions,
                consumption,
            )?;
            return Ok(Self {
                binding_id: corpus.binding.binding_id.clone(),
                match_artifact: None,
                result,
                matches,
                contexts,
            });
        }

        let artifact_id = match_relation_digest(request, &matches);
        let logical_bytes = match_relation_logical_bytes(&matches)?;
        let artifact = MiningArtifactRef {
            port_id: "matches".to_owned(),
            artifact_id: artifact_id.clone(),
            kind: MiningArtifactKind::Relation,
            schema: Some(source_match_contract()),
            media_type: "application/vnd.apache.arrow.stream".to_owned(),
            provider: local_source_provider(),
            source_id: format!("rey-mining://{}/matches", request.request_id),
            source_revision: artifact_id.to_string(),
            logical_bytes,
        };
        let mut dependencies = vec![MiningDependencyEdge {
            artifact_id: artifact_id.clone(),
            kind: MiningDependencyKind::InputArtifact,
            dependency_id: corpus.binding.binding_id.clone(),
        }];
        dependencies.extend(
            corpus
                .binding
                .files
                .iter()
                .map(|file| MiningDependencyEdge {
                    artifact_id: artifact_id.clone(),
                    kind: MiningDependencyKind::SourceRevision,
                    dependency_id: file.artifact_id.clone(),
                }),
        );
        let result = MiningResult::new(
            request,
            operation,
            completeness,
            vec![artifact.clone()],
            search_lineage(operation),
            dependencies,
            omissions,
            consumption,
        )?;
        let evidence = Self {
            binding_id: corpus.binding.binding_id.clone(),
            match_artifact: Some(artifact),
            result,
            matches,
            contexts,
        };
        evidence.verify_against(corpus, request)?;
        Ok(evidence)
    }

    pub fn verify_against(
        &self,
        corpus: &LocalSourceCorpus,
        request: &MiningRequest,
    ) -> Result<(), SourceMiningError> {
        self.verify_detached(corpus.binding(), request)?;
        corpus.verify_frozen()?;
        let operation = builtin_source_search_operation();
        self.result.verify_against(request, &operation)?;
        if self.binding_id != corpus.binding.binding_id {
            return Err(SourceMiningError::EvidenceBinding);
        }
        if request.provider != local_source_provider()
            || request.inputs != [corpus.binding.artifact_ref()]
        {
            return Err(SourceMiningError::EvidenceBinding);
        }
        if self
            .matches
            .windows(2)
            .any(|window| source_match_order(&window[0], &window[1]).is_ge())
        {
            return Err(SourceMiningError::NonCanonicalMatches);
        }
        for source_match in &self.matches {
            validate_digest(&source_match.match_id)?;
            validate_digest(&source_match.source_artifact_id)?;
            validate_digest(&source_match.pattern_id)?;
            validate_digest(&source_match.context_artifact_id)?;
            let file = corpus
                .files
                .iter()
                .find(|file| file.binding.artifact_id == source_match.source_artifact_id)
                .ok_or(SourceMiningError::EvidenceBinding)?;
            if file.binding.path != source_match.path {
                return Err(SourceMiningError::EvidenceBinding);
            }
            let pattern = parameter_utf8(request, "pattern")?;
            let pattern_id = literal_pattern_digest(pattern);
            if source_match.pattern_id != pattern_id
                || source_match.match_id
                    != source_match_digest(
                        &request.request_id,
                        &source_match.source_artifact_id,
                        &pattern_id,
                        source_match.start_byte,
                        source_match.end_byte,
                    )
            {
                return Err(SourceMiningError::EvidenceDigest);
            }
            let start = usize::try_from(source_match.start_byte)
                .map_err(|_| SourceMiningError::EvidenceShape)?;
            let end = usize::try_from(source_match.end_byte)
                .map_err(|_| SourceMiningError::EvidenceShape)?;
            if file.bytes.get(start..end) != Some(source_match.matched_text.as_bytes()) {
                return Err(SourceMiningError::EvidenceBinding);
            }
            let line_starts = line_starts(&file.bytes)?;
            let span = match_span(
                &file.bytes,
                &line_starts,
                start,
                end,
                parameter_u64(request, "context_before")?,
                parameter_u64(request, "context_after")?,
            )?;
            if source_match.start_line != span.start_line
                || source_match.start_byte_in_line != span.start_byte_in_line
                || source_match.end_line != span.end_line
                || source_match.end_byte_in_line != span.end_byte_in_line
                || source_match.context_start_byte != span.context_start_byte as u64
                || source_match.context_end_byte != span.context_end_byte as u64
                || source_match.context_start_line != span.context_start_line
                || source_match.context_end_line != span.context_end_line
                || source_match.context_ref
                    != format!(
                        "rey-local-source://{}#bytes={}-{}",
                        source_match.source_artifact_id,
                        span.context_start_byte,
                        span.context_end_byte
                    )
            {
                return Err(SourceMiningError::EvidenceShape);
            }
            let context = self
                .contexts
                .iter()
                .find(|context| context.artifact_id == source_match.context_artifact_id)
                .ok_or(SourceMiningError::MissingContext)?;
            if context.source_artifact_id != source_match.source_artifact_id
                || context.start_byte != source_match.context_start_byte
                || context.end_byte != source_match.context_end_byte
                || file
                    .bytes
                    .get(span.context_start_byte..span.context_end_byte)
                    != Some(context.text.as_bytes())
                || context_digest(
                    &context.source_artifact_id,
                    context.start_byte as usize,
                    context.end_byte as usize,
                    context.text.as_bytes(),
                ) != context.artifact_id
            {
                return Err(SourceMiningError::ContextDigest);
            }
        }
        let expected_contexts = self
            .matches
            .iter()
            .map(|source_match| &source_match.context_artifact_id)
            .collect::<BTreeSet<_>>();
        let actual_contexts = self
            .contexts
            .iter()
            .map(|context| &context.artifact_id)
            .collect::<BTreeSet<_>>();
        if expected_contexts != actual_contexts || actual_contexts.len() != self.contexts.len() {
            return Err(SourceMiningError::EvidenceShape);
        }
        let productive = self.match_artifact.is_some();
        if productive != !self.result.outputs.is_empty() {
            return Err(SourceMiningError::EvidenceShape);
        }
        if let Some(artifact) = &self.match_artifact {
            if self.result.outputs != [artifact.clone()]
                || artifact.artifact_id != match_relation_digest(request, &self.matches)
                || artifact.logical_bytes != match_relation_logical_bytes(&self.matches)?
            {
                return Err(SourceMiningError::EvidenceDigest);
            }
        } else if !self.matches.is_empty() || !self.contexts.is_empty() {
            return Err(SourceMiningError::EvidenceShape);
        }
        Ok(())
    }

    /// Replays the retained manifest, relation, and native-context identities
    /// without reopening mutable source files. This is the verification level
    /// available to a bounded retained workload index; execution-time proof
    /// still uses `verify_against` with the frozen native corpus.
    pub fn verify_detached(
        &self,
        binding: &SourceCorpusBinding,
        request: &MiningRequest,
    ) -> Result<(), SourceMiningError> {
        binding.verify()?;
        let operation = builtin_source_search_operation();
        self.result.verify_against(request, &operation)?;
        if self.binding_id != binding.binding_id
            || request.provider != local_source_provider()
            || request.inputs != [binding.artifact_ref()]
        {
            return Err(SourceMiningError::EvidenceBinding);
        }
        if self
            .matches
            .windows(2)
            .any(|window| source_match_order(&window[0], &window[1]).is_ge())
        {
            return Err(SourceMiningError::NonCanonicalMatches);
        }
        let pattern_id = literal_pattern_digest(parameter_utf8(request, "pattern")?);
        for source_match in &self.matches {
            let file = binding
                .files
                .iter()
                .find(|file| file.artifact_id == source_match.source_artifact_id)
                .ok_or(SourceMiningError::EvidenceBinding)?;
            if file.path != source_match.path
                || source_match.pattern_id != pattern_id
                || source_match.start_byte >= source_match.end_byte
                || source_match.end_byte > file.byte_len
                || source_match.start_line == 0
                || source_match.end_line < source_match.start_line
                || source_match.context_start_byte > source_match.start_byte
                || source_match.context_end_byte < source_match.end_byte
                || source_match.context_end_byte > file.byte_len
                || source_match.context_start_line == 0
                || source_match.context_end_line < source_match.context_start_line
                || source_match.match_id
                    != source_match_digest(
                        &request.request_id,
                        &source_match.source_artifact_id,
                        &pattern_id,
                        source_match.start_byte,
                        source_match.end_byte,
                    )
            {
                return Err(SourceMiningError::EvidenceShape);
            }
            let context = self
                .contexts
                .iter()
                .find(|context| context.artifact_id == source_match.context_artifact_id)
                .ok_or(SourceMiningError::MissingContext)?;
            if context.source_artifact_id != source_match.source_artifact_id
                || context.start_byte != source_match.context_start_byte
                || context.end_byte != source_match.context_end_byte
                || context.end_byte.saturating_sub(context.start_byte) != context.text.len() as u64
                || context_digest(
                    &context.source_artifact_id,
                    context.start_byte as usize,
                    context.end_byte as usize,
                    context.text.as_bytes(),
                ) != context.artifact_id
            {
                return Err(SourceMiningError::ContextDigest);
            }
        }
        let expected_contexts = self
            .matches
            .iter()
            .map(|source_match| &source_match.context_artifact_id)
            .collect::<BTreeSet<_>>();
        let actual_contexts = self
            .contexts
            .iter()
            .map(|context| &context.artifact_id)
            .collect::<BTreeSet<_>>();
        if expected_contexts != actual_contexts || actual_contexts.len() != self.contexts.len() {
            return Err(SourceMiningError::EvidenceShape);
        }
        let productive = self.match_artifact.is_some();
        if productive != !self.result.outputs.is_empty() {
            return Err(SourceMiningError::EvidenceShape);
        }
        if let Some(artifact) = &self.match_artifact {
            if self.result.outputs != [artifact.clone()]
                || artifact.artifact_id != match_relation_digest(request, &self.matches)
                || artifact.logical_bytes != match_relation_logical_bytes(&self.matches)?
            {
                return Err(SourceMiningError::EvidenceDigest);
            }
        } else if !self.matches.is_empty() || !self.contexts.is_empty() {
            return Err(SourceMiningError::EvidenceShape);
        }
        Ok(())
    }

    pub fn to_frame(&self) -> Result<Frame, SourceMiningError> {
        let artifact = self
            .match_artifact
            .as_ref()
            .ok_or(SourceMiningError::NoMatchRelation)?;
        let rows = &self.matches;
        let dataframe = df!(
            "match_id" => rows.iter().map(|row| row.match_id.as_str()).collect::<Vec<_>>(),
            "source_artifact_id" => rows.iter().map(|row| row.source_artifact_id.as_str()).collect::<Vec<_>>(),
            "path_encoding" => rows.iter().map(|row| row.path.encoding.as_str()).collect::<Vec<_>>(),
            "path_identity" => rows.iter().map(|row| row.path.encoded.as_str()).collect::<Vec<_>>(),
            "path_display" => rows.iter().map(|row| row.path.display.as_str()).collect::<Vec<_>>(),
            "pattern_id" => rows.iter().map(|row| row.pattern_id.as_str()).collect::<Vec<_>>(),
            "start_byte" => rows.iter().map(|row| row.start_byte).collect::<Vec<_>>(),
            "end_byte" => rows.iter().map(|row| row.end_byte).collect::<Vec<_>>(),
            "start_line" => rows.iter().map(|row| row.start_line).collect::<Vec<_>>(),
            "start_byte_in_line" => rows.iter().map(|row| row.start_byte_in_line).collect::<Vec<_>>(),
            "end_line" => rows.iter().map(|row| row.end_line).collect::<Vec<_>>(),
            "end_byte_in_line" => rows.iter().map(|row| row.end_byte_in_line).collect::<Vec<_>>(),
            "matched_text" => rows.iter().map(|row| row.matched_text.as_str()).collect::<Vec<_>>(),
            "context_artifact_id" => rows.iter().map(|row| row.context_artifact_id.as_str()).collect::<Vec<_>>(),
            "context_start_byte" => rows.iter().map(|row| row.context_start_byte).collect::<Vec<_>>(),
            "context_end_byte" => rows.iter().map(|row| row.context_end_byte).collect::<Vec<_>>(),
            "context_start_line" => rows.iter().map(|row| row.context_start_line).collect::<Vec<_>>(),
            "context_end_line" => rows.iter().map(|row| row.context_end_line).collect::<Vec<_>>(),
            "context_ref" => rows.iter().map(|row| row.context_ref.as_str()).collect::<Vec<_>>(),
        )?;
        Ok(Frame::new(
            dataframe,
            FrameMetadata {
                relation: SOURCE_MATCH_RELATION.to_owned(),
                schema_version: SOURCE_MATCH_SCHEMA_VERSION.to_owned(),
                semantic_digest: artifact.artifact_id.to_string(),
                row_count: rows.len() as u64,
                complete: self.result.completeness == MiningCompleteness::Complete,
                key_columns: vec!["match_id".to_owned()],
                attributes: BTreeMap::from([
                    ("rey.binding-id".to_owned(), self.binding_id.to_string()),
                    (
                        "rey.mining-request-id".to_owned(),
                        self.result.request_id.to_string(),
                    ),
                    (
                        "rey.mining-result-id".to_owned(),
                        self.result.result_id.to_string(),
                    ),
                    (
                        "rey.completeness".to_owned(),
                        self.result.completeness.as_str().to_owned(),
                    ),
                ]),
            },
        )?)
    }
}

#[must_use]
pub fn local_source_provider() -> ContractIdentity {
    ContractIdentity::new(
        SOURCE_PROVIDER_ID,
        1,
        "trusted local built-in provider binds explicit canonical regular files without symlink traversal and projects deterministic UTF-8 literal matches; it is not a filesystem sandbox",
    )
}

#[must_use]
pub fn source_search_capability_identity() -> ContractIdentity {
    ContractIdentity::new(SOURCE_SEARCH_CAPABILITY_ID, 1, SOURCE_SEARCH_DEFINITION)
}

#[must_use]
pub fn source_corpus_contract() -> ContractIdentity {
    ContractIdentity::new(
        SOURCE_CORPUS_SCHEMA,
        1,
        "canonical explicit local source corpus with reversible path identity, exact per-file content digests, content classification, byte and line counts, root identity, and binding limits",
    )
}

#[must_use]
pub fn source_match_contract() -> ContractIdentity {
    ContractIdentity::new(
        "rey.source-matches.v1",
        1,
        "ordered non-overlapping UTF-8 literal matches keyed by match_id with reversible source path identity, source and pattern digests, absolute byte and one-based line spans, zero-based byte columns, native context artifact identity, and exact context source reference",
    )
}

#[must_use]
pub fn builtin_source_search_operation() -> MiningOperation {
    MiningOperation::new(
        SOURCE_SEARCH_OPERATION_ID,
        1,
        local_source_provider(),
        MiningFamily::Source,
        MiningOperationKind::Search,
        MiningExecutionClass::Probe,
        MiningDeterminism::FrozenDeterministic,
        vec![MiningArtifactContract {
            port_id: "corpus".to_owned(),
            kind: MiningArtifactKind::Native,
            schema: Some(source_corpus_contract()),
            media_type: Some("application/vnd.rey.source-corpus+json".to_owned()),
            required: true,
        }],
        vec![MiningArtifactContract {
            port_id: "matches".to_owned(),
            kind: MiningArtifactKind::Relation,
            schema: Some(source_match_contract()),
            media_type: Some("application/vnd.apache.arrow.stream".to_owned()),
            required: true,
        }],
        vec![
            MiningParameterContract {
                name: "context_after".to_owned(),
                value_type: MiningParameterType::U64,
                required: false,
                default: Some(MiningParameterValue::U64(0)),
            },
            MiningParameterContract {
                name: "context_before".to_owned(),
                value_type: MiningParameterType::U64,
                required: false,
                default: Some(MiningParameterValue::U64(0)),
            },
            MiningParameterContract {
                name: "pattern".to_owned(),
                value_type: MiningParameterType::Utf8,
                required: true,
                default: None,
            },
        ],
        vec![source_search_capability_identity()],
        vec![
            MiningInvalidation::CapabilitySnapshot,
            MiningInvalidation::ProviderRevision,
            MiningInvalidation::ImplementationRevision,
            MiningInvalidation::InputArtifactRevision,
            MiningInvalidation::ParameterChange,
            MiningInvalidation::EffectiveLimitChange,
        ],
        MiningLimits::default(),
    )
    .expect("built-in source search operation must remain valid")
}

pub fn explicit_source_path_identity(
    relative_path: impl AsRef<Path>,
) -> Result<SourcePathIdentity, SourceMiningError> {
    let relative_path = relative_path.as_ref();
    validate_relative_path(relative_path)?;
    Ok(path_identity(relative_path))
}

fn validate_binding_shape(
    root_id: &SemanticDigest,
    limits: &SourceBindingLimits,
    files: &[SourceFileBinding],
) -> Result<u64, SourceMiningError> {
    validate_digest(root_id)?;
    validate_binding_limits(limits)?;
    if files.is_empty() {
        return Err(SourceMiningError::EmptyCorpus);
    }
    enforce_limit("source file", files.len() as u64, limits.max_files)?;
    let mut total_bytes = 0_u64;
    for file in files {
        validate_path_identity(&file.path, limits.max_path_bytes)?;
        validate_digest(&file.artifact_id)?;
        enforce_limit("source file byte", file.byte_len, limits.max_file_bytes)?;
        enforce_limit("source line", file.line_count, limits.max_lines_per_file)?;
        if file.media_type != file.content_class.media_type() {
            return Err(SourceMiningError::BindingShape);
        }
        total_bytes = total_bytes
            .checked_add(file.byte_len)
            .ok_or(SourceMiningError::CountOverflow)?;
    }
    enforce_limit("source total byte", total_bytes, limits.max_total_bytes)?;
    if files.windows(2).any(|window| {
        (window[0].path.encoding, window[0].path.encoded.as_str())
            >= (window[1].path.encoding, window[1].path.encoded.as_str())
    }) {
        return Err(SourceMiningError::NonCanonicalBinding);
    }
    Ok(total_bytes)
}

fn validate_binding_limits(limits: &SourceBindingLimits) -> Result<(), SourceMiningError> {
    for (kind, value) in [
        ("max_files", limits.max_files),
        ("max_file_bytes", limits.max_file_bytes),
        ("max_total_bytes", limits.max_total_bytes),
        ("max_lines_per_file", limits.max_lines_per_file),
        ("max_path_bytes", limits.max_path_bytes),
    ] {
        if value == 0 {
            return Err(SourceMiningError::ZeroLimit(kind));
        }
    }
    Ok(())
}

fn validate_path_identity(
    identity: &SourcePathIdentity,
    max_path_bytes: u64,
) -> Result<(), SourceMiningError> {
    if identity.encoded.is_empty() || identity.display.is_empty() {
        return Err(SourceMiningError::InvalidPathIdentity);
    }
    let path_bytes = path_identity_bytes(identity)?;
    if path_bytes > max_path_bytes {
        return Err(SourceMiningError::InvalidPathIdentity);
    }
    Ok(())
}

fn path_identity_bytes(identity: &SourcePathIdentity) -> Result<u64, SourceMiningError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(identity.encoded.as_bytes())
        .map_err(|_| SourceMiningError::InvalidPathIdentity)?;
    u64::try_from(bytes.len()).map_err(|_| SourceMiningError::CountOverflow)
}

fn validate_relative_path(path: &Path) -> Result<(), SourceMiningError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(SourceMiningError::UnsafePath(path.to_owned()));
    }
    Ok(())
}

fn read_bound_file(
    root: &Path,
    relative_path: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, SourceMiningError> {
    validate_relative_path(relative_path)?;
    let mut current = root.to_owned();
    for component in relative_path.components() {
        let Component::Normal(component) = component else {
            return Err(SourceMiningError::UnsafePath(relative_path.to_owned()));
        };
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|source| SourceMiningError::File {
                path: relative_path.to_owned(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(SourceMiningError::Symlink(relative_path.to_owned()));
        }
    }
    let canonical = fs::canonicalize(&current).map_err(|source| SourceMiningError::File {
        path: relative_path.to_owned(),
        source,
    })?;
    if !canonical.starts_with(root) {
        return Err(SourceMiningError::PathEscape(relative_path.to_owned()));
    }
    let mut file = File::open(&canonical).map_err(|source| SourceMiningError::File {
        path: relative_path.to_owned(),
        source,
    })?;
    let metadata = file.metadata().map_err(|source| SourceMiningError::File {
        path: relative_path.to_owned(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(SourceMiningError::NotRegularFile(relative_path.to_owned()));
    }
    enforce_limit("source file byte", metadata.len(), max_bytes)?;
    let capture_limit = max_bytes
        .checked_add(1)
        .ok_or(SourceMiningError::CountOverflow)?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(capture_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| SourceMiningError::File {
            path: relative_path.to_owned(),
            source,
        })?;
    enforce_limit("source file byte", bytes.len() as u64, max_bytes)?;
    Ok(bytes)
}

fn classify_content(bytes: &[u8]) -> SourceContentClass {
    if bytes.contains(&0) {
        SourceContentClass::Binary
    } else if std::str::from_utf8(bytes).is_ok() {
        SourceContentClass::Utf8Text
    } else {
        SourceContentClass::InvalidUtf8
    }
}

fn line_count(bytes: &[u8]) -> Result<u64, SourceMiningError> {
    let newlines = u64::try_from(bytes.iter().filter(|byte| **byte == b'\n').count())
        .map_err(|_| SourceMiningError::CountOverflow)?;
    newlines
        .checked_add(1)
        .ok_or(SourceMiningError::CountOverflow)
}

fn line_starts(bytes: &[u8]) -> Result<Vec<usize>, SourceMiningError> {
    let mut starts = Vec::with_capacity(
        bytes
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            .saturating_add(1),
    );
    starts.push(0);
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            starts.push(
                index
                    .checked_add(1)
                    .ok_or(SourceMiningError::CountOverflow)?,
            );
        }
    }
    Ok(starts)
}

struct MatchSpan {
    start_line: u64,
    start_byte_in_line: u64,
    end_line: u64,
    end_byte_in_line: u64,
    context_start_byte: usize,
    context_end_byte: usize,
    context_start_line: u64,
    context_end_line: u64,
}

fn match_span(
    bytes: &[u8],
    line_starts: &[usize],
    start_byte: usize,
    end_byte: usize,
    context_before: u64,
    context_after: u64,
) -> Result<MatchSpan, SourceMiningError> {
    let start_index = line_index(line_starts, start_byte);
    let end_index = line_index(line_starts, end_byte);
    let before = usize::try_from(context_before).unwrap_or(usize::MAX);
    let after = usize::try_from(context_after).unwrap_or(usize::MAX);
    let context_start_index = start_index.saturating_sub(before);
    let context_end_index = end_index
        .saturating_add(after)
        .min(line_starts.len().saturating_sub(1));
    let context_end_byte = line_starts
        .get(context_end_index.saturating_add(1))
        .copied()
        .unwrap_or(bytes.len());
    Ok(MatchSpan {
        start_line: start_index as u64 + 1,
        start_byte_in_line: start_byte.saturating_sub(line_starts[start_index]) as u64,
        end_line: end_index as u64 + 1,
        end_byte_in_line: end_byte.saturating_sub(line_starts[end_index]) as u64,
        context_start_byte: line_starts[context_start_index],
        context_end_byte,
        context_start_line: context_start_index as u64 + 1,
        context_end_line: context_end_index as u64 + 1,
    })
}

fn line_index(line_starts: &[usize], byte_offset: usize) -> usize {
    line_starts
        .partition_point(|start| *start <= byte_offset)
        .saturating_sub(1)
}

fn parameter_utf8<'a>(
    request: &'a MiningRequest,
    name: &'static str,
) -> Result<&'a str, SourceMiningError> {
    match request.parameters.get(name) {
        Some(MiningParameterValue::Utf8(value)) => Ok(value),
        _ => Err(SourceMiningError::Parameter(name)),
    }
}

fn parameter_u64(request: &MiningRequest, name: &'static str) -> Result<u64, SourceMiningError> {
    match request.parameters.get(name) {
        Some(MiningParameterValue::U64(value)) => Ok(*value),
        _ => Err(SourceMiningError::Parameter(name)),
    }
}

fn search_lineage(operation: &MiningOperation) -> Vec<MiningLineage> {
    vec![
        MiningLineage {
            kind: MiningLineageKind::Implementation,
            identity: operation.implementation.clone(),
            execution_id: None,
        },
        MiningLineage {
            kind: MiningLineageKind::Provider,
            identity: local_source_provider(),
            execution_id: None,
        },
    ]
}

fn time_omission() -> MiningOmission {
    MiningOmission {
        kind: MiningOmissionKind::TimeLimit,
        subject_id: None,
        omitted_count: 1,
        reason: "effective wall-time limit reached".to_owned(),
    }
}

fn source_match_order(left: &SourceMatch, right: &SourceMatch) -> std::cmp::Ordering {
    left.path
        .cmp(&right.path)
        .then_with(|| left.start_byte.cmp(&right.start_byte))
        .then_with(|| left.end_byte.cmp(&right.end_byte))
        .then_with(|| left.match_id.cmp(&right.match_id))
}

fn match_string_bytes(
    source_match: &SourceMatch,
    context: &SourceContextArtifact,
) -> Result<u64, SourceMiningError> {
    [
        source_match.path.encoded.len(),
        source_match.path.display.len(),
        source_match.matched_text.len(),
        source_match.context_ref.len(),
        context.text.len(),
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total
            .checked_add(value as u64)
            .ok_or(SourceMiningError::CountOverflow)
    })
}

fn match_relation_logical_bytes(matches: &[SourceMatch]) -> Result<u64, SourceMiningError> {
    matches.iter().try_fold(0_u64, |total, source_match| {
        let row = match_relation_row_bytes(source_match)?;
        total
            .checked_add(row)
            .ok_or(SourceMiningError::CountOverflow)
    })
}

fn match_relation_row_bytes(source_match: &SourceMatch) -> Result<u64, SourceMiningError> {
    source_match
        .path
        .encoded
        .len()
        .checked_add(source_match.path.display.len())
        .and_then(|value| value.checked_add(source_match.matched_text.len()))
        .and_then(|value| value.checked_add(source_match.context_ref.len()))
        .and_then(|value| value.checked_add(16 * std::mem::size_of::<u64>()))
        .map(|value| value as u64)
        .ok_or(SourceMiningError::CountOverflow)
}

fn root_digest(root: &Path) -> SemanticDigest {
    let identity = path_identity(root);
    let mut hasher = SemanticHasher::new("rey.local-source-root.v1");
    hasher.add_str(identity.encoding.as_str());
    hasher.add_str(&identity.encoded);
    hasher.finish()
}

fn source_artifact_digest(
    root_id: &SemanticDigest,
    path: &SourcePathIdentity,
    bytes: &[u8],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.local-source-artifact.v1");
    hasher.add_str(root_id.as_str());
    hasher.add_str(path.encoding.as_str());
    hasher.add_str(&path.encoded);
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn literal_pattern_digest(pattern: &str) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.literal-search-pattern.v1");
    hasher.add_str(pattern);
    hasher.finish()
}

fn source_match_digest(
    request_id: &SemanticDigest,
    source_artifact_id: &SemanticDigest,
    pattern_id: &SemanticDigest,
    start_byte: u64,
    end_byte: u64,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.source-match.v1");
    hasher.add_str(request_id.as_str());
    hasher.add_str(source_artifact_id.as_str());
    hasher.add_str(pattern_id.as_str());
    hasher.add_u64(start_byte);
    hasher.add_u64(end_byte);
    hasher.finish()
}

fn context_digest(
    source_artifact_id: &SemanticDigest,
    start_byte: usize,
    end_byte: usize,
    bytes: &[u8],
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.source-context.v1");
    hasher.add_str(source_artifact_id.as_str());
    hasher.add_u64(start_byte as u64);
    hasher.add_u64(end_byte as u64);
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn match_relation_digest(request: &MiningRequest, matches: &[SourceMatch]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.source-matches.v1");
    hasher.add_str(request.request_id.as_str());
    hasher.add_u64(matches.len() as u64);
    for source_match in matches {
        hasher.add_str(source_match.match_id.as_str());
        hasher.add_str(source_match.source_artifact_id.as_str());
        hasher.add_str(source_match.path.encoding.as_str());
        hasher.add_str(&source_match.path.encoded);
        hasher.add_str(source_match.pattern_id.as_str());
        hasher.add_u64(source_match.start_byte);
        hasher.add_u64(source_match.end_byte);
        hasher.add_u64(source_match.start_line);
        hasher.add_u64(source_match.start_byte_in_line);
        hasher.add_u64(source_match.end_line);
        hasher.add_u64(source_match.end_byte_in_line);
        hasher.add_str(&source_match.matched_text);
        hasher.add_str(source_match.context_artifact_id.as_str());
        hasher.add_u64(source_match.context_start_byte);
        hasher.add_u64(source_match.context_end_byte);
        hasher.add_u64(source_match.context_start_line);
        hasher.add_u64(source_match.context_end_line);
        hasher.add_str(&source_match.context_ref);
    }
    hasher.finish()
}

fn corpus_digest(binding: &SourceCorpusBinding) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SOURCE_CORPUS_SCHEMA);
    binding.provider.add_semantics(&mut hasher);
    hasher.add_str(binding.root_id.as_str());
    add_binding_limits(&mut hasher, &binding.limits);
    hasher.add_u64(binding.total_bytes);
    hasher.add_u64(binding.files.len() as u64);
    for file in &binding.files {
        hasher.add_str(file.path.encoding.as_str());
        hasher.add_str(&file.path.encoded);
        hasher.add_str(file.artifact_id.as_str());
        hasher.add_u64(file.byte_len);
        hasher.add_u64(file.line_count);
        hasher.add_str(file.content_class.as_str());
        hasher.add_str(&file.media_type);
    }
    hasher.finish()
}

fn add_binding_limits(hasher: &mut SemanticHasher, limits: &SourceBindingLimits) {
    hasher.add_u64(limits.max_files);
    hasher.add_u64(limits.max_file_bytes);
    hasher.add_u64(limits.max_total_bytes);
    hasher.add_u64(limits.max_lines_per_file);
    hasher.add_u64(limits.max_path_bytes);
}

#[cfg(unix)]
fn path_identity(path: &Path) -> SourcePathIdentity {
    use std::os::unix::ffi::OsStrExt;

    SourcePathIdentity {
        encoding: SourcePathEncoding::UnixBytesBase64Url,
        encoded: URL_SAFE_NO_PAD.encode(path.as_os_str().as_bytes()),
        display: escaped_path_display(path),
    }
}

#[cfg(windows)]
fn path_identity(path: &Path) -> SourcePathIdentity {
    use std::os::windows::ffi::OsStrExt;

    let mut bytes = Vec::new();
    for unit in path.as_os_str().encode_wide() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    SourcePathIdentity {
        encoding: SourcePathEncoding::WindowsUtf16LeBase64Url,
        encoded: URL_SAFE_NO_PAD.encode(bytes),
        display: escaped_path_display(path),
    }
}

fn escaped_path_display(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .flat_map(char::escape_default)
        .collect()
}

fn enforce_limit(kind: &'static str, observed: u64, limit: u64) -> Result<(), SourceMiningError> {
    if observed > limit {
        Err(SourceMiningError::Limit {
            kind,
            limit,
            observed,
        })
    } else {
        Ok(())
    }
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), SourceMiningError> {
    let value = digest.as_str();
    let Some(hex) = value.strip_prefix("blake3:") else {
        return Err(SourceMiningError::InvalidDigest(value.to_owned()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SourceMiningError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn placeholder_digest(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

#[derive(Debug, Error)]
pub enum SourceMiningError {
    #[error("source root {path} cannot be resolved: {source}")]
    Root { path: PathBuf, source: io::Error },
    #[error("source root {0} is not a directory")]
    RootNotDirectory(PathBuf),
    #[error("source corpus must contain at least one explicit path")]
    EmptyCorpus,
    #[error("unsafe relative source path {0}")]
    UnsafePath(PathBuf),
    #[error("source path escapes the canonical root: {0}")]
    PathEscape(PathBuf),
    #[error("source path contains a symbolic link: {0}")]
    Symlink(PathBuf),
    #[error("source path is not a regular file: {0}")]
    NotRegularFile(PathBuf),
    #[error("source file {path} cannot be read: {source}")]
    File { path: PathBuf, source: io::Error },
    #[error("duplicate reversible source path {0}")]
    DuplicatePath(String),
    #[error("source changed while it was being bound or before retrieval: {0}")]
    SourceChanged(String),
    #[error("source binding limit {0} must be non-zero")]
    ZeroLimit(&'static str),
    #[error("{kind} limit exceeded: limit {limit}, observed {observed}")]
    Limit {
        kind: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("unsupported source corpus schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("source corpus provider does not match the built-in provider")]
    BindingProvider,
    #[error("source corpus binding shape is inconsistent")]
    BindingShape,
    #[error("source corpus is not in canonical path order")]
    NonCanonicalBinding,
    #[error("source corpus digest {declared} does not match recomputed {actual}")]
    BindingDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("invalid reversible source path identity")]
    InvalidPathIdentity,
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("mining request selected a different source provider")]
    RequestProvider,
    #[error("mining request input does not match the exact bound corpus")]
    RequestInput,
    #[error("mining request parameter {0} is unavailable")]
    Parameter(&'static str),
    #[error("frozen source classification does not match its bytes")]
    FrozenClassification,
    #[error("source match or context is not on a UTF-8 boundary")]
    InvalidTextBoundary,
    #[error("source search evidence binds a different corpus")]
    EvidenceBinding,
    #[error("source search matches are not canonical")]
    NonCanonicalMatches,
    #[error("source match context is missing")]
    MissingContext,
    #[error("source match context identity is invalid")]
    ContextDigest,
    #[error("source search evidence shape is inconsistent")]
    EvidenceShape,
    #[error("source search relation identity is invalid")]
    EvidenceDigest,
    #[error("source search produced no match relation")]
    NoMatchRelation,
    #[error("source mining counter overflow")]
    CountOverflow,
    #[error(transparent)]
    Mining(#[from] MiningError),
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    Polars(#[from] polars::error::PolarsError),
}
