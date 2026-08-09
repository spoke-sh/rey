use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};

use rey::env::{
    EnvironmentAddResult, EnvironmentCommitResult, EnvironmentDiff, EnvironmentDiffMode,
    EnvironmentStatus, EnvironmentWorkingState,
};
use rey::workloads::{
    QualificationState, WorkloadFreshness, WorkloadList, WorkloadStatusBatch, WorkloadTestBatch,
};
use rey_mining::MiningCompleteness;
use rey_runtime::{
    BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_NORMALIZE_WORKLOAD_ID,
    BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID, RunStatus, ScenarioEvaluation, TestStatus,
    WorkloadRunResult,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn env_history_is_git_shaped_human_verifiable_and_machine_clean() {
    let workspace = TempDir::new().unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace.path().to_str().unwrap(), "init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let workspace_path = workspace.path().to_str().unwrap();

    for removed in [vec!["environment", "status"], vec!["env", "inspect"]] {
        let output = run_rey(&removed);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
    }

    let unborn = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(unborn.status.success());
    assert!(unborn.stderr.is_empty());
    let unborn = String::from_utf8(unborn.stdout).unwrap();
    for evidence in [
        "ENVIRONMENT STATUS",
        "State                  UNBORN",
        "HEAD                   no commits",
        "Admission index        matches HEAD · no retained index",
        "Delta                  EMPTY → INDEX · EQUAL",
        "Delta                  INDEX → WORKING · DIFFERENT",
        "Changes not staged for admission:",
        "use `rey env add` to admit all working changes",
        "No environment commits yet; add the working environment before committing.",
    ] {
        assert!(
            unborn.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }
    assert!(!workspace.path().join(".rey").exists());

    let premature = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "premature",
    ]);
    assert_eq!(premature.status.code(), Some(1));
    assert!(premature.stdout.is_empty());
    assert!(String::from_utf8_lossy(&premature.stderr).contains("nothing staged"));

    let added = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "json",
    ]);
    assert!(added.status.success());
    assert!(added.stderr.is_empty());
    let added: EnvironmentAddResult = serde_json::from_slice(&added.stdout).unwrap();
    assert!(added.index.is_some());
    assert_eq!(added.staged_delta.source_label, "EMPTY");
    assert_eq!(added.staged_delta.target_label, "INDEX");
    assert!(added.staged_changes > 0);
    assert!(added.unstaged_delta.changes.is_empty());

    let staged = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let staged: EnvironmentStatus = serde_json::from_slice(&staged.stdout).unwrap();
    assert_eq!(staged.state, EnvironmentWorkingState::Staged);
    assert!(staged.admission_index.is_some());
    assert!(!staged.staged_delta.changes.is_empty());
    assert!(staged.unstaged_delta.changes.is_empty());

    let first = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "baseline",
        "--format",
        "json",
    ]);
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    let first: EnvironmentCommitResult = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first.commit.sequence, 1);
    assert_eq!(first.commit.message, "baseline");
    assert!(first.commit.parent_commit_id.is_none());
    assert_eq!(first.delta.source_label, "EMPTY");
    assert_eq!(first.delta.target_label, "INDEX");
    assert!(!workspace.path().join(".rey/env/index.json").exists());

    let clean = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let clean: EnvironmentStatus = serde_json::from_slice(&clean.stdout).unwrap();
    assert_eq!(clean.state, EnvironmentWorkingState::Clean);
    assert_eq!(clean.head_commit_id.as_ref(), Some(&first.commit.commit_id));
    assert!(clean.staged_delta.changes.is_empty());
    assert!(clean.unstaged_delta.changes.is_empty());

    fs::write(workspace.path().join("tracked.txt"), "environment edge\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "--", "tracked.txt"])
            .status()
            .unwrap()
            .success()
    );

    let changed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(changed.status.success());
    let changed = String::from_utf8(changed.stdout).unwrap();
    for evidence in [
        "State                  CHANGED",
        "Delta                  ENV@1 → INDEX · EQUAL",
        "Delta                  INDEX → WORKING · DIFFERENT",
        "Changes not staged for admission:",
        "git.repository.inspect",
    ] {
        assert!(
            changed.contains(evidence),
            "missing changed evidence: {evidence}"
        );
    }

    let diff = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "diff",
        "--format",
        "table",
    ]);
    assert!(diff.status.success());
    let diff = String::from_utf8(diff.stdout).unwrap();
    for evidence in [
        "View                   UNSTAGED · INDEX → WORKING",
        "CAPABILITY PATCH INDEX → WORKING",
        "git.repository.inspect (modified)",
        "content_digest:",
    ] {
        assert!(diff.contains(evidence), "missing diff evidence: {evidence}");
    }

    let added = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "table",
    ]);
    assert!(added.status.success());
    let added = String::from_utf8(added.stdout).unwrap();
    assert!(added.contains("ENVIRONMENT ADMISSION"));
    assert!(added.contains("1 capability changes admitted"));
    assert!(added.contains("0 changes remain unstaged · EQUAL"));

    let staged_diff = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "diff",
        "--staged",
        "--format",
        "json",
    ]);
    let staged_diff: EnvironmentDiff = serde_json::from_slice(&staged_diff.stdout).unwrap();
    assert_eq!(staged_diff.mode, EnvironmentDiffMode::Staged);
    assert_eq!(staged_diff.delta.source_label, "ENV@1");
    assert_eq!(staged_diff.delta.target_label, "INDEX");
    assert_eq!(staged_diff.delta.summary.modified, 1);

    let second = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "stage fixture",
        "--format",
        "table",
    ]);
    assert!(second.status.success());
    let second = String::from_utf8(second.stdout).unwrap();
    assert!(second.contains("[env 2] stage fixture"));
    assert!(second.contains("Delta                  ENV@1 → INDEX · DIFFERENT"));

    let patch_log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-p",
        "-n",
        "2",
        "--format",
        "table",
    ]);
    assert!(patch_log.status.success());
    let patch_log = String::from_utf8(patch_log.stdout).unwrap();
    for evidence in [
        "ENVIRONMENT HISTORY",
        "2 total · 2 shown · newest first",
        "Sequence               ENV@2",
        "Message\n      stage fixture",
        "CAPABILITY PATCH ENV@1 → ENV@2",
        "CAPABILITY PATCH EMPTY → ENV@1",
    ] {
        assert!(
            patch_log.contains(evidence),
            "missing log evidence: {evidence}"
        );
    }

    let env_help = run_rey(&["env", "--help"]);
    assert!(env_help.status.success());
    let env_help = String::from_utf8(env_help.stdout).unwrap();
    assert!(env_help.contains("add"));
    assert!(env_help.contains("status"));
    assert!(!env_help.contains("inspect"));
    assert!(!env_help.contains("prove"));
    assert!(!env_help.contains("verify"));

    for observing_command in ["status", "add", "diff"] {
        let help = run_rey(&["env", observing_command, "--help"]);
        assert!(help.status.success());
        assert!(String::from_utf8(help.stdout).unwrap().contains("--map"));
    }
    for retained_state_command in ["commit", "log"] {
        let help = run_rey(&["env", retained_state_command, "--help"]);
        assert!(help.status.success());
        assert!(!String::from_utf8(help.stdout).unwrap().contains("--map"));
    }
}

