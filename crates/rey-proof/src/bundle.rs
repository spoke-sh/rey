use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rey_core::{SemanticDigest, SemanticHasher};
use rey_dataframe::ARROW_STREAM_MEDIA_TYPE;
use rey_diff::{
    CapabilityDelta, DeltaOptions, TABULAR_DIFF_MEDIA_TYPE, capability_comparator,
    compare_capabilities,
};
use rey_environment::CapabilitySnapshot;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ProofStatus, RequiredCapabilityCertificate, VerificationStatus, required_capability_evaluator,
    verify_required_capability_certificate,
};

pub const LOCAL_PROOF_BUNDLE_SCHEMA: &str = "rey.local-proof-bundle.v1";
pub const LOCAL_BUNDLE_VERIFICATION_SCHEMA: &str = "rey.local-proof-bundle-verification.v1";

const MANIFEST_NAME: &str = "manifest.json";
const OBJECT_DIRECTORY: &str = "objects";
const DIGEST_DIRECTORY: &str = "blake3";
const ARTIFACT_DIGEST_DOMAIN: &str = "rey.local-proof-object.v1";
const STAGING_ATTEMPTS: u64 = 128;
const REQUIRED_ARTIFACT_COUNT: u64 = 6;

static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBundleLimits {
    pub max_artifacts: u64,
    pub max_artifact_bytes: u64,
    pub max_total_bytes: u64,
    pub max_capabilities: u64,
}

impl Default for LocalBundleLimits {
    fn default() -> Self {
        Self {
            max_artifacts: 16,
            max_artifact_bytes: 16 * 1_024 * 1_024,
            max_total_bytes: 64 * 1_024 * 1_024,
            max_capabilities: 4_096,
        }
    }
}

