use std::{fs, path::Path, process::Command};

use rey::workloads::{
    QualificationState, WorkloadFreshness, WorkloadList, WorkloadStatusBatch, WorkloadTestBatch,
};
use rey_dataframe::Frame;
use rey_diff::CapabilityDelta;
use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryLimits, LOCAL_PROVIDER_REVISION,
    TrustClass,
};
use rey_proof::{
    BundleArtifactRole, CertificateVerification, LocalBundleVerification,
    LocalBundleVerificationStatus, LocalProofBundleManifest, ProofStatus,
    RequiredCapabilityCertificate, VerificationStatus,
};
use rey_runtime::{
    BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_NORMALIZE_WORKLOAD_ID, RunStatus, ScenarioEvaluation,
    TestStatus, WorkloadRunResult,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn explicit_json_is_machine_clean_and_standalone() {
    let workspace = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "environment",
            "inspect",
            "--workspace",
            workspace.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["profile"], "standalone");
    assert!(
        document["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| { !row["provider_id"].as_str().unwrap().contains("spoke") })
    );
}

#[test]
fn redirected_auto_output_is_a_decodable_arrow_frame() {
    let workspace = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "environment",
            "inspect",
            "--workspace",
            workspace.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let frame = Frame::from_arrow_stream(&output.stdout).unwrap();
    assert_eq!(frame.metadata().relation, "rey.capabilities");
}

#[test]
fn table_output_contains_every_version_one_column() {
    let workspace = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "environment",
            "inspect",
            "--workspace",
            workspace.path().to_str().unwrap(),
            "--format",
            "table",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let rendered = String::from_utf8(output.stdout).unwrap();
    let mut lines = rendered.lines();
    assert!(lines.next().unwrap().starts_with("snapshot=blake3:"));
    let header = lines.next().unwrap();
    assert_eq!(header.split('\t').count(), 17);
    assert!(lines.all(|row| row.split('\t').count() == 17));
}

#[test]
fn environment_diff_emits_deterministic_structured_arrow_and_tabular_artifacts() {
    let directory = TempDir::new().unwrap();
    let source = snapshot(vec![
        capability("deleted", Some("1"), Availability::Available),
        capability("modified", Some("a->b"), Availability::Available),
    ]);
    let target = snapshot(vec![
        capability("inserted", Some("NULL"), Availability::Available),
        capability("modified", Some("2"), Availability::Available),
    ]);
    let source_path = write_json(directory.path(), "source.json", &source);
    let target_path = write_json(directory.path(), "target.json", &target);

    let structured = run_rey(&[
        "environment",
        "diff",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
    ]);
    assert!(
        structured.status.success(),
        "{}",
        String::from_utf8_lossy(&structured.stderr)
    );
    assert!(structured.stderr.is_empty());
    let delta: CapabilityDelta = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(delta.summary.inserted, 1);
    assert_eq!(delta.summary.deleted, 1);
    assert_eq!(delta.summary.modified, 1);
    let repeated = run_rey(&[
        "environment",
        "diff",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
    ]);
    assert_eq!(structured.stdout, repeated.stdout);

    let arrow = run_rey(&[
        "environment",
        "diff",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
        "--format",
        "arrow",
    ]);
    assert!(arrow.status.success());
    let frame = Frame::from_arrow_stream(&arrow.stdout).unwrap();
    assert_eq!(frame.metadata().relation, "rey.capability-changes");
    assert_eq!(frame.metadata().semantic_digest, delta.delta_id.to_string());

    let tabular = run_rey(&[
        "environment",
        "diff",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
        "--diff-format",
        "tabular-diff",
    ]);
    assert!(tabular.status.success());
    let tabular = String::from_utf8(tabular.stdout).unwrap();
    assert!(tabular.starts_with("@@,provider_id"));
    assert!(tabular.lines().any(|line| line.starts_with("+++")));
    assert!(tabular.lines().any(|line| line.starts_with("---")));
    assert!(tabular.lines().any(|line| line.starts_with("-->")));
}

#[test]
fn required_capability_certificate_verifies_then_becomes_stale() {
    let directory = TempDir::new().unwrap();
    let source = snapshot(Vec::new());
    let target = snapshot(vec![capability(
        "required",
        Some("1"),
        Availability::Available,
    )]);
    let changed_target = snapshot(vec![capability(
        "required",
        Some("1"),
        Availability::Unavailable,
    )]);
    let source_path = write_json(directory.path(), "source.json", &source);
    let target_path = write_json(directory.path(), "target.json", &target);
    let changed_path = write_json(directory.path(), "changed.json", &changed_target);

    let proved = run_rey(&[
        "environment",
        "prove",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
        "--require-capability",
        "required",
    ]);
    assert!(
        proved.status.success(),
        "{}",
        String::from_utf8_lossy(&proved.stderr)
    );
    let certificate: RequiredCapabilityCertificate =
        serde_json::from_slice(&proved.stdout).unwrap();
    assert_eq!(certificate.status, ProofStatus::Passed);
    let certificate_path = write_json(directory.path(), "certificate.json", &certificate);

    let verified = run_rey(&[
        "environment",
        "verify",
        certificate_path.to_str().unwrap(),
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
    ]);
    assert!(verified.status.success());
    let verified: CertificateVerification = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verified.status, VerificationStatus::Verified);

    let stale = run_rey(&[
        "environment",
        "verify",
        certificate_path.to_str().unwrap(),
        source_path.to_str().unwrap(),
        changed_path.to_str().unwrap(),
    ]);
    assert_eq!(stale.status.code(), Some(4));
    let stale: CertificateVerification = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale.status, VerificationStatus::Stale);
}