#[test]
fn env_log_bounds_and_invalid_state_keep_stdout_clean() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    let empty = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "--format",
        "table",
    ]);
    assert!(empty.status.success());
    assert!(empty.stderr.is_empty());
    assert!(String::from_utf8_lossy(&empty.stdout).contains("No environment commits."));
    assert!(!workspace.path().join(".rey").exists());

    let escaped_state = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "--state-dir",
        "../outside",
        "log",
    ]);
    assert_eq!(escaped_state.status.code(), Some(1));
    assert!(escaped_state.stdout.is_empty());
    assert!(String::from_utf8_lossy(&escaped_state.stderr).contains("escapes the workspace"));

    let invalid_limit = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-n",
        "0",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_limit.status.code(), Some(1));
    assert!(invalid_limit.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_limit.stderr).contains("log count"));

    fs::create_dir_all(workspace.path().join(".rey/env")).unwrap();
    fs::write(workspace.path().join(".rey/env/state.json"), "not-json\n").unwrap();
    let invalid_state = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_state.status.code(), Some(1));
    assert!(invalid_state.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_state.stderr).contains("invalid JSON"));
}

#[test]
fn status_json_is_machine_clean_and_contains_the_complete_inventory() {
    let workspace = TempDir::new().unwrap();
    let output = run_rey(&[
        "env",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "status",
        "--format",
        "json",
    ]);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let status: EnvironmentStatus = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status.working_snapshot.profile, "standalone");
    assert!(
        status
            .working_snapshot
            .capabilities
            .iter()
            .all(|row| !row.provider_id.contains("spoke"))
    );
    assert_eq!(status.staged_delta.source_label, "EMPTY");
    assert_eq!(status.unstaged_delta.target_label, "WORKING");
}

