use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use rey::channels::{
    ChannelApplyResult, ChannelDiff, ChannelGraphSnapshot, ChannelStatus, ChannelWorkingState,
};
use rey::conversations::{
    ConversationLog, ConversationMessageAdmission, ConversationSessionAdmission,
    ConversationTranscript, ConversationTransportAvailability,
};
use rey::editor::{EditorCommitResult, EditorStatus, EditorWorkingState};
use rey::env::{
    EnvironmentAddResult, EnvironmentCommitResult, EnvironmentDiff, EnvironmentDiffMode,
    EnvironmentLog, EnvironmentStatus, EnvironmentWorkingState,
};
use rey::git::{
    GitCadenceFailureKind, GitCadenceTickOutcome, GitOperatorStatus, GitPollOutcome,
    GitWatchOutcome, GitWatchStopReason, LocalGitState,
};
use rey::workloads::{
    QualificationState, WorkloadActivationAdmission, WorkloadActivationExecution,
    WorkloadActivationRecomputation, WorkloadCatalogKind, WorkloadChangeSet, WorkloadCreateResult,
    WorkloadFreshness, WorkloadList, WorkloadLog, WorkloadOrigin, WorkloadProposalKind,
    WorkloadRecomputationAssessment, WorkloadRevisionStatus, WorkloadRunView, WorkloadStatusBatch,
    WorkloadTestBatch,
};
use rey_core::ContractIdentity;
use rey_git::{
    GIT_ACTIVATION_TRIGGER_SCHEMA, GitActivationBudget, GitActivationEventClass,
    GitActivationTrigger, GitPathChangeKind, PathIdentity,
};
use rey_mining::MiningCompleteness;
use rey_runtime::{
    AttentionAction, AttentionReason, BUILT_IN_MISMATCH_WORKLOAD_ID,
    BUILT_IN_NORMALIZE_WORKLOAD_ID, BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID,
    BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID, RunStatus, ScenarioEvaluation, SceneAdmissionStatus,
    TestStatus,
};
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn conversation_cli_admits_exact_local_sessions_and_messages_without_transport_effects() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let initial = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    assert!(initial.status.success());
    assert!(initial.stderr.is_empty());
    let initial: ConversationTranscript = serde_json::from_slice(&initial.stdout).unwrap();
    assert_eq!(
        initial.availability,
        ConversationTransportAvailability::Unavailable
    );
    assert!(!initial.browser_write_enabled);
    assert!(!workspace.path().join(".rey").exists());

    fs::write(
        workspace.path().join("session.yaml"),
        r#"schema: rey.conversation-session-proposal.v1
title: Plan coordination
transport:
  kind: local_transcript
  provider: rey.local-transcript
  provider_revision: v1
participants:
  - participant_id: operator
    kind: human
    label: Operator
  - participant_id: codex
    kind: agent
    label: Codex
  - participant_id: observer
    kind: agent
    label: Observer
writer_ids:
  - operator
  - codex
browser_writer_id: operator
"#,
    )
    .unwrap();
    let admitted = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "session",
        "add",
        "session.yaml",
        "--format",
        "json",
    ]);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    assert!(admitted.stderr.is_empty());
    let admitted: ConversationSessionAdmission = serde_json::from_slice(&admitted.stdout).unwrap();
    assert!(admitted.admitted);
    assert_eq!(
        admitted.transcript.availability,
        ConversationTransportAvailability::Available
    );
    assert!(admitted.transcript.browser_write_enabled);
    let session_id = admitted.session.session_id;

    fs::write(
        workspace.path().join("message.yaml"),
        format!(
            "schema: rey.conversation-message-proposal.v1\nsession_id: {session_id}\nauthor_id: codex\nbody: The exact local transcript is ready for operator review.\nreply_to: null\n"
        ),
    )
    .unwrap();
    let message = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "message",
        "add",
        "message.yaml",
        "--format",
        "json",
    ]);
    assert!(
        message.status.success(),
        "{}",
        String::from_utf8_lossy(&message.stderr)
    );
    assert!(message.stderr.is_empty());
    let message: ConversationMessageAdmission = serde_json::from_slice(&message.stdout).unwrap();
    assert!(message.admitted);
    assert_eq!(message.message.sequence, 1);
    assert_eq!(
        message.message.delivery,
        rey::conversations::ConversationDeliveryState::NotAttempted
    );
    assert_eq!(message.transcript.messages.len(), 1);

    let human = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "status",
        "--session",
        session_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "CONVERSATION",
        "AVAILABLE · WORKSPACE-LOCAL TRANSCRIPT",
        "human:operator (Operator)",
        "agent:codex (Codex)",
        "SELF-ASSERTED",
        "delivery not attempted",
        "does not invoke an agent",
        "The exact local transcript is ready for operator review.",
    ] {
        assert!(
            human.contains(evidence),
            "missing evidence: {evidence}\n{human}"
        );
    }

    let listed = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "session",
        "list",
        "--format",
        "json",
    ]);
    assert!(listed.status.success());
    let log: ConversationLog = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(log.sessions.len(), 1);
    assert_eq!(log.messages.len(), 1);

    fs::write(
        workspace.path().join("denied.yaml"),
        format!(
            "schema: rey.conversation-message-proposal.v1\nsession_id: {session_id}\nauthor_id: observer\nbody: This writer is deliberately read-only.\nreply_to: null\n"
        ),
    )
    .unwrap();
    let denied = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "message",
        "add",
        "denied.yaml",
        "--format",
        "json",
    ]);
    assert!(!denied.status.success());
    assert!(denied.stdout.is_empty());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("has no write authority"));

    let retained = run_rey(&[
        "conversations",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let retained: ConversationTranscript = serde_json::from_slice(&retained.stdout).unwrap();
    assert_eq!(retained.messages.len(), 1);
}

#[test]
fn editor_status_is_read_only_before_a_scene_is_initialized() {
    let workspace = TempDir::new().unwrap();
    let output = run_rey(&[
        "editor",
        "--workspace",
        workspace.path().to_str().unwrap(),
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
    assert_eq!(status.schema, "rey.editor-status.v2");
    assert!(!status.initialized);
    assert_eq!(status.state, EditorWorkingState::Clean);
    assert!(status.working.is_none());
    assert!(status.head.is_none());
    assert!(status.index.is_none());
    assert!(status.staged.changes.is_empty());
    assert!(status.unstaged.changes.is_empty());
    assert!(!workspace.path().join(".rey").exists());
    assert!(!workspace.path().join("rey.scene.json").exists());

    let human = run_rey(&[
        "editor",
        "--workspace",
        workspace.path().to_str().unwrap(),
        "status",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "On scene no commits yet",
        "No scene project initialized.",
        "Use `rey editor generate terrain --help` to create WORKING in `.rey/editor`.",
    ] {
        assert!(
            human.contains(evidence),
            "missing status evidence: {evidence}"
        );
    }
    assert!(!workspace.path().join(".rey").exists());

    let help = run_rey(&["editor", "--help"]);
    assert!(help.status.success());
    assert!(
        !String::from_utf8(help.stdout)
            .unwrap()
            .contains("--project")
    );
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
    assert!(generated.contains("project      .rey/editor/project.json"));
    assert!(generated.contains("2 features · 12 coordinate positions"));
    assert!(workspace.path().join(".rey/editor/project.json").is_file());
    assert!(!workspace.path().join("rey.scene.json").exists());

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
    let working = status.working.unwrap();
    assert_eq!(working.coverage.sources, 1);
    assert_eq!(working.coverage.features, 4);
    assert_eq!(working.coverage.coordinates, 28);
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
        "On channels built-in (no commits yet)\n\nnothing to commit, channel working tree clean\n"
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
        "1 channel · 1 subscription · 3 streams · 0 applications · 0 relays · 0 beacons",
        "01 / CHANNELS",
        "workspace@1  Workspace",
        "finding · question · progress · blocker · handoff",
        "02 / SUBSCRIPTIONS",
        "03 / FEED STREAMS",
        "01  Signals  signals@1",
        "02  Admission  admission@1",
        "03  Flow  flow@1",
        "signals → admission → flow",
        "04 / APPLICATIONS",
        "05 / RELAYS",
        "06 / POLLING BEACONS",
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
        "On channels built-in (no commits yet)",
        "Changes not staged for channel commit:",
        "renamed:    stream: admission · name \"Admission\" → \"Review\"",
        "moved:      stream: admission · position 2 → 1",
        "no changes added to channel commit",
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
        "REY CHANNELS DIFF · INDEX → WORKING",
        "01 / CHANNELS",
        "02 / SUBSCRIPTIONS",
        "03 / FEED STREAMS",
        "~  stream admission · name \"Admission\" → \"Review\"",
        "~  stream admission · position 2 → 1",
        "05 / RELAYS",
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

    let added = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "json",
    ]);
    assert!(
        added.status.success(),
        "{}",
        String::from_utf8_lossy(&added.stderr)
    );
    let added: rey::channels::ChannelAddResult = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(added.schema, "rey.channel-add-result.v1");
    assert_eq!(added.staged.summary.renamed, 1);

    let staged_status = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    let staged_status = String::from_utf8(staged_status.stdout).unwrap();
    assert!(staged_status.contains("Changes to be committed:"));
    assert!(staged_status.contains("changes staged in the channel admission index"));

    let committed = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "Arrange local attention",
        "--format",
        "json",
    ]);
    assert!(
        committed.status.success(),
        "{}",
        String::from_utf8_lossy(&committed.stderr)
    );
    let committed: rey::channels::ChannelCommitResult =
        serde_json::from_slice(&committed.stdout).unwrap();
    assert_eq!(committed.commit.sequence, 1);
    assert_eq!(committed.commit.message, "Arrange local attention");

    let clean_after_commit = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert_eq!(
        String::from_utf8(clean_after_commit.stdout).unwrap(),
        "On channels CHANNEL@1\n\nnothing to commit, channel working tree clean\n"
    );

    let log = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "log",
        "-n",
        "1",
        "-p",
        "--format",
        "table",
    ]);
    let log = String::from_utf8(log.stdout).unwrap();
    assert!(log.contains("commit CHANNEL@1"));
    assert!(log.contains("Arrange local attention"));

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
    for command in [
        "list", "status", "diff", "apply", "add", "commit", "log", "message", "relay", "beacon",
    ] {
        assert!(help.contains(command));
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

#[cfg(unix)]
#[test]
fn admitted_channel_message_relays_once_through_admitted_application_and_beacon() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    let bin = workspace.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let slack = bin.join("slack");
    let delivery = workspace.path().join("delivery.txt");
    fs::write(
        &slack,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'slack 1.0'; exit 0; fi\nprintf '%s\\n' \"$*\" >> '{}'\n",
            delivery.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&slack).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&slack, permissions).unwrap();

    let env_added = run_rey_with_env(
        &[
            "env",
            "--workspace",
            workspace_path,
            "add",
            "--format",
            "json",
        ],
        &[("PATH", bin.to_str().unwrap())],
    );
    assert!(
        env_added.status.success(),
        "{}",
        String::from_utf8_lossy(&env_added.stderr)
    );
    let env_committed = run_rey(&[
        "env",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "Admit communications application",
        "--format",
        "json",
    ]);
    assert!(
        env_committed.status.success(),
        "{}",
        String::from_utf8_lossy(&env_committed.stderr)
    );

    let env_commit: EnvironmentCommitResult =
        serde_json::from_slice(&env_committed.stdout).unwrap();
    let slack_digest = env_commit
        .commit
        .snapshot
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == "comms.application.slack.identity")
        .and_then(|capability| capability.content_digest.as_deref())
        .expect("admitted Slack executable digest");
    let graph = format!(
        r#"schema: rey.channel-graph.v1
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
    filters: {{}}
    limit: 64
streams:
  - id: signals
    revision: 1
    name: Signals
    subscription_id: workspace
    lens: signals
  - id: admission
    revision: 1
    name: Admission
    subscription_id: workspace
    lens: admission
  - id: flow
    revision: 1
    name: Flow
    subscription_id: workspace
    lens: flow
layout:
  id: feed
  revision: 1
  stream_ids: [signals, admission, flow]
applications:
  - id: slack
    revision: 1
    environment_capability_id: comms.application.slack.identity
    executable_path: "{}"
    executable_version: null
    executable_digest: "{}"
    relay_argv: [send, --channel, "{{target}}", --message, "{{message}}"]
    timeout_ms: 1000
    max_output_bytes: 4096
relays:
  - id: workspace-to-slack
    revision: 1
    source_channel_id: workspace
    target_channel_locator: "slack://channel/C123"
    provider_id: slack
    hop_limit: 1
beacons:
  - id: slack-poll
    revision: 1
    application_id: slack
    relay_ids: [workspace-to-slack]
    interval_seconds: 60
    batch_limit: 8
"#,
        slack.display(),
        slack_digest,
    );
    fs::write(workspace.path().join("channels.yaml"), graph).unwrap();
    for args in [
        vec![
            "channels",
            "--workspace",
            workspace_path,
            "apply",
            "channels.yaml",
            "--format",
            "json",
        ],
        vec![
            "channels",
            "--workspace",
            workspace_path,
            "add",
            "--format",
            "json",
        ],
        vec![
            "channels",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "Admit Slack relay",
            "--format",
            "json",
        ],
    ] {
        let output = run_rey(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::write(
        workspace.path().join("message.yaml"),
        "schema: rey.channel-message.v1\nchannel_id: workspace\nkind: progress\nbody: Relay only admitted messages.\nevidence_locators: []\n",
    )
    .unwrap();
    let admitted = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "message",
        "add",
        "message.yaml",
        "--format",
        "json",
    ]);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    let admitted: rey::channels::ChannelMessageAdmission =
        serde_json::from_slice(&admitted.stdout).unwrap();

    let relayed = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "relay",
        admitted.message.message_id.as_str(),
        "--relay",
        "workspace-to-slack",
        "--format",
        "json",
    ]);
    assert!(
        relayed.status.success(),
        "{}",
        String::from_utf8_lossy(&relayed.stderr)
    );
    let attempt: rey::channels::RelayAttempt = serde_json::from_slice(&relayed.stdout).unwrap();
    assert_eq!(
        attempt.outcome,
        rey::channels::RelayAttemptOutcome::Delivered
    );
    assert_eq!(
        fs::read_to_string(&delivery).unwrap(),
        "send --channel slack://channel/C123 --message Relay only admitted messages.\n"
    );

    let tick = run_rey(&[
        "channels",
        "--workspace",
        workspace_path,
        "beacon",
        "slack-poll",
        "--format",
        "json",
    ]);
    assert!(
        tick.status.success(),
        "{}",
        String::from_utf8_lossy(&tick.stderr)
    );
    let tick: rey::channels::PollingBeaconTick = serde_json::from_slice(&tick.stdout).unwrap();
    assert_eq!(tick.checked_messages, 1);
    assert_eq!(tick.attempted, 0);
    assert_eq!(
        fs::read_to_string(&delivery).unwrap(),
        "send --channel slack://channel/C123 --message Relay only admitted messages.\n"
    );
}

