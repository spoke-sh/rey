use std::{fs, path::Path, process::Command};

use rey_dataframe::Frame;
use rey_diff::CapabilityDelta;
use rey_environment::{
    Availability, CapabilityRecord, CapabilitySnapshot, DiscoveryLimits, LOCAL_PROVIDER_REVISION,
    TrustClass,
};
use rey_proof::{
    CertificateVerification, ProofStatus, RequiredCapabilityCertificate, VerificationStatus,
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