#[cfg(unix)]
#[test]
fn env_mapping_graph_is_visible_secret_safe_and_diff_directed() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("input.txt"), "first input\n").unwrap();
    let bin = workspace.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let probe = bin.join("rey-map-probe");
    let invocation_marker = workspace.path().join("should-not-exist");
    fs::write(
        &probe,
        format!("#!/bin/sh\ntouch '{}'\n", invocation_marker.display()),
    )
    .unwrap();
    let mut permissions = fs::metadata(&probe).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&probe, permissions).unwrap();
    fs::write(
        workspace.path().join("rey.env.yaml"),
        r#"schema: rey.env-map.v1
nodes:
  - id: mode
    kind: variable
    name: REY_MODE
    capture: digest
  - id: secret
    kind: variable
    name: REY_SECRET
    sensitive: true
    capture: presence
  - id: input
    kind: file
    path: input.txt
    required: true
  - id: probe
    kind: executable
    name: rey-map-probe
    required: true
    potential_capabilities: [source.search]
edges:
  - from: mode
    to: input
    relation: locates
  - from: input
    to: probe
    relation: consumed_by
"#,
    )
    .unwrap();
    let search_path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let workspace_path = workspace.path().to_str().unwrap();
    let variables = [
        ("PATH", search_path.as_str()),
        ("REY_MODE", "development-mode-value"),
        ("REY_SECRET", "never-retain-this-secret"),
    ];

    let inspected = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--format",
            "json",
        ],
        &variables,
    );
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    assert!(inspected.stderr.is_empty());
    let rendered = String::from_utf8(inspected.stdout.clone()).unwrap();
    assert!(!rendered.contains("never-retain-this-secret"));
    assert!(!rendered.contains("development-mode-value"));
    assert!(!invocation_marker.exists());
    let status_document: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    let rows = status_document["working_snapshot"]["capabilities"]
        .as_array()
        .unwrap();
    assert!(rows.iter().any(|row| {
        row["capability_id"] == "env.mapping.graph" && row["capability_kind"] == "environment_map"
    }));
    assert!(rows.iter().any(|row| {
        row["capability_id"] == "env.mapping.node.secret"
            && row["content_digest"].is_null()
            && row["availability"] == "available"
    }));
    assert!(rows.iter().any(|row| {
        row["capability_id"] == "env.mapping.node.mode"
            && row["content_digest"]
                .as_str()
                .is_some_and(|value| value.starts_with("blake3:"))
    }));
    assert!(rows.iter().any(|row| {
        row["capability_id"] == "env.mapping.node.probe"
            && row["unsupported_limits"].as_array().is_some_and(|limits| {
                limits
                    .iter()
                    .any(|value| value == "unadmitted:source.search")
            })
    }));

    let status = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--format",
            "table",
        ],
        &variables,
    );
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains(
        "Mapping                1 graph · 2 variables · 1 file · 1 executable · 2 edges"
    ));
    assert!(status.contains("Mapping graph          rey.env.yaml · blake3:"));

    let added = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--format",
            "json",
        ],
        &variables,
    );
    assert!(added.status.success());
    let committed = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "map environment inputs",
        ],
        &variables,
    );
    assert!(committed.status.success());
    fs::write(workspace.path().join("input.txt"), "second input\n").unwrap();
    let changed_variables = [
        ("PATH", search_path.as_str()),
        ("REY_MODE", "production-mode-value"),
        ("REY_SECRET", "a-different-secret"),
    ];
    let diff = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "diff",
            "--format",
            "json",
        ],
        &changed_variables,
    );
    assert!(diff.status.success());
    assert!(diff.stderr.is_empty());
    let diff: EnvironmentDiff = serde_json::from_slice(&diff.stdout).unwrap();
    assert!(diff.delta.changes.iter().any(|change| {
        change.key.capability_id == "env.mapping.node.mode"
            && change.changed_fields.contains(&"content_digest".to_owned())
    }));
    assert!(diff.delta.changes.iter().any(|change| {
        change.key.capability_id == "env.mapping.node.input"
            && change.changed_fields.contains(&"content_digest".to_owned())
    }));
    assert!(
        !diff
            .delta
            .changes
            .iter()
            .any(|change| change.key.capability_id == "env.mapping.node.secret")
    );

    fs::write(
        workspace.path().join("rey.env.yaml"),
        "schema: rey.env-map.v1\nnodes:\n  - id: secret\n    kind: variable\n    name: REY_SECRET\n    sensitive: true\n    capture: digest\n",
    )
    .unwrap();
    let invalid = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--format",
            "json",
        ],
        &changed_variables,
    );
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("cannot retain a value digest"));
}