#[test]
fn observations_cli_admits_broadcasts_projects_and_resolves_exact_state() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    fs::write(
        workspace.path().join("observation.yaml"),
        format!(
            "schema: rey.observation.v1\nkind: blocker\nauthor:\n  kind: agent\n  id: codex\nsubject_locator: 'rey+local://workload/alpha?revision=alpha%401'\nbody: The exact scenario delta remains unresolved.\ndesired_delta: Qualify the exact scenario.\ncompleteness: complete\nomissions: []\nevidence:\n  - locator: rey+local://evidence/scenario-alpha\n    source_revision: alpha@1\n    content_digest: blake3:{}\nsupersedes: null\n",
            "a".repeat(64)
        ),
    )
    .unwrap();
    let admitted = run_rey(&[
        "observations",
        "--workspace",
        workspace_path,
        "add",
        "observation.yaml",
        "--format",
        "json",
    ]);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    let admitted: rey::observations::ObservationBroadcast =
        serde_json::from_slice(&admitted.stdout).unwrap();
    assert!(admitted.observation_admitted);
    assert_eq!(admitted.broadcast.as_ref().unwrap().targets.len(), 1);
    assert_eq!(
        admitted.broadcast.as_ref().unwrap().targets[0].channel_id,
        "workspace"
    );

    let listed = run_rey(&[
        "observations",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let frontier: rey::observations::ObservationFrontier =
        serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(frontier.summary.unresolved, 1);
    assert_eq!(frontier.rows[0].channel_ids, ["workspace"]);

    let seeded = run_rey(&[
        "journal",
        "--workspace",
        workspace_path,
        "seed",
        admitted.observation.observation_id.as_str(),
        "--author",
        "codex",
        "--format",
        "json",
    ]);
    assert!(
        seeded.status.success(),
        "{}",
        String::from_utf8_lossy(&seeded.stderr)
    );
    let seed: rey::journal_seed::JournalSeed = serde_json::from_slice(&seeded.stdout).unwrap();
    assert_eq!(
        seed.observation_ids.as_slice(),
        std::slice::from_ref(&admitted.observation.observation_id)
    );
    assert_eq!(
        seed.proposal.author.kind,
        rey::journal::JournalAuthorKind::Agent
    );
    assert_eq!(seed.proposal.blocks.len(), 2);
    assert!(!workspace.path().join(".rey/journal/journal.json").exists());
    let seed_table = run_rey(&[
        "journal",
        "--workspace",
        workspace_path,
        "seed",
        admitted.observation.observation_id.as_str(),
        "--author",
        "codex",
        "--format",
        "table",
    ]);
    assert!(seed_table.status.success());
    let seed_table = String::from_utf8(seed_table.stdout).unwrap();
    assert!(seed_table.contains("JOURNAL SEED · UNRETAINED"));
    assert!(seed_table.contains(seed.seed_id.as_str()));
    assert!(seed_table.contains("ordinary journal add/admission required"));
    assert!(!workspace.path().join(".rey/journal/journal.json").exists());

    let shown = run_rey(&[
        "observations",
        "--workspace",
        workspace_path,
        "show",
        admitted.observation.observation_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(shown.status.success());
    let shown = String::from_utf8(shown.stdout).unwrap();
    assert!(shown.contains("self-asserted"));
    assert!(shown.contains("scenario delta remains unresolved"));

    fs::write(
        workspace.path().join("resolution.yaml"),
        format!(
            "schema: rey.observation-resolution.v1\nobservation_id: {}\nauthor:\n  kind: human\n  id: operator\nkind: resolved\nreason: The exact scenario now qualifies.\nevidence: []\n",
            admitted.observation.observation_id
        ),
    )
    .unwrap();
    let resolved = run_rey(&[
        "observations",
        "--workspace",
        workspace_path,
        "resolve",
        "resolution.yaml",
        "--format",
        "json",
    ]);
    assert!(
        resolved.status.success(),
        "{}",
        String::from_utf8_lossy(&resolved.stderr)
    );
    let resolved: rey::observations::ObservationResolutionAdmission =
        serde_json::from_slice(&resolved.stdout).unwrap();
    assert_eq!(
        resolved.detail.state,
        rey::observations::ObservationState::Resolved
    );
    assert_eq!(resolved.frontier.summary.unresolved, 0);
    let closed_seed = run_rey(&[
        "journal",
        "--workspace",
        workspace_path,
        "seed",
        admitted.observation.observation_id.as_str(),
        "--author",
        "codex",
    ]);
    assert_eq!(closed_seed.status.code(), Some(1));
    assert!(closed_seed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&closed_seed.stderr).contains("resolved, not unresolved"));
}

#[test]
fn git_cli_retains_transition_evidence_before_advancing_the_cursor() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "initial"])
            .status()
            .unwrap()
            .success()
    );

    let uninitialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(uninitialized.status.success());
    assert!(uninitialized.stderr.is_empty());
    let uninitialized = String::from_utf8(uninitialized.stdout).unwrap();
    assert!(uninitialized.contains("GIT ACTIVATION STATUS"));
    assert!(uninitialized.contains("Cursor                 UNINITIALIZED"));
    assert!(!workspace.path().join(".rey").exists());

    let initialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--format",
        "table",
    ]);
    assert!(initialized.status.success());
    let initialized = String::from_utf8(initialized.stdout).unwrap();
    assert!(initialized.contains("GIT CURSOR INITIALIZED"));
    assert!(initialized.contains("baseline only · no activation · no execution"));

    let baseline = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    assert!(baseline.status.success());
    let baseline: GitOperatorStatus = serde_json::from_slice(&baseline.stdout).unwrap();
    assert_eq!(baseline.changed_since_cursor, Some(false));
    assert!(baseline.observed_snapshot.index.as_ref().unwrap().complete);
    assert!(
        baseline
            .observed_snapshot
            .index
            .as_ref()
            .unwrap()
            .omitted_semantics
            .is_empty()
    );

    fs::write(workspace.path().join("tracked"), "two\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "second"])
            .status()
            .unwrap()
            .success()
    );

    let observed = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let observed: GitOperatorStatus = serde_json::from_slice(&observed.stdout).unwrap();
    assert_eq!(observed.changed_since_cursor, Some(true));
    let snapshot = &observed.observed_snapshot;
    let trigger = GitActivationTrigger {
        schema: GIT_ACTIVATION_TRIGGER_SCHEMA.to_owned(),
        trigger_id: "fixture.fast-forward".to_owned(),
        revision: 1,
        repository_id: snapshot.repository_id.clone(),
        worktree_id: snapshot.worktree_id.clone(),
        event_classes: vec![GitActivationEventClass::RefFastForward],
        ref_names: Vec::new(),
        path_prefixes: Vec::new(),
        require_complete: true,
        workload_id: "fixture-workload".to_owned(),
        graph: ContractIdentity::new("fixture.graph", 1, "fixture graph"),
        scenario_ids: vec!["fixture-scenario".to_owned()],
        budget: GitActivationBudget::default(),
    };
    fs::write(
        workspace.path().join("trigger.json"),
        serde_json::to_vec_pretty(&trigger).unwrap(),
    )
    .unwrap();

    let polled = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--trigger",
        "trigger.json",
        "--format",
        "table",
    ]);
    assert!(
        polled.status.success(),
        "{}",
        String::from_utf8_lossy(&polled.stderr)
    );
    let polled = String::from_utf8(polled.stdout).unwrap();
    assert!(polled.contains("GIT POLL TRANSITION"));
    assert!(polled.contains("HEAD movement          fast_forward · complete"));
    assert!(polled.contains("Semantic index         "));
    assert!(polled.contains(" · complete"));
    assert!(polled.contains("ref.fast_forward"));
    assert!(polled.contains("Activation proposals   1"));
    assert!(polled.contains("AWAITING EVIDENCE ACK"));

    let replay = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--trigger",
        "trigger.json",
        "--format",
        "json",
    ]);
    assert!(replay.status.success());
    let replay: GitPollOutcome = serde_json::from_slice(&replay.stdout).unwrap();
    assert!(replay.changed);
    assert!(replay.retained);
    assert_eq!(replay.record.proposals.len(), 1);
    assert!(replay.record.transition.source_index_complete);
    assert!(replay.record.transition.target_index_complete);
    assert!(replay.record.transition.omissions.is_empty());

    let stale = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "ack",
        "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    ]);
    assert_eq!(stale.status.code(), Some(1));
    assert!(stale.stdout.is_empty());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("acknowledgement expected"));

    let acknowledged = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "ack",
        replay.record.transition.transition_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(acknowledged.status.success());
    let acknowledged = String::from_utf8(acknowledged.stdout).unwrap();
    assert!(acknowledged.contains("GIT CURSOR ADVANCED"));
    assert!(acknowledged.contains("no Git mutation or workload execution"));

    let clean = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(clean.status.success());
    let clean = String::from_utf8(clean.stdout).unwrap();
    assert!(clean.contains("Observed delta         UNCHANGED"));
    assert!(clean.contains("Retained transitions   1"));
    assert!(clean.contains("Pending transition     none"));
}