#[test]
fn proof_outcomes_and_invalid_snapshots_have_categorized_exits() {
    let directory = TempDir::new().unwrap();
    let source = snapshot(Vec::new());
    let unavailable = snapshot(vec![capability(
        "required",
        None,
        Availability::Unavailable,
    )]);
    let error = snapshot(vec![capability("required", None, Availability::Error)]);
    let source_path = write_json(directory.path(), "source.json", &source);
    let unavailable_path = write_json(directory.path(), "unavailable.json", &unavailable);
    let error_path = write_json(directory.path(), "error.json", &error);

    let failed = run_rey(&[
        "environment",
        "prove",
        source_path.to_str().unwrap(),
        unavailable_path.to_str().unwrap(),
        "--require-capability",
        "required",
    ]);
    assert_eq!(failed.status.code(), Some(2));
    let failed: RequiredCapabilityCertificate = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(failed.status, ProofStatus::Failed);

    let inconclusive = run_rey(&[
        "environment",
        "prove",
        source_path.to_str().unwrap(),
        error_path.to_str().unwrap(),
        "--require-capability",
        "required",
    ]);
    assert_eq!(inconclusive.status.code(), Some(3));
    let inconclusive: RequiredCapabilityCertificate =
        serde_json::from_slice(&inconclusive.stdout).unwrap();
    assert_eq!(inconclusive.status, ProofStatus::Inconclusive);

    let mut tampered = serde_json::to_value(&unavailable).unwrap();
    tampered["capabilities"][0]["version"] = "tampered".into();
    let tampered_path = write_json(directory.path(), "tampered.json", &tampered);
    let invalid = run_rey(&[
        "environment",
        "diff",
        source_path.to_str().unwrap(),
        tampered_path.to_str().unwrap(),
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("does not match recomputed"));
}

#[test]
fn local_bundle_round_trips_and_tampering_fails_closed() {
    let directory = TempDir::new().unwrap();
    let source = snapshot(Vec::new());
    let target = snapshot(vec![capability(
        "required",
        Some("1"),
        Availability::Available,
    )]);
    let source_path = write_json(directory.path(), "source.json", &source);
    let target_path = write_json(directory.path(), "target.json", &target);
    let bundle_path = directory.path().join("bundle");

    let proved = run_rey(&[
        "environment",
        "prove",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
        "--require-capability",
        "required",
        "--bundle",
        bundle_path.to_str().unwrap(),
    ]);
    assert!(
        proved.status.success(),
        "{}",
        String::from_utf8_lossy(&proved.stderr)
    );
    assert!(proved.stderr.is_empty());
    let certificate: RequiredCapabilityCertificate =
        serde_json::from_slice(&proved.stdout).unwrap();
    assert_eq!(certificate.status, ProofStatus::Passed);

    let manifest: LocalProofBundleManifest =
        serde_json::from_slice(&fs::read(bundle_path.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.certificate_id, certificate.certificate_id);
    assert!(manifest.retention.content_addressed_objects);
    assert!(!manifest.retention.remote_durable);
    assert!(!manifest.retention.spoke_durable);
    assert!(!manifest.retention.spoke_fenced_execution);
    assert!(!manifest.retention.spoke_query_semantics);
    assert!(!manifest.retention.spoke_revision_lineage);

    let verified = run_rey(&[
        "environment",
        "verify-bundle",
        bundle_path.to_str().unwrap(),
    ]);
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(verified.stderr.is_empty());
    let verification: LocalBundleVerification = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verification.status, LocalBundleVerificationStatus::Verified);
    assert_eq!(verification.bundle_id, manifest.bundle_id);

    let delta = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.role == BundleArtifactRole::CapabilityDelta)
        .unwrap();
    fs::remove_file(bundle_path.join(&delta.object_path)).unwrap();
    let tampered = run_rey(&[
        "environment",
        "verify-bundle",
        bundle_path.to_str().unwrap(),
    ]);
    assert_eq!(tampered.status.code(), Some(1));
    assert!(tampered.stdout.is_empty());
    assert!(!tampered.stderr.is_empty());
}