#[test]
fn env_add_patch_stages_selected_capabilities_and_commit_ignores_later_drift() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    fs::write(workspace.path().join("alpha.txt"), "alpha one\n").unwrap();
    fs::write(workspace.path().join("beta.txt"), "beta one\n").unwrap();
    fs::write(
        workspace.path().join("rey.env.yaml"),
        r#"schema: rey.env-map.v1
nodes:
  - id: alpha
    kind: file
    path: alpha.txt
    required: true
  - id: beta
    kind: file
    path: beta.txt
    required: true
"#,
    )
    .unwrap();

    assert!(
        run_rey(&["env", "--workspace", workspace_path, "add"])
            .status
            .success()
    );
    assert!(
        run_rey(&[
            "env",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "baseline inputs",
        ])
        .status
        .success()
    );

    fs::write(workspace.path().join("alpha.txt"), "alpha two\n").unwrap();
    fs::write(workspace.path().join("beta.txt"), "beta two\n").unwrap();
    let invalid_format = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "add",
        "-p",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_format.status.code(), Some(1));
    assert!(invalid_format.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_format.stderr).contains("requires human table"));

    let partial = run_rey_with_stdin_env(
        &["env", "--workspace", workspace_path, "add", "-p"],
        "y\nn\n",
        &[],
    );
    assert!(
        partial.status.success(),
        "{}",
        String::from_utf8_lossy(&partial.stderr)
    );
    let partial = String::from_utf8(partial.stdout).unwrap();
    assert!(partial.contains("ENVIRONMENT ADMISSION PATCH"));
    assert!(partial.contains("Change 1/2"));
    assert!(partial.contains("Change 2/2"));
    assert!(partial.contains("1 capability changes admitted"));
    assert!(partial.contains("1 changes remain unstaged"));

    let mixed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let mixed: EnvironmentStatus = serde_json::from_slice(&mixed.stdout).unwrap();
    assert_eq!(mixed.state, EnvironmentWorkingState::Mixed);
    assert_eq!(mixed.staged_delta.summary.modified, 1);
    assert_eq!(mixed.unstaged_delta.summary.modified, 1);
    let staged_snapshot = mixed
        .admission_index
        .as_ref()
        .unwrap()
        .snapshot
        .semantic_digest
        .clone();

    fs::write(workspace.path().join("alpha.txt"), "alpha three\n").unwrap();
    let committed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "admit alpha only",
        "--format",
        "json",
    ]);
    assert!(committed.status.success());
    let committed: EnvironmentCommitResult = serde_json::from_slice(&committed.stdout).unwrap();
    assert_eq!(committed.commit.snapshot.semantic_digest, staged_snapshot);

    let after = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let after: EnvironmentStatus = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(after.state, EnvironmentWorkingState::Changed);
    assert!(after.admission_index.is_none());
    assert!(after.staged_delta.changes.is_empty());
    assert_eq!(after.unstaged_delta.summary.modified, 2);
}