#[test]
fn git_cli_retains_exact_watched_ref_scope_and_projects_ref_matches() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    for args in [
        vec!["add", "tracked"],
        vec!["commit", "-q", "-m", "initial"],
        vec!["branch", "release"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "two\n").unwrap();
    for args in [vec!["add", "tracked"], vec!["commit", "-q", "-m", "second"]] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    let invalid = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--watch-ref",
        "release",
        "--format",
        "json",
    ]);
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("watched-ref scope is invalid"));

    let invalid_limit = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "--max-reachable-commits-per-direction",
        "0",
        "status",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_limit.status.code(), Some(1));
    assert!(invalid_limit.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&invalid_limit.stderr)
            .contains("limits must be within their supported positive bounds")
    );
    let invalid_path_limit = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "--max-path-changes-per-ref",
        "0",
        "status",
        "--format",
        "json",
    ]);
    assert_eq!(invalid_path_limit.status.code(), Some(1));
    assert!(invalid_path_limit.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&invalid_path_limit.stderr)
            .contains("limits must be within their supported positive bounds")
    );

    let initialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--watch-ref",
        "refs/heads/release",
        "--watch-ref",
        "refs/heads/future",
        "--format",
        "json",
    ]);
    assert!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let initialized: LocalGitState = serde_json::from_slice(&initialized.stdout).unwrap();
    let snapshot = initialized.cursor_snapshot.as_ref().unwrap();
    assert_eq!(snapshot.watched_refs.len(), 2);
    assert_eq!(snapshot.watched_refs[0].name, "refs/heads/future");
    assert!(snapshot.watched_refs[0].target_oid.is_none());
    assert_eq!(snapshot.watched_refs[1].name, "refs/heads/release");
    assert!(snapshot.watched_refs[1].target_oid.is_some());

    let status = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("Watched refs           see below"));
    assert!(status.contains("refs/heads/future · ABSENT"));
    assert!(status.contains("refs/heads/release ·"));

    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "branch", "-f", "release", "HEAD"])
            .status()
            .unwrap()
            .success()
    );
    let trigger = GitActivationTrigger {
        schema: GIT_ACTIVATION_TRIGGER_SCHEMA.to_owned(),
        trigger_id: "fixture.watched-release".to_owned(),
        revision: 1,
        repository_id: snapshot.repository_id.clone(),
        worktree_id: snapshot.worktree_id.clone(),
        event_classes: vec![GitActivationEventClass::PathModified],
        ref_names: vec!["refs/heads/release".to_owned()],
        path_prefixes: vec![PathIdentity {
            encoding: "base64url".to_owned(),
            bytes: "dHJhY2tlZA".to_owned(),
            display: "tracked".to_owned(),
        }],
        require_complete: true,
        workload_id: "fixture-workload".to_owned(),
        graph: ContractIdentity::new("fixture.graph", 1, "fixture graph"),
        scenario_ids: vec!["fixture-scenario".to_owned()],
        budget: GitActivationBudget::default(),
    };
    fs::write(
        workspace.path().join("trigger.json"),
        serde_json::to_vec_pretty(&trigger).unwrap(),
    )
    .unwrap();
    let polled = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--trigger",
        "trigger.json",
        "--format",
        "json",
    ]);
    assert!(
        polled.status.success(),
        "{}",
        String::from_utf8_lossy(&polled.stderr)
    );
    let polled: GitPollOutcome = serde_json::from_slice(&polled.stdout).unwrap();
    assert_eq!(polled.record.transition.watched_ref_changes.len(), 1);
    assert_eq!(polled.record.transition.reachability_deltas.len(), 1);
    assert_eq!(
        polled.record.transition.reachability_deltas[0].ref_name,
        "refs/heads/release"
    );
    assert_eq!(
        polled.record.transition.reachability_deltas[0]
            .added_commits
            .len(),
        1
    );
    assert!(polled.record.transition.reachability_deltas[0].complete);
    assert_eq!(polled.record.transition.path_deltas.len(), 1);
    assert_eq!(polled.record.transition.path_deltas[0].changes.len(), 1);
    assert_eq!(
        polled.record.transition.path_deltas[0].changes[0].kind,
        GitPathChangeKind::Modified
    );
    assert_eq!(
        polled.record.transition.path_deltas[0].changes[0]
            .path
            .display,
        "tracked"
    );
    assert_eq!(
        polled.record.transition.watched_ref_changes[0].ref_name,
        "refs/heads/release"
    );
    assert_eq!(polled.record.proposals.len(), 1);
    assert_eq!(
        polled.record.proposals[0].matched_ref_names,
        vec!["refs/heads/release"]
    );
    assert_eq!(polled.record.proposals[0].matched_path_changes.len(), 1);
    assert_eq!(
        polled.record.proposals[0].matched_path_changes[0]
            .path
            .display,
        "tracked"
    );
    let human = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--trigger",
        "trigger.json",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("Watched ref changes    see below"));
    assert!(human.contains("refs/heads/release ·"));
    assert!(human.contains("fast_forward · complete"));
    assert!(human.contains("Reachability deltas    see below"));
    assert!(human.contains("1 added · 0 removed · complete · limit 256 per direction"));
    assert!(human.contains("commit.reachable_added"));
    assert!(human.contains("Path deltas            see below"));
    assert!(human.contains("1 changes · complete · limit 2048"));
    assert!(human.contains("modified tracked · base64url:dHJhY2tlZA"));
    assert!(human.contains("path.modified"));
    assert!(human.contains("matched refs: refs/heads/release"));
    assert!(
        human.contains(
            "matched path: refs/heads/release · modified · tracked · base64url:dHJhY2tlZA"
        )
    );

    let acknowledged = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "ack",
        polled.record.transition.transition_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(acknowledged.status.success());
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "branch", "future", "HEAD"])
            .status()
            .unwrap()
            .success()
    );
    let created = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--format",
        "json",
    ]);
    assert!(created.status.success());
    let created: GitPollOutcome = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(created.record.transition.watched_ref_changes.len(), 1);
    assert_eq!(
        created.record.transition.watched_ref_changes[0].movement,
        rey_git::GitRefMovement::Created
    );
}

#[test]
fn git_watch_retains_every_bounded_tick_and_stops_at_pending_evidence() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    for args in [
        vec!["add", "tracked"],
        vec!["commit", "-q", "-m", "initial"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }

    let initialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--format",
        "json",
    ]);
    assert!(initialized.status.success());
    let initialized: LocalGitState = serde_json::from_slice(&initialized.stdout).unwrap();
    let initial_cursor = initialized.cursor.clone().unwrap();
    let initial_snapshot = initialized.cursor_snapshot.clone().unwrap();

    let quiet = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "2",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
        "--format",
        "json",
    ]);
    assert!(
        quiet.status.success(),
        "{}",
        String::from_utf8_lossy(&quiet.stderr)
    );
    assert!(quiet.stderr.is_empty());
    let quiet: GitWatchOutcome = serde_json::from_slice(&quiet.stdout).unwrap();
    quiet.verify().unwrap();
    assert_eq!(quiet.stop_reason, GitWatchStopReason::IterationLimit);
    assert_eq!(quiet.ticks.len(), 2);
    assert_eq!(quiet.ticks[0].sequence, 1);
    assert_eq!(quiet.ticks[1].sequence, 2);
    assert!(
        quiet
            .ticks
            .iter()
            .all(|tick| tick.outcome == GitCadenceTickOutcome::Unchanged)
    );
    assert!(quiet.pending_transition_id.is_none());

    let timed = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "2",
        "--interval-ms",
        "10",
        "--max-elapsed-ms",
        "1",
        "--format",
        "json",
    ]);
    assert!(timed.status.success());
    let timed: GitWatchOutcome = serde_json::from_slice(&timed.stdout).unwrap();
    assert_eq!(timed.stop_reason, GitWatchStopReason::TimeLimit);
    assert_eq!(timed.ticks.len(), 1);
    assert_eq!(timed.ticks[0].sequence, 3);

    let human = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "1",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "GIT WATCH",
        "1 iterations · 0 retries · 1 ms cadence · 1000 ms elapsed",
        "#4 · UNCHANGED ·",
        "Stop                   iteration_limit",
        "another bounded watch must be explicit",
    ] {
        assert!(
            human.contains(evidence),
            "missing watch evidence: {evidence}"
        );
    }

    fs::write(workspace.path().join("tracked"), "two\n").unwrap();
    for args in [vec!["add", "tracked"], vec!["commit", "-q", "-m", "second"]] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    let trigger = GitActivationTrigger {
        schema: GIT_ACTIVATION_TRIGGER_SCHEMA.to_owned(),
        trigger_id: "fixture.watch-fast-forward".to_owned(),
        revision: 1,
        repository_id: initial_snapshot.repository_id.clone(),
        worktree_id: initial_snapshot.worktree_id.clone(),
        event_classes: vec![GitActivationEventClass::RefFastForward],
        ref_names: Vec::new(),
        path_prefixes: Vec::new(),
        require_complete: true,
        workload_id: "fixture-workload".to_owned(),
        graph: ContractIdentity::new("fixture.graph", 1, "fixture graph"),
        scenario_ids: vec!["fixture-scenario".to_owned()],
        budget: GitActivationBudget::default(),
    };
    fs::write(
        workspace.path().join("trigger.json"),
        serde_json::to_vec_pretty(&trigger).unwrap(),
    )
    .unwrap();
    let changed = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--trigger",
        "trigger.json",
        "--max-iterations",
        "4",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
        "--format",
        "json",
    ]);
    assert!(
        changed.status.success(),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let changed: GitWatchOutcome = serde_json::from_slice(&changed.stdout).unwrap();
    changed.verify().unwrap();
    assert_eq!(changed.stop_reason, GitWatchStopReason::PendingTransition);
    assert_eq!(changed.ticks.len(), 1);
    assert_eq!(changed.ticks[0].sequence, 5);
    assert_eq!(changed.ticks[0].outcome, GitCadenceTickOutcome::Changed);
    assert_eq!(changed.ticks[0].activation_ids.len(), 1);
    let transition_id = changed.pending_transition_id.clone().unwrap();

    let status = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    assert!(status.status.success());
    let status: GitOperatorStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status.state.cursor, Some(initial_cursor));
    assert_eq!(status.state.cadence_ticks.len(), 5);
    assert_eq!(status.state.watch_receipts.len(), 4);
    assert_eq!(
        status.state.watch_receipts.last().unwrap().watch_id,
        changed.watch_id
    );
    assert_eq!(
        status
            .state
            .pending
            .as_ref()
            .map(|record| &record.transition.transition_id),
        Some(&transition_id)
    );

    let pending = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "1",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
    ]);
    assert_eq!(pending.status.code(), Some(1));
    assert!(pending.stdout.is_empty());
    assert!(String::from_utf8_lossy(&pending.stderr).contains("pending acknowledgement"));

    let acknowledged = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "ack",
        transition_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(acknowledged.status.success());
    let state: LocalGitState =
        serde_json::from_slice(&fs::read(workspace.path().join(".rey/git/state.json")).unwrap())
            .unwrap();
    assert!(state.pending.is_none());
    assert_eq!(state.retained_polls.len(), 1);
    assert_eq!(state.cadence_ticks.len(), 5);
    assert_eq!(state.watch_receipts.len(), 4);

    let state_path = workspace.path().join(".rey/git/state.json");
    let mut tampered: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    tampered["cadence_ticks"][0]["sequence"] = Value::from(99);
    fs::write(&state_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    let rejected = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    assert_eq!(rejected.status.code(), Some(1));
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("semantically tampered"));
}

