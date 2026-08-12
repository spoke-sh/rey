use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::{Command, Stdio},
};

use rey::channels::{
    ChannelApplyResult, ChannelDiff, ChannelGraphSnapshot, ChannelStatus, ChannelWorkingState,
};
use rey::editor::{EditorCommitResult, EditorStatus, EditorWorkingState};
use rey::env::{
    EnvironmentAddResult, EnvironmentCommitResult, EnvironmentDiff, EnvironmentDiffMode,
    EnvironmentLog, EnvironmentStatus, EnvironmentWorkingState,
};
use rey::workloads::{
    QualificationState, WorkloadCatalogKind, WorkloadCreateResult, WorkloadFreshness, WorkloadList,
    WorkloadOrigin, WorkloadProposalKind, WorkloadRunView, WorkloadStatusBatch, WorkloadTestBatch,
};
use rey_mining::MiningCompleteness;
use rey_runtime::{
    BUILT_IN_MISMATCH_WORKLOAD_ID, BUILT_IN_NORMALIZE_WORKLOAD_ID,
    BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID, BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID, RunStatus,
    ScenarioEvaluation, TestStatus,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn checked_in_scene_is_available_through_the_default_editor_surface() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let state = TempDir::new().unwrap();
    let output = run_rey(&[
        "editor",
        "--workspace",
        workspace.to_str().unwrap(),
        "--state-dir",
        state.path().to_str().unwrap(),
        "status",
        "--format",
        "json",
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let status: EditorStatus = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status.state, EditorWorkingState::Working);
    assert_eq!(status.working.project_id, "rey-county");
    assert_eq!(status.working.coverage.sources, 5);
    assert_eq!(status.working.coverage.features, 34);
    assert_eq!(status.working.coverage.markers, 12);
    assert_eq!(status.working.coverage.coordinates, 137);
    assert!(status.head.is_none());
    assert!(status.index.is_none());

    let human = run_rey(&[
        "editor",
        "--workspace",
        workspace.to_str().unwrap(),
        "--state-dir",
        state.path().to_str().unwrap(),
        "status",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "On scene no commits yet",
        "Changes not staged for scene commit:",
        "new:       source: rey-county-boundary",
        "new:       feature: rey-county-markers/poi-rey-county-seat",
        "no changes added to scene commit (use `rey editor add` to stage)",
    ] {
        assert!(
            human.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }
    for snapshot_detail in [
        "State         ",
        "HEAD→INDEX",
        "Scene snapshot:",
        "Admission ",
    ] {
        assert!(
            !human.contains(snapshot_detail),
            "status leaked detailed evidence: {snapshot_detail}"
        );
    }
}

#[test]
fn editor_cli_commits_agent_tuned_generated_sources_without_admitting_explore_state() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    for removed in ["init", "import", "validate", "package", "inspect"] {
        let output = run_rey(&["editor", removed]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
    }
    let help = run_rey(&["editor", "--help"]);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for command in ["generate", "status", "add", "commit", "log", "diff"] {
        assert!(
            help.lines()
                .any(|line| line.trim_start().starts_with(command))
        );
    }
    for removed in ["init", "import", "validate", "package", "inspect"] {
        assert!(
            !help
                .lines()
                .any(|line| line.trim_start().starts_with(removed))
        );
    }

    let generated = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "generate",
        "terrain",
        "terrain.geojson",
        "--id",
        "agent-terrain",
        "--scene-id",
        "semantic-atlas",
        "--seed",
        "7",
        "--west",
        "-123",
        "--south",
        "37",
        "--east",
        "-122",
        "--north",
        "38",
        "--features",
        "2",
        "--vertices",
        "5",
        "--format",
        "table",
    ]);
    assert!(generated.status.success());
    assert!(generated.stderr.is_empty());
    let generated = String::from_utf8(generated.stdout).unwrap();
    assert!(generated.contains("created scene project"));
    assert!(generated.contains("2 features · 12 coordinate positions"));
    assert!(workspace.path().join("rey.scene.json").is_file());

    let source_path = workspace.path().join("terrain.geojson");
    let mut source: Value = serde_json::from_slice(&fs::read(&source_path).unwrap()).unwrap();
    source["features"][0]["properties"]["name"] = Value::String("Agent tuned ridge".to_owned());
    fs::write(&source_path, serde_json::to_vec_pretty(&source).unwrap()).unwrap();

    let working = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(working.status.success());
    let working = String::from_utf8(working.stdout).unwrap();
    for evidence in [
        "On scene no commits yet",
        "Changes not staged for scene commit:",
        "new:       source: agent-terrain",
        "new:       feature: agent-terrain/control-0001",
        "no changes added to scene commit",
    ] {
        assert!(
            working.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }

    let added = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "table",
    ]);
    assert!(added.status.success());
    let added = String::from_utf8(added.stdout).unwrap();
    assert!(added.contains("SCENE INDEX"));
    assert!(added.contains("3 scene changes staged"));
    assert!(added.contains("native objects frozen · not admitted"));

    let committed = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "retain agent tuned terrain",
        "--format",
        "table",
    ]);
    assert!(committed.status.success());
    assert!(committed.stderr.is_empty());
    let committed = String::from_utf8(committed.stdout).unwrap();
    assert!(committed.contains("[SCENE@1"));
    assert!(committed.contains("validation complete"));
    assert!(committed.contains("1 sources · 2 features · 0 omissions"));
    assert!(committed.contains("candidate only"));
    assert!(committed.contains("admitted=false · /explore unchanged"));

    let clean = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert_eq!(
        String::from_utf8(clean.stdout).unwrap(),
        "On scene SCENE@1\n\nnothing to commit, working scene clean\n"
    );

    source["features"][0]["properties"]["name"] = Value::String("Second tuning pass".to_owned());
    fs::write(&source_path, serde_json::to_vec_pretty(&source).unwrap()).unwrap();
    let diff = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "diff",
        "--format",
        "table",
    ]);
    let diff = String::from_utf8(diff.stdout).unwrap();
    assert!(diff.contains("INDEX → WORKING"));
    assert!(diff.contains("DIFFERENT · +0 -0 ~2"));
    assert!(diff.contains("~ source  agent-terrain"));
    assert!(diff.contains("~ feature agent-terrain/control-0001"));

    let logged = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "log",
        "-p",
        "--format",
        "table",
    ]);
    let logged = String::from_utf8(logged.stdout).unwrap();
    assert!(logged.contains("commit SCENE@1"));
    assert!(logged.contains("retain agent tuned terrain"));
    assert!(logged.contains("SCENE CHANGE SET"));
    assert!(logged.contains("candidate only · no admission claim"));
}