#[test]
fn workload_list_is_read_only_machine_clean_and_renders_a_portfolio() {
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
    assert_eq!(list.workloads.len(), 3);
    assert_eq!(
        list.workloads
            .iter()
            .map(|workload| workload.workload.id.as_str())
            .collect::<Vec<_>>(),
        [
            BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
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
    assert!(table.stderr.is_empty());
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(table.starts_with("\nWORKLOAD PORTFOLIO\n"));
    assert!(
        table.contains(
            "Qualification          0/3 qualified · 0 failing · 0 inconclusive · 0 stale"
        )
    );
    assert!(
        table.contains("Scenarios              0/6 passing · 0/6 evaluated · 0 stale · 2 optional")
    );
    assert!(table.contains("Runs                   0 passed · 0 blocked · 3 not run"));
    assert!(table.contains("Inventory              3 total · 0 tested · 3 untested"));
    assert!(
        table.contains("Mining                 1 workloads · 0 retained results · 0 incomplete")
    );
    assert_eq!(table.matches("Journey                TEST").count(), 3);
    assert_eq!(
        table
            .matches("░░░░░░░░░░░░░░░░░░░░    0%  0/2 passing · 0/2 evaluated")
            .count(),
        3
    );
    assert!(table.contains("Graph                  rey.fixture.text-normalize.graph@1"));
    assert!(table.contains("Candidate              blake3:"));
    assert_eq!(table.matches("Qualification          UNTESTED").count(), 3);
    assert_eq!(table.matches("Test evidence          none").count(), 3);
    assert!(!table.contains('\t'));
    assert!(!table.contains("\u{1b}["));

    let tested = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
    ]);
    assert_eq!(tested.status.code(), Some(2));
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

    let evolved = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
        "--format",
        "table",
    ]);
    assert!(evolved.status.success());
    assert!(evolved.stderr.is_empty());
    let evolved = String::from_utf8(evolved.stdout).unwrap();
    assert!(
        evolved.contains(
            "Qualification          2/3 qualified · 1 failing · 0 inconclusive · 0 stale"
        )
    );
    assert!(
        evolved
            .contains("Scenarios              5/6 passing · 6/6 evaluated · 0 stale · 2 optional")
    );
    assert!(evolved.contains("Runs                   1 passed · 0 blocked · 2 not run"));
    assert!(evolved.contains("Inventory              3 total · 3 tested · 0 untested"));
    assert!(evolved.contains("Journey                RUN COMPLETE"));
    assert!(evolved.contains("Journey                REVISE GRAPH"));
    assert!(evolved.contains("████████████████████  100%  2/2 passing · 2/2 evaluated"));
    assert!(evolved.contains("██████████░░░░░░░░░░   50%  1/2 passing · 2/2 evaluated"));
    assert!(evolved.contains("Evaluation             1 passed · 1 failed"));
    assert!(evolved.contains("Qualification          QUALIFIED"));
    assert!(evolved.contains("Qualification          FAILING"));
    assert!(evolved.contains("Test evidence          blake3:"));
    assert!(evolved.contains("Last run               passed"));
    assert!(!evolved.contains("\u{1b}["));
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
fn workload_test_table_is_incremental_and_opens_failure_diffs_by_default() {
    let workspace = TempDir::new().unwrap();
    let passing = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
        "--format",
        "table",
    ]);

    assert!(passing.status.success());
    assert!(passing.stderr.is_empty());
    let passing = String::from_utf8(passing.stdout).unwrap();
    assert!(passing.starts_with("Execution path: LOCAL\nMode: READ-ONLY GRAPH"));
    assert!(passing.contains("Stage: EXECUTE SCENARIOS → MINE EVIDENCE → DIFF EXPECTED"));
    assert!(passing.contains("SCENARIOS · results render incrementally in declaration order"));
    assert!(
        passing.contains(
            "PASS rey.fixture.text-normalize · 01/02 plain · 1/1 outputs equal · required"
        )
    );
    assert!(passing.contains(
        "PASS rey.fixture.text-normalize · 02/02 surrounded · 1/1 outputs equal · required"
    ));
    assert!(passing.contains("Workload result: QUALIFIED · 2/2 scenarios passing"));
    assert!(passing.contains("PORTFOLIO CONFORMANCE"));
    assert!(passing.contains("Deltas: 2 equal · 0 different · 0 inconclusive"));
    assert!(!passing.contains("Evidence format:"));
    assert!(!passing.contains("Evidence matches:"));
    assert!(!passing.contains("Exact bindings:"));
    assert!(!passing.contains("\u{1b}["));

    let failing = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_MISMATCH_WORKLOAD_ID,
        "--format",
        "table",
    ]);

    assert_eq!(failing.status.code(), Some(2));
    assert!(failing.stderr.is_empty());
    let failing = String::from_utf8(failing.stdout).unwrap();
    assert!(failing.contains(
        "FAIL rey.fixture.text-mismatch · 02/02 surrounded · 0/1 outputs equal · required"
    ));
    assert!(failing.contains("Evidence deltas:"));
    assert!(failing.contains("Delta (output text):"));
    assert!(failing.contains("@@ text · utf8 @@"));
    assert!(failing.contains("- REY"));
    assert!(failing.contains("+  REY "));
    assert!(failing.contains("Result: GAPS FOUND"));
    assert!(failing.contains("Deltas: 1 equal · 1 different · 0 inconclusive"));
    assert!(!failing.contains("Evidence format:"));
    assert!(!failing.contains("Exact bindings:"));
    assert!(!failing.contains("\u{1b}["));
}