#[test]
fn git_watch_retains_retry_exhaustion_and_recovered_partial_failure() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    initialize_git_repository(&workspace);
    assert!(
        run_rey_workspace(&[
            "git",
            "--workspace",
            workspace_path,
            "init",
            "--format",
            "json",
        ])
        .status
        .success()
    );
    let git_directory = workspace.path().join(".git");
    let unavailable_directory = workspace.path().join(".git-unavailable");
    fs::rename(&git_directory, &unavailable_directory).unwrap();

    let exhausted = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "4",
        "--max-retries",
        "1",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
        "--format",
        "json",
    ]);
    assert_eq!(exhausted.status.code(), Some(3));
    assert!(exhausted.stderr.is_empty());
    let exhausted: GitWatchOutcome = serde_json::from_slice(&exhausted.stdout).unwrap();
    exhausted.verify().unwrap();
    assert_eq!(exhausted.stop_reason, GitWatchStopReason::RetryLimit);
    assert_eq!(exhausted.ticks.len(), 2);
    assert!(!exhausted.complete);
    assert!(exhausted.ticks.iter().all(|tick| {
        tick.outcome == GitCadenceTickOutcome::Failed
            && tick.observed_snapshot_id.is_none()
            && tick.failure.as_ref().is_some_and(|failure| {
                failure.kind == GitCadenceFailureKind::RepositoryUnavailable && failure.retryable
            })
    }));
    assert_eq!(
        exhausted.omissions,
        vec!["Git cadence observation failed: repository_unavailable"]
    );

    let human = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "watch",
        "--max-iterations",
        "1",
        "--max-retries",
        "0",
        "--interval-ms",
        "1",
        "--max-elapsed-ms",
        "1000",
        "--format",
        "table",
    ]);
    assert_eq!(human.status.code(), Some(3));
    assert!(human.stderr.is_empty());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "1 iterations · 0 retries · 1 ms cadence · 1000 ms elapsed",
        "FAILED · partial",
        "failure: repository_unavailable · retryable",
        "Completeness           partial · see omissions",
        "Stop                   retry_limit",
    ] {
        assert!(
            human.contains(evidence),
            "missing watch evidence: {evidence}"
        );
    }

    let mut recovered = Command::new(env!("CARGO_BIN_EXE_rey"));
    recovered
        .args([
            "git",
            "--workspace",
            workspace_path,
            "watch",
            "--max-iterations",
            "3",
            "--max-retries",
            "2",
            "--interval-ms",
            "100",
            "--max-elapsed-ms",
            "2000",
            "--format",
            "json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = recovered.spawn().unwrap();
    let state_path = workspace.path().join(".rey/git/state.json");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state: LocalGitState = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        if state.cadence_ticks.len() >= 4 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "failed observation was not retained"
        );
        thread::sleep(Duration::from_millis(5));
    }
    fs::rename(&unavailable_directory, &git_directory).unwrap();
    let recovered = child.wait_with_output().unwrap();
    assert_eq!(recovered.status.code(), Some(3));
    assert!(recovered.stderr.is_empty());
    let recovered: GitWatchOutcome = serde_json::from_slice(&recovered.stdout).unwrap();
    recovered.verify().unwrap();
    assert_eq!(recovered.stop_reason, GitWatchStopReason::IterationLimit);
    assert_eq!(recovered.ticks.len(), 3);
    assert_eq!(recovered.ticks[0].outcome, GitCadenceTickOutcome::Failed);
    assert!(
        recovered.ticks[1..]
            .iter()
            .all(|tick| tick.outcome == GitCadenceTickOutcome::Unchanged)
    );
    assert!(!recovered.complete);
    assert!(recovered.pending_transition_id.is_none());

    let state: LocalGitState = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state.verify().unwrap();
    assert_eq!(state.cadence_ticks.len(), 6);
    assert_eq!(state.watch_receipts.len(), 3);
    assert!(state.pending.is_none());
}

#[cfg(unix)]
#[test]
fn git_watch_retains_cooperative_signal_cancellation() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    initialize_git_repository(&workspace);
    assert!(
        run_rey_workspace(&[
            "git",
            "--workspace",
            workspace_path,
            "init",
            "--format",
            "json",
        ])
        .status
        .success()
    );

    let mut command = Command::new(env!("CARGO_BIN_EXE_rey"));
    command
        .args([
            "git",
            "--workspace",
            workspace_path,
            "watch",
            "--max-iterations",
            "100",
            "--max-retries",
            "0",
            "--interval-ms",
            "100",
            "--max-elapsed-ms",
            "10000",
            "--format",
            "json",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().unwrap();
    let state_path = workspace.path().join(".rey/git/state.json");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let state: LocalGitState = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        if !state.cadence_ticks.is_empty() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "first cadence tick was not retained"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let cancelled = child.wait_with_output().unwrap();
    assert_eq!(cancelled.status.code(), Some(130));
    assert!(cancelled.stderr.is_empty());
    let cancelled: GitWatchOutcome = serde_json::from_slice(&cancelled.stdout).unwrap();
    cancelled.verify().unwrap();
    assert_eq!(cancelled.stop_reason, GitWatchStopReason::Cancelled);
    assert!(!cancelled.complete);
    assert!(!cancelled.ticks.is_empty());
    assert!(
        cancelled
            .ticks
            .iter()
            .all(|tick| tick.outcome == GitCadenceTickOutcome::Unchanged)
    );
    assert_eq!(
        cancelled.omissions,
        vec!["Git cadence was cancelled at a retained observation boundary before another tick"]
    );
    let state: LocalGitState = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state.verify().unwrap();
    assert_eq!(state.watch_receipts.len(), 1);
    assert_eq!(state.cadence_ticks.len(), cancelled.ticks.len());
}

#[test]
fn workload_git_dependencies_follow_only_the_acknowledged_cursor() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "initial"])
            .status()
            .unwrap()
            .success()
    );

    let initialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--format",
        "json",
    ]);
    assert!(initialized.status.success());
    let baseline = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let baseline: GitOperatorStatus = serde_json::from_slice(&baseline.stdout).unwrap();
    let cursor = baseline.state.cursor_snapshot.as_ref().unwrap();
    let expected_revision = format!(
        "{}:{}",
        cursor.object_format,
        cursor.head.commit_oid.as_deref().unwrap()
    );
    let package_dir = workspace.path().join("sys/git-dependent-normalization");
    fs::create_dir_all(&package_dir).unwrap();
    let package = include_str!("fixtures/workloads/agent-proposed-normalization.yaml")
        .replace(
            "rey.fixture.agent-proposed-normalization",
            "rey.fixture.git-dependent-normalization",
        )
        .replace(
            "\ngraph:\n",
            &format!(
                "\nownership:\n  git_dependencies:\n    - dependency_id: repository-head\n      repository_id: {}\n      worktree_id: {}\n      kind: head\n      symbolic_ref: {}\n      source_revision: {expected_revision}\n    - dependency_id: semantic-index\n      repository_id: {}\n      worktree_id: {}\n      kind: semantic_index\n      source_revision: {}\n\ngraph:\n",
                cursor.repository_id,
                cursor.worktree_id.as_ref().unwrap(),
                cursor.head.symbolic_ref.as_deref().unwrap(),
                cursor.repository_id,
                cursor.worktree_id.as_ref().unwrap(),
                cursor.index.as_ref().unwrap().entry_digest,
            ),
        );
    fs::write(package_dir.join("workload.yaml"), package).unwrap();

    for args in [
        vec!["workloads", "--workspace", workspace_path, "add"],
        vec![
            "workloads",
            "--workspace",
            workspace_path,
            "test",
            "--staged",
            "rey.fixture.git-dependent-normalization",
        ],
        vec![
            "workloads",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "admit Git-dependent fixture",
        ],
    ] {
        let output = run_rey_workspace(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let admitted = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let admitted: WorkloadList = serde_json::from_slice(&admitted.stdout).unwrap();
    assert!(admitted.attention.rows.is_empty());
    assert_eq!(admitted.workloads[0].git_dependencies.len(), 2);
    assert_eq!(
        admitted.workloads[0].git_dependencies[0].source_revision,
        expected_revision
    );

    fs::write(workspace.path().join("tracked"), "two\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "second"])
            .status()
            .unwrap()
            .success()
    );

    let ambient = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let ambient: WorkloadList = serde_json::from_slice(&ambient.stdout).unwrap();
    assert!(
        ambient.attention.rows.is_empty(),
        "ambient repository state must not invalidate admitted workload evidence"
    );

    let polled = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--format",
        "json",
    ]);
    assert!(polled.status.success());
    let polled: GitPollOutcome = serde_json::from_slice(&polled.stdout).unwrap();
    let target = &polled.record.target_snapshot;
    let actual_revision = format!(
        "{}:{}",
        target.object_format,
        target.head.commit_oid.as_deref().unwrap()
    );
    let actual_index_revision = target.index.as_ref().unwrap().entry_digest.to_string();
    let transition_id = polled.record.transition.transition_id.to_string();
    let acknowledged = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "ack",
        &transition_id,
        "--format",
        "json",
    ]);
    assert!(acknowledged.status.success());

    let changed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let changed: WorkloadList = serde_json::from_slice(&changed.stdout).unwrap();
    let row = changed
        .attention
        .rows
        .iter()
        .find(|row| row.subject_id == "rey.fixture.git-dependent-normalization")
        .unwrap();
    assert_eq!(row.action, AttentionAction::Retest);
    assert_eq!(row.reason, AttentionReason::DependencyChanged);
    assert_eq!(
        row.dependency_ids,
        [
            format!("git:repository-head@{actual_revision}"),
            format!("git:semantic-index@{actual_index_revision}"),
        ]
    );
    assert!(row.evidence_ids.contains(&target.snapshot_id));

    let human = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "Git dependency",
        "repository-head · head",
        "semantic-index · semantic_index",
        "dependency_changed",
        &format!("git:repository-head@{actual_revision}"),
        target.snapshot_id.as_str(),
    ] {
        assert!(
            human.contains(evidence),
            "missing Git dependency evidence: {evidence}"
        );
    }
}