impl LocalBundleLimits {
    fn validate(&self) -> Result<(), LocalBundleError> {
        for (name, value) in [
            ("max_artifacts", self.max_artifacts),
            ("max_artifact_bytes", self.max_artifact_bytes),
            ("max_total_bytes", self.max_total_bytes),
            ("max_capabilities", self.max_capabilities),
        ] {
            if value == 0 {
                return Err(LocalBundleError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRetentionContract {
    pub profile: String,
    pub publication: String,
    pub content_addressed_objects: bool,
    pub manifest_written_after_objects: bool,
    pub overwrite_existing: bool,
    pub verification_read_only: bool,
    pub process_crash_durable: bool,
    pub multi_process_transactional: bool,
    pub remote_durable: bool,
    pub authenticated_writer: bool,
}

impl Default for LocalRetentionContract {
    fn default() -> Self {
        Self {
            profile: "local".to_owned(),
            publication: "staged-directory-rename-v1".to_owned(),
            content_addressed_objects: true,
            manifest_written_after_objects: true,
            overwrite_existing: false,
            verification_read_only: true,
            process_crash_durable: false,
            multi_process_transactional: false,
            remote_durable: false,
            authenticated_writer: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleArtifactRole {
    SourceSnapshot,
    TargetSnapshot,
    CapabilityDelta,
    CapabilityDeltaArrow,
    CapabilityTabularDiff,
    RequiredCapabilityCertificate,
}

impl BundleArtifactRole {
    const ALL: [Self; REQUIRED_ARTIFACT_COUNT as usize] = [
        Self::SourceSnapshot,
        Self::TargetSnapshot,
        Self::CapabilityDelta,
        Self::CapabilityDeltaArrow,
        Self::CapabilityTabularDiff,
        Self::RequiredCapabilityCertificate,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::SourceSnapshot => "source_snapshot",
            Self::TargetSnapshot => "target_snapshot",
            Self::CapabilityDelta => "capability_delta",
            Self::CapabilityDeltaArrow => "capability_delta_arrow",
            Self::CapabilityTabularDiff => "capability_tabular_diff",
            Self::RequiredCapabilityCertificate => "required_capability_certificate",
        }
    }

    const fn media_type(self) -> &'static str {
        match self {
            Self::SourceSnapshot | Self::TargetSnapshot => {
                "application/json; profile=rey.capabilities.v1"
            }
            Self::CapabilityDelta => "application/json; profile=rey.capability-delta.v1",
            Self::CapabilityDeltaArrow => ARROW_STREAM_MEDIA_TYPE,
            Self::CapabilityTabularDiff => TABULAR_DIFF_MEDIA_TYPE,
            Self::RequiredCapabilityCertificate => {
                "application/json; profile=rey.required-capability-certificate.v1"
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBundleArtifact {
    pub role: BundleArtifactRole,
    pub media_type: String,
    pub digest: SemanticDigest,
    pub bytes: u64,
    pub object_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalProofBundleManifest {
    pub schema: String,
    pub bundle_id: SemanticDigest,
    pub retention: LocalRetentionContract,
    pub limits: LocalBundleLimits,
    pub source_snapshot: SemanticDigest,
    pub target_snapshot: SemanticDigest,
    pub delta_id: SemanticDigest,
    pub certificate_id: SemanticDigest,
    pub total_artifact_bytes: u64,
    pub artifacts: Vec<LocalBundleArtifact>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalBundleVerificationStatus {
    Verified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalBundleVerification {
    pub schema: String,
    pub bundle_id: SemanticDigest,
    pub certificate_id: SemanticDigest,
    pub certificate_status: ProofStatus,
    pub status: LocalBundleVerificationStatus,
    pub artifact_count: u64,
    pub total_artifact_bytes: u64,
    pub retention: LocalRetentionContract,
}

struct BuiltArtifact {
    role: BundleArtifactRole,
    bytes: Vec<u8>,
}

pub fn create_local_proof_bundle(
    destination: &Path,
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    certificate: &RequiredCapabilityCertificate,
    limits: LocalBundleLimits,
) -> Result<LocalProofBundleManifest, LocalBundleError> {
    limits.validate()?;
    check_capability_count("source", source.capabilities.len(), limits.max_capabilities)?;
    check_capability_count("target", target.capabilities.len(), limits.max_capabilities)?;

    let certificate_verification = verify_required_capability_certificate(
        certificate,
        source,
        target,
        required_capability_evaluator(),
    )?;
    if certificate_verification.status != VerificationStatus::Verified {
        return Err(LocalBundleError::CertificateStale);
    }

    let delta = compare_capabilities(
        source,
        target,
        DeltaOptions {
            source_label: certificate.source_label.clone(),
            target_label: certificate.target_label.clone(),
            limits: certificate.delta_limits.clone(),
            comparator: capability_comparator(),
        },
    )?;
    let artifacts = build_artifacts(source, target, &delta, certificate)?;
    let (entries, total_artifact_bytes) = describe_artifacts(&artifacts, &limits)?;
    let mut manifest = LocalProofBundleManifest {
        schema: LOCAL_PROOF_BUNDLE_SCHEMA.to_owned(),
        bundle_id: SemanticHasher::new("rey.uninitialized-local-proof-bundle").finish(),
        retention: LocalRetentionContract::default(),
        limits: limits.clone(),
        source_snapshot: source.semantic_digest.clone(),
        target_snapshot: target.semantic_digest.clone(),
        delta_id: delta.delta_id,
        certificate_id: certificate.certificate_id.clone(),
        total_artifact_bytes,
        artifacts: entries,
    };
    manifest.bundle_id = bundle_digest(&manifest);

    let destination = resolve_destination(destination)?;
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            if let Ok(existing) = verify_local_proof_bundle(&destination, limits)
                && existing.bundle_id == manifest.bundle_id
            {
                return Ok(manifest);
            }
            return Err(LocalBundleError::DestinationExists(destination));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(io_error("inspect destination", &destination, source)),
    }

    publish_new_bundle(&destination, &manifest, &artifacts)?;
    Ok(manifest)
}

pub fn verify_local_proof_bundle(
    bundle: &Path,
    admission_limits: LocalBundleLimits,
) -> Result<LocalBundleVerification, LocalBundleError> {
    admission_limits.validate()?;
    let root = resolve_existing_bundle(bundle)?;
    let manifest_path = root.join(MANIFEST_NAME);
    let manifest_bytes = read_regular_file(
        &manifest_path,
        admission_limits.max_artifact_bytes,
        "manifest",
    )?;
    let manifest: LocalProofBundleManifest = serde_json::from_slice(&manifest_bytes)?;
    validate_manifest(&manifest, &admission_limits)?;
    if manifest_bytes != json_line(&manifest)? {
        return Err(LocalBundleError::NonCanonicalManifest);
    }
    validate_tree(&root, &manifest, &admission_limits)?;

    let mut retained = BTreeMap::new();
    for artifact in &manifest.artifacts {
        let path = root.join(&artifact.object_path);
        let bytes = read_regular_file(
            &path,
            admission_limits.max_artifact_bytes,
            artifact.role.as_str(),
        )?;
        if bytes.len() as u64 != artifact.bytes {
            return Err(LocalBundleError::ArtifactLength {
                role: artifact.role,
                declared: artifact.bytes,
                actual: bytes.len() as u64,
            });
        }
        let actual = artifact_digest(&bytes);
        if actual != artifact.digest {
            return Err(LocalBundleError::ArtifactDigest {
                role: artifact.role,
                declared: artifact.digest.clone(),
                actual,
            });
        }
        retained.insert(artifact.role, bytes);
    }

    let max_capabilities = admission_limits
        .max_capabilities
        .min(manifest.limits.max_capabilities);
    let source = CapabilitySnapshot::from_json_slice(
        retained_artifact(&retained, BundleArtifactRole::SourceSnapshot)?,
        max_capabilities,
    )?;
    let target = CapabilitySnapshot::from_json_slice(
        retained_artifact(&retained, BundleArtifactRole::TargetSnapshot)?,
        max_capabilities,
    )?;
    ensure_exact_artifact(
        BundleArtifactRole::SourceSnapshot,
        retained_artifact(&retained, BundleArtifactRole::SourceSnapshot)?,
        &json_line(&source)?,
    )?;
    ensure_exact_artifact(
        BundleArtifactRole::TargetSnapshot,
        retained_artifact(&retained, BundleArtifactRole::TargetSnapshot)?,
        &json_line(&target)?,
    )?;

    let certificate: RequiredCapabilityCertificate = serde_json::from_slice(retained_artifact(
        &retained,
        BundleArtifactRole::RequiredCapabilityCertificate,
    )?)?;
    ensure_exact_artifact(
        BundleArtifactRole::RequiredCapabilityCertificate,
        retained_artifact(&retained, BundleArtifactRole::RequiredCapabilityCertificate)?,
        &json_line(&certificate)?,
    )?;
    let certificate_verification = verify_required_capability_certificate(
        &certificate,
        &source,
        &target,
        required_capability_evaluator(),
    )?;
    if certificate_verification.status != VerificationStatus::Verified {
        return Err(LocalBundleError::CertificateStale);
    }

    let delta = compare_capabilities(
        &source,
        &target,
        DeltaOptions {
            source_label: certificate.source_label.clone(),
            target_label: certificate.target_label.clone(),
            limits: certificate.delta_limits.clone(),
            comparator: capability_comparator(),
        },
    )?;
    let retained_delta: CapabilityDelta = serde_json::from_slice(retained_artifact(
        &retained,
        BundleArtifactRole::CapabilityDelta,
    )?)?;
    if retained_delta != delta {
        return Err(LocalBundleError::ArtifactSemantics(
            BundleArtifactRole::CapabilityDelta,
        ));
    }
    ensure_exact_artifact(
        BundleArtifactRole::CapabilityDelta,
        retained_artifact(&retained, BundleArtifactRole::CapabilityDelta)?,
        &json_line(&delta)?,
    )?;
    ensure_exact_artifact(
        BundleArtifactRole::CapabilityDeltaArrow,
        retained_artifact(&retained, BundleArtifactRole::CapabilityDeltaArrow)?,
        &delta.to_frame()?.to_arrow_stream()?,
    )?;
    ensure_exact_artifact(
        BundleArtifactRole::CapabilityTabularDiff,
        retained_artifact(&retained, BundleArtifactRole::CapabilityTabularDiff)?,
        &delta.to_tabular_diff()?,
    )?;

    if manifest.source_snapshot != source.semantic_digest {
        return Err(LocalBundleError::ManifestIdentity("source snapshot"));
    }
    if manifest.target_snapshot != target.semantic_digest {
        return Err(LocalBundleError::ManifestIdentity("target snapshot"));
    }
    if manifest.delta_id != delta.delta_id {
        return Err(LocalBundleError::ManifestIdentity("delta"));
    }
    if manifest.certificate_id != certificate.certificate_id {
        return Err(LocalBundleError::ManifestIdentity("certificate"));
    }

    Ok(LocalBundleVerification {
        schema: LOCAL_BUNDLE_VERIFICATION_SCHEMA.to_owned(),
        bundle_id: manifest.bundle_id,
        certificate_id: certificate.certificate_id,
        certificate_status: certificate.status,
        status: LocalBundleVerificationStatus::Verified,
        artifact_count: manifest.artifacts.len() as u64,
        total_artifact_bytes: manifest.total_artifact_bytes,
        retention: manifest.retention,
    })
}

fn build_artifacts(
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    delta: &CapabilityDelta,
    certificate: &RequiredCapabilityCertificate,
) -> Result<Vec<BuiltArtifact>, LocalBundleError> {
    Ok(vec![
        BuiltArtifact {
            role: BundleArtifactRole::SourceSnapshot,
            bytes: json_line(source)?,
        },
        BuiltArtifact {
            role: BundleArtifactRole::TargetSnapshot,
            bytes: json_line(target)?,
        },
        BuiltArtifact {
            role: BundleArtifactRole::CapabilityDelta,
            bytes: json_line(delta)?,
        },
        BuiltArtifact {
            role: BundleArtifactRole::CapabilityDeltaArrow,
            bytes: delta.to_frame()?.to_arrow_stream()?,
        },
        BuiltArtifact {
            role: BundleArtifactRole::CapabilityTabularDiff,
            bytes: delta.to_tabular_diff()?,
        },
        BuiltArtifact {
            role: BundleArtifactRole::RequiredCapabilityCertificate,
            bytes: json_line(certificate)?,
        },
    ])
}

fn describe_artifacts(
    artifacts: &[BuiltArtifact],
    limits: &LocalBundleLimits,
) -> Result<(Vec<LocalBundleArtifact>, u64), LocalBundleError> {
    if artifacts.len() as u64 > limits.max_artifacts {
        return Err(LocalBundleError::ArtifactCount {
            observed: artifacts.len() as u64,
            limit: limits.max_artifacts,
        });
    }
    let mut total = 0_u64;
    let mut entries = Vec::with_capacity(artifacts.len());
    for artifact in artifacts {
        let bytes = artifact.bytes.len() as u64;
        if bytes > limits.max_artifact_bytes {
            return Err(LocalBundleError::ArtifactLimit {
                role: artifact.role,
                observed: bytes,
                limit: limits.max_artifact_bytes,
            });
        }
        total = total
            .checked_add(bytes)
            .ok_or(LocalBundleError::TotalBytesOverflow)?;
        let digest = artifact_digest(&artifact.bytes);
        entries.push(LocalBundleArtifact {
            role: artifact.role,
            media_type: artifact.role.media_type().to_owned(),
            object_path: object_path(&digest)?,
            digest,
            bytes,
        });
    }
    if total > limits.max_total_bytes {
        return Err(LocalBundleError::TotalLimit {
            observed: total,
            limit: limits.max_total_bytes,
        });
    }
    Ok((entries, total))
}

fn validate_manifest(
    manifest: &LocalProofBundleManifest,
    admission_limits: &LocalBundleLimits,
) -> Result<(), LocalBundleError> {
    if manifest.schema != LOCAL_PROOF_BUNDLE_SCHEMA {
        return Err(LocalBundleError::UnsupportedSchema(manifest.schema.clone()));
    }
    if manifest.retention != LocalRetentionContract::default() {
        return Err(LocalBundleError::RetentionContract);
    }
    manifest.limits.validate()?;
    if manifest.artifacts.len() as u64 > manifest.limits.max_artifacts {
        return Err(LocalBundleError::ArtifactCount {
            observed: manifest.artifacts.len() as u64,
            limit: manifest.limits.max_artifacts,
        });
    }
    if manifest.artifacts.len() as u64 > admission_limits.max_artifacts {
        return Err(LocalBundleError::ArtifactCount {
            observed: manifest.artifacts.len() as u64,
            limit: admission_limits.max_artifacts,
        });
    }
    let roles = manifest
        .artifacts
        .iter()
        .map(|artifact| artifact.role)
        .collect::<Vec<_>>();
    if roles != BundleArtifactRole::ALL {
        return Err(LocalBundleError::NonCanonicalArtifacts);
    }

    let mut total = 0_u64;
    for artifact in &manifest.artifacts {
        if artifact.media_type != artifact.role.media_type() {
            return Err(LocalBundleError::ArtifactMediaType(artifact.role));
        }
        let expected_path = object_path(&artifact.digest)?;
        if artifact.object_path != expected_path {
            return Err(LocalBundleError::ArtifactPath {
                role: artifact.role,
                path: artifact.object_path.clone(),
            });
        }
        let effective_limit = manifest
            .limits
            .max_artifact_bytes
            .min(admission_limits.max_artifact_bytes);
        if artifact.bytes > effective_limit {
            return Err(LocalBundleError::ArtifactLimit {
                role: artifact.role,
                observed: artifact.bytes,
                limit: effective_limit,
            });
        }
        total = total
            .checked_add(artifact.bytes)
            .ok_or(LocalBundleError::TotalBytesOverflow)?;
    }
    if total != manifest.total_artifact_bytes {
        return Err(LocalBundleError::TotalBytes {
            declared: manifest.total_artifact_bytes,
            actual: total,
        });
    }
    let effective_total = manifest
        .limits
        .max_total_bytes
        .min(admission_limits.max_total_bytes);
    if total > effective_total {
        return Err(LocalBundleError::TotalLimit {
            observed: total,
            limit: effective_total,
        });
    }
    let actual = bundle_digest(manifest);
    if manifest.bundle_id != actual {
        return Err(LocalBundleError::BundleDigest {
            declared: manifest.bundle_id.clone(),
            actual,
        });
    }
    Ok(())
}

fn artifact_digest(bytes: &[u8]) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(ARTIFACT_DIGEST_DOMAIN);
    hasher.add_bytes(bytes);
    hasher.finish()
}

fn bundle_digest(manifest: &LocalProofBundleManifest) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(LOCAL_PROOF_BUNDLE_SCHEMA);
    hasher.add_str(&manifest.schema);
    add_retention(&mut hasher, &manifest.retention);
    add_limits(&mut hasher, &manifest.limits);
    hasher.add_str(manifest.source_snapshot.as_str());
    hasher.add_str(manifest.target_snapshot.as_str());
    hasher.add_str(manifest.delta_id.as_str());
    hasher.add_str(manifest.certificate_id.as_str());
    hasher.add_u64(manifest.total_artifact_bytes);
    hasher.add_u64(manifest.artifacts.len() as u64);
    for artifact in &manifest.artifacts {
        hasher.add_str(artifact.role.as_str());
        hasher.add_str(&artifact.media_type);
        hasher.add_str(artifact.digest.as_str());
        hasher.add_u64(artifact.bytes);
        hasher.add_str(&artifact.object_path);
    }
    hasher.finish()
}

fn add_retention(hasher: &mut SemanticHasher, retention: &LocalRetentionContract) {
    hasher.add_str(&retention.profile);
    hasher.add_str(&retention.publication);
    hasher.add_bool(retention.content_addressed_objects);
    hasher.add_bool(retention.manifest_written_after_objects);
    hasher.add_bool(retention.overwrite_existing);
    hasher.add_bool(retention.verification_read_only);
    hasher.add_bool(retention.process_crash_durable);
    hasher.add_bool(retention.multi_process_transactional);
    hasher.add_bool(retention.remote_durable);
    hasher.add_bool(retention.authenticated_writer);
}

fn add_limits(hasher: &mut SemanticHasher, limits: &LocalBundleLimits) {
    hasher.add_u64(limits.max_artifacts);
    hasher.add_u64(limits.max_artifact_bytes);
    hasher.add_u64(limits.max_total_bytes);
    hasher.add_u64(limits.max_capabilities);
}

fn object_path(digest: &SemanticDigest) -> Result<String, LocalBundleError> {
    let Some(hex) = digest.as_str().strip_prefix("blake3:") else {
        return Err(LocalBundleError::InvalidDigest(digest.to_string()));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LocalBundleError::InvalidDigest(digest.to_string()));
    }
    Ok(format!("{OBJECT_DIRECTORY}/{DIGEST_DIRECTORY}/{hex}"))
}

fn json_line(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn publish_new_bundle(
    destination: &Path,
    manifest: &LocalProofBundleManifest,
    artifacts: &[BuiltArtifact],
) -> Result<(), LocalBundleError> {
    let parent = destination
        .parent()
        .ok_or_else(|| LocalBundleError::InvalidDestination(destination.to_owned()))?;
    let mut staging = StagingDirectory::create(parent)?;
    let object_root = staging.path().join(OBJECT_DIRECTORY).join(DIGEST_DIRECTORY);
    fs::create_dir_all(&object_root)
        .map_err(|source| io_error("create object directory", &object_root, source))?;

    let mut written = BTreeSet::new();
    for (entry, artifact) in manifest.artifacts.iter().zip(artifacts) {
        if written.insert(entry.object_path.clone()) {
            write_new_file(&staging.path().join(&entry.object_path), &artifact.bytes)?;
        }
    }
    write_new_file(&staging.path().join(MANIFEST_NAME), &json_line(manifest)?)?;
    if fs::symlink_metadata(destination).is_ok() {
        return Err(LocalBundleError::DestinationExists(destination.to_owned()));
    }
    fs::rename(staging.path(), destination)
        .map_err(|source| io_error("publish bundle", destination, source))?;
    staging.disarm();
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), LocalBundleError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io_error("create bundle file", path, source))?;
    file.write_all(bytes)
        .map_err(|source| io_error("write bundle file", path, source))?;
    file.flush()
        .map_err(|source| io_error("flush bundle file", path, source))?;
    Ok(())
}

fn resolve_destination(destination: &Path) -> Result<PathBuf, LocalBundleError> {
    let name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| LocalBundleError::InvalidDestination(destination.to_owned()))?;
    if destination
        .components()
        .next_back()
        .is_some_and(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(LocalBundleError::InvalidDestination(destination.to_owned()));
    }
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|source| io_error("resolve bundle parent", parent, source))?;
    let metadata = fs::metadata(&canonical_parent)
        .map_err(|source| io_error("inspect bundle parent", &canonical_parent, source))?;
    if !metadata.is_dir() {
        return Err(LocalBundleError::InvalidDestination(destination.to_owned()));
    }
    Ok(canonical_parent.join(name))
}

fn resolve_existing_bundle(bundle: &Path) -> Result<PathBuf, LocalBundleError> {
    let metadata = fs::symlink_metadata(bundle)
        .map_err(|source| io_error("inspect bundle", bundle, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalBundleError::UnsafeEntry(bundle.to_owned()));
    }
    fs::canonicalize(bundle).map_err(|source| io_error("resolve bundle", bundle, source))
}

fn read_regular_file(
    path: &Path,
    max_bytes: u64,
    description: &str,
) -> Result<Vec<u8>, LocalBundleError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(LocalBundleError::MissingFile {
                description: description.to_owned(),
                path: path.to_owned(),
            });
        }
        Err(source) => return Err(io_error("inspect bundle file", path, source)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LocalBundleError::UnsafeEntry(path.to_owned()));
    }
    if metadata.len() > max_bytes {
        return Err(LocalBundleError::FileLimit {
            description: description.to_owned(),
            observed: metadata.len(),
            limit: max_bytes,
        });
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|source| io_error("open bundle file", path, source))?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("read bundle file", path, source))?;
    if bytes.len() as u64 > max_bytes {
        return Err(LocalBundleError::FileLimit {
            description: description.to_owned(),
            observed: bytes.len() as u64,
            limit: max_bytes,
        });
    }
    Ok(bytes)
}

fn validate_tree(
    root: &Path,
    manifest: &LocalProofBundleManifest,
    limits: &LocalBundleLimits,
) -> Result<(), LocalBundleError> {
    validate_directory_entries(
        root,
        BTreeSet::from([
            OsString::from(MANIFEST_NAME),
            OsString::from(OBJECT_DIRECTORY),
        ]),
        2,
    )?;
    let objects = root.join(OBJECT_DIRECTORY);
    ensure_real_directory(&objects)?;
    validate_directory_entries(
        &objects,
        BTreeSet::from([OsString::from(DIGEST_DIRECTORY)]),
        1,
    )?;
    let digests = objects.join(DIGEST_DIRECTORY);
    ensure_real_directory(&digests)?;
    let expected = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            Path::new(&artifact.object_path)
                .file_name()
                .map(OsString::from)
                .ok_or_else(|| LocalBundleError::ArtifactPath {
                    role: artifact.role,
                    path: artifact.object_path.clone(),
                })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    validate_directory_entries(&digests, expected, limits.max_artifacts)?;
    Ok(())
}

fn ensure_real_directory(path: &Path) -> Result<(), LocalBundleError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|source| io_error("inspect bundle directory", path, source))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LocalBundleError::UnsafeEntry(path.to_owned()));
    }
    Ok(())
}