#[test]
fn editor_generate_retains_tunable_deterministic_terrain_lineage() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let generation_args = [
        "editor",
        "--workspace",
        workspace_path,
        "generate",
        "terrain",
        "generated.geojson",
        "--id",
        "procedural-relief",
        "--scene-id",
        "generated-atlas",
        "--seed",
        "42",
        "--west",
        "-123.0",
        "--south",
        "37.0",
        "--east",
        "-122.0",
        "--north",
        "38.0",
        "--features",
        "4",
        "--vertices",
        "6",
        "--uplift-ratio",
        "0.75",
        "--strength",
        "0.8",
        "--roughness",
        "0.4",
        "--anisotropy",
        "2.25",
        "--orientation-degrees",
        "55",
        "--falloff",
        "3.5",
        "--format",
        "table",
    ];
    let generated = run_rey(&generation_args);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    assert!(generated.stderr.is_empty());
    let generated = String::from_utf8(generated.stdout).unwrap();
    for evidence in [
        "Generated deterministic terrain source procedural-relief",
        "created scene project",
        "rey.editor.terrain-controls@1 · seed 42",
        "4 features · 28 coordinate positions",
        "uplift=0.750 · strength=0.800±0.240 · roughness=0.400±0.200 · falloff=3.500",
        "anisotropy=2.250 · orientation=55.0°±45.0°",
        "generated WORKING candidate",
    ] {
        assert!(
            generated.contains(evidence),
            "missing generation evidence: {evidence}"
        );
    }

    let generated_bytes = fs::read(workspace.path().join("generated.geojson")).unwrap();
    let document: Value = serde_json::from_slice(&generated_bytes).unwrap();
    assert_eq!(
        document["rey_generation"]["schema"],
        "rey.scene-generation.v1"
    );
    assert_eq!(document["rey_generation"]["seed"], 42);
    assert_eq!(document["rey_generation"]["parameters"]["anisotropy"], 2.25);
    assert_eq!(document["features"].as_array().unwrap().len(), 4);

    let replayed = run_rey(&generation_args);
    assert!(replayed.status.success());
    assert!(
        String::from_utf8(replayed.stdout)
            .unwrap()
            .contains("Verified deterministic terrain source")
    );
    assert_eq!(
        fs::read(workspace.path().join("generated.geojson")).unwrap(),
        generated_bytes
    );

    let status = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let status: EditorStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status.working.coverage.sources, 1);
    assert_eq!(status.working.coverage.features, 4);
    assert_eq!(status.working.coverage.coordinates, 28);
    assert_eq!(status.unstaged.inserted, 5);

    assert!(
        run_rey(&[
            "editor",
            "--workspace",
            workspace_path,
            "add",
            "--format",
            "json",
        ])
        .status
        .success()
    );
    let committed = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "generate procedural relief",
        "--format",
        "json",
    ]);
    let committed: EditorCommitResult = serde_json::from_slice(&committed.stdout).unwrap();
    assert_eq!(committed.commit.sequence, 1);
    assert_eq!(committed.package.snapshot.coverage.features, 4);

    fs::write(
        workspace.path().join("manual.geojson"),
        r#"{"type":"FeatureCollection","features":[]}"#,
    )
    .unwrap();
    let refused = run_rey(&[
        "editor",
        "--workspace",
        workspace_path,
        "generate",
        "terrain",
        "manual.geojson",
        "--id",
        "manual",
        "--west",
        "-123",
        "--south",
        "37",
        "--east",
        "-122",
        "--north",
        "38",
    ]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("refusing to overwrite"));
}

#[test]
fn channels_topology_is_cli_first_semantic_and_restart_safe() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let clean = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(clean.status.success());
    assert!(clean.stderr.is_empty());
    assert_eq!(
        String::from_utf8(clean.stdout).unwrap(),
        "On channels built-in\n\nnothing to commit, channel working tree clean\n"
    );
    assert!(!workspace.path().join(".rey").exists());

    let listed = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(listed.status.success());
    assert!(listed.stderr.is_empty());
    let listed = String::from_utf8(listed.stdout).unwrap();
    for evidence in [
        "CHANNEL GRAPH",
        "1 channel · 1 subscription · 3 streams · 0 relays",
        "01 / CHANNELS",
        "workspace@1  Workspace",
        "finding · question · progress · blocker · handoff",
        "02 / SUBSCRIPTIONS",
        "03 / FEED STREAMS",
        "01  Signals  signals@1",
        "02  Admission  admission@1",
        "03  Flow  flow@1",
        "signals → admission → flow",
        "04 / RELAYS",
        "none · transport not configured",
    ] {
        assert!(
            listed.contains(evidence),
            "missing Channel evidence: {evidence}"
        );
    }
    assert!(!workspace.path().join(".rey").exists());

    let clean_json = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let clean_json: ChannelStatus = serde_json::from_slice(&clean_json.stdout).unwrap();
    assert_eq!(clean_json.schema, "rey.channel-status.v1");
    assert_eq!(clean_json.state, ChannelWorkingState::Clean);
    assert!(!clean_json.working_present);
    assert!(clean_json.index.is_none());
    assert_eq!(clean_json.working.graph.layout.stream_ids[0], "signals");

    let invalid_revision = r#"schema: rey.channel-graph.v1
channels:
  - id: workspace
    revision: 1
    name: Workspace
    scope: workspace_local
    accepted_observation_kinds: [finding, question, progress, blocker, handoff]
    broadcast_default: true
subscriptions:
  - id: workspace
    revision: 1
    channel_ids: [workspace]
    observation_kinds: [finding, question, progress, blocker, handoff]
    filters: {}
    limit: 64
streams:
  - id: signals
    revision: 1
    name: Signals
    subscription_id: workspace
    lens: signals
  - id: admission
    revision: 1
    name: Review
    subscription_id: workspace
    lens: admission
  - id: flow
    revision: 1
    name: Flow
    subscription_id: workspace
    lens: flow
layout:
  id: feed
  revision: 2
  stream_ids: [admission, signals, flow]
relays: []
"#;
    fs::write(
        workspace.path().join("invalid-channels.yaml"),
        invalid_revision,
    )
    .unwrap();
    let invalid = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "apply",
        "invalid-channels.yaml",
        "--format",
        "json",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&invalid.stderr)
            .contains("stream admission changed without advancing its revision")
    );

    let proposed = invalid_revision.replace(
        "id: admission\n    revision: 1",
        "id: admission\n    revision: 2",
    );
    fs::write(workspace.path().join("channels.yaml"), proposed).unwrap();
    let applied = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "apply",
        "channels.yaml",
        "--format",
        "json",
    ]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert!(applied.stderr.is_empty());
    let applied: ChannelApplyResult = serde_json::from_slice(&applied.stdout).unwrap();
    assert!(applied.applied);
    assert_eq!(applied.schema, "rey.channel-apply-result.v1");
    assert_eq!(applied.snapshot.source.locator, "worktree:///channels.yaml");
    assert_eq!(applied.delta.summary.renamed, 1);
    assert_eq!(applied.delta.summary.moved, 2);
    assert!(
        workspace
            .path()
            .join(".rey/channels/working.json")
            .is_file()
    );

    let status = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    for evidence in [
        "On channels built-in",
        "Changes in channel working tree:",
        "renamed:    stream: admission · name \"Admission\" → \"Review\"",
        "moved:      stream: admission · position 2 → 1",
        "channel working tree differs from built-in",
    ] {
        assert!(
            status.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }
    assert!(!status.contains("content_digest"));
    assert!(!status.contains("provenance"));

    let diff = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "diff",
        "--format",
        "table",
    ]);
    assert!(diff.status.success());
    assert!(diff.stderr.is_empty());
    let diff = String::from_utf8(diff.stdout).unwrap();
    for evidence in [
        "REY CHANNELS DIFF · BUILT-IN → WORKING",
        "01 / CHANNELS",
        "02 / SUBSCRIPTIONS",
        "03 / FEED STREAMS",
        "~  stream admission · name \"Admission\" → \"Review\"",
        "~  stream admission · position 2 → 1",
        "04 / RELAYS",
    ] {
        assert!(diff.contains(evidence), "missing diff evidence: {evidence}");
    }
    assert!(!diff.contains("serialized"));

    let diff_json = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "diff",
        "--format",
        "json",
    ]);
    let diff_json: ChannelDiff = serde_json::from_slice(&diff_json.stdout).unwrap();
    assert_eq!(diff_json.schema, "rey.channel-diff.v1");
    assert_eq!(diff_json.delta.summary.renamed, 1);
    assert_eq!(diff_json.delta.summary.moved, 2);

    let restarted = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let restarted: ChannelGraphSnapshot = serde_json::from_slice(&restarted.stdout).unwrap();
    assert_eq!(restarted.source.locator, "worktree:///channels.yaml");
    assert_eq!(restarted.graph.layout.stream_ids[0], "admission");
    assert_eq!(restarted.graph.stream("admission").unwrap().name, "Review");

    let repeated = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "apply",
        "channels.yaml",
        "--format",
        "table",
    ]);
    assert!(repeated.status.success());
    assert_eq!(
        String::from_utf8(repeated.stdout).unwrap(),
        "nothing to apply, channel working tree unchanged\n"
    );

    let escaped = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "--state-dir",
        "../outside",
        "status",
    ]);
    assert_eq!(escaped.status.code(), Some(1));
    assert!(escaped.stdout.is_empty());
    assert!(String::from_utf8_lossy(&escaped.stderr).contains("escapes the workspace"));

    let help = run_rey(&["channels", "--help"]);
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for command in ["list", "status", "diff", "apply"] {
        assert!(help.contains(command));
    }
    for unavailable in ["add", "commit", "log"] {
        assert!(
            !help
                .lines()
                .any(|line| line.trim_start().starts_with(unavailable))
        );
    }
}