#[test]
fn acknowledged_git_activation_requires_exact_workload_runtime_admission() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "initial"])
            .status()
            .unwrap()
            .success()
    );

    for args in [
        vec!["env", "--workspace", workspace_path, "add"],
        vec![
            "env",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "retain activation capabilities",
        ],
    ] {
        let output = run_rey_workspace(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let package_dir = workspace.path().join("sys/activation-normalization");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("workload.yaml"),
        include_str!("fixtures/workloads/agent-proposed-normalization.yaml").replace(
            "rey.fixture.agent-proposed-normalization",
            "rey.fixture.activation-normalization",
        ),
    )
    .unwrap();
    for args in [
        vec!["workloads", "--workspace", workspace_path, "add"],
        vec![
            "workloads",
            "--workspace",
            workspace_path,
            "test",
            "--staged",
            "rey.fixture.activation-normalization",
        ],
        vec![
            "workloads",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "admit activation fixture",
        ],
    ] {
        let output = run_rey_workspace(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let before = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let before: WorkloadList = serde_json::from_slice(&before.stdout).unwrap();
    let workload = &before.workloads[0];
    let original_test_id = workload.last_test_result_id.clone().unwrap();

    let initialized = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "init",
        "--format",
        "json",
    ]);
    assert!(initialized.status.success());
    fs::write(workspace.path().join("tracked"), "two\n").unwrap();
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "add", "tracked"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("git")
            .args(["-C", workspace_path, "commit", "-q", "-m", "second"])
            .status()
            .unwrap()
            .success()
    );
    let observed = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "json",
    ]);
    let observed: GitOperatorStatus = serde_json::from_slice(&observed.stdout).unwrap();
    let snapshot = observed.observed_snapshot;
    let good_trigger = GitActivationTrigger {
        schema: GIT_ACTIVATION_TRIGGER_SCHEMA.to_owned(),
        trigger_id: "fixture.admit.good".to_owned(),
        revision: 1,
        repository_id: snapshot.repository_id.clone(),
        worktree_id: snapshot.worktree_id.clone(),
        event_classes: vec![GitActivationEventClass::RefFastForward],
        ref_names: Vec::new(),
        path_prefixes: Vec::new(),
        require_complete: true,
        workload_id: workload.workload.id.clone(),
        graph: workload.candidate_graph.clone(),
        scenario_ids: vec!["rey.fixture.activation-normalization.scenario.plain".to_owned()],
        budget: GitActivationBudget::default(),
    };
    let mut bad_trigger = good_trigger.clone();
    bad_trigger.trigger_id = "fixture.admit.bad-graph".to_owned();
    bad_trigger.graph = ContractIdentity::new("fixture.wrong.graph", 1, "wrong graph");
    let mut low_budget_trigger = good_trigger.clone();
    low_budget_trigger.trigger_id = "fixture.admit.low-budget".to_owned();
    low_budget_trigger.budget.max_evidence_bytes = 1;
    let mut duplicate_trigger = good_trigger.clone();
    duplicate_trigger.trigger_id = "fixture.admit.duplicate".to_owned();
    fs::write(
        workspace.path().join("good-trigger.json"),
        serde_json::to_vec_pretty(&good_trigger).unwrap(),
    )
    .unwrap();
    fs::write(
        workspace.path().join("bad-trigger.json"),
        serde_json::to_vec_pretty(&bad_trigger).unwrap(),
    )
    .unwrap();
    fs::write(
        workspace.path().join("low-budget-trigger.json"),
        serde_json::to_vec_pretty(&low_budget_trigger).unwrap(),
    )
    .unwrap();
    fs::write(
        workspace.path().join("duplicate-trigger.json"),
        serde_json::to_vec_pretty(&duplicate_trigger).unwrap(),
    )
    .unwrap();

    let polled = run_rey_workspace(&[
        "git",
        "--workspace",
        workspace_path,
        "poll",
        "--trigger",
        "good-trigger.json",
        "--trigger",
        "bad-trigger.json",
        "--trigger",
        "low-budget-trigger.json",
        "--trigger",
        "duplicate-trigger.json",
        "--format",
        "json",
    ]);
    assert!(polled.status.success());
    let polled: GitPollOutcome = serde_json::from_slice(&polled.stdout).unwrap();
    let good = polled
        .record
        .proposals
        .iter()
        .find(|proposal| proposal.trigger_id == "fixture.admit.good")
        .unwrap();
    let bad = polled
        .record
        .proposals
        .iter()
        .find(|proposal| proposal.trigger_id == "fixture.admit.bad-graph")
        .unwrap();
    let low_budget = polled
        .record
        .proposals
        .iter()
        .find(|proposal| proposal.trigger_id == "fixture.admit.low-budget")
        .unwrap();
    let duplicate = polled
        .record
        .proposals
        .iter()
        .find(|proposal| proposal.trigger_id == "fixture.admit.duplicate")
        .unwrap();

    let pending = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        good.activation_id.as_str(),
    ]);
    assert_eq!(pending.status.code(), Some(1));
    assert!(pending.stdout.is_empty());
    assert!(String::from_utf8_lossy(&pending.stderr).contains("must be acknowledged"));

    let transition_id = polled.record.transition.transition_id.to_string();
    let acknowledged =
        run_rey_workspace(&["git", "--workspace", workspace_path, "ack", &transition_id]);
    assert!(acknowledged.status.success());

    let wrong_graph = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        bad.activation_id.as_str(),
    ]);
    assert_eq!(wrong_graph.status.code(), Some(1));
    assert!(wrong_graph.stdout.is_empty());
    assert!(String::from_utf8_lossy(&wrong_graph.stderr).contains("exact admitted workload HEAD"));

    let admitted = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        good.activation_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(
        admitted.status.success(),
        "{}",
        String::from_utf8_lossy(&admitted.stderr)
    );
    let admitted: WorkloadActivationAdmission = serde_json::from_slice(&admitted.stdout).unwrap();
    assert_eq!(admitted.activation, *good);
    assert_eq!(admitted.declared_scenarios.len(), 2);
    assert_eq!(
        admitted.selected_scenario_ids,
        ["rey.fixture.activation-normalization.scenario.plain"]
    );
    assert_eq!(admitted.effective_budget.max_actions, 1);
    assert_eq!(
        admitted.authority,
        "admitted_for_runtime_scheduling; no workload or Git execution has occurred"
    );

    let replay = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        good.activation_id.as_str(),
        "--format",
        "json",
    ]);
    let replay: WorkloadActivationAdmission = serde_json::from_slice(&replay.stdout).unwrap();
    assert_eq!(replay, admitted);

    let listed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(
        listed.activation_admissions.as_slice(),
        std::slice::from_ref(&admitted)
    );
    assert_eq!(
        listed.workloads[0].last_test_result_id,
        Some(original_test_id.clone())
    );

    let human = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "Runtime admissions",
        "1 Git activation · 0 executed",
        "RUNTIME ADMISSIONS",
        "ADMITTED",
        "revalidate before execution",
        admitted.admission_id.as_str(),
        good.activation_id.as_str(),
    ] {
        assert!(
            human.contains(evidence),
            "missing runtime admission evidence: {evidence}"
        );
    }

    let low_budget_admission = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        low_budget.activation_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(low_budget_admission.status.success());
    let low_budget_admission: WorkloadActivationAdmission =
        serde_json::from_slice(&low_budget_admission.stdout).unwrap();
    assert_eq!(low_budget_admission.effective_budget.max_evidence_bytes, 1);
    let over_budget = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "execute-activation",
        low_budget_admission.admission_id.as_str(),
    ]);
    assert_eq!(over_budget.status.code(), Some(1));
    assert!(over_budget.stdout.is_empty());
    assert!(String::from_utf8_lossy(&over_budget.stderr).contains("over budget"));

    let executed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "execute-activation",
        admitted.admission_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
    let executed: WorkloadActivationExecution = serde_json::from_slice(&executed.stdout).unwrap();
    assert_eq!(executed.admission_id, admitted.admission_id);
    assert_eq!(executed.activation_id, good.activation_id);
    assert_eq!(
        executed.result.selected_scenario_ids,
        ["rey.fixture.activation-normalization.scenario.plain"]
    );
    assert_eq!(executed.result.scenarios.len(), 1);
    assert_eq!(executed.result.status, TestStatus::Passed);
    assert!(executed.evidence_bytes <= admitted.effective_budget.max_evidence_bytes);
    assert_eq!(
        executed.result.authority,
        "scenario_evidence_only; this result does not qualify the workload"
    );
    assert_eq!(
        executed.authority,
        "activation_scenarios_evaluated; no Git mutation occurred"
    );

    let over_budget_with_source = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "execute-activation",
        low_budget_admission.admission_id.as_str(),
    ]);
    assert_eq!(over_budget_with_source.status.code(), Some(1));
    assert!(over_budget_with_source.stdout.is_empty());
    assert!(String::from_utf8_lossy(&over_budget_with_source.stderr).contains("over budget"));

    let execution_replay = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "execute-activation",
        admitted.admission_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(execution_replay.status.success());
    let execution_replay = String::from_utf8(execution_replay.stdout).unwrap();
    for evidence in [
        "WORKLOAD ACTIVATION EXECUTION",
        "retained result replayed · graph was not executed again",
        executed.execution_id.as_str(),
        "selected scenario evidence cannot qualify the workload",
    ] {
        assert!(
            execution_replay.contains(evidence),
            "missing activation execution evidence: {evidence}"
        );
    }

    let recomputation_over_budget = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "verify-activation",
        executed.execution_id.as_str(),
        "--max-evidence-bytes",
        "1",
    ]);
    assert_eq!(recomputation_over_budget.status.code(), Some(1));
    assert!(recomputation_over_budget.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&recomputation_over_budget.stderr)
            .contains("full recomputation evidence uses")
    );

    let recomputed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "verify-activation",
        executed.execution_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(
        recomputed.status.success(),
        "{}",
        String::from_utf8_lossy(&recomputed.stderr)
    );
    let recomputed: WorkloadActivationRecomputation =
        serde_json::from_slice(&recomputed.stdout).unwrap();
    assert_eq!(
        recomputed.assessment,
        WorkloadRecomputationAssessment::Equivalent
    );
    assert_eq!(recomputed.execution_id, executed.execution_id);
    assert_eq!(recomputed.admission_id, admitted.admission_id);
    assert_eq!(recomputed.selected_result_id, executed.result.result_id);
    assert_eq!(recomputed.comparisons.len(), 1);
    assert!(recomputed.comparisons[0].equivalent);
    assert_eq!(recomputed.full_result.selected_scenario_ids.len(), 2);
    assert_eq!(recomputed.full_result.scenarios.len(), 2);
    assert_eq!(
        recomputed.full_result.capability_snapshot_id,
        admitted.capability_snapshot_id
    );
    assert_eq!(
        recomputed.authority,
        "comparison_evidence_only; recomputation does not qualify the workload or execute Git"
    );

    let recomputation_replay = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "verify-activation",
        executed.execution_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(recomputation_replay.status.success());
    let recomputation_replay = String::from_utf8(recomputation_replay.stdout).unwrap();
    for evidence in [
        "WORKLOAD ACTIVATION FULL RECOMPUTATION",
        "EQUIVALENT",
        "retained proof replayed · scenarios were not executed again",
        recomputed.recomputation_id.as_str(),
        "1 selected compared · 2 fully recomputed",
        "qualification unchanged",
    ] {
        assert!(
            recomputation_replay.contains(evidence),
            "missing activation recomputation evidence: {evidence}"
        );
    }

    let duplicate_admission = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "admit-activation",
        duplicate.activation_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(duplicate_admission.status.success());
    let duplicate_admission: WorkloadActivationAdmission =
        serde_json::from_slice(&duplicate_admission.stdout).unwrap();
    let coalesced = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "execute-activation",
        duplicate_admission.admission_id.as_str(),
        "--format",
        "json",
    ]);
    assert!(coalesced.status.success());
    let coalesced: WorkloadActivationExecution = serde_json::from_slice(&coalesced.stdout).unwrap();
    assert_eq!(
        coalesced.source_execution_id.as_ref(),
        Some(&executed.execution_id)
    );
    assert_eq!(coalesced.result, executed.result);
    assert_eq!(coalesced.evidence_bytes, executed.evidence_bytes);
    assert_eq!(
        coalesced.authority,
        "activation_coalesced_with_retained_execution; graph was not executed again"
    );

    let listed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "json",
    ]);
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.activation_executions.len(), 2);
    assert!(listed.activation_executions.contains(&executed));
    assert!(listed.activation_executions.contains(&coalesced));
    assert_eq!(
        listed.activation_recomputations.as_slice(),
        std::slice::from_ref(&recomputed)
    );
    assert_eq!(
        listed.workloads[0].last_test_result_id,
        Some(original_test_id)
    );

    let human = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "3 Git activations · 2 executed · 1 full recomputation proof",
        "EXECUTED",
        "COALESCED",
        executed.execution_id.as_str(),
        coalesced.execution_id.as_str(),
        "graph not rerun",
        "FULL EQUIVALENT",
        recomputed.recomputation_id.as_str(),
        "qualification unchanged",
    ] {
        assert!(
            human.contains(evidence),
            "missing retained activation execution: {evidence}"
        );
    }

    let state_path = workspace.path().join(".rey/workloads/state.json");
    let original_state = fs::read(&state_path).unwrap();
    let mut state: Value = serde_json::from_slice(&original_state).unwrap();
    state["activation_admissions"][0]["authority"] = "tampered".into();
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    let tampered = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert_eq!(tampered.status.code(), Some(1));
    assert!(tampered.stdout.is_empty());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("tampered"));

    fs::write(&state_path, &original_state).unwrap();
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["activation_executions"][0]["evidence_bytes"] = 1.into();
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    let tampered = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert_eq!(tampered.status.code(), Some(1));
    assert!(tampered.stdout.is_empty());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("tampered"));

    fs::write(&state_path, &original_state).unwrap();
    let mut state: Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["activation_recomputations"][0]["authority"] = "tampered".into();
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    let tampered = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert_eq!(tampered.status.code(), Some(1));
    assert!(tampered.stdout.is_empty());
    assert!(String::from_utf8_lossy(&tampered.stderr).contains("tampered"));
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
        "Evidence               DIFFERENT · 16 authoritative capability changes",
        "01 / DIRECTED TEXT",
        "Environment variables · 3 tracked · 1 changed",
        "02 / BOUNDED SEARCH",
        "APPLICATIONS · 15 searched",
        "15 changed",
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
    assert!(added.contains("16 capability changes admitted"));
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
    assert_eq!(staged_diff.delta.summary.modified, 16);

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
        "Evidence               ENV@1 → ENV@2 · DIFFERENT · 16 authoritative capability changes",
        "Environment            3 variables · 15 applications · 0 inputs · 0 references · complete",
        "Changes                1 variable · 15 applications · 0 inputs · 0 references",
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
    assert!(!status.working_snapshot.capabilities.is_empty());
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
        "APPLICATIONS · 17 searched",
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
        "Environment            5 variables · 17 applications · 1 input · 2 references · complete",
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
        "APPLICATIONS · 17 searched",
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
    let package_dir = workspace.path().join("sys/agent-proposed-normalization");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("workload.yaml"),
        include_str!("fixtures/workloads/agent-proposed-normalization.yaml"),
    )
    .unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    assert!(listed.stderr.is_empty());
    assert!(!workspace.path().join(".rey").exists());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.catalog.kind, WorkloadCatalogKind::WorkspacePackages);
    assert!(listed.workloads.is_empty());
    let revision = listed.revision.as_ref().unwrap();
    assert_eq!(revision.working.packages.len(), 1);
    assert_eq!(
        revision.working.packages[0].workload_id,
        "rey.fixture.agent-proposed-normalization"
    );
    assert_eq!(revision.unstaged.inserted, 1);
    assert!(!revision.commit_ready);

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

    let staged = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "table",
    ]);
    assert!(staged.status.success());
    let staged = String::from_utf8(staged.stdout).unwrap();
    assert!(staged.contains("WORKLOAD INDEX"));
    assert!(staged.contains("not admitted · not runnable"));

    let staged_diff = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "diff",
        "--staged",
    ]);
    assert!(staged_diff.status.success());
    let staged_diff: WorkloadChangeSet = serde_json::from_slice(&staged_diff.stdout).unwrap();
    assert_eq!(staged_diff.inserted, 1);
    assert_eq!(staged_diff.source_label, "HEAD");
    assert_eq!(staged_diff.target_label, "INDEX");

    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "--staged",
        "rey.fixture.agent-proposed-normalization",
    ]);
    assert!(tested.status.success());
    let tested: WorkloadTestBatch = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(tested.catalog.kind, WorkloadCatalogKind::WorkspacePackages);
    assert_eq!(tested.results[0].status, TestStatus::Passed);
    assert_eq!(tested.workloads[0].origin, WorkloadOrigin::WorkspacePackage);

    let status = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    assert!(status.status.success());
    let status: WorkloadRevisionStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert!(status.commit_ready);
    assert!(status.qualification_omissions.is_empty());

    let committed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "Approve agent normalization fixture",
    ]);
    assert!(committed.status.success());

    let log = run_rey_workspace(&["workloads", "--workspace", workspace_path, "log", "--patch"]);
    assert!(log.status.success());
    let log: WorkloadLog = serde_json::from_slice(&log.stdout).unwrap();
    assert_eq!(log.total_commits, 1);
    assert_eq!(log.commits[0].sequence, 1);
    assert_eq!(
        log.commits[0].message,
        "Approve agent normalization fixture"
    );

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.workloads.len(), 1);
    let provenance = listed.workloads[0].provenance.as_ref().unwrap();
    assert_eq!(provenance.origin, WorkloadOrigin::WorkspacePackage);
    assert_eq!(
        provenance.generation.as_ref().map(|value| value.kind),
        Some(WorkloadProposalKind::CodingHarness)
    );
    assert_eq!(
        listed.workloads[0].qualification,
        QualificationState::Qualified
    );

    let run = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "rey.fixture.agent-proposed-normalization",
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

    let revised = include_str!("fixtures/workloads/agent-proposed-normalization.yaml").replace(
        "producer_revision: fixture-v1",
        "producer_revision: fixture-v2",
    );
    fs::write(package_dir.join("workload.yaml"), revised).unwrap();
    let status = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    assert!(status.status.success());
    let status: WorkloadRevisionStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status.unstaged.modified, 1);

    let restaged = run_rey_workspace(&["workloads", "--workspace", workspace_path, "add"]);
    assert!(restaged.status.success());
    let status = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    let status: WorkloadRevisionStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert!(!status.commit_ready);
    assert!(
        status
            .qualification_omissions
            .iter()
            .any(|omission| omission.contains("exact INDEX snapshot"))
    );
}