#[test]
fn workload_test_verbose_levels_expand_evidence_without_changing_json() {
    let workspace = TempDir::new().unwrap();
    let verbose = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_NORMALIZE_WORKLOAD_ID,
        "--format",
        "table",
        "-v",
    ]);

    assert!(verbose.status.success());
    assert!(verbose.stderr.is_empty());
    let verbose = String::from_utf8(verbose.stdout).unwrap();
    assert!(verbose.contains("Execution model: DETERMINISTIC SERIAL · 2 nodes"));
    assert_eq!(verbose.matches("Evidence format:").count(), 2);
    assert_eq!(verbose.matches("Evidence matches:").count(), 2);
    assert!(verbose.contains("Match (output text):"));
    assert!(verbose.contains("   \"SPOKE\""));
    assert!(verbose.contains("Stop reason: qualified"));
    assert!(verbose.contains("Qualification: issued"));
    assert!(!verbose.contains("Workload binding:"));
    assert!(!verbose.contains("Exact bindings:"));

    let very_verbose = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_MISMATCH_WORKLOAD_ID,
        "--format",
        "table",
        "-vv",
    ]);

    assert_eq!(very_verbose.status.code(), Some(2));
    assert!(very_verbose.stderr.is_empty());
    let very_verbose = String::from_utf8(very_verbose.stdout).unwrap();
    assert!(very_verbose.contains("Execution model: DETERMINISTIC SERIAL · 1 node"));
    assert!(very_verbose.contains("Workload binding: rey.fixture.text-mismatch@1 · blake3:"));
    assert!(very_verbose.contains("Graph binding: rey.fixture.text-mismatch.graph@1 · blake3:"));
    assert!(
        very_verbose.contains("Scenario suite: rey.fixture.text-mismatch.scenarios@1 · blake3:")
    );
    assert!(very_verbose.contains("Evaluator: rey.scenario.utf8-exact@1 · blake3:"));
    assert_eq!(very_verbose.matches("Exact bindings:").count(), 2);
    assert!(very_verbose.contains("scenario    rey.fixture.text-mismatch.scenario.surrounded@1"));
    assert!(very_verbose.contains("execution   blake3:"));
    assert!(very_verbose.contains("delta       blake3:"));
    assert!(very_verbose.contains("Test result: blake3:"));
    assert!(very_verbose.contains("- REY"));
    assert!(very_verbose.contains("+  REY "));

    let structured = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_MISMATCH_WORKLOAD_ID,
        "-vv",
    ]);
    assert_eq!(structured.status.code(), Some(2));
    assert!(structured.stderr.is_empty());
    let structured: WorkloadTestBatch = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(structured.results.len(), 1);
    assert_eq!(structured.results[0].status, TestStatus::Failed);

    let help = run_rey(&["workloads", "test", "--help"]);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("-v, --verbose..."));
    assert!(help.contains("repeat as -vv for exact identity bindings"));
}