#[cfg(unix)]
#[test]
fn channels_apply_rejects_symlinked_graph_input() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().unwrap();
    fs::write(workspace.path().join("graph.yaml"), "schema: invalid\n").unwrap();
    symlink(
        workspace.path().join("graph.yaml"),
        workspace.path().join("linked.yaml"),
    )
    .unwrap();
    let output = run_rey(&[
        "channels",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "apply",
        "linked.yaml",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("channel graph must be a regular non-symlinked file")
    );
    assert!(!workspace.path().join(".rey").exists());
}

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
        "On environment no commits yet",
        "Changes not staged for environment commit:",
        "new:       environment variable: HOME",
        "new:       application: git",
        "new:       typed interchange: Arrow stream frames (frame.arrow-stream)",
        "new:       mining capability: literal UTF-8 source search (source.search.literal-utf8)",
        "new:       context surface: workspace metadata (workspace.metadata)",
        "No environment commits yet. Use `rey env add`",
    ] {
        assert!(
            unborn.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }
    for inventory_detail in [
        "Workspace              ",
        "Working tree           ",
        "Observation            ",
        "Applications           ",
        "Reasoning map          ",
    ] {
        assert!(!unborn.contains(inventory_detail));
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
    assert_eq!(first.schema, "rey.environment-commit-result.v1");
    assert_eq!(first.commit.schema, "rey.environment-commit.v1");
    assert_eq!(first.commit.sequence, 1);
    assert!(first.commit.committed_at_unix > 0);
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

    let git_changed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(git_changed.status.success());
    let git_changed = String::from_utf8(git_changed.stdout).unwrap();
    assert_eq!(
        git_changed,
        "On environment ENV@1\n\nnothing to commit, working environment clean\n"
    );

    let git_diff = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "diff",
        "--format",
        "json",
    ]);
    assert!(git_diff.status.success());
    let git_diff: EnvironmentDiff = serde_json::from_slice(&git_diff.stdout).unwrap();
    assert!(git_diff.delta.changes.is_empty());

    let changed_path = format!(
        "{}:/rey-environment-path-drift",
        std::env::var("PATH").unwrap_or_default()
    );
    let drift = [("PATH", changed_path.as_str())];
    let changed = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--format",
            "table",
        ],
        &drift,
    );
    assert!(changed.status.success());
    let changed = String::from_utf8(changed.stdout).unwrap();
    for evidence in [
        "On environment ENV@1",
        "Changes not staged for environment commit:",
        "modified:  environment variable: PATH",
        "modified:  application: git",
        "modified:  application: rg",
        "modified:  application: codex",
        "no changes added to environment commit",
    ] {
        assert!(
            changed.contains(evidence),
            "missing changed evidence: {evidence}"
        );
    }
    let diff = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "diff",
            "--format",
            "table",
        ],
        &drift,
    );
    assert!(diff.status.success());
    let diff = String::from_utf8(diff.stdout).unwrap();
    for evidence in [
        "REY ENV DIFF · INDEX → WORKING",
        "View                   UNSTAGED",
        "Evidence               DIFFERENT · 9 authoritative capability changes",
        "01 / DIRECTED TEXT",
        "Environment variables · 3 tracked · 1 changed",
        "02 / BOUNDED SEARCH",
        "APPLICATIONS · 8 searched",
        "8 changed",
        "REFERENCE PLANE",
        "Inputs and topology",
    ] {
        assert!(diff.contains(evidence), "missing diff evidence: {evidence}");
    }
    assert!(!diff.contains("CAPABILITY PATCH"));
    assert!(!diff.contains("git.repository.inspect"));

    let added = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--format",
            "table",
        ],
        &drift,
    );
    assert!(added.status.success());
    let added = String::from_utf8(added.stdout).unwrap();
    assert!(added.contains("ENVIRONMENT ADMISSION"));
    assert!(added.contains("9 capability changes admitted"));
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
    assert_eq!(staged_diff.delta.summary.modified, 9);

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
    assert!(second.stdout.is_empty());
    assert!(second.stderr.is_empty());

    let log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-n",
        "2",
        "--format",
        "table",
    ]);
    assert!(log.status.success());
    assert!(log.stderr.is_empty());
    let log = String::from_utf8(log.stdout).unwrap();
    for evidence in [
        "REY ENV LOG",
        "2 total · 2 shown · newest first",
        "commit ENV@2 ",
        "Parent: ENV@1 ",
        "Date:   ",
        "Evidence               ENV@1 → ENV@2 · DIFFERENT · 9 authoritative capability changes",
        "Environment            3 variables · 8 applications · 0 inputs · 0 references · complete",
        "Changes                1 variable · 8 applications · 0 inputs · 0 references",
        "    stage fixture",
    ] {
        assert!(log.contains(evidence), "missing log evidence: {evidence}");
    }
    assert!(!log.contains("01 / DIRECTED TEXT"));
    assert!(!log.contains("CAPABILITY PATCH"));
    assert!(!log.contains("Delta id"));
    assert!(!log.contains("Retention"));

    let one = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-n",
        "1",
        "--format",
        "table",
    ]);
    assert!(one.status.success());
    let one = String::from_utf8(one.stdout).unwrap();
    assert!(one.contains("1 shown"));
    assert!(one.contains("stage fixture"));
    assert!(!one.contains("baseline"));

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
    assert!(patch_log.stderr.is_empty());
    let patch_log = String::from_utf8(patch_log.stdout).unwrap();
    for evidence in [
        "REY ENV LOG",
        "2 total · 2 shown · newest first",
        "commit ENV@2 ",
        "Parent: ENV@1 ",
        "Date:   ",
        "    stage fixture",
        "01 / DIRECTED TEXT",
        "@@ ENV@1 → ENV@2",
        "02 / BOUNDED SEARCH",
        "REFERENCE PLANE",
        "@@ EMPTY → ENV@1",
    ] {
        assert!(
            patch_log.contains(evidence),
            "missing log evidence: {evidence}"
        );
    }
    assert_eq!(patch_log.matches("01 / DIRECTED TEXT").count(), 2);
    assert!(!patch_log.contains("CAPABILITY PATCH"));

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
    assert!(
        status
            .working_snapshot
            .capabilities
            .iter()
            .any(|row| row.capability_id == "tool.git.identity")
    );
    assert!(
        status
            .working_snapshot
            .capabilities
            .iter()
            .all(|row| row.capability_id != "git.repository.inspect")
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
    capture: value
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
    purpose: Search the bounded fixture corpus
    required: true
    potential_capabilities: [source.search]
  - id: missing
    kind: executable
    name: rey-definitely-missing
    purpose: Exercise missing desired application evidence
    required: false
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

    let process_only = run_rey_with_env(
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
    assert!(process_only.status.success());
    let process_only: Value = serde_json::from_slice(&process_only.stdout).unwrap();
    assert!(process_only["operator"]["mapping"].is_null());
    assert!(
        process_only["operator"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .all(|variable| variable["working"]["name"] != "REY_MODE"
                && variable["working"]["name"] != "REY_SECRET")
    );
    assert!(
        process_only["operator"]["applications"]
            .as_array()
            .unwrap()
            .iter()
            .all(|application| application["working"]["name"] != "rey-map-probe")
    );

    let inspected = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--map",
            "rey.env.yaml",
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
    assert!(rendered.contains("development-mode-value"));
    assert!(!invocation_marker.exists());
    let status_document: Value = serde_json::from_slice(&inspected.stdout).unwrap();
    let rows = status_document["working_snapshot"]["capabilities"]
        .as_array()
        .unwrap();
    assert!(rows.iter().any(|row| {
        row["capability_id"] == "env.mapping.graph" && row["capability_kind"] == "environment_map"
    }));
    assert_eq!(
        status_document["operator"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variable| variable["object_id"] == "mode")
            .unwrap()["working"]["value"],
        "development-mode-value"
    );
    assert!(
        status_document["operator"]["variables"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variable| variable["object_id"] == "secret")
            .unwrap()["working"]["value"]
            .is_null()
    );
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
    let probe_application = status_document["operator"]["applications"]
        .as_array()
        .unwrap()
        .iter()
        .find(|application| application["object_id"] == "probe")
        .unwrap();
    assert_eq!(
        probe_application["working"]["purpose"],
        "Search the bounded fixture corpus"
    );
    assert_eq!(
        status_document["operator"]["schema"],
        "rey.environment-operator-projection.v1"
    );
    assert_eq!(
        status_document["operator"]["application_inventory"]["working"]["schema"],
        "rey.environment-application-inventory.v1"
    );
    assert!(
        status_document["operator"]["application_inventory"]["working"]["inventory_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("blake3:"))
    );

    let status = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "status",
            "--map",
            "rey.env.yaml",
            "--format",
            "table",
        ],
        &variables,
    );
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    for evidence in [
        "On environment no commits yet",
        "Changes not staged for environment commit:",
        "new:       environment variable: REY_MODE",
        "new:       environment variable: REY_SECRET",
        "new:       application: rey-map-probe",
        "new:       application: rey-definitely-missing",
        "new:       input: input.txt",
        "new:       reference: mode --locates--> input",
    ] {
        assert!(
            status.contains(evidence),
            "missing env status evidence: {evidence}"
        );
    }
    assert!(!status.contains("never-retain-this-secret"));

    let added = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--map",
            "rey.env.yaml",
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
            "--map",
            "rey.env.yaml",
            "--format",
            "table",
        ],
        &changed_variables,
    );
    assert!(diff.status.success());
    assert!(diff.stderr.is_empty());
    let diff = String::from_utf8(diff.stdout).unwrap();
    for evidence in [
        "REY ENV DIFF · INDEX → WORKING",
        "View                   UNSTAGED",
        "01 / DIRECTED TEXT",
        "Environment variables · 5 tracked · 1 changed",
        "@@ INDEX → WORKING",
        "- REY_MODE=development-mode-value",
        "+ REY_MODE=production-mode-value",
        "  REY_SECRET=<present:redacted>",
        "02 / BOUNDED SEARCH",
        "APPLICATIONS · 10 searched",
        "0 errors · 0 changed",
        "rey-map-probe",
        "rey-definitely-missing",
        "REFERENCE PLANE",
        "Inputs and topology",
        "INPUTS · 1 tracked · 1 changed",
        "- input.txt · required",
        "+ input.txt · required",
        "TOPOLOGY · 2 declared edges · 0 changed",
        "mode --locates--> input",
        "input --consumed_by--> probe",
    ] {
        assert!(
            diff.contains(evidence),
            "missing mapped diff evidence: {evidence}\n{diff}"
        );
    }
    assert!(!diff.contains("never-retain-this-secret"));
    assert!(!diff.contains("a-different-secret"));
    assert!(!diff.contains("CAPABILITY PATCH"));

    let diff = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "diff",
            "--map",
            "rey.env.yaml",
            "--format",
            "json",
        ],
        &changed_variables,
    );
    assert!(diff.status.success());
    assert!(diff.stderr.is_empty());
    let diff: EnvironmentDiff = serde_json::from_slice(&diff.stdout).unwrap();
    assert_eq!(diff.schema, "rey.environment-diff.v1");
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

    let staged = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--map",
            "rey.env.yaml",
            "--format",
            "json",
        ],
        &changed_variables,
    );
    assert!(staged.status.success());
    let staged_diff = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "diff",
            "--map",
            "rey.env.yaml",
            "--staged",
            "--format",
            "table",
        ],
        &changed_variables,
    );
    assert!(staged_diff.status.success());
    assert!(staged_diff.stderr.is_empty());
    let staged_diff = String::from_utf8(staged_diff.stdout).unwrap();
    for evidence in [
        "REY ENV DIFF · ENV@1 → INDEX",
        "View                   STAGED",
        "@@ ENV@1 → INDEX",
        "- REY_MODE=development-mode-value",
        "+ REY_MODE=production-mode-value",
        "INPUTS · 1 tracked · 1 changed",
    ] {
        assert!(
            staged_diff.contains(evidence),
            "missing staged diff evidence: {evidence}\n{staged_diff}"
        );
    }

    let committed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "update mapped environment",
        "--format",
        "json",
    ]);
    assert!(committed.status.success());
    assert!(committed.stderr.is_empty());

    let log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-n",
        "1",
        "--format",
        "table",
    ]);
    assert!(log.status.success());
    assert!(log.stderr.is_empty());
    let log = String::from_utf8(log.stdout).unwrap();
    for evidence in [
        "REY ENV LOG",
        "commit ENV@2 ",
        "Parent: ENV@1 ",
        "Date:   ",
        "Evidence               ENV@1 → ENV@2 · DIFFERENT",
        "Environment            5 variables · 10 applications · 1 input · 2 references · complete",
        "Changes                1 variable · 0 applications · 1 input · 0 references",
        "Reasoning map          rey.env.yaml · rey.env-map.v1",
        "    update mapped environment",
    ] {
        assert!(
            log.contains(evidence),
            "missing mapped log evidence: {evidence}\n{log}"
        );
    }
    assert!(!log.contains("01 / DIRECTED TEXT"));
    assert!(!log.contains("Snapshot"));
    assert!(!log.contains("Capabilities"));
    assert!(!log.contains("Delta id"));
    assert!(!log.contains("Retention"));

    let patch_log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-p",
        "-n",
        "1",
        "--format",
        "table",
    ]);
    assert!(patch_log.status.success());
    assert!(patch_log.stderr.is_empty());
    let patch_log = String::from_utf8(patch_log.stdout).unwrap();
    for evidence in [
        "01 / DIRECTED TEXT",
        "@@ ENV@1 → ENV@2",
        "- REY_MODE=development-mode-value",
        "+ REY_MODE=production-mode-value",
        "02 / BOUNDED SEARCH",
        "APPLICATIONS · 10 searched",
        "0 errors · 0 changed",
        "REFERENCE PLANE",
        "INPUTS · 1 tracked · 1 changed",
        "- input.txt · required",
        "+ input.txt · required",
        "TOPOLOGY · 2 declared edges · 0 changed",
    ] {
        assert!(
            patch_log.contains(evidence),
            "missing mapped patch log evidence: {evidence}\n{patch_log}"
        );
    }
    assert!(!patch_log.contains("never-retain-this-secret"));
    assert!(!patch_log.contains("a-different-secret"));
    assert!(!patch_log.contains("CAPABILITY PATCH"));

    let json_log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-p",
        "-n",
        "1",
        "--format",
        "json",
    ]);
    assert!(json_log.status.success());
    assert!(json_log.stderr.is_empty());
    let json_log: EnvironmentLog = serde_json::from_slice(&json_log.stdout).unwrap();
    assert_eq!(json_log.schema, "rey.environment-log.v1");
    assert!(json_log.entries[0].commit.committed_at_unix > 0);
    assert!(json_log.patch);
    assert_eq!(json_log.entries.len(), 1);
    assert_eq!(json_log.entries[0].delta.source_label, "ENV@1");
    assert_eq!(json_log.entries[0].delta.target_label, "ENV@2");

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
            "--map",
            "rey.env.yaml",
            "--format",
            "json",
        ],
        &changed_variables,
    );
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("cannot retain a value digest"));

    let retained_log = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "log",
        "-p",
        "-n",
        "1",
        "--format",
        "table",
    ]);
    assert!(retained_log.status.success());
    assert!(retained_log.stderr.is_empty());
    let retained_log = String::from_utf8(retained_log.stdout).unwrap();
    assert!(retained_log.contains("+ REY_MODE=production-mode-value"));
    assert!(retained_log.contains("rey-map-probe"));
    assert!(retained_log.contains("mode --locates--> input"));
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
        run_rey(&[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--map",
            "rey.env.yaml",
        ])
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
        "--map",
        "rey.env.yaml",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_format.status.code(), Some(1));
    assert!(invalid_format.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid_format.stderr).contains("requires human table"));

    let partial = run_rey_with_stdin_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "-p",
            "--map",
            "rey.env.yaml",
        ],
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
    assert!(partial.contains("Hunk 1/2"));
    assert!(partial.contains("Hunk 2/2"));
    assert!(partial.contains("diff --rey a/environment/"));
    assert!(partial.contains("Stage this hunk [y,n,q,a,d,?]?"));
    assert!(partial.contains("1 capability changes admitted"));
    assert!(partial.contains("1 changes remain unstaged"));

    let mixed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--map",
        "rey.env.yaml",
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
        "--map",
        "rey.env.yaml",
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
fn env_add_patch_never_dumps_structured_provenance() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    fs::write(workspace.path().join("alpha.txt"), "alpha\n").unwrap();
    fs::write(workspace.path().join("beta.txt"), "beta\n").unwrap();
    fs::write(
        workspace.path().join("rey.env.yaml"),
        "schema: rey.env-map.v1\nnodes:\n  - id: alpha\n    kind: file\n    path: alpha.txt\n    required: true\n",
    )
    .unwrap();
    assert!(
        run_rey(&[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--map",
            "rey.env.yaml",
        ])
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
            "baseline map",
        ])
        .status
        .success()
    );
    fs::write(
        workspace.path().join("rey.env.yaml"),
        "schema: rey.env-map.v1\nnodes:\n  - id: alpha\n    kind: file\n    path: alpha.txt\n    required: true\n  - id: beta\n    kind: file\n    path: beta.txt\n    required: true\n",
    )
    .unwrap();

    let partial = run_rey_with_stdin_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "-p",
            "--map",
            "rey.env.yaml",
        ],
        "y\nn\n",
        &[],
    );
    assert!(
        partial.status.success(),
        "{}",
        String::from_utf8_lossy(&partial.stderr)
    );
    let partial = String::from_utf8(partial.stdout).unwrap();
    assert!(partial.contains("env.mapping.graph"));
    assert!(partial.contains(
        "provenance: changed · structured value omitted; inspect with `rey env diff --format json`"
    ));
    assert!(!partial.contains("\\\"nodes\\\""));
}