#[test]
fn reyignore_filters_typed_workload_and_environment_working_objects() {
    let workspace = TempDir::new().unwrap();
    let package_dir = workspace.path().join("sys/context-anchor-survey");
    fs::create_dir_all(&package_dir).unwrap();
    fs::write(
        package_dir.join("workload.yaml"),
        include_str!("../../../sys/context-anchor-survey/workload.yaml"),
    )
    .unwrap();
    fs::write(
        workspace.path().join(".reyignore"),
        "# local operator scope\nworkload: context-anchor-survey\nenvironment variable:*\n",
    )
    .unwrap();
    let workspace_path = workspace.path().to_str().unwrap();

    let workload = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(workload.status.success());
    let workload = String::from_utf8(workload.stdout).unwrap();
    assert!(workload.contains("Ignore file    .reyignore · 1 rules · 1 working objects omitted"));
    assert!(
        workload.contains("ignored:      workload: context-anchor-survey · 1 matches · line 2")
    );
    assert!(!workload.contains("new:      workload: context-anchor-survey"));

    let workload = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    let workload: WorkloadRevisionStatus = serde_json::from_slice(&workload.stdout).unwrap();
    assert!(workload.working.packages.is_empty());
    assert_eq!(workload.working.ignore.as_ref().unwrap().ignored, 1);
    assert_ne!(
        workload.working.snapshot_revision,
        rey_core::SemanticHasher::new("unrelated-empty").finish()
    );

    let environment = run_rey_workspace(&[
        "env",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(environment.status.success());
    let environment = String::from_utf8(environment.stdout).unwrap();
    assert!(
        environment.contains("Ignore file    .reyignore · 1 rules · 3 working objects omitted")
    );
    assert!(environment.contains("ignored:      environment variable: * · 3 matches · line 3"));
    assert!(!environment.contains("environment variable: PATH"));
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
        "Admission              AWAITING HARNESS",
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

    let request_path = workspace.path().join("sys/api-drift/request.yaml");
    let request_before = fs::read(&request_path).unwrap();
    assert!(
        !workspace
            .path()
            .join("sys/api-drift/workload.yaml")
            .exists()
    );
    let request: Value = serde_json::from_slice(&request_before).unwrap();
    assert_eq!(request["schema"], "rey.workload-creation-request.v1");
    assert_eq!(request["proposer"], "coding_harness");
    assert_eq!(request["target_package"], "sys/api-drift/workload.yaml");

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed.catalog.workload_count, 1);
    assert_eq!(listed.catalog.admitted_count, 0);
    assert_eq!(listed.catalog.draft_count, 1);
    assert!(listed.workloads.is_empty());
    assert_eq!(listed.drafts[0].request.workload_id, "api-drift");

    let status = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    assert!(status.status.success());
    let status: WorkloadRevisionStatus = serde_json::from_slice(&status.stdout).unwrap();
    assert!(status.working.packages.is_empty());
    assert_eq!(
        status.drafts[0].request.intent.as_deref(),
        Some("Mine API drift and formalize authoritative scenarios")
    );

    let unqualified = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "api-drift",
    ]);
    assert_eq!(unqualified.status.code(), Some(1));
    assert!(unqualified.stdout.is_empty());
    assert!(String::from_utf8_lossy(&unqualified.stderr).contains("requires --staged"));

    let unadmitted = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "api-drift",
    ]);
    assert_eq!(unadmitted.status.code(), Some(1));
    assert!(unadmitted.stdout.is_empty());
    assert!(String::from_utf8_lossy(&unadmitted.stderr).contains("unknown workload api-drift"));

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
    assert_eq!(machine.created_files, ["sys/schema-mining/request.yaml"]);

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
fn selected_create_attention_binds_the_harness_response_through_human_admission() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    fs::write(workspace.path().join("input.txt"), "unowned surface\n").unwrap();
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
    assert!(
        run_rey_workspace(&[
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
        run_rey_workspace(&[
            "env",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "admit input surface",
        ])
        .status
        .success()
    );

    let listed = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(listed.status.success());
    let listed: WorkloadList = serde_json::from_slice(&listed.stdout).unwrap();
    let create = listed
        .attention
        .rows
        .iter()
        .find(|row| row.action == AttentionAction::Create)
        .unwrap();
    let runtime = listed.runtime.as_ref().unwrap();
    assert_eq!(runtime.scheduling.selected.len(), 1);
    assert!(runtime.frontier.rows.iter().any(|row| {
        row.row_id == runtime.scheduling.selected[0].frontier_row_id
            && row
                .claim_ids
                .contains(&format!("rey.workload-attention-row:{}", create.row_id))
    }));

    let stale = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "create",
        "stale-request",
        "--attention-row",
        "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    ]);
    assert_eq!(stale.status.code(), Some(1));
    assert!(stale.stdout.is_empty());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("unknown portfolio-attention row"));
    assert!(!workspace.path().join("sys/stale-request").exists());

    let created = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "create",
        "input-surface",
        "--attention-row",
        create.row_id.as_str(),
        "--format",
        "table",
    ]);
    assert!(created.status.success());
    assert!(created.stderr.is_empty());
    let created = String::from_utf8(created.stdout).unwrap();
    for needle in [
        "Attention row          blake3:",
        "unowned_surface · input.txt",
        "Portfolio              blake3:",
        "Environment            blake3:",
        "Frontier               blake3:",
        "Frontier row           blake3:",
        "Scheduling             blake3:",
        "Reasoning surface      blake3:",
        "Permitted action       rey.action.create-workload",
        "Current package        ABSENT · CREATE",
        "Failing delta refs     0 · typed empty",
        "Surface bounds",
        "immutable request preconditions",
        "Admission              AWAITING HARNESS",
    ] {
        assert!(
            created.contains(needle),
            "missing request binding: {needle}"
        );
    }

    let draft_list = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(draft_list.status.success());
    let draft_list: WorkloadList = serde_json::from_slice(&draft_list.stdout).unwrap();
    let draft = &draft_list.drafts[0];
    let binding = draft.request.attention.as_ref().unwrap();
    assert_eq!(binding.attention_row_id, create.row_id);
    assert_eq!(binding.frontier_id, runtime.frontier.frontier_id);
    assert_eq!(
        binding.scheduling_decision_id,
        runtime.scheduling.decision_id
    );
    assert_eq!(
        binding.reasoning_surface_id,
        runtime.surface.as_ref().unwrap().surface_id
    );
    let awaiting = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(awaiting.status.success());
    assert!(
        String::from_utf8_lossy(&awaiting.stdout)
            .contains("Admission state        AWAITING HARNESS")
    );

    let package_path = workspace.path().join("sys/input-surface/workload.yaml");
    let fixture = include_str!("fixtures/workloads/agent-proposed-normalization.yaml")
        .replace("rey.fixture.agent-proposed-normalization", "input-surface");
    fs::write(&package_path, &fixture).unwrap();
    let unbound = run_rey_workspace(&["workloads", "--workspace", workspace_path, "status"]);
    assert_eq!(unbound.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&unbound.stderr)
            .contains("does not cite the exact retained harness request bytes")
    );

    let response = fixture
        .replace("source: AGENTS.md", &format!("source: {}", draft.source))
        .replace(
            "revision: fixture:agents-v1",
            &format!("revision: {}", draft.source_digest),
        );
    fs::write(&package_path, response).unwrap();

    let working = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(working.status.success());
    let working = String::from_utf8(working.stdout).unwrap();
    assert!(working.contains("Admission state        WORKING"));
    assert!(working.contains("Changes not staged for workload admission"));
    assert!(working.contains("new:       workload: input-surface"));

    let added = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "table",
    ]);
    assert!(added.status.success());
    assert!(String::from_utf8_lossy(&added.stdout).contains("WORKLOAD INDEX"));

    let unqualified = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(unqualified.status.success());
    let unqualified = String::from_utf8_lossy(&unqualified.stdout);
    assert!(unqualified.contains("Admission state        INDEX UNQUALIFIED"));
    assert!(unqualified.contains("INDEX is not ready"));

    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "--staged",
        "input-surface",
        "--format",
        "table",
        "-vv",
    ]);
    assert!(tested.status.success());
    assert!(tested.stderr.is_empty());
    assert!(String::from_utf8_lossy(&tested.stdout).contains("Result      QUALIFIED"));

    let qualified = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(qualified.status.success());
    let qualified = String::from_utf8_lossy(&qualified.stdout);
    assert!(qualified.contains("Admission state        INDEX QUALIFIED"));
    assert!(qualified.contains("INDEX is qualified and awaiting human approval"));

    let committed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "admit input surface workload",
        "--format",
        "table",
    ]);
    assert!(committed.status.success());
    let committed = String::from_utf8(committed.stdout).unwrap();
    assert!(committed.contains("WORKLOAD@1"));

    let head = run_rey_workspace(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(head.status.success());
    let head: WorkloadList = serde_json::from_slice(&head.stdout).unwrap();
    assert_eq!(head.workloads.len(), 1);
    assert_eq!(head.workloads[0].workload.id, "input-surface");
    assert_eq!(head.catalog.admitted_count, 1);
    assert!(head.attention.rows.iter().any(|row| {
        row.action == AttentionAction::Create
            && row.subject_id == "input.txt"
            && row.readiness == rey_runtime::AttentionReadiness::Ready
    }));

    let admitted = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "status",
        "--format",
        "table",
    ]);
    assert!(admitted.status.success());
    assert!(String::from_utf8_lossy(&admitted.stdout).contains("Admission state        HEAD"));
}