#[test]
fn local_bundle_limit_failure_publishes_nothing() {
    let directory = TempDir::new().unwrap();
    let source = snapshot(Vec::new());
    let target = snapshot(vec![capability(
        "required",
        Some("1"),
        Availability::Available,
    )]);
    let source_path = write_json(directory.path(), "source.json", &source);
    let target_path = write_json(directory.path(), "target.json", &target);
    let bundle_path = directory.path().join("bundle");

    let output = run_rey(&[
        "environment",
        "prove",
        source_path.to_str().unwrap(),
        target_path.to_str().unwrap(),
        "--require-capability",
        "required",
        "--bundle",
        bundle_path.to_str().unwrap(),
        "--max-bundle-bytes",
        "1",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!bundle_path.exists());
}

#[test]
fn workload_list_is_read_only_machine_clean_and_reports_exact_graphs() {
    let workspace = TempDir::new().unwrap();
    let output = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
    ]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(!workspace.path().join(".rey").exists());
    let list: WorkloadList = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.workloads.len(), 2);
    assert_eq!(
        list.workloads
            .iter()
            .map(|workload| workload.workload.id.as_str())
            .collect::<Vec<_>>(),
        [
            BUILT_IN_NORMALIZE_WORKLOAD_ID,
            BUILT_IN_MISMATCH_WORKLOAD_ID
        ]
    );
    assert!(list.workloads.iter().all(|workload| {
        workload.freshness == WorkloadFreshness::Untested
            && workload.qualification == QualificationState::Untested
            && workload.passed == 0
            && workload.evaluated == 0
            && workload.candidate_graph.revision == 1
    }));

    let table = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
        "--format",
        "table",
    ]);
    assert!(table.status.success());
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(table.contains("progress"));
    assert_eq!(table.matches("\t[..]\t0/0\tuntested\tuntested").count(), 2);
}