#[test]
fn workspace_package_is_the_default_catalog_and_retains_harness_provenance() {
    let workspace = TempDir::new().unwrap();
    let package_dir = workspace
        .path()
        .join("workloads/portfolio-label-normalization");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("workload.yaml"),
        include_str!("../../../workloads/portfolio-label-normalization/workload.yaml"),
    )
    .unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    assert!(listed.stderr.is_empty());
    assert!(!workspace.path().join(".rey").exists());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.catalog.kind, WorkloadCatalogKind::WorkspacePackages);
    assert_eq!(listed.workloads.len(), 1);
    assert_eq!(
        listed.workloads[0].workload.id,
        "rey.portfolio.label-normalization"
    );
    let provenance = listed.workloads[0].provenance.as_ref().unwrap();
    assert_eq!(provenance.origin, WorkloadOrigin::WorkspacePackage);
    assert_eq!(
        provenance.generation.as_ref().map(|value| value.kind),
        Some(WorkloadProposalKind::CodingHarness)
    );
    assert!(
        listed
            .workloads
            .iter()
            .all(|workload| !workload.workload.id.starts_with("rey.fixture."))
    );
    let conformance = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "--catalog",
        "conformance",
        "list",
    ]);
    assert!(conformance.status.success());
    let conformance: WorkloadList = serde_json::from_slice(&conformance.stdout).unwrap();
    assert_eq!(
        conformance.catalog.kind,
        WorkloadCatalogKind::BuiltInConformance
    );
    assert_eq!(conformance.workloads.len(), 4);

    let table = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(table.contains("Catalog                WORKSPACE PACKAGES · 1 admitted · 0 draft"));
    assert!(table.contains("Origin                 WORKSPACE PACKAGE · workloads/"));
    assert!(table.contains("Generator              CODING HARNESS · codex@gpt-5"));
    assert!(table.contains("Scenario oracle        FROZEN AT ADMISSION"));

    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "rey.portfolio.label-normalization",
    ]);
    assert!(tested.status.success());
    let tested: WorkloadTestBatch = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(tested.catalog.kind, WorkloadCatalogKind::WorkspacePackages);
    assert_eq!(tested.results[0].status, TestStatus::Passed);
    assert_eq!(tested.workloads[0].origin, WorkloadOrigin::WorkspacePackage);

    let status = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "rey.portfolio.label-normalization",
    ]);
    assert!(status.status.success());
    let status: WorkloadStatusBatch = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status.catalog.kind, WorkloadCatalogKind::WorkspacePackages);
    assert_eq!(
        status.statuses[0]
            .summary
            .provenance
            .as_ref()
            .map(|value| value.origin),
        Some(WorkloadOrigin::WorkspacePackage)
    );
    assert_eq!(
        status.statuses[0].summary.qualification,
        QualificationState::Qualified
    );

    let run = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "rey.portfolio.label-normalization",
        "--input",
        " create ",
    ]);
    assert!(run.status.success());
    let run: WorkloadRunView = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(run.provenance.origin, WorkloadOrigin::WorkspacePackage);
    assert_eq!(run.result.status, RunStatus::Passed);
    assert_eq!(
        run.result.outputs["text"],
        rey_runtime::WorkloadValue::Utf8("CREATE".to_owned())
    );

    let revised = include_str!("../../../workloads/portfolio-label-normalization/workload.yaml")
        .replace(
            "producer_revision: gpt-5",
            "producer_revision: gpt-5-revised",
        );
    fs::write(package_dir.join("workload.yaml"), revised).unwrap();
    let relisted = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(relisted.status.success());
    let relisted: WorkloadList = serde_json::from_slice(&relisted.stdout).unwrap();
    assert_eq!(
        relisted.workloads[0].qualification,
        QualificationState::Stale
    );
}

