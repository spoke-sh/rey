use super::*;

fn digest(label: &str) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.mining.test");
    hasher.add_str(label);
    hasher.finish()
}

fn contract(id: &str) -> ContractIdentity {
    ContractIdentity::new(id, 1, &format!("{id} definition"))
}

fn artifact_contract(
    port_id: &str,
    kind: MiningArtifactKind,
    required: bool,
) -> MiningArtifactContract {
    MiningArtifactContract {
        port_id: port_id.to_owned(),
        kind,
        schema: (kind != MiningArtifactKind::Native).then(|| contract("schema.mining-artifact")),
        media_type: Some(
            match kind {
                MiningArtifactKind::Native => "text/plain",
                MiningArtifactKind::Relation => "application/vnd.apache.arrow.stream",
                _ => "application/octet-stream",
            }
            .to_owned(),
        ),
        required,
    }
}

fn operation() -> MiningOperation {
    MiningOperation::new(
        "rey.source-search",
        1,
        contract("rey.source-search.builtin"),
        MiningFamily::Source,
        MiningOperationKind::Search,
        MiningExecutionClass::PureProjection,
        MiningDeterminism::FrozenDeterministic,
        vec![artifact_contract(
            "source",
            MiningArtifactKind::Native,
            true,
        )],
        vec![artifact_contract(
            "matches",
            MiningArtifactKind::Relation,
            true,
        )],
        vec![
            MiningParameterContract {
                name: "pattern".to_owned(),
                value_type: MiningParameterType::Utf8,
                required: true,
                default: None,
            },
            MiningParameterContract {
                name: "case_sensitive".to_owned(),
                value_type: MiningParameterType::Bool,
                required: false,
                default: Some(MiningParameterValue::Bool(true)),
            },
        ],
        vec![
            contract("capability.source.read"),
            contract("capability.frame.arrow"),
        ],
        vec![
            MiningInvalidation::ParameterChange,
            MiningInvalidation::ProviderRevision,
            MiningInvalidation::CapabilitySnapshot,
            MiningInvalidation::ImplementationRevision,
            MiningInvalidation::InputArtifactRevision,
            MiningInvalidation::EffectiveLimitChange,
        ],
        MiningLimits::default(),
    )
    .expect("operation")
}

fn artifact(port_id: &str, kind: MiningArtifactKind, label: &str) -> MiningArtifactRef {
    MiningArtifactRef {
        port_id: port_id.to_owned(),
        artifact_id: digest(label),
        kind,
        schema: (kind != MiningArtifactKind::Native).then(|| contract("schema.mining-artifact")),
        media_type: match kind {
            MiningArtifactKind::Native => "text/plain",
            MiningArtifactKind::Relation => "application/vnd.apache.arrow.stream",
            _ => "application/octet-stream",
        }
        .to_owned(),
        provider: contract("provider.local"),
        source_id: format!("fixture://{label}"),
        source_revision: digest(&format!("{label}-revision")).to_string(),
        logical_bytes: 128,
    }
}

fn context() -> MiningRequestContext {
    MiningRequestContext {
        workload: contract("workload.mining"),
        graph: contract("graph.mining"),
        scenario: Some(contract("scenario.match")),
        campaign_id: Some(digest("campaign")),
        space: contract("space.local"),
        active_transition_id: Some(digest("transition")),
        graph_node_id: "search".to_owned(),
        rationale: MiningRationaleKind::Frontier,
        frontier_row_ids: vec![digest("frontier-b"), digest("frontier-a")],
        delta_ids: vec![digest("delta")],
    }
}

fn request(operation: &MiningOperation) -> MiningRequest {
    MiningRequest::new(
        context(),
        operation,
        contract("provider.local"),
        digest("capability-snapshot"),
        vec![artifact("source", MiningArtifactKind::Native, "source")],
        BTreeMap::from([(
            "pattern".to_owned(),
            MiningParameterValue::Utf8("needle".to_owned()),
        )]),
        MiningLimits::default(),
        MiningLimits::default(),
    )
    .expect("request")
}

fn consumption() -> MiningConsumption {
    MiningConsumption {
        files: 1,
        rows: 1,
        matches: 1,
        nodes: 0,
        edges: 0,
        depth: 1,
        bytes_read: 128,
        bytes_written: 128,
        observed_time_ms: None,
    }
}

fn complete_result(operation: &MiningOperation, request: &MiningRequest) -> MiningResult {
    let output = artifact("matches", MiningArtifactKind::Relation, "matches");
    MiningResult::new(
        request,
        operation,
        MiningCompleteness::Complete,
        vec![output.clone()],
        vec![
            MiningLineage {
                kind: MiningLineageKind::Provider,
                identity: request.provider.clone(),
                execution_id: None,
            },
            MiningLineage {
                kind: MiningLineageKind::Implementation,
                identity: operation.implementation.clone(),
                execution_id: Some(digest("execution")),
            },
        ],
        vec![MiningDependencyEdge {
            artifact_id: output.artifact_id,
            kind: MiningDependencyKind::Request,
            dependency_id: request.request_id.clone(),
        }],
        Vec::new(),
        consumption(),
    )
    .expect("result")
}

