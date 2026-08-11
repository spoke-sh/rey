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

pub const JOURNAL_PROPOSAL_SCHEMA: &str = "rey.journal-entry-proposal.v1";
pub const JOURNAL_ENTRY_SCHEMA: &str = "rey.journal-entry.v1";
pub const JOURNAL_LOG_SCHEMA: &str = "rey.journal-log.v1";
pub const JOURNAL_ADMISSION_SCHEMA: &str = "rey.journal-admission.v1";
pub const MAX_JOURNAL_ENTRIES: usize = 256;
pub const MAX_JOURNAL_BLOCKS: usize = 32;
pub const MAX_JOURNAL_STATE_BYTES: u64 = 8 * 1_024 * 1_024;
pub const MAX_JOURNAL_PROPOSAL_BYTES: u64 = 1_024 * 1_024;
const MAX_TITLE_CHARS: usize = 240;
const MAX_AUTHOR_CHARS: usize = 128;
const MAX_COORDINATE_BYTES: usize = 4_096;
const MAX_BLOCK_ID_CHARS: usize = 80;
const MAX_PROSE_NODES: usize = 128;
const MAX_PROSE_CHARS: usize = 64 * 1_024;
const MAX_QUERY_CHARS: usize = 32 * 1_024;
const MAX_PARAMETERS: usize = 64;
const MAX_FRAME_COLUMNS: usize = 64;
const MAX_FRAME_PREVIEW_ROWS: usize = 100;
const MAX_CELL_CHARS: usize = 4_096;
const MAX_REFERENCES: usize = 128;
const STATE_FILE_NAME: &str = "journal.json";
const LOCK_FILE_NAME: &str = "journal.lock";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalAuthorKind {
    Human,
    Agent,
    System,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalAuthor {
    pub kind: JournalAuthorKind,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalBinding {
    pub coordinate: String,
    pub scale: f64,
    pub source_revision: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalProseKind {
    Heading,
    Paragraph,
    Bullet,
    Quote,
    Code,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalProseNode {
    pub kind: JournalProseKind,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalFrameColumn {
    pub name: String,
    pub data_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalBlock {
    Prose {
        id: String,
        document: Vec<JournalProseNode>,
    },
    Explore {
        id: String,
        coordinate: String,
        scale: f64,
        source_revision: String,
        caption: Option<String>,
    },
    Query {
        id: String,
        language: String,
        provider: String,
        mode: String,
        statement: String,
        parameters: BTreeMap<String, String>,
    },
    Frame {
        id: String,
        source_block_id: String,
        snapshot_id: String,
        columns: Vec<JournalFrameColumn>,
        preview_rows: Vec<BTreeMap<String, Option<String>>>,
        row_count: u64,
        truncated: bool,
    },
    Diff {
        id: String,
        source: String,
        target: String,
        direction: String,
        assessment: String,
        summary: String,
    },
    Action {
        id: String,
        operation: String,
        desired_delta: String,
        evidence_ids: Vec<String>,
        dependency_ids: Vec<String>,
    },
}

impl JournalBlock {
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Prose { id, .. }
            | Self::Explore { id, .. }
            | Self::Query { id, .. }
            | Self::Frame { id, .. }
            | Self::Diff { id, .. }
            | Self::Action { id, .. } => id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntryProposal {
    pub schema: String,
    pub title: String,
    pub author: JournalAuthor,
    pub binding: JournalBinding,
    #[serde(default)]
    pub supersedes: Option<SemanticDigest>,
    pub blocks: Vec<JournalBlock>,
}

impl JournalEntryProposal {
    pub fn validate(&self) -> Result<(), JournalError> {
        if self.schema != JOURNAL_PROPOSAL_SCHEMA {
            return Err(JournalError::Schema {
                expected: JOURNAL_PROPOSAL_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        validate_text("title", &self.title, 1, MAX_TITLE_CHARS)?;
        validate_identifier("author id", &self.author.id, MAX_AUTHOR_CHARS)?;
        validate_binding(&self.binding)?;
        if self.blocks.is_empty() || self.blocks.len() > MAX_JOURNAL_BLOCKS {
            return Err(JournalError::BlockLimit {
                actual: self.blocks.len(),
                limit: MAX_JOURNAL_BLOCKS,
            });
        }
        let mut block_ids = BTreeSet::new();
        for block in &self.blocks {
            validate_identifier("block id", block.id(), MAX_BLOCK_ID_CHARS)?;
            if !block_ids.insert(block.id()) {
                return Err(JournalError::DuplicateBlock(block.id().to_owned()));
            }
            validate_block(block)?;
        }
        for (index, block) in self.blocks.iter().enumerate() {
            if let JournalBlock::Frame {
                source_block_id, ..
            } = block
            {
                let source_index = self
                    .blocks
                    .iter()
                    .position(|candidate| candidate.id() == source_block_id)
                    .ok_or_else(|| JournalError::MissingBlock(source_block_id.clone()))?;
                if source_index >= index
                    || !matches!(&self.blocks[source_index], JournalBlock::Query { .. })
                {
                    return Err(JournalError::FrameSource(source_block_id.clone()));
                }
            }
        }
        Ok(())
    }

    fn identity(&self) -> Result<SemanticDigest, JournalError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        let mut hasher = SemanticHasher::new(JOURNAL_ENTRY_SCHEMA);
        hasher.add_bytes(&bytes);
        Ok(hasher.finish())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    pub schema: String,
    pub entry_id: SemanticDigest,
    pub sequence: u64,
    pub admitted_at: String,
    pub title: String,
    pub author: JournalAuthor,
    pub binding: JournalBinding,
    pub supersedes: Option<SemanticDigest>,
    pub blocks: Vec<JournalBlock>,
}

impl JournalEntry {
    #[must_use]
    pub fn slug(&self) -> String {
        let title = ascii_slug_component(&self.title, Some(80));
        let identity = ascii_slug_component(self.entry_id.as_str(), None);
        format!(
            "j{}-{}--{}",
            self.sequence,
            if title.is_empty() { "entry" } else { &title },
            identity
        )
    }

    fn from_proposal(
        proposal: JournalEntryProposal,
        sequence: u64,
        admitted_at: &str,
    ) -> Result<Self, JournalError> {
        DateTime::parse_from_rfc3339(admitted_at)
            .map_err(|_| JournalError::Timestamp(admitted_at.to_owned()))?;
        let entry_id = proposal.identity()?;
        Ok(Self {
            schema: JOURNAL_ENTRY_SCHEMA.to_owned(),
            entry_id,
            sequence,
            admitted_at: admitted_at.to_owned(),
            title: proposal.title,
            author: proposal.author,
            binding: proposal.binding,
            supersedes: proposal.supersedes,
            blocks: proposal.blocks,
        })
    }

    fn proposal(&self) -> JournalEntryProposal {
        JournalEntryProposal {
            schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
            title: self.title.clone(),
            author: self.author.clone(),
            binding: self.binding.clone(),
            supersedes: self.supersedes.clone(),
            blocks: self.blocks.clone(),
        }
    }

    fn verify(&self, expected_sequence: u64) -> Result<(), JournalError> {
        if self.schema != JOURNAL_ENTRY_SCHEMA {
            return Err(JournalError::Schema {
                expected: JOURNAL_ENTRY_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.sequence != expected_sequence {
            return Err(JournalError::Sequence {
                expected: expected_sequence,
                actual: self.sequence,
            });
        }
        DateTime::parse_from_rfc3339(&self.admitted_at)
            .map_err(|_| JournalError::Timestamp(self.admitted_at.clone()))?;
        let actual = self.proposal().identity()?;
        if actual != self.entry_id {
            return Err(JournalError::Identity {
                expected: self.entry_id.clone(),
                actual,
            });
        }
        Ok(())
    }
}

fn ascii_slug_component(value: &str, limit: Option<usize>) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        } else {
            separator = true;
        }
        if limit.is_some_and(|limit| slug.len() >= limit) {
            break;
        }
    }
    slug.truncate(limit.unwrap_or(slug.len()).min(slug.len()));
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalLog {
    pub schema: String,
    pub log_id: SemanticDigest,
    pub entries: Vec<JournalEntry>,
}

impl Default for JournalLog {
    fn default() -> Self {
        let mut log = Self {
            schema: JOURNAL_LOG_SCHEMA.to_owned(),
            log_id: SemanticHasher::new(JOURNAL_LOG_SCHEMA).finish(),
            entries: Vec::new(),
        };
        log.log_id = log.identity();
        log
    }
}

impl JournalLog {
    pub fn verify(&self) -> Result<(), JournalError> {
        if self.schema != JOURNAL_LOG_SCHEMA {
            return Err(JournalError::Schema {
                expected: JOURNAL_LOG_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        if self.entries.len() > MAX_JOURNAL_ENTRIES {
            return Err(JournalError::EntryLimit(MAX_JOURNAL_ENTRIES));
        }
        let mut ids = BTreeSet::new();
        for (index, entry) in self.entries.iter().enumerate() {
            entry.verify(index as u64 + 1)?;
            if !ids.insert(entry.entry_id.as_str()) {
                return Err(JournalError::DuplicateEntry(entry.entry_id.clone()));
            }
            if let Some(supersedes) = &entry.supersedes
                && !ids.contains(supersedes.as_str())
            {
                return Err(JournalError::MissingSuperseded(supersedes.clone()));
            }
        }
        let actual = self.identity();
        if actual != self.log_id {
            return Err(JournalError::LogIdentity {
                expected: self.log_id.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn admit(
        &mut self,
        proposal: JournalEntryProposal,
        admitted_at: &str,
    ) -> Result<(JournalEntry, bool), JournalError> {
        self.verify()?;
        let entry_id = proposal.identity()?;
        if let Some(existing) = self.entries.iter().find(|entry| entry.entry_id == entry_id) {
            return Ok((existing.clone(), false));
        }
        if self.entries.len() >= MAX_JOURNAL_ENTRIES {
            return Err(JournalError::EntryLimit(MAX_JOURNAL_ENTRIES));
        }
        if let Some(supersedes) = &proposal.supersedes
            && !self
                .entries
                .iter()
                .any(|entry| &entry.entry_id == supersedes)
        {
            return Err(JournalError::MissingSuperseded(supersedes.clone()));
        }
        let entry =
            JournalEntry::from_proposal(proposal, self.entries.len() as u64 + 1, admitted_at)?;
        self.entries.push(entry.clone());
        self.log_id = self.identity();
        self.verify()?;
        Ok((entry, true))
    }

    fn identity(&self) -> SemanticDigest {
        let mut hasher = SemanticHasher::new(JOURNAL_LOG_SCHEMA);
        hasher.add_u64(self.entries.len() as u64);
        for entry in &self.entries {
            hasher.add_str(entry.entry_id.as_str());
            hasher.add_u64(entry.sequence);
            hasher.add_str(&entry.admitted_at);
        }
        hasher.finish()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JournalAdmission {
    pub schema: String,
    pub admitted: bool,
    pub entry: JournalEntry,
    pub log: JournalLog,
}

#[derive(Clone, Debug)]
pub struct LocalJournalStore {
    directory: PathBuf,
}

impl LocalJournalStore {
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

    pub fn load(&self) -> Result<JournalLog, JournalError> {
        self.verify_directory_boundary()?;
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(JournalLog::default());
            }
            Err(source) => return Err(JournalError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(JournalError::UnsafePath(path));
        }
        if metadata.len() > MAX_JOURNAL_STATE_BYTES {
            return Err(JournalError::ByteLimit(MAX_JOURNAL_STATE_BYTES));
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| JournalError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_JOURNAL_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| JournalError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_JOURNAL_STATE_BYTES {
            return Err(JournalError::ByteLimit(MAX_JOURNAL_STATE_BYTES));
        }
        let log: JournalLog = serde_json::from_slice(&bytes)?;
        log.verify()?;
        Ok(log)
    }

    pub fn admit(
        &self,
        proposal: JournalEntryProposal,
        admitted_at: &str,
    ) -> Result<JournalAdmission, JournalError> {
        self.prepare_directory()?;
        let lock_path = self.directory.join(LOCK_FILE_NAME);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(JournalError::UnsafePath(lock_path));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| JournalError::Write {
                path: lock_path.clone(),
                source,
            })?;
        File::lock(&lock).map_err(|source| JournalError::Lock {
            path: lock_path.clone(),
            source,
        })?;
        let result = (|| {
            let mut log = self.load()?;
            let (entry, admitted) = log.admit(proposal, admitted_at)?;
            if admitted {
                self.save(&log)?;
            }
            Ok(JournalAdmission {
                schema: JOURNAL_ADMISSION_SCHEMA.to_owned(),
                admitted,
                entry,
                log,
            })
        })();
        let unlock = File::unlock(&lock).map_err(|source| JournalError::Lock {
            path: lock_path,
            source,
        });
        match (result, unlock) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn save(&self, log: &JournalLog) -> Result<(), JournalError> {
        log.verify()?;
        let bytes = serde_json::to_vec_pretty(log)?;
        if bytes.len().saturating_add(1) as u64 > MAX_JOURNAL_STATE_BYTES {
            return Err(JournalError::ByteLimit(MAX_JOURNAL_STATE_BYTES));
        }
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(JournalError::UnsafePath(target));
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
            return Err(JournalError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), JournalError> {
        self.verify_directory_boundary()?;
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                Err(JournalError::UnsafePath(self.directory.clone()))
            }
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| JournalError::Write {
                    path: self.directory.clone(),
                    source,
                })
            }
            Err(source) => Err(JournalError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn verify_directory_boundary(&self) -> Result<(), JournalError> {
        for ancestor in self.directory.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(JournalError::UnsafePath(ancestor.to_owned()));
                }
                Ok(metadata) if ancestor == self.directory && !metadata.is_dir() => {
                    return Err(JournalError::UnsafePath(ancestor.to_owned()));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(JournalError::Read {
                        path: ancestor.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), JournalError> {
        for attempt in 0..32_u8 {
            let path = self.directory.join(format!(
                ".{STATE_FILE_NAME}.tmp-{}-{attempt}",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(JournalError::Write { path, source }),
            }
        }
        Err(JournalError::TemporaryLimit(self.directory.clone()))
    }
}

fn validate_block(block: &JournalBlock) -> Result<(), JournalError> {
    match block {
        JournalBlock::Prose { document, .. } => {
            if document.is_empty() || document.len() > MAX_PROSE_NODES {
                return Err(JournalError::ProseNodeLimit {
                    actual: document.len(),
                    limit: MAX_PROSE_NODES,
                });
            }
            let mut total = 0_usize;
            for node in document {
                validate_text("prose node", &node.text, 1, MAX_PROSE_CHARS)?;
                total = total.saturating_add(node.text.chars().count());
            }
            if total > MAX_PROSE_CHARS {
                return Err(JournalError::TextLimit {
                    field: "prose document",
                    actual: total,
                    limit: MAX_PROSE_CHARS,
                });
            }
        }
        JournalBlock::Explore {
            coordinate,
            scale,
            source_revision,
            caption,
            ..
        } => {
            validate_binding(&JournalBinding {
                coordinate: coordinate.clone(),
                scale: *scale,
                source_revision: source_revision.clone(),
            })?;
            if let Some(caption) = caption {
                validate_text("explore caption", caption, 1, MAX_TITLE_CHARS)?;
            }
        }
        JournalBlock::Query {
            language,
            provider,
            mode,
            statement,
            parameters,
            ..
        } => {
            validate_identifier("query language", language, 32)?;
            validate_identifier("query provider", provider, 128)?;
            if mode != "read_only" {
                return Err(JournalError::QueryMode(mode.clone()));
            }
            validate_text("query statement", statement, 1, MAX_QUERY_CHARS)?;
            if parameters.len() > MAX_PARAMETERS {
                return Err(JournalError::ReferenceLimit {
                    field: "query parameters",
                    actual: parameters.len(),
                    limit: MAX_PARAMETERS,
                });
            }
            for (name, value) in parameters {
                validate_identifier("query parameter", name, 128)?;
                validate_text("query parameter value", value, 0, MAX_CELL_CHARS)?;
            }
        }
        JournalBlock::Frame {
            source_block_id,
            snapshot_id,
            columns,
            preview_rows,
            row_count,
            truncated,
            ..
        } => {
            validate_identifier("frame source block", source_block_id, MAX_BLOCK_ID_CHARS)?;
            validate_reference("frame snapshot", snapshot_id)?;
            if columns.is_empty() || columns.len() > MAX_FRAME_COLUMNS {
                return Err(JournalError::ReferenceLimit {
                    field: "frame columns",
                    actual: columns.len(),
                    limit: MAX_FRAME_COLUMNS,
                });
            }
            let mut names = BTreeSet::new();
            for column in columns {
                validate_identifier("frame column", &column.name, 128)?;
                validate_identifier("frame data type", &column.data_type, 64)?;
                if !names.insert(column.name.as_str()) {
                    return Err(JournalError::DuplicateColumn(column.name.clone()));
                }
            }
            if preview_rows.len() > MAX_FRAME_PREVIEW_ROWS {
                return Err(JournalError::ReferenceLimit {
                    field: "frame preview rows",
                    actual: preview_rows.len(),
                    limit: MAX_FRAME_PREVIEW_ROWS,
                });
            }
            if *row_count < preview_rows.len() as u64
                || (*row_count > preview_rows.len() as u64 && !truncated)
            {
                return Err(JournalError::FrameCompleteness);
            }
            for row in preview_rows {
                if row.keys().any(|name| !names.contains(name.as_str())) {
                    return Err(JournalError::UnknownFrameColumn);
                }
                for value in row.values().flatten() {
                    validate_text("frame cell", value, 0, MAX_CELL_CHARS)?;
                }
            }
        }
        JournalBlock::Diff {
            source,
            target,
            direction,
            assessment,
            summary,
            ..
        } => {
            validate_reference("diff source", source)?;
            validate_reference("diff target", target)?;
            validate_identifier("diff direction", direction, 128)?;
            if !matches!(assessment.as_str(), "equal" | "different" | "inconclusive") {
                return Err(JournalError::DiffAssessment(assessment.clone()));
            }
            validate_text("diff summary", summary, 1, MAX_PROSE_CHARS)?;
        }
        JournalBlock::Action {
            operation,
            desired_delta,
            evidence_ids,
            dependency_ids,
            ..
        } => {
            validate_identifier("action operation", operation, 128)?;
            validate_text("desired delta", desired_delta, 1, MAX_PROSE_CHARS)?;
            validate_references("action evidence", evidence_ids)?;
            validate_references("action dependencies", dependency_ids)?;
        }
    }
    Ok(())
}

fn validate_binding(binding: &JournalBinding) -> Result<(), JournalError> {
    if binding.coordinate.len() > MAX_COORDINATE_BYTES {
        return Err(JournalError::CoordinateLimit(MAX_COORDINATE_BYTES));
    }
    validate_reference("source revision", &binding.source_revision)?;
    validate_explorer_scale(binding.scale)?;
    let coordinate = parse_canonical_coordinate(&binding.coordinate)?;
    if coordinate.revision != binding.source_revision {
        return Err(JournalError::CoordinateRevision {
            coordinate: coordinate.revision,
            binding: binding.source_revision.clone(),
        });
    }
    Ok(())
}

struct ParsedCoordinate {
    revision: String,
}

fn parse_canonical_coordinate(value: &str) -> Result<ParsedCoordinate, JournalError> {
    let local = value
        .strip_prefix("rey+local://")
        .ok_or_else(|| JournalError::Coordinate(value.to_owned()))?;
    let (address, dimensions) = local
        .split_once('?')
        .ok_or_else(|| JournalError::Coordinate(value.to_owned()))?;
    let (kind, encoded_identity) = address
        .split_once('/')
        .ok_or_else(|| JournalError::Coordinate(value.to_owned()))?;
    if !matches!(
        kind,
        "agent" | "attention" | "cluster" | "portfolio" | "workload"
    ) {
        return Err(JournalError::Coordinate(value.to_owned()));
    }
    let identity = percent_decode(encoded_identity)?;
    if identity.is_empty() {
        return Err(JournalError::Coordinate(value.to_owned()));
    }

    let mut dimensions_by_name = BTreeMap::new();
    for part in dimensions.split('&') {
        let (key, encoded) = part
            .split_once('=')
            .ok_or_else(|| JournalError::Coordinate(value.to_owned()))?;
        let key = percent_decode(key)?;
        let decoded = percent_decode(encoded)?;
        if decoded.is_empty()
            || !matches!(key.as_str(), "revision" | "role")
            || dimensions_by_name.insert(key, decoded).is_some()
        {
            return Err(JournalError::Coordinate(value.to_owned()));
        }
    }
    let revision = dimensions_by_name
        .get("revision")
        .ok_or_else(|| JournalError::Coordinate(value.to_owned()))?
        .clone();
    let role = dimensions_by_name.get("role");
    if (kind == "agent") != role.is_some()
        || role.is_some_and(|role| !matches!(role.as_str(), "coding_harness" | "human" | "rule"))
    {
        return Err(JournalError::Coordinate(value.to_owned()));
    }
    let canonical_dimensions = dimensions_by_name
        .iter()
        .map(|(key, value)| format!("{}={}", percent_encode(key), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let canonical = format!(
        "rey+local://{kind}/{}?{}",
        percent_encode(&identity),
        canonical_dimensions
    );
    if canonical != value {
        return Err(JournalError::NonCanonicalCoordinate {
            actual: value.to_owned(),
            canonical,
        });
    }
    Ok(ParsedCoordinate { revision })
}

fn validate_explorer_scale(scale: f64) -> Result<(), JournalError> {
    if scale.is_finite() && (0.05..=5.4).contains(&scale) {
        Ok(())
    } else {
        Err(JournalError::ExplorerScale(scale))
    }
}

fn percent_decode(value: &str) -> Result<String, JournalError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(JournalError::PercentEncoding(value.to_owned()));
            }
            let high = hex(bytes[index + 1])?;
            let low = hex(bytes[index + 2])?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| JournalError::PercentEncoding(value.to_owned()))
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

fn hex(value: u8) -> Result<u8, JournalError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(JournalError::PercentEncoding(char::from(value).to_string())),
    }
}

fn validate_references(field: &'static str, values: &[String]) -> Result<(), JournalError> {
    if values.len() > MAX_REFERENCES {
        return Err(JournalError::ReferenceLimit {
            field,
            actual: values.len(),
            limit: MAX_REFERENCES,
        });
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_reference(field, value)?;
        if !unique.insert(value) {
            return Err(JournalError::DuplicateReference {
                field,
                value: value.clone(),
            });
        }
    }
    Ok(())
}

fn validate_reference(field: &'static str, value: &str) -> Result<(), JournalError> {
    validate_text(field, value, 1, MAX_COORDINATE_BYTES)
}

fn validate_identifier(field: &'static str, value: &str, limit: usize) -> Result<(), JournalError> {
    validate_text(field, value, 1, limit)?;
    if value.bytes().any(|byte| {
        !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/'))
    }) {
        return Err(JournalError::Identifier {
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
    minimum: usize,
    limit: usize,
) -> Result<(), JournalError> {
    let count = value.chars().count();
    if count < minimum || count > limit || value.contains('\0') {
        return Err(JournalError::TextLimit {
            field,
            actual: count,
            limit,
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal schema must be {expected}, got {actual}")]
    Schema {
        expected: &'static str,
        actual: String,
    },
    #[error("journal {field} contains {actual} characters; limit is {limit}")]
    TextLimit {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("journal {field} is not a safe identifier: {value}")]
    Identifier { field: &'static str, value: String },
    #[error("journal block count {actual} is outside 1..={limit}")]
    BlockLimit { actual: usize, limit: usize },
    #[error("journal entry limit {0} reached")]
    EntryLimit(usize),
    #[error("duplicate journal block id {0}")]
    DuplicateBlock(String),
    #[error("duplicate journal entry {0}")]
    DuplicateEntry(SemanticDigest),
    #[error("journal block references missing block {0}")]
    MissingBlock(String),
    #[error("journal frame source must name an earlier query block: {0}")]
    FrameSource(String),
    #[error("journal entry supersedes missing entry {0}")]
    MissingSuperseded(SemanticDigest),
    #[error("journal prose node count {actual} is outside 1..={limit}")]
    ProseNodeLimit { actual: usize, limit: usize },
    #[error("journal reference count for {field} is {actual}; limit is {limit}")]
    ReferenceLimit {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("duplicate journal reference in {field}: {value}")]
    DuplicateReference { field: &'static str, value: String },
    #[error("journal query mode must be read_only, got {0}")]
    QueryMode(String),
    #[error("duplicate journal frame column {0}")]
    DuplicateColumn(String),
    #[error("journal frame preview references an unknown column")]
    UnknownFrameColumn,
    #[error("journal frame row_count/truncated metadata does not bound its preview")]
    FrameCompleteness,
    #[error("journal diff assessment must be equal, different, or inconclusive, got {0}")]
    DiffAssessment(String),
    #[error("journal semantic coordinate exceeds {0} bytes")]
    CoordinateLimit(usize),
    #[error("invalid journal semantic coordinate {0}")]
    Coordinate(String),
    #[error("non-canonical journal semantic coordinate {actual}; canonical form is {canonical}")]
    NonCanonicalCoordinate { actual: String, canonical: String },
    #[error("invalid percent encoding in journal coordinate: {0}")]
    PercentEncoding(String),
    #[error("journal coordinate revision {coordinate} does not match binding {binding}")]
    CoordinateRevision { coordinate: String, binding: String },
    #[error("journal Explorer scale must be finite within 0.05..=5.4, got {0}")]
    ExplorerScale(f64),
    #[error("journal timestamp is not RFC 3339: {0}")]
    Timestamp(String),
    #[error("journal entry sequence must be {expected}, got {actual}")]
    Sequence { expected: u64, actual: u64 },
    #[error("journal entry identity mismatch: expected {expected}, recomputed {actual}")]
    Identity {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("journal log identity mismatch: expected {expected}, recomputed {actual}")]
    LogIdentity {
        expected: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("journal state exceeds {0} bytes")]
    ByteLimit(u64),
    #[error("unsafe symlink or file type in journal state path {0}")]
    UnsafePath(PathBuf),
    #[error("could not read journal state {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not write journal state {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not lock journal state {path}: {source}")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not allocate a journal temporary file in {0}")]
    TemporaryLimit(PathBuf),
    #[error("journal JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use tempfile::TempDir;

    use super::{
        JOURNAL_PROPOSAL_SCHEMA, JournalAuthor, JournalAuthorKind, JournalBinding, JournalBlock,
        JournalEntryProposal, JournalProseKind, JournalProseNode, LocalJournalStore,
    };

    fn proposal() -> JournalEntryProposal {
        JournalEntryProposal {
            schema: JOURNAL_PROPOSAL_SCHEMA.to_owned(),
            title: "Inspect source coverage".to_owned(),
            author: JournalAuthor {
                kind: JournalAuthorKind::Agent,
                id: "codex".to_owned(),
            },
            binding: JournalBinding {
                coordinate: "rey+local://workload/source-mining?revision=blake3%3Aabc".to_owned(),
                scale: 1.46,
                source_revision: "blake3:abc".to_owned(),
            },
            supersedes: None,
            blocks: vec![
                JournalBlock::Prose {
                    id: "context".to_owned(),
                    document: vec![JournalProseNode {
                        kind: JournalProseKind::Paragraph,
                        text: "Coverage moved after the latest survey.".to_owned(),
                    }],
                },
                JournalBlock::Query {
                    id: "query".to_owned(),
                    language: "sql".to_owned(),
                    provider: "spoke".to_owned(),
                    mode: "read_only".to_owned(),
                    statement: "select * from coverage".to_owned(),
                    parameters: BTreeMap::new(),
                },
            ],
        }
    }

    #[test]
    fn journal_admission_is_content_identified_ordered_and_idempotent() {
        let directory = TempDir::new().unwrap();
        let store = LocalJournalStore::new(directory.path().join("journal"));
        let first = store.admit(proposal(), "2026-08-10T20:00:00Z").unwrap();
        assert!(first.admitted);
        assert_eq!(first.entry.sequence, 1);
        assert_eq!(
            first.entry.slug(),
            format!(
                "j1-inspect-source-coverage--{}",
                first.entry.entry_id.as_str().replace(':', "-")
            )
        );
        assert_eq!(first.log.entries.len(), 1);
        let repeated = store.admit(proposal(), "2026-08-10T20:01:00Z").unwrap();
        assert!(!repeated.admitted);
        assert_eq!(repeated.entry.entry_id, first.entry.entry_id);
        assert_eq!(repeated.log.entries.len(), 1);
        assert_eq!(store.load().unwrap(), repeated.log);
    }

    #[test]
    fn query_is_inert_and_only_read_only_mode_is_admitted() {
        let mut proposal = proposal();
        if let JournalBlock::Query { mode, .. } = &mut proposal.blocks[1] {
            *mode = "execute".to_owned();
        }
        assert!(
            proposal
                .validate()
                .unwrap_err()
                .to_string()
                .contains("read_only")
        );
    }

    #[test]
    fn frame_diff_and_action_blocks_preserve_bounds_and_explicit_nulls() {
        let mut proposal = proposal();
        proposal.blocks.extend([
            JournalBlock::Frame {
                id: "frame".to_owned(),
                source_block_id: "query".to_owned(),
                snapshot_id: "blake3:frame".to_owned(),
                columns: vec![super::JournalFrameColumn {
                    name: "surface".to_owned(),
                    data_type: "Utf8".to_owned(),
                }],
                preview_rows: vec![BTreeMap::from([("surface".to_owned(), None)])],
                row_count: 1,
                truncated: false,
            },
            JournalBlock::Diff {
                id: "diff".to_owned(),
                source: "frame://before".to_owned(),
                target: "frame://after".to_owned(),
                direction: "expected_to_observed".to_owned(),
                assessment: "different".to_owned(),
                summary: "One row remains.".to_owned(),
            },
            JournalBlock::Action {
                id: "action".to_owned(),
                operation: "refine".to_owned(),
                desired_delta: "Reduce remaining rows to zero.".to_owned(),
                evidence_ids: vec!["blake3:frame".to_owned()],
                dependency_ids: Vec::new(),
            },
        ]);
        proposal.validate().unwrap();
        let replayed: JournalEntryProposal =
            serde_json::from_slice(&serde_json::to_vec(&proposal).unwrap()).unwrap();
        assert_eq!(replayed, proposal);

        if let JournalBlock::Frame { row_count, .. } = &mut proposal.blocks[2] {
            *row_count = 2;
        }
        assert!(
            proposal
                .validate()
                .unwrap_err()
                .to_string()
                .contains("row_count/truncated")
        );
        if let JournalBlock::Frame {
            row_count,
            source_block_id,
            ..
        } = &mut proposal.blocks[2]
        {
            *row_count = 1;
            *source_block_id = "context".to_owned();
        }
        assert!(
            proposal
                .validate()
                .unwrap_err()
                .to_string()
                .contains("earlier query block")
        );
    }

    #[test]
    fn coordinate_is_canonical_and_revision_bound() {
        let mut candidate = proposal();
        candidate.binding.source_revision = "blake3:other".to_owned();
        assert!(
            candidate
                .validate()
                .unwrap_err()
                .to_string()
                .contains("does not match")
        );
        candidate.binding.source_revision = "blake3:abc".to_owned();
        candidate.binding.coordinate =
            "rey+local://agent/codex?role=coding_harness&revision=blake3%3Aabc".to_owned();
        assert!(
            candidate
                .validate()
                .unwrap_err()
                .to_string()
                .contains("canonical")
        );

        let mut unsupported = proposal();
        unsupported.schema = "rey.journal-entry-proposal.unsupported".to_owned();
        assert!(
            unsupported
                .validate()
                .unwrap_err()
                .to_string()
                .contains("schema")
        );
        unsupported.schema = JOURNAL_PROPOSAL_SCHEMA.to_owned();
        unsupported.binding.coordinate =
            "/explore/workload/source-mining;at=blake3%3Aabc;lens=objects".to_owned();
        assert!(
            unsupported
                .validate()
                .unwrap_err()
                .to_string()
                .contains("semantic coordinate")
        );

        let mut invalid_scale = proposal();
        invalid_scale.binding.scale = 0.04;
        assert!(
            invalid_scale
                .validate()
                .unwrap_err()
                .to_string()
                .contains("scale")
        );
        for scale in [0.05, 5.4] {
            let mut boundary = proposal();
            boundary.binding.scale = scale;
            boundary.validate().unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_journal_state_fails_closed() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let journal = directory.path().join("journal");
        fs::create_dir(&journal).unwrap();
        let target = directory.path().join("outside");
        fs::write(&target, "outside").unwrap();
        symlink(&target, journal.join("journal.json")).unwrap();
        assert!(LocalJournalStore::new(journal).load().is_err());
    }
}