#[test]
fn workload_create_is_a_visible_coding_harness_request_and_admission_boundary() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let created = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "create",
        "api-drift",
        "--title",
        "API drift mining",
        "--intent",
        "Mine API drift and formalize authoritative scenarios",
        "--format",
        "table",
    ]);
    assert!(created.status.success());
    assert!(created.stderr.is_empty());
    let created = String::from_utf8(created.stdout).unwrap();
    for evidence in [
        "Execution path: LOCAL STATE",
        "Mode: APPLY",
        "CREATE REQUEST → AWAIT CODING HARNESS",
        "WORKLOAD CREATION",
        "Admission              AWAITING CODING HARNESS",
        "Graph                  MISSING",
        "Scenario oracle        NOT ADMITTED",
        "AGENT INSTRUCTIONS",
        "never derive expected values from candidate execution",
        "Further action required YES",
    ] {
        assert!(
            created.contains(evidence),
            "missing create evidence: {evidence}"
        );
    }

    let request_path = workspace.path().join("workloads/api-drift/request.yaml");
    let request_before = fs::read(&request_path).unwrap();
    assert!(
        !workspace
            .path()
            .join("workloads/api-drift/workload.yaml")
            .exists()
    );
    let request: Value = serde_json::from_slice(&request_before).unwrap();
    assert_eq!(request["schema"], "rey.workload-creation-request.v1");
    assert_eq!(request["proposer"], "coding_harness");
    assert_eq!(
        request["target_package"],
        "workloads/api-drift/workload.yaml"
    );

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.catalog.workload_count, 1);
    assert_eq!(listed.catalog.admitted_count, 0);
    assert_eq!(listed.catalog.draft_count, 1);
    assert!(listed.workloads.is_empty());
    assert_eq!(listed.drafts[0].request.workload_id, "api-drift");

    let status = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "api-drift",
    ]);
    assert!(status.status.success());
    let status: WorkloadStatusBatch = serde_json::from_slice(&status.stdout).unwrap();
    assert!(status.statuses.is_empty());
    assert_eq!(
        status.drafts[0].request.intent.as_deref(),
        Some("Mine API drift and formalize authoritative scenarios")
    );

    for command in ["test", "run"] {
        let rejected = run_rey_workspace(&[
            "workloads",
            "--workspace",
            workspace_path,
            command,
            "api-drift",
        ]);
        assert_eq!(rejected.status.code(), Some(1));
        assert!(rejected.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&rejected.stderr).contains("awaiting coding harness hydration")
        );
    }

    let duplicate = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "create",
        "api-drift",
    ]);
    assert_eq!(duplicate.status.code(), Some(1));
    assert!(duplicate.stdout.is_empty());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("refusing to overwrite"));
    assert_eq!(fs::read(&request_path).unwrap(), request_before);

    let machine = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "create",
        "schema-mining",
    ]);
    assert!(machine.status.success());
    let machine: WorkloadCreateResult = serde_json::from_slice(&machine.stdout).unwrap();
    assert_eq!(machine.draft.request.workload_id, "schema-mining");
    assert!(machine.action_required);
    assert_eq!(
        machine.created_files,
        ["workloads/schema-mining/request.yaml"]
    );

    let immutable = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "--catalog",
        "conformance",
        "create",
        "not-allowed",
    ]);
    assert_eq!(immutable.status.code(), Some(1));
    assert!(immutable.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&immutable.stderr)
            .contains("built-in conformance workloads are immutable")
    );
}