#[test]
fn manifests_round_trip_and_replay_verify() {
    let operation = operation();
    let request = request(&operation);
    let result = complete_result(&operation, &request);
    assert_eq!(result.dependencies.len(), 4);

    let encoded = serde_json::to_vec(&(&operation, &request, &result)).expect("encode");
    let (decoded_operation, decoded_request, decoded_result): (
        MiningOperation,
        MiningRequest,
        MiningResult,
    ) = serde_json::from_slice(&encoded).expect("decode");

    decoded_operation.verify().expect("operation verifies");
    decoded_request
        .verify_against(&decoded_operation)
        .expect("request verifies");
    decoded_result
        .verify_against(&decoded_request, &decoded_operation)
        .expect("result verifies");
}

#[test]
fn constructors_canonicalize_identity_inputs_and_resolve_defaults() {
    let operation = operation();
    assert!(
        operation
            .required_capabilities
            .windows(2)
            .all(|window| { contract_order(&window[0], &window[1]) == std::cmp::Ordering::Less })
    );
    assert!(
        operation
            .invalidation
            .windows(2)
            .all(|window| window[0] < window[1])
    );

    let request = request(&operation);
    assert!(
        request
            .context
            .frontier_row_ids
            .windows(2)
            .all(|window| window[0] < window[1])
    );
    assert_eq!(
        request.parameters.get("case_sensitive"),
        Some(&MiningParameterValue::Bool(true))
    );
}

#[test]
fn operation_and_request_tampering_is_detected() {
    let mut tampered_operation = operation();
    tampered_operation.limits.max_matches -= 1;
    assert!(matches!(
        tampered_operation.verify(),
        Err(MiningError::DigestMismatch {
            kind: "mining operation",
            ..
        })
    ));

    let operation = operation();
    let mut request = request(&operation);
    request.inputs[0].source_revision = digest("different-source").to_string();
    assert!(matches!(
        request.verify(),
        Err(MiningError::DigestMismatch {
            kind: "mining request",
            ..
        })
    ));
}

#[test]
fn result_tampering_and_stale_request_are_detected() {
    let operation = operation();
    let request = request(&operation);
    let result = complete_result(&operation, &request);
    let mut tampered = result.clone();
    tampered.consumption.matches = 2;
    assert!(matches!(
        tampered.verify(),
        Err(MiningError::DigestMismatch {
            kind: "mining result",
            ..
        })
    ));

    let mut other_context = context();
    other_context.active_transition_id = Some(digest("other-transition"));
    let other_request = MiningRequest::new(
        other_context,
        &operation,
        contract("provider.local"),
        digest("capability-snapshot"),
        vec![artifact("source", MiningArtifactKind::Native, "source")],
        BTreeMap::from([(
            "pattern".to_owned(),
            MiningParameterValue::Utf8("needle".to_owned()),
        )]),
        MiningLimits::default(),
        MiningLimits::default(),
    )
    .expect("other request");
    assert_eq!(
        result.verify_against(&other_request, &operation),
        Err(MiningError::BindingMismatch("request"))
    );
}

#[test]
fn effective_limits_and_consumption_are_hard_bounds() {
    let operation = operation();
    let mut effective = MiningLimits::default();
    effective.max_files += 1;
    assert_eq!(
        MiningRequest::new(
            context(),
            &operation,
            contract("provider.local"),
            digest("capability-snapshot"),
            vec![artifact("source", MiningArtifactKind::Native, "source")],
            BTreeMap::from([(
                "pattern".to_owned(),
                MiningParameterValue::Utf8("needle".to_owned()),
            )]),
            MiningLimits::default(),
            effective,
        ),
        Err(MiningError::EffectiveLimitsExceed("requested limits"))
    );

    let request = request(&operation);
    let output = artifact("matches", MiningArtifactKind::Relation, "matches");
    let mut over = consumption();
    over.files = request.effective_limits.max_files + 1;
    assert!(matches!(
        MiningResult::new(
            &request,
            &operation,
            MiningCompleteness::Complete,
            vec![output],
            vec![MiningLineage {
                kind: MiningLineageKind::Implementation,
                identity: operation.implementation.clone(),
                execution_id: None,
            }],
            Vec::new(),
            Vec::new(),
            over,
        ),
        Err(MiningError::Limit { kind: "file", .. })
    ));

    let rationale_bound = MiningLimits {
        max_rationale_refs: 1,
        ..MiningLimits::default()
    };
    assert!(matches!(
        MiningRequest::new(
            context(),
            &operation,
            contract("provider.local"),
            digest("capability-snapshot"),
            vec![artifact("source", MiningArtifactKind::Native, "source")],
            BTreeMap::from([(
                "pattern".to_owned(),
                MiningParameterValue::Utf8("needle".to_owned()),
            )]),
            MiningLimits::default(),
            rationale_bound,
        ),
        Err(MiningError::Limit {
            kind: "request rationale reference",
            ..
        })
    ));

    let string_bound = MiningLimits {
        max_string_bytes: 1,
        ..MiningLimits::default()
    };
    assert!(matches!(
        MiningRequest::new(
            context(),
            &operation,
            contract("provider.local"),
            digest("capability-snapshot"),
            vec![artifact("source", MiningArtifactKind::Native, "source")],
            BTreeMap::from([(
                "pattern".to_owned(),
                MiningParameterValue::Utf8("needle".to_owned()),
            )]),
            MiningLimits::default(),
            string_bound,
        ),
        Err(MiningError::Limit { .. })
    ));
}

