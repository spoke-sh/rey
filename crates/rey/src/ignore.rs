use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rey_core::{SemanticDigest, SemanticHasher};
use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DISCOVERY_APPLICATION_SCHEMA,
    DISCOVERY_SEED_PROVIDER_ID, DiscoveryApplicationProvenance, DiscoveryError,
    DiscoverySeedProvenance, EnvironmentMapEdge, EnvironmentMapNode, EnvironmentMapNodeProvenance,
    TrustClass,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const REY_IGNORE_FILE_NAME: &str = ".reyignore";
pub const REY_IGNORE_SCHEMA: &str = "rey.ignore.v1";
pub const REY_IGNORE_PROVIDER_ID: &str = "rey.ignore";
const MAX_IGNORE_BYTES: u64 = 64 * 1_024;
const MAX_IGNORE_RULES: usize = 256;
const MAX_IGNORE_LINE_BYTES: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReyIgnoreRule {
    pub kind: String,
    pub pattern: String,
    pub source_line: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReyIgnoreFile {
    pub schema: String,
    pub source: String,
    pub source_digest: SemanticDigest,
    pub rules: Vec<ReyIgnoreRule>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReyIgnoreOmission {
    pub rule: ReyIgnoreRule,
    pub matched: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReyIgnoreProjection {
    pub schema: String,
    pub source: String,
    pub source_digest: SemanticDigest,
    pub rules: Vec<ReyIgnoreRule>,
    pub omissions: Vec<ReyIgnoreOmission>,
    pub ignored: u64,
}

impl ReyIgnoreFile {
    pub fn load(workspace: &Path) -> Result<Option<Self>, ReyIgnoreError> {
        let path = workspace.join(REY_IGNORE_FILE_NAME);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(ReyIgnoreError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ReyIgnoreError::UnsafePath(path));
        }
        if metadata.len() > MAX_IGNORE_BYTES {
            return Err(ReyIgnoreError::ByteLimit {
                path,
                limit: MAX_IGNORE_BYTES,
                actual: metadata.len(),
            });
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| ReyIgnoreError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_IGNORE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| ReyIgnoreError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_IGNORE_BYTES {
            return Err(ReyIgnoreError::ByteLimit {
                path,
                limit: MAX_IGNORE_BYTES,
                actual: bytes.len() as u64,
            });
        }
        let source_digest = source_digest(&bytes);
        let text = std::str::from_utf8(&bytes).map_err(|source| ReyIgnoreError::Encoding {
            path: path.clone(),
            source,
        })?;
        let mut rules = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let source_line = index as u64 + 1;
            if raw.len() > MAX_IGNORE_LINE_BYTES {
                return Err(ReyIgnoreError::LineLimit {
                    line: source_line,
                    limit: MAX_IGNORE_LINE_BYTES,
                });
            }
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if rules.len() >= MAX_IGNORE_RULES {
                return Err(ReyIgnoreError::RuleLimit(MAX_IGNORE_RULES));
            }
            let (kind, pattern) = line
                .split_once(':')
                .ok_or(ReyIgnoreError::InvalidRule { line: source_line })?;
            let kind = kind.trim();
            let pattern = pattern.trim();
            if kind.is_empty()
                || pattern.is_empty()
                || kind.contains('*')
                || kind.contains('?')
                || kind.chars().any(char::is_control)
                || pattern.chars().any(char::is_control)
            {
                return Err(ReyIgnoreError::InvalidRule { line: source_line });
            }
            rules.push(ReyIgnoreRule {
                kind: kind.to_owned(),
                pattern: pattern.to_owned(),
                source_line,
            });
        }
        Ok(Some(Self {
            schema: REY_IGNORE_SCHEMA.to_owned(),
            source: REY_IGNORE_FILE_NAME.to_owned(),
            source_digest,
            rules,
        }))
    }

    #[must_use]
    pub fn matches(&self, kind: &str, name: &str) -> bool {
        self.rules
            .iter()
            .any(|rule| rule.kind == kind && wildcard_match(&rule.pattern, name))
    }

    #[must_use]
    pub fn project(&self, candidates: &[(&str, &str)], kinds: &[&str]) -> ReyIgnoreProjection {
        let rules = self
            .rules
            .iter()
            .filter(|rule| kinds.contains(&rule.kind.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        let omissions = self
            .rules
            .iter()
            .filter(|rule| kinds.contains(&rule.kind.as_str()))
            .cloned()
            .map(|rule| {
                let matched = candidates
                    .iter()
                    .filter(|(kind, name)| {
                        *kind == rule.kind && wildcard_match(&rule.pattern, name)
                    })
                    .count() as u64;
                ReyIgnoreOmission { rule, matched }
            })
            .collect::<Vec<_>>();
        let ignored = candidates
            .iter()
            .filter(|(kind, name)| {
                omissions.iter().any(|omission| {
                    omission.rule.kind == *kind && wildcard_match(&omission.rule.pattern, name)
                })
            })
            .count() as u64;
        ReyIgnoreProjection {
            schema: REY_IGNORE_SCHEMA.to_owned(),
            source: self.source.clone(),
            source_digest: self.source_digest.clone(),
            rules,
            ignored,
            omissions,
        }
    }
}

pub fn apply_environment_ignore(
    snapshot: CapabilitySnapshot,
    ignore: &ReyIgnoreFile,
) -> Result<(CapabilitySnapshot, ReyIgnoreProjection), ReyIgnoreError> {
    let candidates = snapshot
        .capabilities
        .iter()
        .filter_map(|record| environment_selector(record).transpose())
        .collect::<Result<Vec<_>, _>>()?;
    let candidate_refs = candidates
        .iter()
        .map(|(kind, name)| (*kind, name.as_str()))
        .collect::<Vec<_>>();
    let projection = ignore.project(
        &candidate_refs,
        &["environment variable", "application", "input", "reference"],
    );
    if projection.rules.is_empty() {
        return Ok((snapshot, projection));
    }
    let mut capabilities = Vec::with_capacity(snapshot.capabilities.len());
    for capability in snapshot.capabilities {
        let ignored = environment_selector(&capability)?
            .is_some_and(|(kind, name)| ignore.matches(kind, &name));
        if !ignored {
            capabilities.push(capability);
        }
    }
    // A relevant zero-match rule is still an explicit WORKING-scope policy.
    // Retain it so edits to that policy produce semantic drift even when the
    // currently observed object set happens to be unchanged.
    if !projection.rules.is_empty() {
        capabilities.push(ignore_environment_capability(&projection)?);
    }
    let snapshot = CapabilitySnapshot::new(snapshot.profile, snapshot.limits, capabilities)?;
    Ok((snapshot, projection))
}

pub fn retained_environment_ignore(
    snapshot: &CapabilitySnapshot,
) -> Result<Option<ReyIgnoreProjection>, ReyIgnoreError> {
    snapshot
        .capabilities
        .iter()
        .find(|record| {
            record.provider_id == REY_IGNORE_PROVIDER_ID
                && record.capability_kind == "ignore_policy"
        })
        .map(parse_provenance)
        .transpose()
}

fn ignore_environment_capability(
    projection: &ReyIgnoreProjection,
) -> Result<CapabilityRecord, ReyIgnoreError> {
    Ok(CapabilityRecord {
        provider_id: REY_IGNORE_PROVIDER_ID.to_owned(),
        provider_revision: 1,
        provider_kind: "workspace_policy".to_owned(),
        capability_id: "rey.ignore.environment".to_owned(),
        capability_kind: "ignore_policy".to_owned(),
        resolved_location: Some(projection.source.clone()),
        version: Some(REY_IGNORE_SCHEMA.to_owned()),
        content_digest: Some(projection.source_digest.to_string()),
        provenance: Some(serde_json::to_string(projection)?),
        availability: Availability::Available,
        trust_class: TrustClass::ExplicitLocal,
        operations: vec!["filter_working_observation".to_owned()],
        enforced_limits: vec![
            "typed_resource_kind".to_owned(),
            "bounded_wildcard".to_owned(),
            "workspace_root_file".to_owned(),
        ],
        unsupported_limits: Vec::new(),
        observed_at: None,
        error_code: None,
        error_detail: None,
    })
}

pub fn environment_selector(
    record: &CapabilityRecord,
) -> Result<Option<(&'static str, String)>, ReyIgnoreError> {
    if record.provider_id == DISCOVERY_SEED_PROVIDER_ID
        && record.capability_kind == "environment_seed"
    {
        let provenance: DiscoverySeedProvenance = parse_provenance(record)?;
        return Ok(Some(("environment variable", provenance.name)));
    }
    if record.provider_kind == "known_tool" && record.capability_kind == "identity_probe" {
        let provenance: DiscoveryApplicationProvenance = parse_provenance(record)?;
        if provenance.schema != DISCOVERY_APPLICATION_SCHEMA {
            return Err(ReyIgnoreError::EnvironmentProvenance(
                record.capability_id.clone(),
            ));
        }
        return Ok(Some(("application", provenance.name)));
    }
    if record.provider_id != rey_environment::ENVIRONMENT_MAP_PROVIDER_ID {
        return Ok(None);
    }
    match record.capability_kind.as_str() {
        "environment_variable" => match environment_node(record)? {
            EnvironmentMapNode::Variable { name, .. } => Ok(Some(("environment variable", name))),
            _ => Err(ReyIgnoreError::EnvironmentProvenance(
                record.capability_id.clone(),
            )),
        },
        "potential_executable" => match environment_node(record)? {
            EnvironmentMapNode::Executable { name, .. } => Ok(Some(("application", name))),
            _ => Err(ReyIgnoreError::EnvironmentProvenance(
                record.capability_id.clone(),
            )),
        },
        "input_file" => match environment_node(record)? {
            EnvironmentMapNode::File { path, .. } => {
                Ok(Some(("input", path.to_string_lossy().into_owned())))
            }
            _ => Err(ReyIgnoreError::EnvironmentProvenance(
                record.capability_id.clone(),
            )),
        },
        "environment_edge" => {
            let edge: EnvironmentMapEdge = parse_provenance(record)?;
            Ok(Some((
                "reference",
                format!("{} --{}--> {}", edge.from, edge.relation, edge.to),
            )))
        }
        _ => Ok(None),
    }
}

fn environment_node(record: &CapabilityRecord) -> Result<EnvironmentMapNode, ReyIgnoreError> {
    let value = record
        .provenance
        .as_deref()
        .ok_or_else(|| ReyIgnoreError::EnvironmentProvenance(record.capability_id.clone()))?;
    if let Ok(provenance) = serde_json::from_str::<EnvironmentMapNodeProvenance>(value) {
        return Ok(provenance.declaration);
    }
    serde_json::from_str(value)
        .map_err(|_| ReyIgnoreError::EnvironmentProvenance(record.capability_id.clone()))
}

fn parse_provenance<T: serde::de::DeserializeOwned>(
    record: &CapabilityRecord,
) -> Result<T, ReyIgnoreError> {
    serde_json::from_str(
        record
            .provenance
            .as_deref()
            .ok_or_else(|| ReyIgnoreError::EnvironmentProvenance(record.capability_id.clone()))?,
    )
    .map_err(|_| ReyIgnoreError::EnvironmentProvenance(record.capability_id.clone()))
}

fn source_digest(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.ignore-source.v1");
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn wildcard_match(pattern: &str, candidate: &str) -> bool {
    let pattern = pattern.as_bytes();
    let candidate = candidate.as_bytes();
    let (mut pattern_index, mut candidate_index) = (0, 0);
    let (mut star, mut star_candidate) = (None, 0);
    while candidate_index < candidate.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?'
                || pattern[pattern_index] == candidate[candidate_index])
        {
            pattern_index += 1;
            candidate_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star = Some(pattern_index);
            star_candidate = candidate_index;
            pattern_index += 1;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            star_candidate += 1;
            candidate_index = star_candidate;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

#[derive(Debug, Error)]
pub enum ReyIgnoreError {
    #[error(".reyignore path {0} must be a regular non-symlinked file")]
    UnsafePath(PathBuf),
    #[error(".reyignore {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(".reyignore {path} exceeds the {limit}-byte limit with {actual} bytes")]
    ByteLimit {
        path: PathBuf,
        limit: u64,
        actual: u64,
    },
    #[error(".reyignore {path} is not valid UTF-8: {source}")]
    Encoding {
        path: PathBuf,
        source: std::str::Utf8Error,
    },
    #[error(".reyignore line {line} exceeds the {limit}-byte limit")]
    LineLimit { line: u64, limit: usize },
    #[error(".reyignore exceeds the {0}-rule limit")]
    RuleLimit(usize),
    #[error(".reyignore line {line} must be `kind: pattern` with a literal kind")]
    InvalidRule { line: u64 },
    #[error("environment capability {0} has invalid provenance required for .reyignore matching")]
    EnvironmentProvenance(String),
    #[error(".reyignore environment projection JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{ReyIgnoreError, ReyIgnoreFile, wildcard_match};

    #[test]
    fn parses_typed_rules_and_matches_bounded_wildcards() {
        let workspace = TempDir::new().unwrap();
        fs::write(
            workspace.path().join(".reyignore"),
            "# local scope\nworkload: context-*\nenvironment variable:*\napplication: code?\n",
        )
        .unwrap();
        let ignore = ReyIgnoreFile::load(workspace.path()).unwrap().unwrap();
        assert_eq!(ignore.rules.len(), 3);
        assert!(ignore.matches("workload", "context-anchor-survey"));
        assert!(ignore.matches("environment variable", "PATH"));
        assert!(ignore.matches("application", "codex"));
        assert!(!ignore.matches("application", "git"));
        assert!(wildcard_match("*", ""));
        assert!(wildcard_match("a**b", "axxb"));
    }

    #[test]
    fn invalid_and_symlinked_ignore_files_fail_closed() {
        let workspace = TempDir::new().unwrap();
        fs::write(workspace.path().join(".reyignore"), "workload\n").unwrap();
        assert!(matches!(
            ReyIgnoreFile::load(workspace.path()),
            Err(ReyIgnoreError::InvalidRule { line: 1 })
        ));
        #[cfg(unix)]
        {
            let target = workspace.path().join("target");
            fs::write(&target, "workload:*\n").unwrap();
            fs::remove_file(workspace.path().join(".reyignore")).unwrap();
            std::os::unix::fs::symlink(target, workspace.path().join(".reyignore")).unwrap();
            assert!(matches!(
                ReyIgnoreFile::load(workspace.path()),
                Err(ReyIgnoreError::UnsafePath(_))
            ));
        }
    }
}