#[test]
fn journal_cli_admits_agent_entries_without_executing_typed_blocks() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    let proposal = workspace.path().join("entry.yaml");
    fs::write(
        &proposal,
        r#"schema: rey.journal-entry-proposal.v1
title: Inspect source coverage
author:
  kind: agent
  id: codex
binding:
  coordinate: rey+local://workload/source-mining?revision=blake3%3Aabc
  scale: 1.46
  source_revision: blake3:abc
blocks:
  - kind: prose
    id: context
    document:
      - kind: paragraph
        text: Coverage moved after the latest survey.
  - kind: query
    id: coverage-query
    language: sql
    provider: spoke
    mode: read_only
    statement: select * from coverage
    parameters: {}
"#,
    )
    .unwrap();

    let admitted = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "add",
        "entry.yaml",
        "--format",
        "json",
    ]);
    assert!(admitted.status.success());
    assert!(admitted.stderr.is_empty());
    let admitted: Value = serde_json::from_slice(&admitted.stdout).unwrap();
    assert_eq!(admitted["schema"], "rey.journal-admission.v1");
    assert_eq!(admitted["admitted"], true);
    assert_eq!(admitted["entry"]["sequence"], 1);
    assert_eq!(admitted["entry"]["author"]["kind"], "agent");
    assert_eq!(admitted["entry"]["blocks"][1]["kind"], "query");
    assert_eq!(admitted["entry"]["blocks"][1]["mode"], "read_only");

    let repeated = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "add",
        "entry.yaml",
        "--format",
        "table",
    ]);
    assert!(repeated.status.success());
    let repeated = String::from_utf8(repeated.stdout).unwrap();
    assert!(repeated.contains("JOURNAL ENTRY ALREADY ADMITTED"));
    assert!(repeated.contains("agent / codex"));
    assert!(repeated.contains("rey+local://workload/source-mining"));
    assert!(repeated.contains("/journal/j1-inspect-source-coverage--blake3-"));

    let listed = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    assert!(listed.status.success());
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed["schema"], "rey.journal-log.v1");
    assert_eq!(listed["entries"].as_array().unwrap().len(), 1);
    assert!(workspace.path().join(".rey/journal/journal.json").is_file());

    let human_proposal = fs::read_to_string(&proposal)
        .unwrap()
        .replace("kind: agent", "kind: human");
    fs::write(workspace.path().join("human.yaml"), human_proposal).unwrap();
    let rejected = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "add",
        "human.yaml",
    ]);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("rey journal add accepts agent-authored entries")
    );
}

