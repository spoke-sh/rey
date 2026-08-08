use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DeltaAssessment;

pub const SCENARIO_OUTPUT_DELTA_SCHEMA: &str = "rey.scenario-output-delta.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioValueType {
    Utf8,
}

impl ScenarioValueType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf8",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioDeltaInputs {
    pub workload: ContractIdentity,
    pub graph: ContractIdentity,
    pub scenario: ContractIdentity,
    pub output_id: String,
    pub comparator: ContractIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioDeltaLimits {
    pub max_value_bytes: u64,
    pub max_string_bytes: u64,
}

impl Default for ScenarioDeltaLimits {
    fn default() -> Self {
        Self {
            max_value_bytes: 64 * 1_024,
            max_string_bytes: 256 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScenarioOutputDelta {
    pub schema: String,
    pub delta_id: SemanticDigest,
    pub inputs: ScenarioDeltaInputs,
    pub value_type: ScenarioValueType,
    pub expected: String,
    pub observed: String,
    pub assessment: DeltaAssessment,
    pub limits: ScenarioDeltaLimits,
}

impl ScenarioOutputDelta {
    pub fn verify(&self) -> Result<(), ScenarioDeltaError> {
        if self.schema != SCENARIO_OUTPUT_DELTA_SCHEMA {
            return Err(ScenarioDeltaError::UnsupportedSchema {
                expected: SCENARIO_OUTPUT_DELTA_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        let recomputed = compare_scenario_utf8(
            self.inputs.clone(),
            self.expected.clone(),
            self.observed.clone(),
            self.limits.clone(),
        )?;
        if self.delta_id != recomputed.delta_id {
            return Err(ScenarioDeltaError::Digest {
                declared: self.delta_id.clone(),
                actual: recomputed.delta_id,
            });
        }
        if self.assessment != recomputed.assessment {
            return Err(ScenarioDeltaError::Assessment);
        }
        Ok(())
    }
}

pub fn compare_scenario_utf8(
    inputs: ScenarioDeltaInputs,
    expected: String,
    observed: String,
    limits: ScenarioDeltaLimits,
) -> Result<ScenarioOutputDelta, ScenarioDeltaError> {
    validate_limits(&limits)?;
    validate_contract("workload", &inputs.workload)?;
    validate_contract("graph", &inputs.graph)?;
    validate_contract("scenario", &inputs.scenario)?;
    validate_contract("comparator", &inputs.comparator)?;
    validate_text("output id", &inputs.output_id)?;
    validate_value("expected", &expected, limits.max_value_bytes)?;
    validate_value("observed", &observed, limits.max_value_bytes)?;

    let string_bytes = contract_string_bytes(&inputs.workload)
        .checked_add(contract_string_bytes(&inputs.graph))
        .and_then(|value| value.checked_add(contract_string_bytes(&inputs.scenario)))
        .and_then(|value| value.checked_add(contract_string_bytes(&inputs.comparator)))
        .and_then(|value| value.checked_add(inputs.output_id.len() as u64))
        .and_then(|value| value.checked_add(expected.len() as u64))
        .and_then(|value| value.checked_add(observed.len() as u64))
        .ok_or(ScenarioDeltaError::CountOverflow)?;
    if string_bytes > limits.max_string_bytes {
        return Err(ScenarioDeltaError::StringByteLimit {
            limit: limits.max_string_bytes,
            observed: string_bytes,
        });
    }

    let assessment = if expected == observed {
        DeltaAssessment::Equal
    } else {
        DeltaAssessment::Different
    };
    let mut delta = ScenarioOutputDelta {
        schema: SCENARIO_OUTPUT_DELTA_SCHEMA.to_owned(),
        delta_id: SemanticHasher::new("rey.scenario-output-delta.placeholder").finish(),
        inputs,
        value_type: ScenarioValueType::Utf8,
        expected,
        observed,
        assessment,
        limits,
    };
    delta.delta_id = delta_digest(&delta);
    Ok(delta)
}

fn validate_limits(limits: &ScenarioDeltaLimits) -> Result<(), ScenarioDeltaError> {
    if limits.max_value_bytes == 0 || limits.max_string_bytes == 0 {
        return Err(ScenarioDeltaError::InvalidLimit);
    }
    Ok(())
}

fn validate_contract(
    role: &'static str,
    contract: &ContractIdentity,
) -> Result<(), ScenarioDeltaError> {
    validate_text(role, &contract.id)?;
    if contract.revision == 0 {
        return Err(ScenarioDeltaError::InvalidContract { role });
    }
    validate_digest(&contract.semantic_digest)
}

fn validate_text(role: &'static str, value: &str) -> Result<(), ScenarioDeltaError> {
    if value.is_empty() || value.contains('\0') {
        return Err(ScenarioDeltaError::InvalidText { role });
    }
    Ok(())
}

fn validate_value(role: &'static str, value: &str, limit: u64) -> Result<(), ScenarioDeltaError> {
    if value.len() as u64 > limit {
        return Err(ScenarioDeltaError::ValueByteLimit {
            role,
            limit,
            observed: value.len() as u64,
        });
    }
    Ok(())
}

fn validate_digest(digest: &SemanticDigest) -> Result<(), ScenarioDeltaError> {
    let value = digest.as_str();
    if value.len() != "blake3:".len() + 64
        || !value.starts_with("blake3:")
        || !value["blake3:".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ScenarioDeltaError::InvalidDigest(value.to_owned()));
    }
    Ok(())
}

fn contract_string_bytes(contract: &ContractIdentity) -> u64 {
    contract.id.len() as u64 + contract.semantic_digest.as_str().len() as u64
}

fn add_contract(hasher: &mut SemanticHasher, contract: &ContractIdentity) {
    hasher.add_str(&contract.id);
    hasher.add_u64(contract.revision);
    hasher.add_str(contract.semantic_digest.as_str());
}

fn delta_digest(delta: &ScenarioOutputDelta) -> SemanticDigest {
    let mut hasher = SemanticHasher::new(SCENARIO_OUTPUT_DELTA_SCHEMA);
    add_contract(&mut hasher, &delta.inputs.workload);
    add_contract(&mut hasher, &delta.inputs.graph);
    add_contract(&mut hasher, &delta.inputs.scenario);
    hasher.add_str(&delta.inputs.output_id);
    add_contract(&mut hasher, &delta.inputs.comparator);
    hasher.add_str(delta.value_type.as_str());
    hasher.add_str(&delta.expected);
    hasher.add_str(&delta.observed);
    hasher.add_str(delta.assessment.as_str());
    hasher.add_u64(delta.limits.max_value_bytes);
    hasher.add_u64(delta.limits.max_string_bytes);
    hasher.finish()
}

#[derive(Debug, Error)]
pub enum ScenarioDeltaError {
    #[error("scenario delta limits must be greater than zero")]
    InvalidLimit,
    #[error("invalid {role} contract")]
    InvalidContract { role: &'static str },
    #[error("invalid {role} text")]
    InvalidText { role: &'static str },
    #[error("invalid semantic digest {0}")]
    InvalidDigest(String),
    #[error("scenario {role} exceeds the {limit}-byte limit with {observed} bytes")]
    ValueByteLimit {
        role: &'static str,
        limit: u64,
        observed: u64,
    },
    #[error("scenario delta string-byte limit {limit} exceeded by {observed}")]
    StringByteLimit { limit: u64, observed: u64 },
    #[error("scenario delta count overflowed")]
    CountOverflow,
    #[error("unsupported scenario delta schema {actual}; expected {expected}")]
    UnsupportedSchema {
        expected: &'static str,
        actual: String,
    },
    #[error("scenario delta digest mismatch: declared {declared}, actual {actual}")]
    Digest {
        declared: SemanticDigest,
        actual: SemanticDigest,
    },
    #[error("scenario delta assessment does not match expected and observed values")]
    Assessment,
}

#[cfg(test)]
mod tests {
    use rey_core::{ContractIdentity, SemanticHasher};

    use super::{ScenarioDeltaInputs, ScenarioDeltaLimits, compare_scenario_utf8};
    use crate::DeltaAssessment;

    fn contract(id: &str) -> ContractIdentity {
        ContractIdentity::new(id, 1, id)
    }

    fn inputs() -> ScenarioDeltaInputs {
        ScenarioDeltaInputs {
            workload: contract("workload"),
            graph: contract("graph"),
            scenario: contract("scenario"),
            output_id: "text".to_owned(),
            comparator: contract("utf8-exact"),
        }
    }

    #[test]
    fn equal_and_different_values_are_typed_and_verified() {
        let equal = compare_scenario_utf8(
            inputs(),
            "REY".to_owned(),
            "REY".to_owned(),
            ScenarioDeltaLimits::default(),
        )
        .unwrap();
        assert_eq!(equal.assessment, DeltaAssessment::Equal);
        equal.verify().unwrap();

        let different = compare_scenario_utf8(
            inputs(),
            "REY".to_owned(),
            " REY ".to_owned(),
            ScenarioDeltaLimits::default(),
        )
        .unwrap();
        assert_eq!(different.assessment, DeltaAssessment::Different);
        different.verify().unwrap();
        assert_ne!(equal.delta_id, different.delta_id);
    }

    #[test]
    fn tampering_and_bounds_fail_closed() {
        let mut delta = compare_scenario_utf8(
            inputs(),
            "REY".to_owned(),
            "REY".to_owned(),
            ScenarioDeltaLimits::default(),
        )
        .unwrap();
        delta.delta_id = SemanticHasher::new("tampered").finish();
        assert!(delta.verify().is_err());

        assert!(
            compare_scenario_utf8(
                inputs(),
                "too large".to_owned(),
                String::new(),
                ScenarioDeltaLimits {
                    max_value_bytes: 1,
                    max_string_bytes: 1_024,
                },
            )
            .is_err()
        );
    }
}