#[test]
fn source_mining_is_verifiable_across_test_list_status_and_run() {
    let workspace = TempDir::new().unwrap();

    let tested = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "test",
        BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
        "--format",
        "table",
        "-vv",
    ]);
    assert!(tested.status.success());
    assert!(tested.stderr.is_empty());
    let tested = String::from_utf8(tested.stdout).unwrap();
    for needle in [
        "Mining admission: VERIFIED",
        "PASS rey.fixture.source-search · 01/04 empty",
        "PASS rey.fixture.source-search · 02/04 exact",
        "FAIL rey.fixture.source-search · 03/04 mismatch",
        "INCONCLUSIVE rey.fixture.source-search · 04/04 truncated",
        "rey.source-match-delta.v1 (typed relation)",
        "Match relation: DIFFERENT",
        "OMISSION match_limit",
        "operation   rey.source-search.literal-utf8@1",
        "provider    rey.local-source.builtin@1",
        "request     blake3:",
        "result      blake3:",
        "text view   rey.text-delta.terminal-patch@1",
        "match view  rey.source-matches.terminal-table@1",
        "context     rey-local-source://",
        "Delta-directed reasoning:",
        "frontier    blake3:",
        "scheduled   blake3:",
        "surface     blake3:",
    ] {
        assert!(tested.contains(needle), "missing test evidence: {needle}");
    }

    let listed = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "list",
        "--format",
        "table",
    ]);
    assert!(listed.status.success());
    assert!(listed.stderr.is_empty());
    let listed = String::from_utf8(listed.stdout).unwrap();
    assert!(
        listed.contains("Mining                 1 workloads · 4 retained results · 1 incomplete")
    );
    assert!(listed.contains(
        "Operations             rey.source-search.literal-utf8@1 → rey.builtin.source-matches.render-lines@1"
    ));
    assert!(listed.contains(
        "Mining evidence        4 results · 3 complete · 1 incomplete · 4 relation deltas · 1 reasoning surfaces"
    ));

    let status = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "status",
        BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    assert!(status.stderr.is_empty());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("scenario=rey.fixture.source-search.scenario.mismatch"));
    assert!(status.contains("Match relation: DIFFERENT"));
    assert!(status.contains("Delta-directed reasoning:"));
    assert!(status.contains("qualification=blake3:"));

    fs::write(
        workspace.path().join("sample.txt"),
        "alpha evidence\nbeta\n",
    )
    .unwrap();
    let run = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "run",
        BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
        "--input",
        "evidence",
        "--source",
        "sample.txt",
        "--format",
        "table",
    ]);
    assert!(run.status.success());
    assert!(run.stderr.is_empty());
    let run = String::from_utf8(run.stdout).unwrap();
    assert!(run.contains("status=Passed reason=completed"));
    assert!(run.contains("node_order=[\"search\",\"render\"]"));
    assert!(run.contains("completeness=complete"));
    assert!(run.contains("operation=rey.source-search.literal-utf8@1"));
    assert!(run.contains("provider=rey.local-source.builtin@1"));
    assert!(run.contains("location=sample.txt:1:6-14 text=\"evidence\""));
    assert!(run.contains("bindings corpus=blake3:"));
    assert!(run.contains("view=rey.source-matches.terminal-table@1:blake3:"));
    assert!(run.contains("context=rey-local-source://"));

    let structured = run_rey(&[
        "workloads",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "run",
        BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
        "--input",
        "evidence",
        "--source",
        "sample.txt",
    ]);
    assert!(structured.status.success());
    assert!(structured.stderr.is_empty());
    let structured: WorkloadRunResult = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(structured.status, RunStatus::Passed);
    assert_eq!(structured.mining.len(), 1);
    assert_eq!(
        structured.mining[0].evidence.result.completeness,
        MiningCompleteness::Complete
    );
    assert_eq!(structured.mining[0].evidence.matches.len(), 1);
    structured.verify().unwrap();
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
    assert_eq!(batch.results.len(), 3);
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

fn run_rey(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rey"))
        .args(args)
        .output()
        .unwrap()
}

fn run_rey_with_env(args: &[&str], variables: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rey"));
    command.args(args);
    for (name, value) in variables {
        command.env(name, value);
    }
    command.output().unwrap()
}

fn run_rey_with_stdin_env(
    args: &[&str],
    input: &str,
    variables: &[(&str, &str)],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rey"));
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in variables {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
