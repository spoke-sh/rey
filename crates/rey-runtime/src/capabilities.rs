use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryError, DiscoveryLimits,
    TrustClass, builtin_source_search_operation, source_search_capability_identity,
};

pub const RUNTIME_CAPABILITY_PROFILE: &str = "rey-runtime";

pub fn runtime_capability_snapshot() -> Result<CapabilitySnapshot, DiscoveryError> {
    CapabilitySnapshot::new(
        RUNTIME_CAPABILITY_PROFILE,
        DiscoveryLimits {
            max_capabilities: 2,
            ..DiscoveryLimits::default()
        },
        vec![arrow_frame_capability(), source_search_capability()],
    )
}

fn arrow_frame_capability() -> CapabilityRecord {
    CapabilityRecord {
        provider_id: "rey.runtime".to_owned(),
        provider_revision: 1,
        provider_kind: "intrinsic_runtime".to_owned(),
        capability_id: "frame.arrow-stream".to_owned(),
        capability_kind: "typed_frame".to_owned(),
        resolved_location: None,
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        content_digest: None,
        provenance: Some("compiled_rey_runtime".to_owned()),
        availability: Availability::Available,
        trust_class: TrustClass::BuiltIn,
        operations: vec!["encode_arrow_stream".to_owned(), "render_table".to_owned()],
        enforced_limits: vec!["bounded_input_rows".to_owned()],
        unsupported_limits: Vec::new(),
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

fn source_search_capability() -> CapabilityRecord {
    let operation = builtin_source_search_operation();
    let capability = source_search_capability_identity();
    CapabilityRecord {
        provider_id: operation.implementation.id.clone(),
        provider_revision: operation.implementation.revision,
        provider_kind: "intrinsic_runtime".to_owned(),
        capability_id: capability.id,
        capability_kind: "source_mining".to_owned(),
        resolved_location: None,
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        content_digest: Some(capability.semantic_digest.to_string()),
        provenance: Some("compiled_deterministic_baseline".to_owned()),
        availability: Availability::Available,
        trust_class: TrustClass::BuiltIn,
        operations: vec![
            "bind_explicit_source_corpus".to_owned(),
            "search_literal_utf8".to_owned(),
            format!(
                "{}@{}#{}",
                operation.operation.id,
                operation.operation.revision,
                operation.operation.semantic_digest
            ),
        ],
        enforced_limits: vec![
            "canonical_request_root".to_owned(),
            "context_lines".to_owned(),
            "explicit_file_set".to_owned(),
            "file_bytes".to_owned(),
            "match_rows".to_owned(),
            "no_symlink_traversal".to_owned(),
            "total_bytes".to_owned(),
        ],
        unsupported_limits: vec![
            "filesystem_sandbox".to_owned(),
            "generated_file_policy".to_owned(),
            "ignore_file_semantics".to_owned(),
            "regex".to_owned(),
        ],
        observed_at: None,
        error_code: None,
        error_detail: None,
    }
}

#[cfg(test)]
mod tests {
    use super::runtime_capability_snapshot;

    #[test]
    fn runtime_capabilities_are_intrinsic_and_workspace_independent() {
        let snapshot = runtime_capability_snapshot().unwrap();
        assert_eq!(snapshot.profile, "rey-runtime");
        assert_eq!(
            snapshot
                .capabilities
                .iter()
                .map(|capability| capability.capability_id.as_str())
                .collect::<Vec<_>>(),
            ["source.search.literal-utf8", "frame.arrow-stream"]
        );
        assert!(
            snapshot
                .capabilities
                .iter()
                .all(|capability| capability.resolved_location.is_none())
        );
        assert_eq!(
            snapshot.semantic_digest,
            runtime_capability_snapshot().unwrap().semantic_digest
        );
    }
}
