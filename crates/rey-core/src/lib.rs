#![forbid(unsafe_code)]

use std::fmt;

use serde::{Deserialize, Serialize};

/// A versioned semantic contract used to invalidate derived artifacts when an
/// evaluator or comparator definition changes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContractIdentity {
    pub id: String,
    pub revision: u64,
    pub semantic_digest: SemanticDigest,
}

impl ContractIdentity {
    #[must_use]
    pub fn new(id: impl Into<String>, revision: u64, definition: &str) -> Self {
        let id = id.into();
        let mut hasher = SemanticHasher::new("rey.contract-identity.v1");
        hasher.add_str(&id);
        hasher.add_u64(revision);
        hasher.add_str(definition);
        Self {
            id,
            revision,
            semantic_digest: hasher.finish(),
        }
    }

    pub fn add_semantics(&self, hasher: &mut SemanticHasher) {
        hasher.add_str(&self.id);
        hasher.add_u64(self.revision);
        hasher.add_str(self.semantic_digest.as_str());
    }
}

/// A BLAKE3 digest over an explicitly framed semantic payload.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SemanticDigest(String);

impl SemanticDigest {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SemanticDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Length-frames fields before hashing so concatenation cannot create an
/// ambiguous identity. Callers remain responsible for a stable field order.
pub struct SemanticHasher(blake3::Hasher);

impl SemanticHasher {
    #[must_use]
    pub fn new(domain: &str) -> Self {
        let mut hasher = blake3::Hasher::new();
        add_framed(&mut hasher, domain.as_bytes());
        Self(hasher)
    }

    pub fn add_bytes(&mut self, value: &[u8]) {
        add_framed(&mut self.0, value);
    }

    pub fn add_str(&mut self, value: &str) {
        self.add_bytes(value.as_bytes());
    }

    pub fn add_optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.add_bytes(&[1]);
                self.add_str(value);
            }
            None => self.add_bytes(&[0]),
        }
    }

    pub fn add_u64(&mut self, value: u64) {
        self.add_bytes(&value.to_le_bytes());
    }

    pub fn add_bool(&mut self, value: bool) {
        self.add_bytes(&[u8::from(value)]);
    }

    #[must_use]
    pub fn finish(self) -> SemanticDigest {
        SemanticDigest(format!("blake3:{}", self.0.finalize().to_hex()))
    }
}

fn add_framed(hasher: &mut blake3::Hasher, value: &[u8]) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value);
}

#[cfg(test)]
mod tests {
    use super::SemanticHasher;

    #[test]
    fn length_framing_distinguishes_concatenations() {
        let mut left = SemanticHasher::new("test.v1");
        left.add_str("ab");
        left.add_str("c");

        let mut right = SemanticHasher::new("test.v1");
        right.add_str("a");
        right.add_str("bc");

        assert_ne!(left.finish(), right.finish());
    }

    #[test]
    fn domains_separate_equal_payloads() {
        let mut left = SemanticHasher::new("left.v1");
        left.add_str("payload");
        let mut right = SemanticHasher::new("right.v1");
        right.add_str("payload");

        assert_ne!(left.finish(), right.finish());
    }
}