#[test]
fn completeness_states_enforce_artifact_and_omission_shapes() {
    let operation = operation();
    let request = request(&operation);
    let lineage = vec![MiningLineage {
        kind: MiningLineageKind::Implementation,
        identity: operation.implementation.clone(),
        execution_id: None,
    }];
    let omission = MiningOmission {
        kind: MiningOmissionKind::MatchLimit,
        subject_id: Some("matches".to_owned()),
        omitted_count: 1,
        reason: "effective match limit reached".to_owned(),
    };

    assert_eq!(
        MiningResult::new(
            &request,
            &operation,
            MiningCompleteness::Complete,
            vec![artifact("matches", MiningArtifactKind::Relation, "matches")],
            lineage.clone(),
            Vec::new(),
            vec![omission.clone()],
            consumption(),
        ),
        Err(MiningError::CompletenessShape)
    );
    assert_eq!(
        MiningResult::new(
            &request,
            &operation,
            MiningCompleteness::Partial,
            Vec::new(),
            lineage.clone(),
            Vec::new(),
            vec![omission.clone()],
            consumption(),
        ),
        Err(MiningError::CompletenessShape)
    );
    MiningResult::new(
        &request,
        &operation,
        MiningCompleteness::Truncated,
        vec![artifact(
            "matches",
            MiningArtifactKind::Relation,
            "truncated-matches",
        )],
        lineage.clone(),
        Vec::new(),
        vec![omission.clone()],
        consumption(),
    )
    .expect("truncation carries partial output and a limit omission");
    MiningResult::new(
        &request,
        &operation,
        MiningCompleteness::Unavailable,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![MiningOmission {
            kind: MiningOmissionKind::ProviderUnavailable,
            subject_id: None,
            omitted_count: 1,
            reason: "provider not present in the frozen snapshot".to_owned(),
        }],
        consumption(),
    )
    .expect("unavailable is an explicit result");
}

#[test]
fn artifact_contracts_and_dependencies_are_closed() {
    let mut missing_schema = operation();
    missing_schema.outputs[0].schema = None;
    assert_eq!(
        missing_schema.verify(),
        Err(MiningError::MissingArtifactSchema("matches".to_owned()))
    );

    let operation = operation();
    assert_eq!(
        MiningRequest::new(
            context(),
            &operation,
            contract("provider.local"),
            digest("capability-snapshot"),
            Vec::new(),
            BTreeMap::from([(
                "pattern".to_owned(),
                MiningParameterValue::Utf8("needle".to_owned()),
            )]),
            MiningLimits::default(),
            MiningLimits::default(),
        ),
        Err(MiningError::MissingArtifactPort("source".to_owned()))
    );
    let request = request(&operation);
    let wrong_output = artifact("matches", MiningArtifactKind::Native, "wrong");
    assert!(matches!(
        MiningResult::new(
            &request,
            &operation,
            MiningCompleteness::Complete,
            vec![wrong_output],
            vec![MiningLineage {
                kind: MiningLineageKind::Implementation,
                identity: operation.implementation.clone(),
                execution_id: None,
            }],
            Vec::new(),
            Vec::new(),
            consumption(),
        ),
        Err(MiningError::ArtifactContract(port)) if port == "matches"
    ));

    let output = artifact("matches", MiningArtifactKind::Relation, "matches");
    assert_eq!(
        MiningResult::new(
            &request,
            &operation,
            MiningCompleteness::Complete,
            vec![output],
            vec![MiningLineage {
                kind: MiningLineageKind::Implementation,
                identity: operation.implementation.clone(),
                execution_id: None,
            }],
            vec![MiningDependencyEdge {
                artifact_id: digest("unknown-output"),
                kind: MiningDependencyKind::Request,
                dependency_id: request.request_id.clone(),
            }],
            Vec::new(),
            consumption(),
        ),
        Err(MiningError::UnknownDependencyArtifact)
    );
}

#[test]
fn duplicate_and_non_canonical_contract_inputs_are_rejected() {
    let operation = operation();
    let source = artifact("source", MiningArtifactKind::Native, "source");
    assert_eq!(
        MiningRequest::new(
            context(),
            &operation,
            contract("provider.local"),
            digest("capability-snapshot"),
            vec![source.clone(), source],
            BTreeMap::from([(
                "pattern".to_owned(),
                MiningParameterValue::Utf8("needle".to_owned()),
            )]),
            MiningLimits::default(),
            MiningLimits::default(),
        ),
        Err(MiningError::NonCanonical("request input artifacts"))
    );
}