#[test]
fn ui_cli_serves_the_embedded_precision_operator_surface_with_explicit_exposure() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let mut table_child = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "ui",
            "--workspace",
            workspace_path,
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            "--format",
            "table",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let table_stdout = table_child.stdout.take().unwrap();
    let mut table_reader = BufReader::new(table_stdout);
    let mut table = String::new();
    loop {
        let mut line = String::new();
        assert!(table_reader.read_line(&mut line).unwrap() > 0);
        table.push_str(&line);
        if line.contains("Press Ctrl-C") {
            break;
        }
    }
    for evidence in [
        "REY UI",
        "Status                 LISTENING",
        "Exposure               LOOPBACK ONLY",
        "Application            TANSTACK ROUTER · EMBEDDED",
        "Grammar                HIFI KINETIC · PRECISION",
        "Data plane             LIVE READS · UNAUTHENTICATED JOURNAL WRITE",
        "Human entry            /explore",
        "Revalidation           5000ms · PASSIVE · NO REFRESH CONTROL",
        "/api/v1/health · /api/v1/cadence · /api/v1/environment · /api/v1/journal · /api/v1/workloads",
        "Grammar revision       git:058c6504fc10740360717e97e687fd77bef6a5c5",
        "Implementation         https://github.com/spoke-sh/rey · ",
    ] {
        assert!(table.contains(evidence), "missing UI evidence: {evidence}");
    }
    let address = table
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("Address").map(str::trim))
        .filter(|address| !address.is_empty())
        .unwrap();
    let response = http_request(address, "GET /api/v1/health HTTP/1.1");
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\"theme\":\"precision\""));
    let environment = http_request(address, "GET /api/v1/environment HTTP/1.1");
    assert!(environment.starts_with("HTTP/1.1 200"));
    assert!(environment.contains("\"schema\":\"rey.environment-status.v1\""));
    let cadence = http_request(address, "GET /api/v1/cadence HTTP/1.1");
    assert!(cadence.starts_with("HTTP/1.1 200"));
    assert!(cadence.contains("\"schema\":\"rey.ui-cadence.v1\""));
    assert!(cadence.contains("\"ordering\":\"partial\""));
    table_child.kill().unwrap();
    let table_output = table_child.wait_with_output().unwrap();
    assert!(table_output.stderr.is_empty());

    let mut network_child = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "ui",
            "--workspace",
            workspace_path,
            "--host",
            "0.0.0.0",
            "--port",
            "0",
            "--format",
            "json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut network_stdout = BufReader::new(network_child.stdout.take().unwrap());
    let mut descriptor_line = String::new();
    network_stdout.read_line(&mut descriptor_line).unwrap();
    let descriptor: Value = serde_json::from_str(&descriptor_line).unwrap();
    assert_eq!(descriptor["schema"], "rey.ui-server.v1");
    assert_eq!(descriptor["host"], "0.0.0.0");
    assert_eq!(descriptor["loopback_only"], false);
    assert_eq!(descriptor["read_only"], false);
    assert_eq!(descriptor["journal_write_enabled"], true);
    assert_eq!(descriptor["application"], "tanstack_router");
    assert_eq!(descriptor["grammar"], "kinetic");
    assert_eq!(descriptor["theme"], "precision");
    assert_eq!(descriptor["entry_route"], "/explore");
    assert_eq!(descriptor["live_refresh_interval_ms"], 5_000);
    assert_eq!(
        descriptor["source_repository"],
        "https://github.com/spoke-sh/rey"
    );
    assert!(descriptor["implementation_revision"].is_string());
    assert_eq!(
        descriptor["grammar_revision"],
        "git:058c6504fc10740360717e97e687fd77bef6a5c5"
    );
    let mut network_stderr = BufReader::new(network_child.stderr.take().unwrap());
    let mut warning = String::new();
    network_stderr.read_line(&mut warning).unwrap();
    assert!(warning.contains("unauthenticated Journal writes enabled"));

    let network_address = format!("127.0.0.1:{}", descriptor["port"].as_u64().unwrap());
    let proposal = serde_json::json!({
        "schema": "rey.journal-entry-proposal.v1",
        "title": "Write through the network listener",
        "author": { "kind": "human", "id": "operator" },
        "binding": {
            "coordinate": "rey+local://portfolio/current?revision=blake3%3Anetwork",
            "scale": 0.68,
            "source_revision": "blake3:network"
        },
        "blocks": [{
            "kind": "prose",
            "id": "context",
            "document": [{ "kind": "paragraph", "text": "No authentication required." }]
        }]
    })
    .to_string();
    let admitted = http_request_with_body(
        &network_address,
        "POST /api/v1/journal HTTP/1.1",
        &[("Content-Type", "application/json")],
        &proposal,
    );
    assert!(admitted.starts_with("HTTP/1.1 201"));
    assert!(admitted.contains("\"admitted\":true"));
    network_child.kill().unwrap();
    network_child.wait().unwrap();
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
    assert_eq!(list.workloads.len(), 4);
    assert_eq!(
        list.workloads
            .iter()
            .map(|workload| workload.workload.id.as_str())
            .collect::<Vec<_>>(),
        [
            BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
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
            "Qualification          0/4 qualified · 0 failing · 0 inconclusive · 0 stale"
        )
    );
    assert!(
        table.contains(
            "Scenarios              0/12 passing · 0/12 evaluated · 0 stale · 2 optional"
        )
    );
    assert!(table.contains("Runs                   0 passed · 0 blocked · 4 not run"));
    assert!(
        table.contains(
            "Inventory              4 total · 4 admitted · 0 draft · 0 tested · 4 untested"
        )
    );
    assert!(
        table.contains("Mining                 2 workloads · 0 retained results · 0 incomplete")
    );
    assert!(table.contains(
        "Attention              0 refine · 2 retest · 0 create · 0 blocked · 2 policy excluded"
    ));
    assert!(table.contains("Coverage               0 mapped surfaces · 0 owned · 0 unowned"));
    assert!(table.contains("ATTENTION FRONTIER"));
    assert!(table.contains("rey.fixture.source-search · untested · ready"));
    assert_eq!(table.matches("Journey                TEST").count(), 4);
    assert_eq!(
        table
            .matches("░░░░░░░░░░░░░░░░░░░░    0%  0/2 passing · 0/2 evaluated")
            .count(),
        3
    );
    assert!(table.contains("0%  0/6 passing · 0/6 evaluated"));
    assert!(table.contains("Graph                  rey.fixture.text-normalize.graph@1"));
    assert!(table.contains("Candidate              blake3:"));
    assert_eq!(table.matches("Qualification          UNTESTED").count(), 4);
    assert_eq!(table.matches("Test evidence          none").count(), 4);
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
            "Qualification          3/4 qualified · 1 failing · 0 inconclusive · 0 stale"
        )
    );
    assert!(
        evolved.contains(
            "Scenarios              11/12 passing · 12/12 evaluated · 0 stale · 2 optional"
        )
    );
    assert!(evolved.contains("Runs                   1 passed · 0 blocked · 3 not run"));
    assert!(
        evolved.contains(
            "Inventory              4 total · 4 admitted · 0 draft · 4 tested · 0 untested"
        )
    );
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
    let blocked: WorkloadRunView = serde_json::from_slice(&blocked.stdout).unwrap();
    let blocked = &blocked.result;
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
    let executed: WorkloadRunView = serde_json::from_slice(&executed.stdout).unwrap();
    let executed = &executed.result;
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
        listed.contains("Mining                 2 workloads · 4 retained results · 1 incomplete")
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
    assert!(run.contains("Result                 PASSED"));
    assert!(run.contains("Stop reason            completed"));
    assert!(run.contains("Node order             search → render"));
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
    let structured: WorkloadRunView = serde_json::from_slice(&structured.stdout).unwrap();
    let structured = &structured.result;
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
fn context_topography_is_verifiable_across_cli_structured_state_and_ui_read_model() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    materialize_context_topography_fixture(workspace.path());

    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "context-anchor-survey",
        "--format",
        "table",
        "-vv",
    ]);
    assert!(
        tested.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&tested.stdout),
        String::from_utf8_lossy(&tested.stderr),
    );
    assert!(tested.stderr.is_empty());
    let tested = String::from_utf8(tested.stdout).unwrap();
    for needle in [
        "Topography admission: VERIFIED",
        "Topography patch:",
        "Coverage:",
        "Directed patch:",
        "World geometry: 1 admitted chart",
        "Survey atmosphere:",
        "Natural-feature basis:",
        "retained seed edges remain inspector provenance",
        "Hydrology projection:",
        "no discovered or built path claim",
        "Projection packet:",
        "Projection boundary:",
        "synthetic distance is not language or semantic distance",
        "SEED AGENTS.md",
        "LOCATOR",
        "ANCHOR",
        "EDGE",
        "REGION",
        "PROBE",
        "verify absence or repair reference",
        "OMISSION candidate_limit",
        "Exact topography bindings:",
        "Exact projection bindings:",
        "field pyramid 3 levels · 12953 cells · 712415 bytes allocated",
        "field level overview · 31×21 · stride 4 · 651 cells · 35805 bytes · world",
        "field level local · 121×81 · stride 1 · 9801 cells · 539055 bytes · neighborhoods/objects/evidence",
        "elevation · scalar",
        "projection omission",
        "operation   rey.context-anchor-survey.locate@1",
        "provider    rey.provider.local-worktree-topography@1",
        "limits      seeds=32",
    ] {
        assert!(
            tested.contains(needle),
            "missing topography evidence: {needle}"
        );
    }

    let readme_path = workspace.path().join("README.md");
    let mut expanded_readme = fs::read_to_string(&readme_path).unwrap();
    for index in 0..30 {
        expanded_readme.push_str(&format!("\n[extra-{index}](docs/MISSING-{index}.md)"));
    }
    fs::write(readme_path, expanded_readme).unwrap();

    let run = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "context-anchor-survey",
        "--source",
        "AGENTS.md",
        "--source",
        "README.md",
        "--format",
        "table",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(run.stderr.is_empty());
    let run = String::from_utf8(run.stdout).unwrap();
    assert!(run.contains("Result                 PASSED"));
    assert!(run.contains("Node order             survey → render"));
    assert!(run.contains("Topography patch:"));
    assert!(run.contains("CLI projection folds"));
    assert!(run.contains("structured output retains all rows"));

    let structured = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "context-anchor-survey",
        "--source",
        "AGENTS.md",
        "--source",
        "README.md",
    ]);
    assert!(structured.status.success());
    assert!(structured.stderr.is_empty());
    let structured: WorkloadRunView = serde_json::from_slice(&structured.stdout).unwrap();
    assert_eq!(structured.result.topography.len(), 1);
    let patch = &structured.result.topography[0];
    assert_eq!(patch.schema, "rey.topography-patch.v1");
    assert_eq!(patch.coverage.requested_seeds, 2);
    assert!(patch.coverage.candidates > 24);
    assert!(!patch.anchors.is_empty());
    assert!(!patch.regions.is_empty());
    assert!(!patch.frontier.is_empty());
    assert_eq!(patch.delta.target_revision, patch.topography_revision);
    structured.result.verify().unwrap();

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    assert!(listed.stderr.is_empty());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    let summary = listed
        .workloads
        .iter()
        .find(|summary| summary.workload.id == "context-anchor-survey")
        .unwrap();
    assert_eq!(summary.topography_results, 4);
    assert_eq!(
        summary.topography_revision.as_ref(),
        Some(&patch.topography_revision)
    );
    assert_eq!(summary.topography_coverage.as_ref(), Some(&patch.coverage));
    assert_eq!(summary.topography_patch.as_ref(), Some(patch));
    let projection = summary.topography_projection.as_ref().unwrap();
    projection.verify_for(patch).unwrap();
    assert_eq!(projection.schema, "rey.projection-packet.v1");
    assert_eq!(projection.source_patch_id, patch.patch_id);
    assert_eq!(projection.field_pyramid.levels.len(), 3);
    assert_eq!(projection.field_pyramid.total_cells, 12_953);
    assert_eq!(projection.field_pyramid.total_bytes, 712_415);
    assert_eq!(
        projection.excluded_source_relationships,
        patch.edges.len() as u64
    );
    assert!(
        projection
            .objects
            .iter()
            .all(|object| matches!(object.kind.as_str(), "anchor" | "frontier"))
    );
    let atlas = listed.semantic_atlas.as_ref().unwrap();
    atlas.verify().unwrap();
    assert_eq!(atlas.schema, "rey.semantic-atlas.v1");
    assert_eq!(atlas.regions.len(), 1);
    assert_eq!(atlas.clusters.len(), 1);
    assert_eq!(atlas.regions[0].source_patch_id, patch.patch_id);
    assert_eq!(
        atlas.regions[0].source_topography_revision,
        patch.topography_revision
    );
    assert_eq!(atlas.coordinate_system.kind, "synthetic_semantic_sphere");
    assert!(atlas.coordinate_system.earth_crs.is_none());
    assert_eq!(
        atlas.layout_policy.zoom_rule,
        "zoom selects retained level of detail and never reclusters"
    );

    let listed_table = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(listed_table.status.success());
    assert!(listed_table.stderr.is_empty());
    let listed_table = String::from_utf8(listed_table.stdout).unwrap();
    assert!(listed_table.contains("Semantic atlas"));
    assert!(listed_table.contains("1 regions in 1 world clusters"));
    assert!(listed_table.contains("synthetic semantic longitude/latitude"));
    assert!(listed_table.contains("not Earth CRS84"));
    assert!(listed_table.contains("zoom selects retained LOD and never reclusters"));

    let status = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "context-anchor-survey",
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("Topography revision"));
    assert!(status.contains("Topography coverage"));
    assert!(status.contains("unique candidates resolved ·"));
    assert!(status.contains("frontier"));
    assert!(status.contains("Topography patch:"));

    let mut ui = Command::new(env!("CARGO_BIN_EXE_rey"))
        .args([
            "ui",
            "--workspace",
            workspace_path,
            "--host",
            "127.0.0.1",
            "--port",
            "0",
            "--format",
            "json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = BufReader::new(ui.stdout.take().unwrap());
    let mut descriptor = String::new();
    reader.read_line(&mut descriptor).unwrap();
    let descriptor: Value = serde_json::from_str(&descriptor).unwrap();
    let address = format!("127.0.0.1:{}", descriptor["port"].as_u64().unwrap());
    let response = http_request(&address, "GET /api/v1/workloads HTTP/1.1");
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\"topography_patch\""));
    assert!(response.contains("\"topography_projection\""));
    assert!(response.contains("\"schema\":\"rey.projection-packet.v1\""));
    assert!(response.contains("\"schema\":\"rey.semantic-atlas.v1\""));
    assert!(response.contains("\"kind\":\"synthetic_semantic_sphere\""));
    assert!(response.contains(patch.patch_id.as_str()));
    assert!(response.contains("\"state\":\"unexplored\""));
    ui.kill().unwrap();
    ui.wait().unwrap();
}