#[test]
fn workload_test_qualifies_then_run_executes_the_same_graph() {
    let workspace = TempDir::new().unwrap();

    let blocked = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "run",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
        "--input",
        " rey ",
    ]);
    assert_eq!(blocked.status.code(), Some(3));
    assert!(blocked.stderr.is_empty());
    let blocked: WorkloadRunResult = serde_json::from_slice(&blocked.stdout).unwrap();
    assert_eq!(blocked.status, RunStatus::Blocked);
    assert_eq!(blocked.stop_reason, "qualification_missing_or_stale");

    let tested = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
    ]);
    assert!(tested.status.success());
    assert!(tested.stderr.is_empty());
    let tested: WorkloadTestBatch = serde_json::from_slice(&tested.stdout).unwrap();
    let test_result = &tested.results[0];
    assert_eq!(test_result.status, TestStatus::Passed);
    assert_eq!(test_result.summary.passed, 2);
    assert!(test_result.qualification.is_some());

    let repeated = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
    ]);
    assert!(repeated.status.success());
    assert_eq!(tested.results[0].result_id, {
        let repeated: WorkloadTestBatch = serde_json::from_slice(&repeated.stdout).unwrap();
        repeated.results[0].result_id.clone()
    });

    let executed = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "run",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
        "--input",
        " rey ",
    ]);
    assert!(executed.status.success());
    assert!(executed.stderr.is_empty());
    let executed: WorkloadRunResult = serde_json::from_slice(&executed.stdout).unwrap();
    assert_eq!(executed.status, RunStatus::Passed);
    assert_eq!(executed.graph, test_result.graph);
    assert_eq!(
        executed.qualification_id,
        test_result
            .qualification
            .as_ref()
            .map(|qualification| qualification.qualification_id.clone())
    );
    assert_eq!(
        executed.outputs["text"],
        rey_runtime::WorkloadValue::Utf8("REY".to_owned())
    );
    assert_eq!(executed.node_order, ["trim", "uppercase"]);

    let status = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "status",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
    ]);
    assert!(status.status.success());
    let status: WorkloadStatusBatch = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status.statuses.len(), 1);
    assert_eq!(
        status.statuses[0].summary.qualification,
        QualificationState::Qualified
    );
    assert_eq!(
        status.statuses[0].last_run.as_ref().map(|run| run.status),
        Some(RunStatus::Passed)
    );
}

#[test]
fn workload_failure_is_a_typed_delta_with_a_semantic_exit() {
    let workspace = TempDir::new().unwrap();
    let output = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_MISMATCH_WORKLOAD_ID,
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let batch: WorkloadTestBatch = serde_json::from_slice(&output.stdout).unwrap();
    let result = &batch.results[0];
    assert_eq!(result.status, TestStatus::Failed);
    assert_eq!(result.summary.passed, 1);
    assert_eq!(result.summary.failed, 1);
    assert!(result.qualification.is_none());
    let scenario = result
        .scenarios
        .iter()
        .find(|scenario| scenario.evaluation == ScenarioEvaluation::Failed)
        .unwrap();
    let delta = &scenario.deltas[0];
    assert_eq!(delta.expected, "REY");
    assert_eq!(delta.observed, " REY ");
    delta.verify().unwrap();

    let list = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
    ]);
    let list: WorkloadList = serde_json::from_slice(&list.stdout).unwrap();
    let mismatch = list
        .workloads
        .iter()
        .find(|workload| workload.workload.id == BUILT_IN_MISMATCH_WORKLOAD_ID)
        .unwrap();
    assert_eq!(mismatch.qualification, QualificationState::Failing);
    assert_eq!(mismatch.passed, 1);
    assert_eq!(mismatch.evaluated, 2);
}

#[test]
fn aggregate_test_and_invalid_state_preserve_stdout_stderr_contracts() {
    let workspace = TempDir::new().unwrap();
    let output = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
    ]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let batch: WorkloadTestBatch = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(batch.results.len(), 2);
    assert!(
        batch
            .results
            .iter()
            .any(|result| result.status == TestStatus::Passed)
    );
    assert!(
        batch
            .results
            .iter()
            .any(|result| result.status == TestStatus::Failed)
    );

    fs::write(
        workspace.path().join(".rey/workloads/state.json"),
        b"not-json",
    )
    .unwrap();
    let invalid = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid JSON"));
}

fn snapshot(rows: Vec<CapabilityRecord>) -> CapabilitySnapshot {
    let limits = DiscoveryLimits {
        max_capabilities: 16,
        ..DiscoveryLimits::default()
    };
    CapabilitySnapshot::new("fixture", limits, rows).unwrap()
}

fn capability(id: &str, version: Option<&str>, availability: Availability) -> CapabilityRecord {
    CapabilityRecord {
        provider_id: "fixture".to_owned(),
        provider_revision: LOCAL_PROVIDER_REVISION,
        provider_kind: "fixture".to_owned(),
        capability_id: id.to_owned(),
        capability_kind: "identity".to_owned(),
        resolved_location: None,
        version: version.map(str::to_owned),
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

fn write_json(path: &Path, name: &str, value: &impl serde::Serialize) -> std::path::PathBuf {
    let path = path.join(name);
    fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    path
}

fn run_rey(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rey"))
        .args(args)
        .output()
        .unwrap()
}