fn validate_directory_entries(
    directory: &Path,
    expected: BTreeSet<OsString>,
    max_entries: u64,
) -> Result<(), LocalBundleError> {
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory)
        .map_err(|source| io_error("read bundle directory", directory, source))?
    {
        let entry = entry.map_err(|source| io_error("read bundle entry", directory, source))?;
        actual.insert(entry.file_name());
        if actual.len() as u64 > max_entries {
            return Err(LocalBundleError::DirectoryEntryLimit {
                path: directory.to_owned(),
                limit: max_entries,
            });
        }
    }
    if actual != expected {
        return Err(LocalBundleError::UnexpectedEntries(directory.to_owned()));
    }
    Ok(())
}

fn retained_artifact(
    retained: &BTreeMap<BundleArtifactRole, Vec<u8>>,
    role: BundleArtifactRole,
) -> Result<&[u8], LocalBundleError> {
    retained
        .get(&role)
        .map(Vec::as_slice)
        .ok_or(LocalBundleError::MissingArtifact(role))
}

fn ensure_exact_artifact(
    role: BundleArtifactRole,
    retained: &[u8],
    recomputed: &[u8],
) -> Result<(), LocalBundleError> {
    if retained != recomputed {
        return Err(LocalBundleError::ArtifactSemantics(role));
    }
    Ok(())
}