fn materialize_context_topography_fixture(workspace: &std::path::Path) {
    fs::create_dir_all(workspace.join("workloads/context-anchor-survey")).unwrap();
    fs::create_dir_all(workspace.join("docs")).unwrap();
    fs::create_dir_all(workspace.join("docs/decisions")).unwrap();
    fs::create_dir_all(workspace.join("plans")).unwrap();
    fs::write(
        workspace.join("workloads/context-anchor-survey/workload.yaml"),
        include_str!("../../../workloads/context-anchor-survey/workload.yaml"),
    )
    .unwrap();
    fs::write(
        workspace.join("workloads/context-anchor-survey/request.yaml"),
        include_str!("../../../workloads/context-anchor-survey/request.yaml"),
    )
    .unwrap();
    fs::write(
        workspace.join("AGENTS.md"),
        include_str!("../../rey-runtime/tests/fixtures/topography-projects/basic/AGENTS.md"),
    )
    .unwrap();
    fs::write(
        workspace.join("README.md"),
        include_str!("../../rey-runtime/tests/fixtures/topography-projects/basic/README.md"),
    )
    .unwrap();
    fs::write(
        workspace.join("docs/GUIDE.md"),
        include_str!("../../rey-runtime/tests/fixtures/topography-projects/basic/docs/GUIDE.md"),
    )
    .unwrap();
    fs::write(
        workspace.join("plans/0017-incremental-context-topography.md"),
        include_str!("../../../plans/0017-incremental-context-topography.md"),
    )
    .unwrap();
    fs::write(
        workspace.join("docs/decisions/0041-continuous-coordinate-topography.md"),
        include_str!("../../../docs/decisions/0041-continuous-coordinate-topography.md"),
    )
    .unwrap();
}

#[test]
fn portfolio_mining_is_verifiable_across_test_list_status_and_run() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let initial = run_rey(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(initial.status.success());
    assert!(initial.stderr.is_empty());
    assert!(!workspace.path().join(".rey").exists());
    let initial: WorkloadList = serde_json::from_slice(&initial.stdout).unwrap();
    assert_eq!(initial.attention.summary.retest, 2);
    assert_eq!(initial.attention.summary.policy_excluded, 2);
    assert_eq!(initial.attention.summary.create, 0);

    let tested = run_rey(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
        "--format",
        "table",
        "-vv",
    ]);
    assert!(tested.status.success());
    assert!(tested.stderr.is_empty());
    let tested = String::from_utf8(tested.stdout).unwrap();
    for needle in [
        "Portfolio mining: VERIFIED",
        "PASS rey.portfolio.attention · 01/06 blocked",
        "PASS rey.portfolio.attention · 02/06 clean",
        "PASS rey.portfolio.attention · 03/06 create",
        "PASS rey.portfolio.attention · 04/06 excluded",
        "PASS rey.portfolio.attention · 05/06 refine",
        "PASS rey.portfolio.attention · 06/06 retest",
        "rey.workload-attention.v1 (typed relation)",
        "required_capability_unavailable",
        "No unresolved portfolio attention",
        "dependency_changed",
        "policy_excluded",
        "derivation  rey.portfolio.attention.derive@1",
    ] {
        assert!(
            tested.contains(needle),
            "missing portfolio evidence: {needle}"
        );
    }

    let run = run_rey(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
    ]);
    assert!(run.status.success());
    assert!(run.stderr.is_empty());
    let run: WorkloadRunView = serde_json::from_slice(&run.stdout).unwrap();
    let run = &run.result;
    assert_eq!(run.attention.len(), 1);
    assert_eq!(run.attention[0].summary.retest, 2);
    assert_eq!(run.attention[0].summary.policy_excluded, 1);
    run.verify().unwrap();

    let status = run_rey(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("WORKLOAD STATUS"));
    assert!(status.contains("rey.portfolio.attention"));
    assert!(status.contains("ATTENTION FRONTIER"));
    assert!(status.contains("rey.fixture.text-normalize · untested · ready"));

    fs::write(workspace.path().join("input.txt"), "portfolio surface\n").unwrap();
    fs::write(
        workspace.path().join("rey.env.yaml"),
        r#"schema: rey.env-map.v1
nodes:
  - id: input
    kind: file
    path: input.txt
    required: true
edges: []
"#,
    )
    .unwrap();
    let added = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "add",
        "--map",
        "rey.env.yaml",
        "--format",
        "json",
    ]);
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    let committed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "admit portfolio surface",
    ]);
    assert!(committed.status.success());

    let listed = run_rey(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(listed.status.success());
    let listed = String::from_utf8(listed.stdout).unwrap();
    assert!(listed.contains("Attention              0 refine · 2 retest · 1 create"));
    assert!(listed.contains("Coverage               1 mapped surfaces · 0 owned · 1 unowned"));
    assert!(listed.contains("input.txt · unowned_surface · ready"));

    let rerun = run_rey(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
        "--format",
        "table",
    ]);
    assert!(rerun.status.success());
    let rerun = String::from_utf8(rerun.stdout).unwrap();
    assert!(rerun.contains("Portfolio attention: 4 rows"));
    assert!(rerun.contains("create   input.txt · unowned_surface"));
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
    assert_eq!(batch.results.len(), 4);
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_rey"));
    command.args(args);
    // Runner regressions in this file predate workspace packages and deliberately
    // exercise the compiled diagnostic catalog. Workspace-package behavior has
    // a separate end-to-end test that calls `run_rey_workspace`.
    if args.first() == Some(&"workloads") {
        command.args(["--catalog", "conformance"]);
    }
    command.output().unwrap()
}

fn run_rey_workspace(args: &[&str]) -> std::process::Output {
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

fn http_request(address: &str, request_line: &str) -> String {
    http_request_with_body(address, request_line, &[], "")
}

fn http_request_with_body(
    address: &str,
    request_line: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    write!(
        stream,
        "{request_line}\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    )
    .unwrap();
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n").unwrap();
    }
    write!(stream, "\r\n{body}").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}
