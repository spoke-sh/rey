#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const COORDINATE_BINDING_SCHEMA: &str = "rey.coordinate-binding.v1";
pub const LOCATOR_SCHEMA: &str = "rey.locator.v1";
pub const LOCATOR_RESOLUTION_SCHEMA: &str = "rey.locator-resolution.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateProfile {
    LocalStandalone,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateIdentityClass {
    Immutable,
    RevisionBound,
    Mutable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCoordinate {
    pub kind: String,
    pub identity: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dimensions: BTreeMap<String, String>,
}

impl LocalCoordinate {
    pub fn new(
        kind: impl Into<String>,
        identity: impl Into<String>,
        revision: impl Into<String>,
        dimensions: BTreeMap<String, String>,
    ) -> Result<Self, LocatorError> {
        let coordinate = Self {
            kind: kind.into(),
            identity: identity.into(),
            revision: revision.into(),
            dimensions,
        };
        coordinate.verify()?;
        Ok(coordinate)
    }

    pub fn parse(value: &str) -> Result<Self, LocatorError> {
        let rest = value
            .strip_prefix("rey+local://")
            .ok_or(LocatorError::CoordinateScheme)?;
        let (path, query) = rest.split_once('?').ok_or(LocatorError::CoordinateShape)?;
        if query.contains('?') || path.contains('#') || query.contains('#') {
            return Err(LocatorError::CoordinateShape);
        }
        let (kind, identity) = path.split_once('/').ok_or(LocatorError::CoordinateShape)?;
        if identity.contains('/') {
            return Err(LocatorError::CoordinateShape);
        }
        let kind = decode_component(kind)?;
        let identity = decode_component(identity)?;
        let mut revision = None;
        let mut dimensions = BTreeMap::new();
        let mut previous = None::<String>;
        for part in query.split('&') {
            let (name, value) = part.split_once('=').ok_or(LocatorError::CoordinateShape)?;
            let name = decode_component(name)?;
            let value = decode_component(value)?;
            if name.is_empty() || value.is_empty() || previous.as_ref().is_some_and(|p| p >= &name)
            {
                return Err(LocatorError::NonCanonicalCoordinate);
            }
            previous = Some(name.clone());
            if name == "revision" {
                revision = Some(value);
            } else if dimensions.insert(name.clone(), value).is_some() {
                return Err(LocatorError::DuplicateDimension(name));
            }
        }
        let coordinate = Self {
            kind,
            identity,
            revision: revision.ok_or(LocatorError::MissingRevision)?,
            dimensions,
        };
        coordinate.verify()?;
        if coordinate.to_uri() != value {
            return Err(LocatorError::NonCanonicalCoordinate);
        }
        Ok(coordinate)
    }

    #[must_use]
    pub fn to_uri(&self) -> String {
        let mut query = BTreeMap::from([("revision".to_owned(), self.revision.clone())]);
        query.extend(self.dimensions.clone());
        let query = query
            .into_iter()
            .map(|(name, value)| {
                format!("{}={}", encode_component(&name), encode_component(&value))
            })
            .collect::<Vec<_>>()
            .join("&");
        format!(
            "rey+local://{}/{}?{query}",
            encode_component(&self.kind),
            encode_component(&self.identity)
        )
    }

    fn verify(&self) -> Result<(), LocatorError> {
        if !valid_token(&self.kind) || self.identity.is_empty() || self.revision.is_empty() {
            return Err(LocatorError::CoordinateShape);
        }
        if self.identity.len() > 4_096 || self.revision.len() > 4_096 {
            return Err(LocatorError::CoordinateLimit);
        }
        if self.dimensions.contains_key("revision")
            || self.dimensions.iter().any(|(name, value)| {
                !valid_token(name)
                    || matches!(
                        name.as_str(),
                        "scale" | "zoom" | "lens" | "camera" | "viewport" | "selection"
                    )
                    || value.is_empty()
                    || value.len() > 4_096
            })
        {
            return Err(LocatorError::CoordinateDimension);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateBinding {
    pub schema: String,
    pub binding_id: SemanticDigest,
    pub profile: CoordinateProfile,
    pub provider: ContractIdentity,
    pub coordinate: String,
    pub identity_class: CoordinateIdentityClass,
    pub source_revision: String,
    pub retention: String,
}

impl CoordinateBinding {
    pub fn local(
        provider: ContractIdentity,
        coordinate: LocalCoordinate,
        identity_class: CoordinateIdentityClass,
        source_revision: impl Into<String>,
    ) -> Result<Self, LocatorError> {
        let mut binding = Self {
            schema: COORDINATE_BINDING_SCHEMA.to_owned(),
            binding_id: placeholder("rey.coordinate-binding.placeholder"),
            profile: CoordinateProfile::LocalStandalone,
            provider,
            coordinate: coordinate.to_uri(),
            identity_class,
            source_revision: source_revision.into(),
            retention: "process-local evidence; no remote durability or federation".to_owned(),
        };
        binding.binding_id = binding_digest(&binding);
        binding.verify()?;
        Ok(binding)
    }

    pub fn verify(&self) -> Result<(), LocatorError> {
        if self.schema != COORDINATE_BINDING_SCHEMA {
            return Err(LocatorError::Schema);
        }
        validate_contract(&self.provider)?;
        if self.coordinate.is_empty()
            || self.coordinate.len() > 16_384
            || self.source_revision.is_empty()
            || self.retention.is_empty()
        {
            return Err(LocatorError::CoordinateLimit);
        }
        match self.profile {
            CoordinateProfile::LocalStandalone => {
                let coordinate = LocalCoordinate::parse(&self.coordinate)?;
                if coordinate.revision != self.source_revision {
                    return Err(LocatorError::RevisionBinding);
                }
            }
        }
        let actual = binding_digest(self);
        if actual != self.binding_id {
            return Err(LocatorError::Digest);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatorKind {
    HttpUri,
    HttpsUri,
    WorkspaceReference,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Locator {
    pub schema: String,
    pub kind: LocatorKind,
    pub payload: String,
    pub fragment: Option<String>,
}

impl Locator {
    pub fn parse(value: &str) -> Result<Self, LocatorError> {
        if value.is_empty() || value.len() > 16_384 || value.contains(['\0', '\n', '\r']) {
            return Err(LocatorError::LocatorShape);
        }
        let (kind, rest) = if let Some(rest) = value.strip_prefix("https://") {
            (LocatorKind::HttpsUri, rest)
        } else if let Some(rest) = value.strip_prefix("http://") {
            (LocatorKind::HttpUri, rest)
        } else {
            (LocatorKind::WorkspaceReference, value)
        };
        let (payload, fragment) = rest
            .split_once('#')
            .map_or((rest, None), |(payload, fragment)| {
                (payload, Some(fragment.to_owned()))
            });
        if payload.is_empty()
            || payload.chars().any(char::is_whitespace)
            || fragment.as_deref().is_some_and(str::is_empty)
            || (kind != LocatorKind::WorkspaceReference && !payload.contains('.'))
            || invalid_percent_encoding(payload)
            || fragment.as_deref().is_some_and(invalid_percent_encoding)
        {
            return Err(LocatorError::LocatorShape);
        }
        let locator = Self {
            schema: LOCATOR_SCHEMA.to_owned(),
            kind,
            payload: payload.to_owned(),
            fragment,
        };
        if locator.to_machine_string() != value {
            return Err(LocatorError::NonCanonicalLocator);
        }
        Ok(locator)
    }

    #[must_use]
    pub fn to_machine_string(&self) -> String {
        let prefix = match self.kind {
            LocatorKind::HttpUri => "http://",
            LocatorKind::HttpsUri => "https://",
            LocatorKind::WorkspaceReference => "",
        };
        match &self.fragment {
            Some(fragment) => format!("{prefix}{}#{fragment}", self.payload),
            None => format!("{prefix}{}", self.payload),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Resolved,
    Missing,
    Stale,
    Unsupported,
    Unauthorized,
    Malformed,
    Truncated,
}

impl ResolutionStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::Unsupported => "unsupported",
            Self::Unauthorized => "unauthorized",
            Self::Malformed => "malformed",
            Self::Truncated => "truncated",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionLimits {
    pub max_locator_bytes: u64,
    pub max_source_bytes: u64,
    pub max_candidates: u64,
    pub max_depth: u64,
}

impl Default for ResolutionLimits {
    fn default() -> Self {
        Self {
            max_locator_bytes: 16_384,
            max_source_bytes: 1_048_576,
            max_candidates: 4_096,
            max_depth: 32,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocatorResolution {
    pub schema: String,
    pub resolution_id: SemanticDigest,
    pub candidate: String,
    pub locator: Option<Locator>,
    pub status: ResolutionStatus,
    pub coordinate: Option<CoordinateBinding>,
    pub provider: ContractIdentity,
    pub source_revision: String,
    pub capability_snapshot_id: SemanticDigest,
    pub limits: ResolutionLimits,
    pub complete: bool,
    pub detail: String,
}

impl LocatorResolution {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate: impl Into<String>,
        locator: Option<Locator>,
        status: ResolutionStatus,
        coordinate: Option<CoordinateBinding>,
        provider: ContractIdentity,
        source_revision: impl Into<String>,
        capability_snapshot_id: SemanticDigest,
        limits: ResolutionLimits,
        complete: bool,
        detail: impl Into<String>,
    ) -> Result<Self, LocatorError> {
        let mut resolution = Self {
            schema: LOCATOR_RESOLUTION_SCHEMA.to_owned(),
            resolution_id: placeholder("rey.locator-resolution.placeholder"),
            candidate: candidate.into(),
            locator,
            status,
            coordinate,
            provider,
            source_revision: source_revision.into(),
            capability_snapshot_id,
            limits,
            complete,
            detail: detail.into(),
        };
        resolution.resolution_id = resolution_digest(&resolution);
        resolution.verify()?;
        Ok(resolution)
    }

    pub fn verify(&self) -> Result<(), LocatorError> {
        if self.schema != LOCATOR_RESOLUTION_SCHEMA {
            return Err(LocatorError::Schema);
        }
        validate_contract(&self.provider)?;
        if self.candidate.is_empty()
            || self.candidate.len() as u64 > self.limits.max_locator_bytes
            || self.source_revision.is_empty()
            || self.detail.is_empty()
            || self.limits.max_locator_bytes == 0
            || self.limits.max_source_bytes == 0
            || self.limits.max_candidates == 0
            || self.limits.max_depth == 0
        {
            return Err(LocatorError::ResolutionShape);
        }
        match self.status {
            ResolutionStatus::Resolved if self.locator.is_some() && self.coordinate.is_some() => {}
            ResolutionStatus::Malformed if self.locator.is_none() && self.coordinate.is_none() => {}
            ResolutionStatus::Missing
            | ResolutionStatus::Stale
            | ResolutionStatus::Unsupported
            | ResolutionStatus::Unauthorized
            | ResolutionStatus::Truncated
                if self.locator.is_some() && self.coordinate.is_none() => {}
            _ => return Err(LocatorError::ResolutionShape),
        }
        if let Some(coordinate) = &self.coordinate {
            coordinate.verify()?;
        }
        let actual = resolution_digest(self);
        if actual != self.resolution_id {
            return Err(LocatorError::Digest);
        }
        Ok(())
    }
}

fn binding_digest(binding: &CoordinateBinding) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(COORDINATE_BINDING_SCHEMA);
    hasher.add_str(match binding.profile {
        CoordinateProfile::LocalStandalone => "local_standalone",
    });
    binding.provider.add_semantics(&mut hasher);
    hasher.add_str(&binding.coordinate);
    hasher.add_str(match binding.identity_class {
        CoordinateIdentityClass::Immutable => "immutable",
        CoordinateIdentityClass::RevisionBound => "revision_bound",
        CoordinateIdentityClass::Mutable => "mutable",
    });
    hasher.add_str(&binding.source_revision);
    hasher.add_str(&binding.retention);
    hasher.finish()
}

fn resolution_digest(resolution: &LocatorResolution) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(LOCATOR_RESOLUTION_SCHEMA);
    hasher.add_str(&resolution.candidate);
    hasher.add_bool(resolution.locator.is_some());
    if let Some(locator) = &resolution.locator {
        hasher.add_str(&locator.to_machine_string());
    }
    hasher.add_str(resolution.status.as_str());
    hasher.add_bool(resolution.coordinate.is_some());
    if let Some(coordinate) = &resolution.coordinate {
        hasher.add_str(coordinate.binding_id.as_str());
    }
    resolution.provider.add_semantics(&mut hasher);
    hasher.add_str(&resolution.source_revision);
    hasher.add_str(resolution.capability_snapshot_id.as_str());
    hasher.add_u64(resolution.limits.max_locator_bytes);
    hasher.add_u64(resolution.limits.max_source_bytes);
    hasher.add_u64(resolution.limits.max_candidates);
    hasher.add_u64(resolution.limits.max_depth);
    hasher.add_bool(resolution.complete);
    hasher.add_str(&resolution.detail);
    hasher.finish()
}

fn validate_contract(contract: &ContractIdentity) -> Result<(), LocatorError> {
    if contract.id.is_empty()
        || contract.revision == 0
        || contract.semantic_digest.as_str().is_empty()
    {
        return Err(LocatorError::Provider);
    }
    Ok(())
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(b"0123456789ABCDEF"[(byte >> 4) as usize]));
            encoded.push(char::from(b"0123456789ABCDEF"[(byte & 0x0f) as usize]));
        }
    }
    encoded
}

fn decode_component(value: &str) -> Result<String, LocatorError> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(LocatorError::Encoding);
            }
            decoded.push((hex(bytes[index + 1])? << 4) | hex(bytes[index + 2])?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| LocatorError::Encoding)
}

fn hex(value: u8) -> Result<u8, LocatorError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(LocatorError::Encoding),
    }
}

fn invalid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || hex(bytes[index + 1]).is_err()
                || hex(bytes[index + 2]).is_err()
            {
                return true;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    false
}

fn placeholder(domain: &str) -> SemanticDigest {
    SemanticHasher::new(domain).finish()
}

#[derive(Debug, Error)]
pub enum LocatorError {
    #[error("unsupported coordinate scheme")]
    CoordinateScheme,
    #[error("invalid coordinate shape")]
    CoordinateShape,
    #[error("coordinate exceeds its bound")]
    CoordinateLimit,
    #[error("coordinate dimensions are invalid")]
    CoordinateDimension,
    #[error("coordinate is missing its revision")]
    MissingRevision,
    #[error("duplicate coordinate dimension {0}")]
    DuplicateDimension(String),
    #[error("coordinate is not canonically encoded")]
    NonCanonicalCoordinate,
    #[error("invalid percent encoding")]
    Encoding,
    #[error("invalid locator shape")]
    LocatorShape,
    #[error("locator is not canonical")]
    NonCanonicalLocator,
    #[error("invalid locator resolution")]
    ResolutionShape,
    #[error("coordinate revision does not match its source binding")]
    RevisionBinding,
    #[error("unsupported schema")]
    Schema,
    #[error("invalid provider identity")]
    Provider,
    #[error("semantic digest mismatch")]
    Digest,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ContractIdentity {
        ContractIdentity::new(
            "rey.provider.local-worktree",
            1,
            "bounded local worktree resolver",
        )
    }

    #[test]
    fn local_coordinates_round_trip_canonically_and_separate_view_state() {
        let coordinate = LocalCoordinate::new(
            "file",
            "docs/Context Map.md",
            "blake3:source",
            BTreeMap::new(),
        )
        .unwrap();
        let uri = coordinate.to_uri();
        assert_eq!(
            uri,
            "rey+local://file/docs%2FContext%20Map.md?revision=blake3%3Asource"
        );
        assert_eq!(LocalCoordinate::parse(&uri).unwrap(), coordinate);
        assert!(LocalCoordinate::parse(&format!("{uri}&scale=1.5")).is_err());
        assert!(
            LocalCoordinate::parse("rey+local://file/x?role=human&revision=blake3%3Asource")
                .is_err()
        );
    }

    #[test]
    fn locators_and_resolution_outcomes_keep_failure_classes_distinct() {
        let locator = Locator::parse("README.md#usage").unwrap();
        assert_eq!(locator.kind, LocatorKind::WorkspaceReference);
        assert_eq!(locator.to_machine_string(), "README.md#usage");
        assert!(Locator::parse("https://example.com/%zz").is_err());

        let coordinate =
            LocalCoordinate::new("file", "README.md", "blake3:readme", BTreeMap::new()).unwrap();
        let binding = CoordinateBinding::local(
            provider(),
            coordinate,
            CoordinateIdentityClass::RevisionBound,
            "blake3:readme",
        )
        .unwrap();
        let resolution = LocatorResolution::new(
            "README.md#usage",
            Some(locator),
            ResolutionStatus::Resolved,
            Some(binding),
            provider(),
            "blake3:seed",
            SemanticHasher::new("capability").finish(),
            ResolutionLimits::default(),
            true,
            "resolved below the admitted workspace root",
        )
        .unwrap();
        resolution.verify().unwrap();

        for status in [
            ResolutionStatus::Missing,
            ResolutionStatus::Stale,
            ResolutionStatus::Unsupported,
            ResolutionStatus::Unauthorized,
            ResolutionStatus::Truncated,
        ] {
            let outcome = LocatorResolution::new(
                "README.md#usage",
                Some(Locator::parse("README.md#usage").unwrap()),
                status,
                None,
                provider(),
                "blake3:seed",
                SemanticHasher::new("capability").finish(),
                ResolutionLimits::default(),
                status != ResolutionStatus::Truncated,
                format!("explicit {} outcome", status.as_str()),
            )
            .unwrap();
            outcome.verify().unwrap();
        }
        LocatorResolution::new(
            "https://example.com/%zz",
            None,
            ResolutionStatus::Malformed,
            None,
            provider(),
            "blake3:seed",
            SemanticHasher::new("capability").finish(),
            ResolutionLimits::default(),
            true,
            "malformed percent encoding",
        )
        .unwrap()
        .verify()
        .unwrap();
    }
}
