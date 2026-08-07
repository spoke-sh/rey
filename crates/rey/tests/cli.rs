use std::process::Command;

use rey_dataframe::Frame;
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
