#![forbid(unsafe_code)]

mod bundle;

pub use bundle::{
    BundleArtifactRole, LocalBundleArtifact, LocalBundleError, LocalBundleLimits,
    LocalBundleVerification, LocalBundleVerificationStatus, LocalProofBundleManifest,
    LocalRetentionContract, create_local_proof_bundle, verify_local_proof_bundle,
};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_diff::{
    CapabilityKey, DeltaError, DeltaLimits, DeltaOptions, capability_comparator,
    compare_capabilities,
};
use rey_environment::{Availability, CapabilitySnapshot};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const REQUIRED_CAPABILITY_CLAIM: &str = "rey.environment.required-capabilities.v1";
pub const REQUIRED_CAPABILITY_CERTIFICATE_SCHEMA: &str = "rey.required-capability-certificate.v1";
pub const CERTIFICATE_VERIFICATION_SCHEMA: &str = "rey.certificate-verification.v1";

const EVALUATOR_ID: &str = "rey.required-capability-evaluator";
const EVALUATOR_REVISION: u64 = 1;
const EVALUATOR_DEFINITION: &str = "target capability-id existential availability; available passes, known absence or unavailable fails, error or incomplete unknown absence is inconclusive; conjunction fails on any false then is inconclusive on any unknown";

#[must_use]
pub fn required_capability_evaluator() -> ContractIdentity {
    ContractIdentity::new(EVALUATOR_ID, EVALUATOR_REVISION, EVALUATOR_DEFINITION)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequiredCapabilitiesClaim {
    pub claim: String,
    pub required_capability_ids: Vec<String>,
}

impl RequiredCapabilitiesClaim {
    pub fn new(
        required_capability_ids: impl IntoIterator<Item = String>,
    ) -> Result<Self, ProofError> {
        let mut required_capability_ids = required_capability_ids.into_iter().collect::<Vec<_>>();
        required_capability_ids.sort();
        required_capability_ids.dedup();
        let claim = Self {
            claim: REQUIRED_CAPABILITY_CLAIM.to_owned(),
            required_capability_ids,
        };
        claim.validate()?;
        Ok(claim)
    }

    fn validate(&self) -> Result<(), ProofError> {
        if self.claim != REQUIRED_CAPABILITY_CLAIM {
            return Err(ProofError::UnsupportedClaim(self.claim.clone()));
        }
        if self.required_capability_ids.is_empty() {
            return Err(ProofError::EmptyRequirements);
        }
        if self
            .required_capability_ids
            .windows(2)
            .any(|ids| ids[0] >= ids[1])
        {
            return Err(ProofError::NonCanonicalRequirements);
        }
        if self
            .required_capability_ids
            .iter()
            .any(|id| id.is_empty() || id.chars().count() > 256 || id.chars().any(char::is_control))
        {
            return Err(ProofError::InvalidRequirement);
        }
        Ok(())
    }

    fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(&self.claim);
        hasher.add_u64(self.required_capability_ids.len() as u64);
        for capability_id in &self.required_capability_ids {
            hasher.add_str(capability_id);
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Passed,
    Failed,
    Inconclusive,
}

impl ProofStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Passed,
    Failed,
    Inconclusive,
}

impl RequirementStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityMatch {
    pub key: CapabilityKey,
    pub availability: Availability,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequirementCheck {
    pub capability_id: String,
    pub status: RequirementStatus,
    pub matches: Vec<CapabilityMatch>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RequiredCapabilityCertificate {
    pub schema: String,
    pub certificate_id: SemanticDigest,
    pub input_digest: SemanticDigest,
    pub source_snapshot: SemanticDigest,
    pub target_snapshot: SemanticDigest,
    pub delta_id: SemanticDigest,
    pub source_label: String,
    pub target_label: String,
    pub delta_limits: DeltaLimits,
    pub comparator: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub claim: RequiredCapabilitiesClaim,
    pub status: ProofStatus,
    pub checks: Vec<RequirementCheck>,
}

#[derive(Clone, Debug)]
pub struct EvaluationOptions {
    pub source_label: String,
    pub target_label: String,
    pub delta_limits: DeltaLimits,
    pub comparator: ContractIdentity,
    pub evaluator: ContractIdentity,
}

impl Default for EvaluationOptions {
    fn default() -> Self {
        Self {
            source_label: "SOURCE".to_owned(),
            target_label: "TARGET".to_owned(),
            delta_limits: DeltaLimits::default(),
            comparator: capability_comparator(),
            evaluator: required_capability_evaluator(),
        }
    }
}

pub fn evaluate_required_capabilities(
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    claim: RequiredCapabilitiesClaim,
    options: EvaluationOptions,
) -> Result<RequiredCapabilityCertificate, ProofError> {
    claim.validate()?;
    let delta = compare_capabilities(
        source,
        target,
        DeltaOptions {
            source_label: options.source_label.clone(),
            target_label: options.target_label.clone(),
            limits: options.delta_limits.clone(),
            comparator: options.comparator.clone(),
        },
    )?;
    let checks = claim
        .required_capability_ids
        .iter()
        .map(|capability_id| check_requirement(target, capability_id))
        .collect::<Vec<_>>();
    let status = if checks
        .iter()
        .any(|check| check.status == RequirementStatus::Failed)
    {
        ProofStatus::Failed
    } else if checks
        .iter()
        .any(|check| check.status == RequirementStatus::Inconclusive)
    {
        ProofStatus::Inconclusive
    } else {
        ProofStatus::Passed
    };
    let input_digest = input_digest(
        source.semantic_digest.as_str(),
        target.semantic_digest.as_str(),
        delta.delta_id.as_str(),
        &options.source_label,
        &options.target_label,
        &options.delta_limits,
        &options.comparator,
        &options.evaluator,
        &claim,
    );
    let mut certificate = RequiredCapabilityCertificate {
        schema: REQUIRED_CAPABILITY_CERTIFICATE_SCHEMA.to_owned(),
        certificate_id: SemanticHasher::new("rey.uninitialized-certificate").finish(),
        input_digest,
        source_snapshot: source.semantic_digest.clone(),
        target_snapshot: target.semantic_digest.clone(),
        delta_id: delta.delta_id,
        source_label: options.source_label,
        target_label: options.target_label,
        delta_limits: options.delta_limits,
        comparator: options.comparator,
        evaluator: options.evaluator,
        claim,
        status,
        checks,
    };
    certificate.certificate_id = certificate_digest(&certificate);
    Ok(certificate)
}

fn check_requirement(target: &CapabilitySnapshot, capability_id: &str) -> RequirementCheck {
    let matches = target
        .capabilities
        .iter()
        .filter(|row| row.capability_id == capability_id)
        .map(|row| CapabilityMatch {
            key: CapabilityKey::from(row),
            availability: row.availability,
        })
        .collect::<Vec<_>>();
    let status = if matches
        .iter()
        .any(|candidate| candidate.availability == Availability::Available)
    {
        RequirementStatus::Passed
    } else if matches
        .iter()
        .any(|candidate| candidate.availability == Availability::Error)
        || (matches.is_empty() && !target.complete)
    {
        RequirementStatus::Inconclusive
    } else {
        RequirementStatus::Failed
    };
    RequirementCheck {
        capability_id: capability_id.to_owned(),
        status,
        matches,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Stale,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    SourceSnapshot,
    TargetSnapshot,
    Comparator,
    Evaluator,
    Delta,
    EvaluationInput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CertificateVerification {
    pub schema: String,
    pub certificate_id: SemanticDigest,
    pub status: VerificationStatus,
    pub stale_reasons: Vec<StaleReason>,
    pub recomputed_input_digest: Option<SemanticDigest>,
}

pub fn verify_required_capability_certificate(
    certificate: &RequiredCapabilityCertificate,
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    current_evaluator: ContractIdentity,
) -> Result<CertificateVerification, ProofError> {
    validate_certificate(certificate)?;
    let mut stale_reasons = Vec::new();
    if certificate.source_snapshot != source.semantic_digest {
        stale_reasons.push(StaleReason::SourceSnapshot);
    }
    if certificate.target_snapshot != target.semantic_digest {
        stale_reasons.push(StaleReason::TargetSnapshot);
    }
    if certificate.comparator != capability_comparator() {
        stale_reasons.push(StaleReason::Comparator);
    }
    if certificate.evaluator != current_evaluator {
        stale_reasons.push(StaleReason::Evaluator);
    }

    let snapshots_changed = stale_reasons.iter().any(|reason| {
        matches!(
            reason,
            StaleReason::SourceSnapshot | StaleReason::TargetSnapshot
        )
    });
    let current = match evaluate_required_capabilities(
        source,
        target,
        certificate.claim.clone(),
        EvaluationOptions {
            source_label: certificate.source_label.clone(),
            target_label: certificate.target_label.clone(),
            delta_limits: certificate.delta_limits.clone(),
            comparator: capability_comparator(),
            evaluator: current_evaluator.clone(),
        },
    ) {
        Ok(current) => Some(current),
        Err(ProofError::Delta(DeltaError::ChangeLimit { .. })) if snapshots_changed => None,
        Err(error) => return Err(error),
    };
    if let Some(current) = &current {
        if certificate.delta_id != current.delta_id {
            stale_reasons.push(StaleReason::Delta);
        }
        if certificate.input_digest != current.input_digest {
            stale_reasons.push(StaleReason::EvaluationInput);
        }
    }
    stale_reasons.sort();
    stale_reasons.dedup();
    let status = if stale_reasons.is_empty() {
        let current = current
            .as_ref()
            .expect("an unchanged bounded certificate always re-evaluates");
        if certificate != current {
            return Err(ProofError::InconsistentEvaluation);
        }
        VerificationStatus::Verified
    } else {
        VerificationStatus::Stale
    };
    Ok(CertificateVerification {
        schema: CERTIFICATE_VERIFICATION_SCHEMA.to_owned(),
        certificate_id: certificate.certificate_id.clone(),
        status,
        stale_reasons,
        recomputed_input_digest: current.map(|current| current.input_digest),
    })
}

fn validate_certificate(certificate: &RequiredCapabilityCertificate) -> Result<(), ProofError> {
    if certificate.schema != REQUIRED_CAPABILITY_CERTIFICATE_SCHEMA {
        return Err(ProofError::UnsupportedCertificateSchema(
            certificate.schema.clone(),
        ));
    }
    certificate.claim.validate()?;
    let expected = certificate_digest(certificate);
    if certificate.certificate_id != expected {
        return Err(ProofError::CertificateDigest {
            declared: certificate.certificate_id.clone(),
            actual: expected,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn input_digest(
    source_snapshot: &str,
    target_snapshot: &str,
    delta_id: &str,
    source_label: &str,
    target_label: &str,
    delta_limits: &DeltaLimits,
    comparator: &ContractIdentity,
    evaluator: &ContractIdentity,
    claim: &RequiredCapabilitiesClaim,
) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.required-capability-input.v1");
    hasher.add_str(source_snapshot);
    hasher.add_str(target_snapshot);
    hasher.add_str(delta_id);
    hasher.add_str(source_label);
    hasher.add_str(target_label);
    hasher.add_u64(delta_limits.max_changes);
    comparator.add_semantics(&mut hasher);
    evaluator.add_semantics(&mut hasher);
    claim.add_semantics(&mut hasher);
    hasher.finish()
}

fn certificate_digest(certificate: &RequiredCapabilityCertificate) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(REQUIRED_CAPABILITY_CERTIFICATE_SCHEMA);
    hasher.add_str(&certificate.schema);
    hasher.add_str(certificate.input_digest.as_str());
    hasher.add_str(certificate.source_snapshot.as_str());
    hasher.add_str(certificate.target_snapshot.as_str());
    hasher.add_str(certificate.delta_id.as_str());
    hasher.add_str(&certificate.source_label);
    hasher.add_str(&certificate.target_label);
    hasher.add_u64(certificate.delta_limits.max_changes);
    certificate.comparator.add_semantics(&mut hasher);
    certificate.evaluator.add_semantics(&mut hasher);
    certificate.claim.add_semantics(&mut hasher);
    hasher.add_str(certificate.status.as_str());
    hasher.add_u64(certificate.checks.len() as u64);
    for check in &certificate.checks {
        hasher.add_str(&check.capability_id);
        hasher.add_str(check.status.as_str());
        hasher.add_u64(check.matches.len() as u64);
        for candidate in &check.matches {
            hasher.add_str(&candidate.key.provider_id);
            hasher.add_u64(candidate.key.provider_revision);
            hasher.add_str(&candidate.key.capability_id);
            hasher.add_str(candidate.availability.as_str());
        }
    }
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("unsupported proof claim {0}")]
    UnsupportedClaim(String),
    #[error("at least one required capability must be supplied")]
    EmptyRequirements,
    #[error("required capability ids must be sorted and unique")]
    NonCanonicalRequirements,
    #[error("required capability ids must contain 1-256 non-control characters")]
    InvalidRequirement,
    #[error("unsupported certificate schema {0}")]
    UnsupportedCertificateSchema(String),
    #[error("certificate digest {declared} does not match recomputed {actual}")]
    CertificateDigest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("certificate evaluation differs despite identical bound inputs")]
    InconsistentEvaluation,
    #[error(transparent)]
    Delta(#[from] DeltaError),
}

#[cfg(test)]
mod tests {
    use rey_environment::{CapabilityRecord, DiscoveryLimits, LOCAL_PROVIDER_REVISION, TrustClass};

    use super::*;

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
            operations: Vec::new(),
            enforced_limits: Vec::new(),
            unsupported_limits: Vec::new(),
            observed_at: None,
            error_code: (availability == Availability::Error).then(|| "probe".to_owned()),
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
    fn certificate_passes_and_verifies_for_bound_inputs() {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record("tool.git.identity", Availability::Available)]);
        let claim = RequiredCapabilitiesClaim::new(["tool.git.identity".to_owned()]).unwrap();
        let certificate =
            evaluate_required_capabilities(&source, &target, claim, EvaluationOptions::default())
                .unwrap();

        assert_eq!(certificate.status, ProofStatus::Passed);
        let verification = verify_required_capability_certificate(
            &certificate,
            &source,
            &target,
            required_capability_evaluator(),
        )
        .unwrap();
        assert_eq!(verification.status, VerificationStatus::Verified);
    }

    #[test]
    fn snapshot_or_evaluator_changes_make_certificate_stale() {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record("tool.git.identity", Availability::Available)]);
        let claim = RequiredCapabilitiesClaim::new(["tool.git.identity".to_owned()]).unwrap();
        let certificate =
            evaluate_required_capabilities(&source, &target, claim, EvaluationOptions::default())
                .unwrap();
        let changed_target = snapshot(vec![record("tool.git.identity", Availability::Unavailable)]);
        let verification = verify_required_capability_certificate(
            &certificate,
            &source,
            &changed_target,
            required_capability_evaluator(),
        )
        .unwrap();
        assert_eq!(verification.status, VerificationStatus::Stale);
        assert!(
            verification
                .stale_reasons
                .contains(&StaleReason::TargetSnapshot)
        );

        let changed_source = snapshot(vec![record("source-drift", Availability::Available)]);
        let verification = verify_required_capability_certificate(
            &certificate,
            &changed_source,
            &target,
            required_capability_evaluator(),
        )
        .unwrap();
        assert_eq!(verification.status, VerificationStatus::Stale);
        assert!(
            verification
                .stale_reasons
                .contains(&StaleReason::SourceSnapshot)
        );

        let changed_evaluator = ContractIdentity::new(
            EVALUATOR_ID,
            EVALUATOR_REVISION + 1,
            "fixture changed semantics",
        );
        let verification = verify_required_capability_certificate(
            &certificate,
            &source,
            &target,
            changed_evaluator,
        )
        .unwrap();
        assert_eq!(verification.status, VerificationStatus::Stale);
        assert!(verification.stale_reasons.contains(&StaleReason::Evaluator));
    }

    #[test]
    fn snapshot_growth_beyond_the_original_delta_limit_is_stale() {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record("required", Availability::Available)]);
        let claim = RequiredCapabilitiesClaim::new(["required".to_owned()]).unwrap();
        let certificate = evaluate_required_capabilities(
            &source,
            &target,
            claim,
            EvaluationOptions {
                delta_limits: DeltaLimits { max_changes: 1 },
                ..EvaluationOptions::default()
            },
        )
        .unwrap();
        let changed_target = snapshot(vec![
            record("additional", Availability::Available),
            record("required", Availability::Available),
        ]);

        let verification = verify_required_capability_certificate(
            &certificate,
            &source,
            &changed_target,
            required_capability_evaluator(),
        )
        .unwrap();

        assert_eq!(verification.status, VerificationStatus::Stale);
        assert!(
            verification
                .stale_reasons
                .contains(&StaleReason::TargetSnapshot)
        );
        assert_eq!(verification.recomputed_input_digest, None);
    }

    #[test]
    fn damaged_certificate_is_invalid_not_stale() {
        let source = snapshot(Vec::new());
        let target = snapshot(vec![record("required", Availability::Available)]);
        let claim = RequiredCapabilitiesClaim::new(["required".to_owned()]).unwrap();
        let mut certificate =
            evaluate_required_capabilities(&source, &target, claim, EvaluationOptions::default())
                .unwrap();
        certificate.status = ProofStatus::Failed;

        let result = verify_required_capability_certificate(
            &certificate,
            &source,
            &target,
            required_capability_evaluator(),
        );
        assert!(matches!(result, Err(ProofError::CertificateDigest { .. })));
    }
}