#[test]
fn journal_cli_admits_agent_entries_without_executing_typed_blocks() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    let proposal = workspace.path().join("entry.yaml");
    fs::write(
        &proposal,
        r#"schema: rey.journal-entry-proposal.v2
title: Inspect source coverage
author:
  kind: agent
  id: codex
binding:
  coordinate: rey+local://workload/source-mining?revision=blake3%3Aabc
  scale: 1.46
  source_revision: blake3:abc
layout:
  kind: broadsheet
  columns: 12
  bands:
    - id: evidence
      cells:
        - block_id: context
          span: 8
        - block_id: coverage-query
          span: 4
    - id: bearing
      cells:
        - block_id: refine-coverage
          span: 12
blocks:
  - kind: prose
    id: context
    document:
      - kind: paragraph
        text: Coverage moved after the latest survey.
  - kind: query
    id: coverage-query
    language: rey
    provider: rey.observations
    mode: read_only
    statement: frontier
    parameters:
      limit: "2"
  - kind: action
    id: refine-coverage
    operation: refine
    desired_delta: Reduce uncovered source surfaces to zero.
    evidence_ids:
      - blake3:coverage
    dependency_ids: []
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
    assert_eq!(admitted["schema"], "rey.journal-admission.v2");
    assert_eq!(admitted["admitted"], true);
    assert_eq!(admitted["entry"]["sequence"], 1);
    assert_eq!(admitted["entry"]["author"]["kind"], "agent");
    assert_eq!(admitted["entry"]["blocks"][1]["kind"], "query");
    assert_eq!(admitted["entry"]["blocks"][1]["mode"], "read_only");
    assert_eq!(admitted["entry"]["blocks"][2]["kind"], "action");

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
    assert!(repeated.contains("Broadsheet"));
    assert!(repeated.contains("12 columns · 2 bands"));
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
    assert_eq!(listed["schema"], "rey.journal-log.v2");
    assert_eq!(listed["entries"].as_array().unwrap().len(), 1);
    assert!(workspace.path().join(".rey/journal/journal.json").is_file());

    let listed_table = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "list",
        "--format",
        "table",
    ]);
    assert!(listed_table.status.success());
    let listed_table = String::from_utf8(listed_table.stdout).unwrap();
    assert!(listed_table.contains("3 cells / 2 bands"));
    assert!(listed_table.contains("[evidence] context:prose 8/12 | coverage-query:query 4/12"));
    assert!(listed_table.contains("[bearing] refine-coverage:action 12/12"));

    let opportunities = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "opportunities",
        "--format",
        "json",
    ]);
    assert!(opportunities.status.success());
    let opportunities: rey::journal_opportunities::JournalOpportunitySurface =
        serde_json::from_slice(&opportunities.stdout).unwrap();
    assert_eq!(opportunities.summary.current_entries, 1);
    assert_eq!(opportunities.summary.authored_actions, 1);
    assert_eq!(opportunities.rows[0].block_id, "refine-coverage");
    assert_eq!(opportunities.rows[0].readiness, "authored_only");
    assert_eq!(opportunities.rows[0].authority, "none");

    let opportunities_table = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "opportunities",
        "--format",
        "table",
    ]);
    assert!(opportunities_table.status.success());
    let opportunities_table = String::from_utf8(opportunities_table.stdout).unwrap();
    assert!(opportunities_table.contains("JOURNAL OPPORTUNITIES · AUTHORED ONLY"));
    assert!(opportunities_table.contains("Reduce uncovered source surfaces to zero."));
    assert!(opportunities_table.contains("no assignment or execution"));
    assert!(opportunities_table.contains("requires_verified_selected_ready_create_attention_row"));

    fs::write(
        workspace.path().join("observation.yaml"),
        r#"schema: rey.observation.v1
kind: finding
author:
  kind: agent
  id: codex
subject_locator: rey+local://workload/source-mining?revision=blake3%3Aabc
body: One source coverage gap remains open.
desired_delta: Reduce uncovered source surfaces to zero.
completeness: complete
omissions: []
evidence: []
supersedes: null
"#,
    )
    .unwrap();
    let observation = run_rey_workspace(&[
        "observations",
        "--workspace",
        workspace_path,
        "add",
        "observation.yaml",
        "--no-broadcast",
        "--format",
        "json",
    ]);
    assert!(observation.status.success());

    let entry_id = admitted["entry"]["entry_id"].as_str().unwrap();
    let query_admission = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "query",
        "admit",
        entry_id,
        "coverage-query",
        "--format",
        "json",
    ]);
    assert!(
        query_admission.status.success(),
        "{}",
        String::from_utf8_lossy(&query_admission.stderr)
    );
    let query_admission: rey::journal_queries::JournalQueryAdmissionResult =
        serde_json::from_slice(&query_admission.stdout).unwrap();
    assert!(query_admission.admitted);
    assert_eq!(query_admission.admission.limits.max_rows, 2);
    assert!(
        workspace
            .path()
            .join(".rey/journal/journal-queries.json")
            .is_file()
    );
    let unchanged: Value = serde_json::from_slice(
        &run_rey_workspace(&[
            "journal",
            "--workspace",
            workspace_path,
            "list",
            "--format",
            "json",
        ])
        .stdout,
    )
    .unwrap();
    assert_eq!(unchanged["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        unchanged["entries"][0]["blocks"].as_array().unwrap().len(),
        3
    );

    let query_execution = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "query",
        "execute",
        query_admission.admission.admission_id.as_str(),
        "--author",
        "codex",
        "--proposal-out",
        "query-result.json",
        "--format",
        "json",
    ]);
    assert!(
        query_execution.status.success(),
        "{}",
        String::from_utf8_lossy(&query_execution.stderr)
    );
    let query_execution: Value = serde_json::from_slice(&query_execution.stdout).unwrap();
    assert_eq!(
        query_execution["schema"],
        "rey.journal-query-execution-output.v1"
    );
    assert_eq!(query_execution["result"]["executed"], true);
    assert_eq!(
        query_execution["result"]["execution"]["delta"]["inserted_rows"],
        1
    );
    assert_eq!(query_execution["proposal_path"], "query-result.json");
    assert!(workspace.path().join("query-result.json").is_file());
    let unchanged_after_execution: Value = serde_json::from_slice(
        &run_rey_workspace(&[
            "journal",
            "--workspace",
            workspace_path,
            "list",
            "--format",
            "json",
        ])
        .stdout,
    )
    .unwrap();
    assert_eq!(
        unchanged_after_execution["entries"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let query_list = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "query",
        "list",
        "--format",
        "table",
    ]);
    assert!(query_list.status.success());
    let query_list = String::from_utf8(query_list.stdout).unwrap();
    assert!(query_list.contains("JOURNAL QUERIES"));
    assert!(query_list.contains("1 admissions · 1 executions"));
    assert!(query_list.contains("EXECUTED"));

    let superseding = run_rey_workspace(&[
        "journal",
        "--workspace",
        workspace_path,
        "add",
        "query-result.json",
        "--format",
        "json",
    ]);
    assert!(superseding.status.success());
    let superseding: Value = serde_json::from_slice(&superseding.stdout).unwrap();
    assert_eq!(superseding["entry"]["sequence"], 2);
    assert_eq!(superseding["entry"]["supersedes"], entry_id);
    assert_eq!(superseding["entry"]["blocks"].as_array().unwrap().len(), 5);
    assert_eq!(superseding["entry"]["blocks"][3]["kind"], "frame");
    assert_eq!(superseding["entry"]["blocks"][4]["kind"], "diff");

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
        "Data plane             LIVE READS · JOURNAL/CONVERSATION WRITE · CHANNEL WORKING WRITE · WORKLOAD APPROVAL",
        "Human entry            /explore",
        "Workload admission     ENABLED · EXACT WORKING FILES → QUALIFIED INDEX → HEAD",
        "Channel write          ENABLED · UNAUTHENTICATED · EXPECTED HEAD/WORKING → WORKING ONLY",
        "Conversation write     ENDPOINT ENABLED · EXACT SESSION DECIDES COMPOSER · UNAUTHENTICATED APPEND ONLY",
        "Revalidation           5000ms · PASSIVE · NO REFRESH CONTROL",
        "/api/v1/health · /api/v1/cadence · /api/v1/channels · /api/v1/channels/working · /api/v1/conversations · /api/v1/conversations/messages · /api/v1/environment · /api/v1/journal · /api/v1/journal/opportunities · /api/v1/journal/queries · /api/v1/journal/seed · /api/v1/observations · /api/v1/workloads · /api/v1/workloads/evidence · /api/v1/workloads/{id}/scenarios/{execution} · /api/v1/workloads/{id}/deltas/{delta} · /api/v1/workloads/admit",
        "Grammar revision       git:058c6504fc10740360717e97e687fd77bef6a5c5",
        "Implementation         UNBOUND · ",
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
    let channels = http_request(address, "GET /api/v1/channels HTTP/1.1");
    assert!(channels.starts_with("HTTP/1.1 200"));
    assert!(channels.contains("\"schema\":\"rey.ui-channels.v1\""));
    assert!(channels.contains("\"state\":\"clean\""));
    assert!(channels.contains("\"loopback_only\":true"));
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
    assert_eq!(descriptor["workload_admission_enabled"], true);
    assert_eq!(descriptor["channel_write_enabled"], true);
    assert_eq!(descriptor["conversation_write_enabled"], true);
    assert_eq!(
        descriptor["channel_root"],
        workspace.path().join(".rey/channels").display().to_string()
    );
    assert_eq!(
        descriptor["conversation_root"],
        workspace
            .path()
            .join(".rey/conversations")
            .display()
            .to_string()
    );
    assert_eq!(descriptor["application"], "tanstack_router");
    assert_eq!(descriptor["grammar"], "kinetic");
    assert_eq!(descriptor["theme"], "precision");
    assert_eq!(descriptor["entry_route"], "/explore");
    assert_eq!(descriptor["live_refresh_interval_ms"], 5_000);
    assert!(descriptor["source_repository"].is_null());
    assert!(descriptor["implementation_revision"].is_string());
    assert_eq!(
        descriptor["grammar_revision"],
        "git:058c6504fc10740360717e97e687fd77bef6a5c5"
    );
    let mut network_stderr = BufReader::new(network_child.stderr.take().unwrap());
    let mut warning = String::new();
    network_stderr.read_line(&mut warning).unwrap();
    assert!(warning.contains("unauthenticated Journal, conversation, and Channel WORKING writes"));
    assert!(warning.contains("exact workload approval enabled"));

    let network_address = format!("127.0.0.1:{}", descriptor["port"].as_u64().unwrap());
    let proposal = serde_json::json!({
        "schema": "rey.journal-entry-proposal.v2",
        "title": "Write through the network listener",
        "author": { "kind": "human", "id": "operator" },
        "binding": {
            "coordinate": "rey+local://portfolio/current?revision=blake3%3Anetwork",
            "scale": 0.68,
            "source_revision": "blake3:network"
        },
        "layout": {
            "kind": "broadsheet",
            "columns": 12,
            "bands": [{
                "id": "lead",
                "cells": [{ "block_id": "context", "span": 12 }]
            }]
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
    let channels = http_request(&network_address, "GET /api/v1/channels HTTP/1.1");
    assert!(channels.starts_with("HTTP/1.1 200"));
    assert!(channels.contains("\"loopback_only\":false"));
    assert!(channels.contains("without authentication"));
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
    assert!(passing.starts_with("WORKLOAD TEST · compiled catalog\n"));
    assert!(passing.contains("Assertions  EXPECTED → ACTUAL"));
    assert!(passing.contains("PASS 01/02 plain · 1/1 assertions satisfied · required"));
    assert!(passing.contains("PASS 02/02 surrounded · 1/1 assertions satisfied · required"));
    assert!(passing.contains("Result      QUALIFIED · 2/2 required scenarios passing"));
    assert!(passing.contains("TEST SUMMARY"));
    assert!(passing.contains("Output deltas        2 equal · 0 different · 0 inconclusive"));
    assert!(!passing.contains("Assertions (EXPECTED → ACTUAL)"));
    assert!(!passing.contains("Evidence (exact)"));
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
    assert!(failing.contains("FAIL 02/02 surrounded · 0/1 assertions satisfied · required"));
    assert!(failing.contains("Assertions (EXPECTED → ACTUAL)"));
    assert!(failing.contains("! output.text · DIFFERENT"));
    assert!(failing.contains("EXPECTED \"REY\""));
    assert!(failing.contains("ACTUAL   \" REY \""));
    assert!(failing.contains("@@ -1,1 +1,1 @@"));
    assert!(failing.contains("- REY"));
    assert!(failing.contains("+  REY "));
    assert!(failing.contains("Result               GAPS FOUND"));
    assert!(failing.contains("Output deltas        1 equal · 1 different · 0 inconclusive"));
    assert!(!failing.contains("Evidence (exact)"));
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
    assert!(verbose.contains("Graph       trim → uppercase · deterministic serial"));
    assert_eq!(verbose.matches("Assertions (EXPECTED → ACTUAL)").count(), 2);
    assert!(verbose.contains("= output.text · EQUAL"));
    assert!(verbose.contains("EXPECTED \"ATLAS\""));
    assert!(verbose.contains("ACTUAL   \"ATLAS\""));
    assert!(verbose.contains("Qualification issued"));
    assert!(!verbose.contains("Evidence (exact)"));
    assert!(!verbose.contains("Workload    rey.fixture.text-normalize@1"));

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
    assert!(very_verbose.contains("Graph       uppercase · deterministic serial"));
    assert!(very_verbose.contains("Workload    rey.fixture.text-mismatch@1 · blake3:"));
    assert!(very_verbose.contains("Graph id    rey.fixture.text-mismatch.graph@1 · blake3:"));
    assert!(very_verbose.contains("Suite       rey.fixture.text-mismatch.scenarios@1 · blake3:"));
    assert!(very_verbose.contains("Evaluator   rey.scenario.utf8-exact@1 · blake3:"));
    assert_eq!(very_verbose.matches("Evidence (exact)").count(), 2);
    assert_eq!(very_verbose.matches("Exact bindings:").count(), 2);
    assert!(very_verbose.contains("scenario    rey.fixture.text-mismatch.scenario.surrounded@1"));
    assert!(very_verbose.contains("execution   blake3:"));
    assert!(very_verbose.contains("delta       blake3:"));
    assert!(very_verbose.contains("Test result   blake3:"));
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
    assert!(help.contains("Render every EXPECTED → ACTUAL assertion"));
    assert!(help.contains("repeat as -vv for exact evidence bindings"));
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
        "Mining      VERIFIED",
        "PASS 01/04 empty",
        "PASS 02/04 exact",
        "FAIL 03/04 mismatch",
        "INCONCLUSIVE 04/04 truncated",
        "source.matches · DIFFERENT",
        "source.complete · INCONCLUSIVE",
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
fn scene_admission_is_qualified_and_run_from_an_exact_editor_commit() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    fs::create_dir_all(workspace.path().join("sys/scene-admission")).unwrap();
    fs::write(
        workspace.path().join("sys/scene-admission/workload.yaml"),
        include_str!("../../../sys/scene-admission/workload.yaml"),
    )
    .unwrap();

    let generated = run_rey_workspace(&[
        "editor",
        "--workspace",
        workspace_path,
        "generate",
        "terrain",
        "terrain.geojson",
        "--id",
        "regional-controls",
        "--scene-id",
        "regional-demo",
        "--seed",
        "17",
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
    ]);
    assert!(generated.status.success());
    assert!(
        run_rey_workspace(&["editor", "--workspace", workspace_path, "add",])
            .status
            .success()
    );
    assert!(
        run_rey_workspace(&[
            "editor",
            "--workspace",
            workspace_path,
            "commit",
            "-m",
            "Freeze regional candidate",
        ])
        .status
        .success()
    );

    assert!(
        run_rey_workspace(&["workloads", "--workspace", workspace_path, "add",])
            .status
            .success()
    );
    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "--staged",
        "scene-admission",
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
    for evidence in [
        "scene-admission · 11 scenarios · 22 assertions",
        "PASS 01/11 accepted · 2/2 assertions satisfied · required",
        "scene.admission · EQUAL",
        "OUTCOME ACCEPTED · accepted",
        "OUTCOME REJECTED · object_tampering",
        "NATIVE OGC:CRS84 longitude/latitude",
        "SYNTHETIC semantic longitude/latitude",
        "MERCATOR spherical chart",
        "COUNTY local east/north/up",
        "CAMERA view only",
        "COORDINATE {\"space\":\"native_crs84\"",
        "VALIDITY {",
        "AUTHORITY qualified workload result only",
    ] {
        assert!(
            tested.contains(evidence),
            "missing scene evidence: {evidence}"
        );
    }

    let committed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "Admit scene validation workload",
    ]);
    assert!(committed.status.success());

    let human = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "scene-admission",
        "--scene",
        "SCENE@1",
        "--format",
        "table",
    ]);
    assert!(
        human.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&human.stdout),
        String::from_utf8_lossy(&human.stderr),
    );
    let human = String::from_utf8(human.stdout).unwrap();
    for evidence in [
        "Result                 PASSED",
        "Node order             admit → render",
        "SCENE regional-demo · SCENE@1 · 2 native objects",
        "BINDING scene=blake3:",
        "package=blake3:",
        "packet=blake3:",
        "COORDINATE {\"space\":\"camera\"",
        "terrain height explicitly unsupported",
    ] {
        assert!(human.contains(evidence), "missing run evidence: {evidence}");
    }

    let structured = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "run",
        "scene-admission",
        "--scene",
        "SCENE@1",
    ]);
    assert!(structured.status.success());
    assert!(structured.stderr.is_empty());
    let structured: WorkloadRunView = serde_json::from_slice(&structured.stdout).unwrap();
    structured.result.verify().unwrap();
    assert_eq!(structured.result.scene_admissions.len(), 1);
    let admission = &structured.result.scene_admissions[0];
    assert_eq!(admission.status, SceneAdmissionStatus::Accepted);
    let scene = admission.scene.as_ref().unwrap();
    assert_eq!(scene.admission.editor_sequence, 1);
    assert_eq!(scene.projection.coordinate_bindings.len(), 5);
    assert_eq!(scene.projection.objects.len(), 2);
    assert!(scene.artifacts.terrain_program_id.is_none());
}