fn check_capability_count(
    side: &'static str,
    observed: usize,
    limit: u64,
) -> Result<(), LocalBundleError> {
    if observed as u64 > limit {
        return Err(LocalBundleError::CapabilityLimit {
            side,
            observed: observed as u64,
            limit,
        });
    }
    Ok(())
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> LocalBundleError {
    LocalBundleError::Io {
        operation,
        path: path.to_owned(),
        source,
    }
}

struct StagingDirectory {
    path: Option<PathBuf>,
}

impl StagingDirectory {
    fn create(parent: &Path) -> Result<Self, LocalBundleError> {
        for _ in 0..STAGING_ATTEMPTS {
            let id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".rey-proof-staging-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path: Some(path) }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io_error("create staging directory", &path, source)),
            }
        }
        Err(LocalBundleError::StagingExhausted(parent.to_owned()))
    }

    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("staging path exists until disarmed")
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[derive(Debug, Error)]
pub enum LocalBundleError {
    #[error("local bundle limit {0} must be greater than zero")]
    InvalidLimit(&'static str),
    #[error("{side} snapshot contains {observed} capabilities, exceeding bundle limit {limit}")]
    CapabilityLimit {
        side: &'static str,
        observed: u64,
        limit: u64,
    },
    #[error("certificate is stale under the current snapshots or evaluator contract")]
    CertificateStale,
    #[error("bundle contains {observed} artifacts, exceeding limit {limit}")]
    ArtifactCount { observed: u64, limit: u64 },
    #[error("bundle artifact {role:?} contains {observed} bytes, exceeding limit {limit}")]
    ArtifactLimit {
        role: BundleArtifactRole,
        observed: u64,
        limit: u64,
    },
    #[error("bundle artifact byte total overflowed u64")]
    TotalBytesOverflow,
    #[error("bundle contains {observed} logical artifact bytes, exceeding limit {limit}")]
    TotalLimit { observed: u64, limit: u64 },
    #[error("bundle declares {declared} logical artifact bytes but entries contain {actual}")]
    TotalBytes { declared: u64, actual: u64 },
    #[error("invalid local bundle destination {0}")]
    InvalidDestination(PathBuf),
    #[error("local bundle destination already exists and is not the same verified bundle: {0}")]
    DestinationExists(PathBuf),
    #[error("could not allocate a staging directory beneath {0}")]
    StagingExhausted(PathBuf),
    #[error("unsupported local proof bundle schema {0}")]
    UnsupportedSchema(String),
    #[error("local proof bundle retention contract is not canonical")]
    RetentionContract,
    #[error("local proof bundle manifest JSON is not canonical")]
    NonCanonicalManifest,
    #[error("local proof bundle artifacts are missing, duplicated, or out of canonical order")]
    NonCanonicalArtifacts,
    #[error("bundle artifact {0:?} has a non-canonical media type")]
    ArtifactMediaType(BundleArtifactRole),
    #[error("bundle artifact {role:?} has unsafe or non-canonical object path {path}")]
    ArtifactPath {
        role: BundleArtifactRole,
        path: String,
    },
    #[error("invalid BLAKE3 semantic digest {0}")]
    InvalidDigest(String),
    #[error("bundle digest {declared} does not match recomputed {actual}")]
    BundleDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("bundle artifact {role:?} declares {declared} bytes but contains {actual}")]
    ArtifactLength {
        role: BundleArtifactRole,
        declared: u64,
        actual: u64,
    },
    #[error("bundle artifact {role:?} digest {declared} does not match recomputed {actual}")]
    ArtifactDigest {
        role: BundleArtifactRole,
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("bundle artifact {0:?} does not match its recomputed semantic content")]
    ArtifactSemantics(BundleArtifactRole),
    #[error("bundle is missing required artifact {0:?}")]
    MissingArtifact(BundleArtifactRole),
    #[error("bundle is missing {description} file at {path}")]
    MissingFile { description: String, path: PathBuf },
    #[error("bundle manifest {0} identity does not match retained evidence")]
    ManifestIdentity(&'static str),
    #[error("bundle entry is a symlink or has an unsupported filesystem type: {0}")]
    UnsafeEntry(PathBuf),
    #[error("bundle directory {path} exceeds its {limit}-entry bound")]
    DirectoryEntryLimit { path: PathBuf, limit: u64 },
    #[error("bundle directory contains unexpected entries: {0}")]
    UnexpectedEntries(PathBuf),
    #[error("bundle {description} contains {observed} bytes, exceeding limit {limit}")]
    FileLimit {
        description: String,
        observed: u64,
        limit: u64,
    },
    #[error("could not {operation} at {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Snapshot(#[from] rey_environment::DiscoveryError),
    #[error(transparent)]
    Delta(#[from] rey_diff::DeltaError),
    #[error(transparent)]
    Frame(#[from] rey_dataframe::FrameError),
    #[error(transparent)]
    Proof(#[from] crate::ProofError),
    #[error("local bundle JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_diff::DeltaLimits;
    use rey_environment::{
        Availability, CapabilityRecord, DiscoveryLimits, LOCAL_PROVIDER_REVISION, TrustClass,
    };
    use tempfile::TempDir;

    use super::*;
    use crate::{EvaluationOptions, RequiredCapabilitiesClaim, evaluate_required_capabilities};

    fn record(id: &str, availability: Availability) -> CapabilityRecord {
        CapabilityRecord {
            provider_id: "fixture".to_owned(),
            provider_revision: LOCAL_PROVIDER_REVISION,
            provider_kind: "fixture".to_owned(),
            capability_id: id.to_owned(),
            capability_kind: "identity".to_owned(),
            resolved_location: None,
            version: Some("1".to_owned()),
            content_digest: None,
            provenance: Some("fixture".to_owned()),
            availability,
            trust_class: TrustClass::BuiltIn,
            operations: vec!["inspect".to_owned()],
            enforced_limits: Vec::new(),
            unsupported_limits: Vec::new(),
            observed_at: None,
            error_code: (availability == Availability::Error).then(|| "probe".to_owned()),
            error_detail: None,
        }
    }

    fn snapshot(rows: Vec<CapabilityRecord>) -> CapabilitySnapshot {
        CapabilitySnapshot::new(
            "fixture",
            DiscoveryLimits {
                max_capabilities: 16,
                ..DiscoveryLimits::default()
            },
            rows,
        )
        .unwrap()
    }

    fn fixture(
        capability_id: &str,
    ) -> (
        CapabilitySnapshot,
        CapabilitySnapshot,
        RequiredCapabilityCertificate,
    ) {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record(capability_id, Availability::Available)]);
        let claim = RequiredCapabilitiesClaim::new([capability_id.to_owned()]).unwrap();
        let certificate = evaluate_required_capabilities(
            &source,
            &target,
            claim,
            EvaluationOptions {
                delta_limits: DeltaLimits { max_changes: 16 },
                ..EvaluationOptions::default()
            },
        )
        .unwrap();
        (source, target, certificate)
    }

    #[test]
    fn bundle_is_deterministic_idempotent_and_explicitly_local() {
        let directory = TempDir::new().unwrap();
        let (source, target, certificate) = fixture("required");
        let first_path = directory.path().join("first");
        let second_path = directory.path().join("second");
        let limits = LocalBundleLimits::default();

        let first =
            create_local_proof_bundle(&first_path, &source, &target, &certificate, limits.clone())
                .unwrap();
        let second =
            create_local_proof_bundle(&second_path, &source, &target, &certificate, limits.clone())
                .unwrap();
        let replayed =
            create_local_proof_bundle(&first_path, &source, &target, &certificate, limits.clone())
                .unwrap();

        assert_eq!(first, second);
        assert_eq!(first, replayed);
        assert_eq!(
            fs::read(first_path.join(MANIFEST_NAME)).unwrap(),
            fs::read(second_path.join(MANIFEST_NAME)).unwrap()
        );
        assert_eq!(first.artifacts.len(), REQUIRED_ARTIFACT_COUNT as usize);
        assert!(first.retention.content_addressed_objects);
        assert!(first.retention.verification_read_only);
        assert!(!first.retention.process_crash_durable);
        assert!(!first.retention.remote_durable);

        let verification = verify_local_proof_bundle(&first_path, limits).unwrap();
        assert_eq!(verification.status, LocalBundleVerificationStatus::Verified);
        assert_eq!(verification.bundle_id, first.bundle_id);
        assert_eq!(verification.certificate_status, ProofStatus::Passed);
    }

    #[test]
    fn missing_and_digest_tampered_objects_are_rejected() {
        let directory = TempDir::new().unwrap();
        let (source, target, certificate) = fixture("required");
        let missing_path = directory.path().join("missing");
        let tampered_path = directory.path().join("tampered");
        let limits = LocalBundleLimits::default();
        let missing = create_local_proof_bundle(
            &missing_path,
            &source,
            &target,
            &certificate,
            limits.clone(),
        )
        .unwrap();
        let tampered = create_local_proof_bundle(
            &tampered_path,
            &source,
            &target,
            &certificate,
            limits.clone(),
        )
        .unwrap();
        let missing_delta = missing
            .artifacts
            .iter()
            .find(|artifact| artifact.role == BundleArtifactRole::CapabilityDelta)
            .unwrap();
        fs::remove_file(missing_path.join(&missing_delta.object_path)).unwrap();
        assert!(matches!(
            verify_local_proof_bundle(&missing_path, limits.clone()),
            Err(LocalBundleError::UnexpectedEntries(_)) | Err(LocalBundleError::MissingFile { .. })
        ));

        let tampered_delta = tampered
            .artifacts
            .iter()
            .find(|artifact| artifact.role == BundleArtifactRole::CapabilityDelta)
            .unwrap();
        let object = tampered_path.join(&tampered_delta.object_path);
        let mut bytes = fs::read(&object).unwrap();
        bytes[0] ^= 1;
        fs::write(object, bytes).unwrap();
        assert!(matches!(
            verify_local_proof_bundle(&tampered_path, limits),
            Err(LocalBundleError::ArtifactDigest { .. })
        ));
    }

    #[test]
    fn self_consistent_but_semantically_tampered_delta_is_rejected() {
        let directory = TempDir::new().unwrap();
        let bundle_path = directory.path().join("bundle");
        let (source, target, certificate) = fixture("required");
        let limits = LocalBundleLimits::default();
        let mut manifest =
            create_local_proof_bundle(&bundle_path, &source, &target, &certificate, limits.clone())
                .unwrap();
        let delta_entry = manifest
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.role == BundleArtifactRole::CapabilityDelta)
            .unwrap();
        let old_path = bundle_path.join(&delta_entry.object_path);
        let mut delta: CapabilityDelta =
            serde_json::from_slice(&fs::read(&old_path).unwrap()).unwrap();
        delta.summary.unchanged += 1;
        let tampered = json_line(&delta).unwrap();
        let digest = artifact_digest(&tampered);
        let new_path = object_path(&digest).unwrap();
        fs::write(bundle_path.join(&new_path), &tampered).unwrap();
        fs::remove_file(old_path).unwrap();
        manifest.total_artifact_bytes =
            manifest.total_artifact_bytes - delta_entry.bytes + tampered.len() as u64;
        delta_entry.digest = digest;
        delta_entry.bytes = tampered.len() as u64;
        delta_entry.object_path = new_path;
        manifest.bundle_id = bundle_digest(&manifest);
        fs::write(
            bundle_path.join(MANIFEST_NAME),
            json_line(&manifest).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            verify_local_proof_bundle(&bundle_path, limits),
            Err(LocalBundleError::ArtifactSemantics(
                BundleArtifactRole::CapabilityDelta
            ))
        ));
    }

    #[test]
    fn publication_and_verification_limits_fail_closed() {
        let directory = TempDir::new().unwrap();
        let (source, target, certificate) = fixture("required");
        let rejected_path = directory.path().join("rejected");
        let error = create_local_proof_bundle(
            &rejected_path,
            &source,
            &target,
            &certificate,
            LocalBundleLimits {
                max_artifacts: REQUIRED_ARTIFACT_COUNT - 1,
                ..LocalBundleLimits::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, LocalBundleError::ArtifactCount { .. }));
        assert!(!rejected_path.exists());

        let bundle_path = directory.path().join("bundle");
        create_local_proof_bundle(
            &bundle_path,
            &source,
            &target,
            &certificate,
            LocalBundleLimits::default(),
        )
        .unwrap();
        let error = verify_local_proof_bundle(
            &bundle_path,
            LocalBundleLimits {
                max_total_bytes: 1,
                ..LocalBundleLimits::default()
            },
        )
        .unwrap_err();
        assert!(matches!(error, LocalBundleError::TotalLimit { .. }));
    }

    #[test]
    fn an_existing_different_bundle_is_never_overwritten() {
        let directory = TempDir::new().unwrap();
        let bundle_path = directory.path().join("bundle");
        let (source, target, certificate) = fixture("first");
        let first = create_local_proof_bundle(
            &bundle_path,
            &source,
            &target,
            &certificate,
            LocalBundleLimits::default(),
        )
        .unwrap();
        let (other_source, other_target, other_certificate) = fixture("second");

        let error = create_local_proof_bundle(
            &bundle_path,
            &other_source,
            &other_target,
            &other_certificate,
            LocalBundleLimits::default(),
        )
        .unwrap_err();
        assert!(matches!(error, LocalBundleError::DestinationExists(_)));
        let verified =
            verify_local_proof_bundle(&bundle_path, LocalBundleLimits::default()).unwrap();
        assert_eq!(verified.bundle_id, first.bundle_id);
    }

    #[test]
    fn manifest_paths_cannot_escape_the_bundle() {
        let directory = TempDir::new().unwrap();
        let bundle_path = directory.path().join("bundle");
        let (source, target, certificate) = fixture("required");
        let mut manifest = create_local_proof_bundle(
            &bundle_path,
            &source,
            &target,
            &certificate,
            LocalBundleLimits::default(),
        )
        .unwrap();
        manifest.artifacts[0].object_path = "../outside".to_owned();
        manifest.bundle_id = bundle_digest(&manifest);
        fs::write(
            bundle_path.join(MANIFEST_NAME),
            json_line(&manifest).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            verify_local_proof_bundle(&bundle_path, LocalBundleLimits::default()),
            Err(LocalBundleError::ArtifactPath { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_bundle_objects_are_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let bundle_path = directory.path().join("bundle");
        let outside = directory.path().join("outside");
        fs::write(&outside, b"outside").unwrap();
        let (source, target, certificate) = fixture("required");
        let manifest = create_local_proof_bundle(
            &bundle_path,
            &source,
            &target,
            &certificate,
            LocalBundleLimits::default(),
        )
        .unwrap();
        let delta = manifest
            .artifacts
            .iter()
            .find(|artifact| artifact.role == BundleArtifactRole::CapabilityDelta)
            .unwrap();
        let object = bundle_path.join(&delta.object_path);
        fs::remove_file(&object).unwrap();
        symlink(&outside, &object).unwrap();

        assert!(matches!(
            verify_local_proof_bundle(&bundle_path, LocalBundleLimits::default()),
            Err(LocalBundleError::UnsafeEntry(_))
        ));
    }
}