#[test]
fn context_topography_is_verifiable_across_cli_structured_state_and_ui_read_model() {
    let workspace = TempDir::new().unwrap();
    let workspace_path = workspace.path().to_str().unwrap();
    materialize_context_topography_fixture(workspace.path());

    let staged = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "add",
        "--format",
        "table",
    ]);
    assert!(staged.status.success());
    assert!(String::from_utf8_lossy(&staged.stdout).contains("WORKLOAD INDEX"));

    let tested = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "--staged",
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
        "Topography  VERIFIED",
        "topography.complete · EQUAL",
        "topography.complete · INCONCLUSIVE",
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
        "terrain program rey.projection.procedural-terrain@1",
        "working set ≤255×255 · ≤65025 cells · ≤3576375 bytes",
        "terrain band macro · wavelength 420 · amplitude 0.210 · 2 octave(s)",
        "terrain band micro · wavelength 24 · amplitude 0.018 · 2 octave(s)",
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

    let compact = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "test",
        "--staged",
        "context-anchor-survey",
        "--format",
        "table",
        "-v",
    ]);
    assert!(compact.status.success());
    assert!(compact.stderr.is_empty());
    let compact = String::from_utf8(compact.stdout).unwrap();
    for needle in [
        "context-anchor-survey · 3 scenarios · 6 assertions",
        "INCONCLUSIVE 01/03 bounded · 1/2 assertions satisfied · optional",
        "Assertions (EXPECTED → ACTUAL)",
        "= output.text · EQUAL",
        "EXPECTED 8 lines",
        "ACTUAL   8 lines",
        "? topography.complete · INCONCLUSIVE",
        "ACTUAL   bounded · seeds 1/1",
        "PASS 02/03 exact · 2/2 assertions satisfied · required",
        "= topography.complete · EQUAL",
    ] {
        assert!(
            compact.contains(needle),
            "missing compact assertion: {needle}"
        );
    }
    for folded in [
        "Evidence (exact)",
        "Projection packet:",
        "         ANCHOR",
        "         EDGE",
        "         REGION",
    ] {
        assert!(
            !compact.contains(folded),
            "compact assertions leaked exact evidence: {folded}"
        );
    }

    let committed = run_rey_workspace(&[
        "workloads",
        "--workspace",
        workspace_path,
        "commit",
        "-m",
        "Admit context anchor survey",
    ]);
    assert!(
        committed.status.success(),
        "{}",
        String::from_utf8_lossy(&committed.stderr)
    );

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
    assert_eq!(projection.terrain_program.schema, "rey.terrain-program.v1");
    assert_eq!(projection.terrain_program.bands.len(), 3);
    assert_eq!(projection.terrain_program.working_set.max_cells, 65_025);
    assert_eq!(projection.terrain_program.working_set.max_bytes, 3_576_375);
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
        "--format",
        "table",
    ]);
    assert!(status.status.success());
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(status.contains("On workload WORKLOAD@1"));
    assert!(status.contains("nothing to admit, working workload catalog clean"));

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
    fs::create_dir_all(workspace.join("sys/context-anchor-survey")).unwrap();
    fs::create_dir_all(workspace.join("docs")).unwrap();
    fs::create_dir_all(workspace.join("docs/decisions")).unwrap();
    fs::create_dir_all(workspace.join("plans")).unwrap();
    fs::write(
        workspace.join("sys/context-anchor-survey/workload.yaml"),
        include_str!("../../../sys/context-anchor-survey/workload.yaml"),
    )
    .unwrap();
    fs::write(
        workspace.join("sys/context-anchor-survey/request.yaml"),
        include_str!("../../../sys/context-anchor-survey/request.yaml"),
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
        workspace.join("plans/0003-scene-to-explorer.md"),
        include_str!("../../../plans/0003-scene-to-explorer.md"),
    )
    .unwrap();
    fs::write(
        workspace.join("docs/decisions/README.md"),
        include_str!("../../../docs/decisions/README.md"),
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
    assert!(initial.runtime.is_none());

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
        "Portfolio   VERIFIED",
        "PASS 01/06 blocked",
        "PASS 02/06 clean",
        "PASS 03/06 create",
        "PASS 04/06 excluded",
        "PASS 05/06 refine",
        "PASS 06/06 retest",
        "Portfolio attention:",
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
    assert!(status.contains("RUNTIME FRONTIER"));
    assert!(status.contains("Unavailable · no retained environment snapshot"));

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

    let structured = run_rey(&["workloads", "--workspace", workspace_path, "list"]);
    assert!(structured.status.success());
    assert!(structured.stderr.is_empty());
    let structured: WorkloadList = serde_json::from_slice(&structured.stdout).unwrap();
    let runtime = structured.runtime.as_ref().unwrap();
    runtime.frontier.verify().unwrap();
    runtime
        .scheduling
        .verify_against(&runtime.frontier)
        .unwrap();
    runtime.surface.as_ref().unwrap().verify().unwrap();
    assert_eq!(
        runtime.frontier.inputs.trace_id,
        structured.attention.attention_id
    );
    assert_eq!(runtime.frontier.rows.len(), 3);
    assert_eq!(runtime.scheduling.selected.len(), 1);
    assert_eq!(runtime.scheduling.selected_cost_units, 1);
    assert_eq!(runtime.surface.as_ref().unwrap().rows.len(), 1);
    assert_ne!(
        structured.attention.attention_id,
        runtime.frontier.frontier_id
    );
    assert_ne!(runtime.frontier.frontier_id, runtime.scheduling.decision_id);
    assert_ne!(
        runtime.scheduling.decision_id,
        runtime.surface.as_ref().unwrap().surface_id
    );

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
    for needle in [
        "RUNTIME FRONTIER",
        "Frontier               blake3:",
        "3 schedulable rows",
        "Attention trace        blake3:",
        "Portfolio snapshot     blake3:",
        "Environment            blake3:",
        "Scheduling             blake3:",
        "1 selected · cost 1/5",
        "Reasoning surface      blake3:",
        "1 rows · 2 evidence · 1 actions",
        "Surface budget",
        "Progress               not derived · no prior runtime frontier",
        "Proof                  not derived · no evaluated runtime transition",
    ] {
        assert!(
            listed.contains(needle),
            "missing runtime evidence: {needle}"
        );
    }

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

fn initialize_git_repository(workspace: &TempDir) {
    let workspace_path = workspace.path().to_str().unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "Rey Test"],
        vec!["config", "user.email", "rey@example.invalid"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
    fs::write(workspace.path().join("tracked"), "one\n").unwrap();
    for args in [
        vec!["add", "tracked"],
        vec!["commit", "-q", "-m", "initial"],
    ] {
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(workspace_path)
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    }
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
