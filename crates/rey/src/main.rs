#![forbid(unsafe_code)]

mod ui;

use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs::{self, File},
    io::{self, BufRead, IsTerminal, Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use rey::{
    ReyError,
    channels::{
        ChannelAddResult, ChannelApplyResult, ChannelCommitResult, ChannelDiff, ChannelGraph,
        ChannelGraphChange, ChannelGraphError, ChannelGraphSource, ChannelLog, ChannelMessage,
        ChannelMessageAdmission, ChannelMessageProposal, ChannelObjectKind, ChannelStatus,
        ChannelWorkingState, LocalChannelStore, MAX_CHANNEL_GRAPH_INPUT_BYTES,
        POLLING_BEACON_TICK_SCHEMA, PollingBeaconTick, RelayAttempt, RelayAttemptOutcome,
        relay_output_digest,
    },
    editor::{
        EditorAddResult, EditorCommitResult, EditorError, EditorGenerateResult, EditorLog,
        EditorStatus, LocalEditorStore, SceneBounds, SceneChangeKind, SceneChangeSet, SceneCommit,
        SceneObjectKind, SceneTerrainGenerationParameters,
    },
    env::{
        EnvironmentAddResult, EnvironmentAdmissionIndex, EnvironmentApplicationObservation,
        EnvironmentCommit, EnvironmentCommitResult, EnvironmentDiff, EnvironmentDiffMode,
        EnvironmentInputObservation, EnvironmentLog, EnvironmentObjectChange,
        EnvironmentObjectStatus, EnvironmentOperatorProjection, EnvironmentReferenceObservation,
        EnvironmentStatus, EnvironmentVariableObservation, EnvironmentWorkingState,
        LocalEnvironmentHistory, LocalEnvironmentHistoryError, LocalEnvironmentStore,
        effective_index_snapshot, stage_selected_capabilities,
    },
    git::{
        GIT_ACKNOWLEDGEMENT_SCHEMA, GIT_POLL_OUTCOME_SCHEMA, GitAcknowledgement, GitOperatorStatus,
        GitPollOutcome, GitPollRecord, GitWatchOutcome, GitWatchStopReason, LocalGitState,
        LocalGitStateError, LocalGitStore, MAX_GIT_WATCH_ELAPSED_MS, MAX_GIT_WATCH_INTERVAL_MS,
        MAX_GIT_WATCH_ITERATIONS,
    },
    inspect_environment, inspect_environment_with_mapping,
    journal::{
        JournalAdmission, JournalAuthorKind, JournalBlock, JournalEntryProposal, JournalError,
        JournalLog, LocalJournalStore, MAX_JOURNAL_PROPOSAL_BYTES,
    },
    workloads::{
        LocalWorkloadStateError, LocalWorkloadStore, MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES,
        ResolvedWorkload, WorkloadActivationAdmission, WorkloadActivationExecution,
        WorkloadActivationRecomputation, WorkloadAddResult, WorkloadCatalog,
        WorkloadCatalogDescriptor, WorkloadCatalogError, WorkloadChangeKind, WorkloadChangeSet,
        WorkloadCommitResult, WorkloadCreateResult, WorkloadCreationAttentionBinding,
        WorkloadDraft, WorkloadList, WorkloadLog, WorkloadRecomputationAssessment,
        WorkloadRevisionStatus, WorkloadRunView, WorkloadStatusBatch, WorkloadStatusView,
        WorkloadSummary, WorkloadTestBatch, WorkloadWorkingState, derive_portfolio_snapshot,
        fresh_qualification,
    },
};
use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::{
    CapabilityChange, CapabilityChangeKind, CapabilityDelta, CapabilitySemanticRecord,
    DeltaAssessment, ScenarioOutputDelta, SourceMatchChangeKind, TextLineKind,
    source_match_table_projection, text_patch_projection,
};
use rey_environment::{
    Availability, CapabilitySnapshot, CommandRequest, DiscoveryLimits, EnvironmentMapLimits,
    SourceBindingLimits, VariableCapture, resolve_executable, run_bounded,
};
use rey_git::{GitActivationTrigger, GitInspector, GitLimits};
use rey_locator::ResolutionLimits;
use rey_mining::{
    MiningCompleteness, MiningLimits, ProjectionPacket, TopographyLimits, TopographyPatch,
};
use rey_runtime::{
    BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID, BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
    CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID, PortfolioReasoningEvidence, RunStatus, ScenarioEvaluation,
    ScenarioResult, SourceRunInput, TestStatus, TopographySurveyInput, WorkloadAttention,
    WorkloadDefinition, WorkloadRunResult, WorkloadTestResult, WorkloadValue,
    execute_workload_scenario_selection_with_snapshot, orient_portfolio_attention, run_workload,
    run_workload_with_source, run_workload_with_topography, source_fixture_root,
    test_workload_with_observer_and_snapshot,
};
use serde::Serialize;
use thiserror::Error;

const ENVIRONMENT_VALUE_DISPLAY_CHARS: usize = 180;

#[derive(Debug, Parser)]
#[command(
    name = "rey",
    version,
    about = "Environment-aware diff-directed compute runtime"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect and propose workspace-local collaboration topology.
    Channels(ChannelsArgs),
    /// Track bounded compute environment revisions.
    Env(EnvArgs),
    /// Observe Git transitions and retain proposal-only activation evidence.
    Git(GitArgs),
    /// Track and generate read-first scene revisions for explicit Rey admission.
    Editor(EditorArgs),
    /// Inspect, test, qualify, and execute bounded compute graphs.
    Workloads(WorkloadsArgs),
    /// Read and admit bounded collaboration journal entries.
    Journal(JournalArgs),
    /// Serve the Rey operator interface.
    Ui(UiArgs),
}

#[derive(Debug, Args)]
struct GitArgs {
    /// Explicit workspace root; repository discovery cannot cross it.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local Git-activation state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    /// Total bounded Git observation deadline in milliseconds.
    #[arg(long, global = true, default_value_t = 5_000)]
    total_timeout_ms: u64,

    /// Per-command Git observation deadline in milliseconds.
    #[arg(long, global = true, default_value_t = 2_000)]
    command_timeout_ms: u64,

    /// Maximum captured Git output bytes per command.
    #[arg(long, global = true, default_value_t = 4 * 1_024 * 1_024)]
    max_capture_bytes: u64,

    /// Maximum logical Git index entries.
    #[arg(long, global = true, default_value_t = 10_000)]
    max_index_entries: u64,

    #[command(subcommand)]
    command: GitCommand,
}

#[derive(Debug, Subcommand)]
enum GitCommand {
    /// Observe current repository and retained cursor state without changing either.
    Status(GitOutputArgs),
    /// Retain the exact current repository snapshot as the initial poll cursor.
    Init(GitInitArgs),
    /// Observe and retain one exact pending transition without advancing its cursor.
    Poll(GitPollArgs),
    /// Repeatedly observe under explicit cadence bounds, retaining every tick.
    Watch(GitWatchArgs),
    /// Acknowledge retained transition evidence and advance the cursor exactly once.
    Ack(GitAckArgs),
}

#[derive(Debug, Args)]
struct GitOutputArgs {
    /// Human evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct GitInitArgs {
    /// Exact full Git ref to retain in the poll scope; repeatable.
    #[arg(long = "watch-ref")]
    watched_refs: Vec<String>,

    /// Human evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct GitPollArgs {
    /// Workspace-confined YAML or JSON typed activation trigger; repeatable.
    #[arg(long = "trigger")]
    triggers: Vec<PathBuf>,

    /// Human transition evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct GitWatchArgs {
    /// Workspace-confined YAML or JSON typed activation trigger; repeatable.
    #[arg(long = "trigger")]
    triggers: Vec<PathBuf>,

    /// Maximum retained observations before stopping.
    #[arg(long, default_value_t = 32)]
    max_iterations: u64,

    /// Delay between retained observations in milliseconds.
    #[arg(long, default_value_t = 1_000)]
    interval_ms: u64,

    /// Maximum elapsed cadence time checked between observations in milliseconds.
    #[arg(long, default_value_t = 60_000)]
    max_elapsed_ms: u64,

    /// Human cadence evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct GitAckArgs {
    /// Exact retained transition identity to acknowledge.
    transition_id: String,

    /// Human receipt or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorArgs {
    /// Workspace containing the agent-authored native sources.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local editor-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: EditorCommand,
}

#[derive(Debug, Subcommand)]
enum EditorCommand {
    /// Deterministically generate tunable scene features into WORKING.
    Generate(EditorGenerateArgs),
    /// Show HEAD, INDEX, and WORKING state without changing it.
    Status(EditorOutputArgs),
    /// Stage the exact verified project and immutable native-source objects.
    Add(EditorOutputArgs),
    /// Validate and commit exactly the staged INDEX, then emit its unadmitted request.
    Commit(EditorCommitArgs),
    /// Show committed scene revisions newest first.
    Log(EditorLogArgs),
    /// Show INDEX to WORKING changes, or HEAD to INDEX with --staged.
    Diff(EditorDiffArgs),
}

#[derive(Debug, Args)]
struct EditorOutputArgs {
    /// Human evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorDiffArgs {
    /// Compare the current scene HEAD with the INDEX.
    #[arg(long)]
    staged: bool,

    /// Human semantic changes or typed JSON change set.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorCommitArgs {
    /// Commit message bound into the scene revision identity.
    #[arg(short = 'm', long = "message", required = true)]
    message: String,

    /// Human receipt or typed JSON commit, package, and admission request.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorLogArgs {
    /// Render each exact parent-to-commit scene patch.
    #[arg(short = 'p', long = "patch")]
    patch: bool,

    /// Maximum number of newest commits to show.
    #[arg(short = 'n', long = "max-count", default_value_t = 32)]
    max_count: usize,

    /// Human history or typed JSON log.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorGenerateArgs {
    #[command(subcommand)]
    generator: EditorGeneratorCommand,
}

#[derive(Debug, Subcommand)]
enum EditorGeneratorCommand {
    /// Generate deterministic polygonal uplift and depression controls.
    Terrain(EditorGenerateTerrainArgs),
}

#[derive(Debug, Args)]
struct EditorGenerateTerrainArgs {
    /// Workspace-relative generated GeoJSON output.
    output: PathBuf,

    /// Stable source identity registered in the scene project.
    #[arg(long)]
    id: String,

    /// Scene identity used when generate creates the project; defaults to source id.
    #[arg(long)]
    scene_id: Option<String>,

    /// Deterministic generator seed.
    #[arg(long, default_value_t = 1)]
    seed: u64,

    /// Western CRS84 longitude bound.
    #[arg(long, allow_hyphen_values = true)]
    west: f64,

    /// Southern CRS84 latitude bound.
    #[arg(long, allow_hyphen_values = true)]
    south: f64,

    /// Eastern CRS84 longitude bound.
    #[arg(long, allow_hyphen_values = true)]
    east: f64,

    /// Northern CRS84 latitude bound.
    #[arg(long, allow_hyphen_values = true)]
    north: f64,

    /// Number of generated terrain controls.
    #[arg(long, default_value_t = 24)]
    features: u64,

    /// Polygon vertices per terrain control.
    #[arg(long, default_value_t = 9)]
    vertices: u64,

    /// Minimum feature scale as a fraction of the generation bounds.
    #[arg(long, default_value_t = 0.025)]
    scale_min: f64,

    /// Maximum feature scale as a fraction of the generation bounds.
    #[arg(long, default_value_t = 0.09)]
    scale_max: f64,

    /// Fraction of controls producing uplift instead of depression.
    #[arg(long, default_value_t = 0.68)]
    uplift_ratio: f64,

    /// Mean normalized terrain-effect strength.
    #[arg(long, default_value_t = 0.72)]
    strength: f64,

    /// Symmetric normalized variation around effect strength.
    #[arg(long, default_value_t = 0.24)]
    strength_jitter: f64,

    /// Mean normalized surface roughness.
    #[arg(long, default_value_t = 0.58)]
    roughness: f64,

    /// Symmetric normalized variation around roughness.
    #[arg(long, default_value_t = 0.2)]
    roughness_jitter: f64,

    /// Major-to-minor feature-axis ratio.
    #[arg(long, default_value_t = 1.8)]
    anisotropy: f64,

    /// Mean feature-axis orientation in degrees.
    #[arg(long, default_value_t = 30.0, allow_hyphen_values = true)]
    orientation_degrees: f64,

    /// Symmetric orientation variation in degrees.
    #[arg(long, default_value_t = 45.0)]
    orientation_jitter_degrees: f64,

    /// Normalized per-vertex outline variation.
    #[arg(long, default_value_t = 0.14)]
    edge_jitter: f64,

    /// Positive radial influence falloff exponent.
    #[arg(long, default_value_t = 2.2)]
    falloff: f64,

    /// Human generation receipt or typed JSON lineage.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelsArgs {
    /// Workspace used as the Channel graph and local-state boundary.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local Channel-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: ChannelsCommand,
}

#[derive(Debug, Subcommand)]
enum ChannelsCommand {
    /// List the effective channels, subscriptions, and ordered Feed streams.
    List(ChannelListArgs),
    /// Show Channel HEAD, INDEX, and WORKING state.
    Status(ChannelStatusArgs),
    /// Display INDEX to WORKING changes, or HEAD to INDEX with --staged.
    Diff(ChannelDiffArgs),
    /// Validate a workspace-contained YAML graph and write Channel WORKING.
    Apply(ChannelApplyArgs),
    /// Stage the exact Channel WORKING graph in the admission INDEX.
    Add(ChannelOutputArgs),
    /// Commit exactly the staged Channel INDEX.
    Commit(ChannelCommitArgs),
    /// Show committed Channel revisions newest first.
    Log(ChannelLogArgs),
    /// Admit or inspect immutable channel messages.
    Message(ChannelMessageArgs),
    /// Relay one admitted message through one admitted application and relay.
    Relay(ChannelRelayArgs),
    /// Run one explicit bounded polling-beacon tick.
    Beacon(ChannelBeaconArgs),
}

#[derive(Debug, Args)]
struct ChannelOutputArgs {
    /// Human evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelListArgs {
    /// Human inventory or typed JSON graph snapshot.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelStatusArgs {
    /// Human working-tree status or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelDiffArgs {
    /// Compare current Channel HEAD with the admission INDEX.
    #[arg(long)]
    staged: bool,

    /// Human semantic patch or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelCommitArgs {
    /// Commit message bound into the Channel revision identity.
    #[arg(short = 'm', long = "message", required = true)]
    message: String,

    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelLogArgs {
    /// Render each retained semantic patch.
    #[arg(short = 'p', long = "patch")]
    patch: bool,

    /// Maximum number of newest commits to show.
    #[arg(short = 'n', long = "max-count", default_value_t = 32)]
    max_count: usize,

    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelMessageArgs {
    #[command(subcommand)]
    command: ChannelMessageCommand,
}

#[derive(Debug, Subcommand)]
enum ChannelMessageCommand {
    /// Admit a workspace-contained rey.channel-message.v1 YAML file.
    Add(ChannelMessageAddArgs),
    /// List admitted immutable messages.
    List(ChannelOutputArgs),
}

#[derive(Debug, Args)]
struct ChannelMessageAddArgs {
    /// Workspace-relative rey.channel-message.v1 YAML file.
    message: PathBuf,

    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelRelayArgs {
    /// Exact admitted message digest.
    message_id: String,

    /// Exact relay id from Channel HEAD.
    #[arg(long)]
    relay: String,

    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelBeaconArgs {
    /// Exact polling beacon id from Channel HEAD.
    beacon_id: String,

    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct ChannelApplyArgs {
    /// Workspace-contained YAML graph using rey.channel-graph.v1.
    graph: PathBuf,

    /// Human receipt or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct JournalArgs {
    /// Workspace used as the journal and default local-state boundary.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local journal-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: JournalCommand,
}

#[derive(Debug, Subcommand)]
enum JournalCommand {
    /// Admit one typed YAML journal entry proposal without executing its blocks.
    Add(JournalAddArgs),
    /// List retained journal entries in admission order.
    List(JournalListArgs),
}

#[derive(Debug, Args)]
struct JournalAddArgs {
    /// Workspace-contained YAML proposal using rey.journal-entry-proposal.v2.
    proposal: PathBuf,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct JournalListArgs {
    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct UiArgs {
    /// Workspace projected through the operator UI.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local workload-state directory; relative paths resolve below the workspace.
    #[arg(long)]
    state_dir: Option<PathBuf>,

    /// Explicit local journal-state directory; relative paths resolve below the workspace.
    #[arg(long)]
    journal_state_dir: Option<PathBuf>,

    /// Workspace-relative workload package root.
    #[arg(long, default_value = "sys")]
    catalog_dir: PathBuf,

    /// IP address to bind; defaults to IPv4 loopback.
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// TCP port to bind; use 0 to select an available ephemeral port.
    #[arg(long, default_value_t = 5_714)]
    port: u16,

    /// Startup representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadsArgs {
    /// Workspace used as the default local result-state boundary.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local result-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    /// Workload catalog to resolve; workspace packages are the product surface.
    #[arg(long, global = true, value_enum, default_value_t = WorkloadCatalogSelection::Workspace)]
    catalog: WorkloadCatalogSelection,

    /// Workspace-relative package root used by the workspace catalog.
    #[arg(long, global = true, default_value = "sys")]
    catalog_dir: PathBuf,

    #[command(subcommand)]
    command: WorkloadsCommand,
}

#[derive(Debug, Subcommand)]
enum WorkloadsCommand {
    /// Create a strict workload request for an external coding harness.
    Create(WorkloadCreateArgs),
    /// List the admitted workload HEAD without executing it.
    List(WorkloadOutputArgs),
    /// Show HEAD, INDEX, and WORKING workload admission state.
    Status(WorkloadStatusArgs),
    /// Stage exact verified WORKING workload packages into INDEX.
    Add(WorkloadOutputArgs),
    /// Approve the qualified INDEX and advance workload HEAD.
    Commit(WorkloadCommitArgs),
    /// Show admitted workload commits newest first.
    Log(WorkloadLogArgs),
    /// Show INDEX to WORKING changes, or HEAD to INDEX with --staged.
    Diff(WorkloadDiffArgs),
    /// Execute required scenarios against the exact staged INDEX.
    Test(WorkloadTestArgs),
    /// Execute an exactly qualified graph admitted in HEAD.
    Run(WorkloadRunArgs),
    /// Admit one acknowledged Git activation into workload runtime scheduling.
    AdmitActivation(WorkloadAdmitActivationArgs),
    /// Execute the exact selected scenarios for one retained activation admission.
    ExecuteActivation(WorkloadExecuteActivationArgs),
    /// Fully recompute and compare one retained activation execution.
    VerifyActivation(WorkloadVerifyActivationArgs),
}

#[derive(Debug, Args)]
struct WorkloadCreateArgs {
    /// Stable workload id and package-directory name.
    workload_id: String,

    /// Human-readable purpose; defaults exactly to the workload id.
    #[arg(long)]
    title: Option<String>,

    /// Bounded objective the coding harness should mine and formalize.
    #[arg(long)]
    intent: Option<String>,

    /// Exact selected CREATE attention row to bind into the immutable harness request.
    #[arg(long)]
    attention_row: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadStatusArgs {
    /// Diagnostic workload id; only valid with --catalog conformance.
    workload_id: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadOutputArgs {
    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadCommitArgs {
    /// Human approval message bound into workload history.
    #[arg(short = 'm', long = "message", required = true)]
    message: String,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadLogArgs {
    /// Render each exact parent-to-commit package patch.
    #[arg(short = 'p', long = "patch")]
    patch: bool,

    /// Maximum number of newest commits to show.
    #[arg(short = 'n', long = "max-count", default_value_t = 32)]
    max_count: usize,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadDiffArgs {
    /// Compare workload HEAD with the staged INDEX.
    #[arg(long)]
    staged: bool,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadTestArgs {
    /// Workload id; omit to test every workload in the selected catalog.
    workload_id: Option<String>,

    /// Qualify the exact frozen INDEX; required for workspace workloads.
    #[arg(long)]
    staged: bool,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,

    /// Render every EXPECTED → ACTUAL assertion; repeat as -vv for exact evidence bindings.
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Args)]
struct WorkloadAdmitActivationArgs {
    /// Exact acknowledged Git activation proposal identity.
    activation_id: String,

    /// Human admission receipt or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadExecuteActivationArgs {
    /// Exact retained workload activation admission identity.
    admission_id: String,

    /// Human execution receipt or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadVerifyActivationArgs {
    /// Exact retained workload activation execution identity.
    execution_id: String,

    /// Maximum serialized bytes retained for the full recomputation result.
    #[arg(long, default_value_t = MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES)]
    max_evidence_bytes: u64,

    /// Human comparison receipt or typed JSON proof.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadRunArgs {
    /// Exact workload id in the selected catalog.
    workload_id: String,

    /// UTF-8 value bound to a text workload; omitted for portfolio mining.
    #[arg(long)]
    input: Option<String>,

    /// Workspace-relative regular source file; repeat to bind an explicit corpus.
    #[arg(long = "source")]
    sources: Vec<PathBuf>,

    /// Complete source lines retained before each match.
    #[arg(long, default_value_t = 0)]
    context_before: u64,

    /// Complete source lines retained after each match.
    #[arg(long, default_value_t = 0)]
    context_after: u64,

    /// Maximum accepted source matches.
    #[arg(long, default_value_t = 100_000)]
    max_matches: u64,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum WorkloadOutputFormat {
    Auto,
    Table,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum WorkloadCatalogSelection {
    Workspace,
    Conformance,
}

impl WorkloadOutputFormat {
    fn resolve(self) -> Self {
        match (self, io::stdout().is_terminal()) {
            (Self::Auto, true) => Self::Table,
            (Self::Auto, false) => Self::Json,
            (selected, _) => selected,
        }
    }
}

#[derive(Debug, Args)]
struct EnvArgs {
    /// Workspace used as the environment and default local-history boundary.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local history directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: EnvCommand,
}

#[derive(Debug, Subcommand)]
enum EnvCommand {
    /// Show HEAD, admitted changes, and fresh working changes.
    Status(EnvStatusArgs),
    /// Add the working environment to the admission index.
    Add(EnvAddArgs),
    /// Commit the verified environment admission index; success is silent.
    Commit(EnvCommitArgs),
    /// Show committed environment revisions newest first.
    Log(EnvLogArgs),
    /// Display unstaged or staged environment changes.
    Diff(EnvDiffArgs),
}

#[derive(Clone, Debug, Args)]
struct EnvDiscoveryArgs {
    /// Explicit workspace-relative agent-generated environment mapping resource.
    #[arg(long)]
    map: Option<PathBuf>,

    /// Total environment discovery deadline in milliseconds.
    #[arg(long, default_value_t = 5_000)]
    total_timeout_ms: u64,

    /// Per-process identity-probe deadline in milliseconds.
    #[arg(long, default_value_t = 1_000)]
    probe_timeout_ms: u64,

    /// Maximum bytes captured from each process output stream.
    #[arg(long, default_value_t = 65_536)]
    max_capture_bytes: u64,
}

impl EnvDiscoveryArgs {
    fn limits(&self) -> Result<DiscoveryLimits, CliError> {
        if self.total_timeout_ms == 0 || self.probe_timeout_ms == 0 || self.max_capture_bytes == 0 {
            return Err(CliError::InvalidLimit);
        }
        Ok(DiscoveryLimits {
            total_timeout_ms: self.total_timeout_ms,
            probe_timeout_ms: self.probe_timeout_ms,
            max_capture_bytes: self.max_capture_bytes,
            max_capabilities: 64,
        })
    }
}

#[derive(Debug, Args)]
struct EnvStatusArgs {
    /// Maximum capability changes admitted to the working delta.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,

    /// Human document or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = EnvHistoryOutputFormat::Table)]
    format: EnvHistoryOutputFormat,

    #[command(flatten)]
    discovery: EnvDiscoveryArgs,
}

#[derive(Debug, Args)]
struct EnvAddArgs {
    /// Interactively confirm environment hunks to admit.
    #[arg(short = 'p', long = "patch")]
    patch: bool,

    /// Maximum capability changes admitted to the index operation.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,

    /// Human receipt or typed JSON envelope; patch selection requires table.
    #[arg(long, value_enum, default_value_t = EnvHistoryOutputFormat::Table)]
    format: EnvHistoryOutputFormat,

    #[command(flatten)]
    discovery: EnvDiscoveryArgs,
}

#[derive(Debug, Args)]
struct EnvCommitArgs {
    /// Commit message bound into the environment revision identity.
    #[arg(short = 'm', long = "message", required = true)]
    message: String,

    /// Maximum capability changes admitted to the commit summary.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,

    /// Silent success or a typed JSON receipt.
    #[arg(long, value_enum, default_value_t = EnvHistoryOutputFormat::Table)]
    format: EnvHistoryOutputFormat,
}

#[derive(Debug, Args)]
struct EnvLogArgs {
    /// Render the exact parent-to-commit environment patch.
    #[arg(short = 'p', long = "patch")]
    patch: bool,

    /// Maximum number of newest commits to show.
    #[arg(short = 'n', long = "max-count", default_value_t = 32)]
    max_count: usize,

    /// Maximum capability changes admitted per retained delta.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,

    /// Human history or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = EnvHistoryOutputFormat::Table)]
    format: EnvHistoryOutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EnvHistoryOutputFormat {
    Table,
    Json,
}

#[derive(Debug, Args)]
struct EnvDiffArgs {
    /// Maximum capability changes admitted to the working delta.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,

    /// Human patch or typed JSON envelope.
    #[arg(long, value_enum, default_value_t = EnvHistoryOutputFormat::Table)]
    format: EnvHistoryOutputFormat,

    /// Compare committed HEAD with the admission index.
    #[arg(long)]
    staged: bool,

    #[command(flatten)]
    discovery: EnvDiscoveryArgs,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(exit_code) => exit_code,
        Err(error) => {
            eprintln!("rey: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, CliError> {
    match cli.command {
        Command::Channels(args) => channels_command(args),
        Command::Env(args) => env_command(args),
        Command::Git(args) => git_command(args),
        Command::Editor(args) => editor_command(args),
        Command::Workloads(args) => workloads(args),
        Command::Journal(args) => journal_command(args),
        Command::Ui(args) => ui_command(args),
    }
}

impl GitArgs {
    fn limits(&self) -> Result<GitLimits, CliError> {
        if self.total_timeout_ms == 0
            || self.command_timeout_ms == 0
            || self.max_capture_bytes == 0
            || self.max_index_entries == 0
        {
            return Err(CliError::InvalidLimit);
        }
        Ok(GitLimits {
            total_timeout_ms: self.total_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            max_capture_bytes: self.max_capture_bytes,
            max_index_entries: self.max_index_entries,
        })
    }
}

fn git_command(args: GitArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let limits = args.limits()?;
    let state_dir = match args.state_dir {
        Some(path) if path.is_absolute() => path,
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => workspace.join(path),
        None => workspace.join(".rey/git"),
    };
    let paths = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let git_program = resolve_executable("git", &paths).ok_or(CliError::GitUnavailable)?;
    let inspector = GitInspector {
        git_program,
        workspace: workspace.clone(),
        limits,
    };
    let store = LocalGitStore::new(state_dir);
    match args.command {
        GitCommand::Status(command) => git_status(&store, &inspector, command),
        GitCommand::Init(command) => git_init(&store, &inspector, command),
        GitCommand::Poll(command) => git_poll(&workspace, &store, &inspector, command),
        GitCommand::Watch(command) => git_watch(&workspace, &store, &inspector, command),
        GitCommand::Ack(command) => git_ack(&store, command),
    }
}

fn git_snapshot(
    inspector: &GitInspector,
    watched_refs: &[String],
) -> Result<rey_git::GitSnapshot, CliError> {
    inspector
        .inspect_with_watched_refs(watched_refs)?
        .ok_or(CliError::GitRepositoryAbsent)
}

fn git_status(
    store: &LocalGitStore,
    inspector: &GitInspector,
    args: GitOutputArgs,
) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let watched_refs = state
        .cursor
        .as_ref()
        .map(|cursor| {
            cursor
                .watched_refs
                .iter()
                .map(|watched| watched.name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let status = GitOperatorStatus::new(git_snapshot(inspector, &watched_refs)?, state)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        WorkloadOutputFormat::Table => write_git_status(&mut stdout, &status)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn git_init(
    store: &LocalGitStore,
    inspector: &GitInspector,
    args: GitInitArgs,
) -> Result<ExitCode, CliError> {
    let state = store.initialize(git_snapshot(inspector, &args.watched_refs)?)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &state)?,
        WorkloadOutputFormat::Table => write_git_initialized(&mut stdout, &state)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn git_poll(
    workspace: &Path,
    store: &LocalGitStore,
    inspector: &GitInspector,
    args: GitPollArgs,
) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let cursor = state
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    let (target, transition) = inspector
        .inspect_transition(cursor)?
        .ok_or(CliError::GitRepositoryAbsent)?;
    let triggers = args
        .triggers
        .iter()
        .map(|path| load_git_trigger(workspace, path))
        .collect::<Result<Vec<_>, _>>()?;
    let record = GitPollRecord::new(target, transition, triggers)?;
    let changed = record.transition.source_snapshot_id != record.transition.target_snapshot_id
        || !record.transition.events.is_empty();
    let retained = if changed {
        store.retain_poll(record.clone())?;
        true
    } else {
        false
    };
    let outcome = GitPollOutcome {
        schema: GIT_POLL_OUTCOME_SCHEMA.to_owned(),
        changed,
        retained,
        record,
    };
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &outcome)?,
        WorkloadOutputFormat::Table => write_git_poll(&mut stdout, &outcome)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn git_watch(
    workspace: &Path,
    store: &LocalGitStore,
    inspector: &GitInspector,
    args: GitWatchArgs,
) -> Result<ExitCode, CliError> {
    if args.max_iterations == 0
        || args.max_iterations > MAX_GIT_WATCH_ITERATIONS
        || args.interval_ms == 0
        || args.interval_ms > MAX_GIT_WATCH_INTERVAL_MS
        || args.max_elapsed_ms == 0
        || args.max_elapsed_ms > MAX_GIT_WATCH_ELAPSED_MS
    {
        return Err(CliError::InvalidLimit);
    }
    let initial = store.load()?;
    if let Some(pending) = initial.pending {
        return Err(LocalGitStateError::PendingPoll(pending.transition.transition_id).into());
    }
    initial
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    let triggers = args
        .triggers
        .iter()
        .map(|path| load_git_trigger(workspace, path))
        .collect::<Result<Vec<_>, _>>()?;
    let started = Instant::now();
    let cadence = Duration::from_millis(args.interval_ms);
    let elapsed_limit = Duration::from_millis(args.max_elapsed_ms);
    let mut ticks = Vec::new();
    let stop_reason = loop {
        let state = store.load()?;
        let cursor = state
            .cursor
            .as_ref()
            .ok_or(LocalGitStateError::Uninitialized)?;
        let (target, transition) = inspector
            .inspect_transition(cursor)?
            .ok_or(CliError::GitRepositoryAbsent)?;
        let record = GitPollRecord::new(target, transition, triggers.clone())?;
        let (_, tick) = store.retain_cadence_poll(record, args.interval_ms)?;
        let changed = tick.changed;
        ticks.push(tick);
        if changed {
            break GitWatchStopReason::PendingTransition;
        }
        if ticks.len() as u64 == args.max_iterations {
            break GitWatchStopReason::IterationLimit;
        }
        if started.elapsed() >= elapsed_limit
            || started.elapsed().saturating_add(cadence) > elapsed_limit
        {
            break GitWatchStopReason::TimeLimit;
        }
        std::thread::sleep(cadence);
    };
    let outcome = GitWatchOutcome::new(
        args.max_iterations,
        args.interval_ms,
        args.max_elapsed_ms,
        started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        ticks,
        stop_reason,
    )?;
    store.retain_watch_outcome(&outcome)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &outcome)?,
        WorkloadOutputFormat::Table => write_git_watch(&mut stdout, &outcome)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn git_ack(store: &LocalGitStore, args: GitAckArgs) -> Result<ExitCode, CliError> {
    let state = store.acknowledge(&args.transition_id)?;
    let cursor = state
        .cursor
        .clone()
        .ok_or(LocalGitStateError::Uninitialized)?;
    let result = GitAcknowledgement {
        schema: GIT_ACKNOWLEDGEMENT_SCHEMA.to_owned(),
        acknowledged_transition_id: cursor.retained_evidence_id.clone(),
        cursor,
        retained_transition_count: state.retained_polls.len() as u64,
        authority: "cursor advanced from retained evidence; no Git mutation or workload execution"
            .to_owned(),
    };
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_git_acknowledgement(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn load_git_trigger(workspace: &Path, requested: &Path) -> Result<GitActivationTrigger, CliError> {
    const MAX_TRIGGER_BYTES: u64 = 1_024 * 1_024;
    let path = if requested.is_absolute() {
        requested.to_owned()
    } else {
        workspace.join(requested)
    };
    let metadata = fs::symlink_metadata(&path).map_err(|source| CliError::GitTriggerInput {
        path: path.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::GitTriggerInputType(path));
    }
    let canonical = path
        .canonicalize()
        .map_err(|source| CliError::GitTriggerInput {
            path: path.clone(),
            source,
        })?;
    if !canonical.starts_with(workspace) {
        return Err(CliError::GitTriggerInputEscape(canonical));
    }
    if metadata.len() > MAX_TRIGGER_BYTES {
        return Err(CliError::GitTriggerInputLimit(MAX_TRIGGER_BYTES));
    }
    let mut bytes = Vec::new();
    File::open(&canonical)
        .map_err(|source| CliError::GitTriggerInput {
            path: canonical.clone(),
            source,
        })?
        .take(MAX_TRIGGER_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::GitTriggerInput {
            path: canonical.clone(),
            source,
        })?;
    if bytes.len() as u64 > MAX_TRIGGER_BYTES {
        return Err(CliError::GitTriggerInputLimit(MAX_TRIGGER_BYTES));
    }
    let trigger: GitActivationTrigger = serde_saphyr::from_slice(&bytes)?;
    trigger.verify()?;
    Ok(trigger)
}

fn editor_command(args: EditorArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let store = match args.state_dir {
        Some(path) if path.is_absolute() => LocalEditorStore::new(workspace.clone(), path),
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => LocalEditorStore::new(workspace.clone(), workspace.join(path)),
        None => LocalEditorStore::default_for_workspace(&workspace),
    };
    match args.command {
        EditorCommand::Generate(command) => editor_generate(&store, command),
        EditorCommand::Status(command) => editor_status(&store, command),
        EditorCommand::Add(command) => editor_add(&store, command),
        EditorCommand::Commit(command) => editor_commit(&store, command),
        EditorCommand::Log(command) => editor_log(&store, command),
        EditorCommand::Diff(command) => editor_diff(&store, command),
    }
}

fn editor_status(store: &LocalEditorStore, args: EditorOutputArgs) -> Result<ExitCode, CliError> {
    let status = store.status()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        WorkloadOutputFormat::Table => {
            write_editor_status(&mut stdout, &status, TerminalStyle::stdout())?
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_diff(store: &LocalEditorStore, args: EditorDiffArgs) -> Result<ExitCode, CliError> {
    let diff = store.diff(args.staged)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        WorkloadOutputFormat::Table => write_editor_diff(&mut stdout, &diff)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_add(store: &LocalEditorStore, args: EditorOutputArgs) -> Result<ExitCode, CliError> {
    let result = store.add()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_add(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_generate(
    store: &LocalEditorStore,
    args: EditorGenerateArgs,
) -> Result<ExitCode, CliError> {
    let EditorGeneratorCommand::Terrain(args) = args.generator;
    let result = store.generate_terrain(
        &args.output,
        args.scene_id,
        args.id,
        args.seed,
        SceneBounds {
            west: args.west,
            south: args.south,
            east: args.east,
            north: args.north,
        },
        SceneTerrainGenerationParameters {
            feature_count: args.features,
            vertices: args.vertices,
            scale_min: args.scale_min,
            scale_max: args.scale_max,
            uplift_ratio: args.uplift_ratio,
            strength: args.strength,
            strength_jitter: args.strength_jitter,
            roughness: args.roughness,
            roughness_jitter: args.roughness_jitter,
            anisotropy: args.anisotropy,
            orientation_degrees: args.orientation_degrees,
            orientation_jitter_degrees: args.orientation_jitter_degrees,
            edge_jitter: args.edge_jitter,
            falloff: args.falloff,
        },
    )?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_generate(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_commit(store: &LocalEditorStore, args: EditorCommitArgs) -> Result<ExitCode, CliError> {
    let result = store.commit(args.message)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_commit(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_log(store: &LocalEditorStore, args: EditorLogArgs) -> Result<ExitCode, CliError> {
    let log = store.log(args.max_count, args.patch)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &log)?,
        WorkloadOutputFormat::Table => {
            write_editor_log(&mut stdout, &log, TerminalStyle::stdout())?
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channels_command(args: ChannelsArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let store = match args.state_dir {
        Some(path) if path.is_absolute() => LocalChannelStore::new(path),
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => LocalChannelStore::new(workspace.join(path)),
        None => LocalChannelStore::default_for_workspace(&workspace),
    };
    match args.command {
        ChannelsCommand::List(command) => channel_list(&store, command),
        ChannelsCommand::Status(command) => channel_status(&store, command),
        ChannelsCommand::Diff(command) => channel_diff(&store, command),
        ChannelsCommand::Apply(command) => channel_apply(&store, &workspace, command),
        ChannelsCommand::Add(command) => channel_add(&store, command),
        ChannelsCommand::Commit(command) => channel_commit(&store, command),
        ChannelsCommand::Log(command) => channel_log(&store, command),
        ChannelsCommand::Message(command) => channel_message(&store, &workspace, command),
        ChannelsCommand::Relay(command) => channel_relay(&store, &workspace, command),
        ChannelsCommand::Beacon(command) => channel_beacon(&store, &workspace, command),
    }
}

fn channel_list(store: &LocalChannelStore, args: ChannelListArgs) -> Result<ExitCode, CliError> {
    let status = store.status()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status.working)?,
        WorkloadOutputFormat::Table => write_channel_list(&mut stdout, &status)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_status(
    store: &LocalChannelStore,
    args: ChannelStatusArgs,
) -> Result<ExitCode, CliError> {
    let status = store.status()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        WorkloadOutputFormat::Table => write_channel_status(&mut stdout, &status)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_diff(store: &LocalChannelStore, args: ChannelDiffArgs) -> Result<ExitCode, CliError> {
    let diff = store.diff(args.staged)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        WorkloadOutputFormat::Table => write_channel_diff(&mut stdout, &diff)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_add(store: &LocalChannelStore, args: ChannelOutputArgs) -> Result<ExitCode, CliError> {
    let result = store.add()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_channel_add(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_commit(
    store: &LocalChannelStore,
    args: ChannelCommitArgs,
) -> Result<ExitCode, CliError> {
    let result = store.commit(args.message)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_channel_commit(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_log(store: &LocalChannelStore, args: ChannelLogArgs) -> Result<ExitCode, CliError> {
    let log = store.log(args.max_count, args.patch)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &log)?,
        WorkloadOutputFormat::Table => write_channel_log(&mut stdout, &log)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_message(
    store: &LocalChannelStore,
    workspace: &Path,
    args: ChannelMessageArgs,
) -> Result<ExitCode, CliError> {
    let mut stdout = io::stdout().lock();
    match args.command {
        ChannelMessageCommand::Add(args) => {
            let bytes = read_workspace_channel_input(
                workspace,
                &args.message,
                MAX_CHANNEL_GRAPH_INPUT_BYTES,
            )?;
            let proposal: ChannelMessageProposal = serde_saphyr::from_slice(&bytes)?;
            let result = store.admit_message(proposal)?;
            match args.format.resolve() {
                WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
                WorkloadOutputFormat::Table => {
                    write_channel_message_admission(&mut stdout, &result)?
                }
                WorkloadOutputFormat::Auto => {
                    unreachable!("auto output is resolved before rendering")
                }
            }
        }
        ChannelMessageCommand::List(args) => {
            let messages = store.messages()?;
            match args.format.resolve() {
                WorkloadOutputFormat::Json => write_json_line(&mut stdout, &messages)?,
                WorkloadOutputFormat::Table => write_channel_messages(&mut stdout, &messages)?,
                WorkloadOutputFormat::Auto => {
                    unreachable!("auto output is resolved before rendering")
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn channel_relay(
    store: &LocalChannelStore,
    workspace: &Path,
    args: ChannelRelayArgs,
) -> Result<ExitCode, CliError> {
    let attempt = execute_channel_relay(store, workspace, &args.message_id, &args.relay)?;
    let exit = if attempt.outcome != RelayAttemptOutcome::Failed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    };
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &attempt)?,
        WorkloadOutputFormat::Table => write_channel_relay_attempt(&mut stdout, &attempt)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(exit)
}

fn channel_beacon(
    store: &LocalChannelStore,
    workspace: &Path,
    args: ChannelBeaconArgs,
) -> Result<ExitCode, CliError> {
    let head = store.admitted_head()?;
    let beacon = head
        .snapshot
        .graph
        .beacons
        .iter()
        .find(|beacon| beacon.id == args.beacon_id)
        .cloned()
        .ok_or_else(|| CliError::UnknownBeacon(args.beacon_id.clone()))?;
    let messages = store.messages()?;
    let retained = store.relay_attempts()?;
    let mut attempts = Vec::new();
    let mut checked_messages = 0_u64;
    for message in messages.iter().take(beacon.batch_limit as usize) {
        checked_messages += 1;
        for relay_id in &beacon.relay_ids {
            if retained.iter().any(|attempt| {
                attempt.message_id == message.message_id
                    && attempt.relay_id == *relay_id
                    && attempt.outcome == RelayAttemptOutcome::Delivered
            }) {
                continue;
            }
            attempts.push(execute_channel_relay(
                store,
                workspace,
                message.message_id.as_str(),
                relay_id,
            )?);
        }
    }
    let tick = PollingBeaconTick {
        schema: POLLING_BEACON_TICK_SCHEMA.to_owned(),
        beacon_id: beacon.id,
        beacon_revision: beacon.revision,
        checked_messages,
        attempted: attempts.len() as u64,
        delivered: attempts
            .iter()
            .filter(|attempt| attempt.outcome == RelayAttemptOutcome::Delivered)
            .count() as u64,
        failed: attempts
            .iter()
            .filter(|attempt| attempt.outcome == RelayAttemptOutcome::Failed)
            .count() as u64,
        skipped: attempts
            .iter()
            .filter(|attempt| attempt.outcome == RelayAttemptOutcome::SkippedAlreadyDelivered)
            .count() as u64,
        attempts,
    };
    let exit = if tick.failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    };
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &tick)?,
        WorkloadOutputFormat::Table => write_polling_beacon_tick(&mut stdout, &tick)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(exit)
}

fn execute_channel_relay(
    store: &LocalChannelStore,
    workspace: &Path,
    message_id: &str,
    relay_id: &str,
) -> Result<RelayAttempt, CliError> {
    let head = store.admitted_head()?;
    let relay = head
        .snapshot
        .graph
        .relays
        .iter()
        .find(|relay| relay.id == relay_id)
        .ok_or_else(|| CliError::UnknownRelay(relay_id.to_owned()))?;
    let application = head
        .snapshot
        .graph
        .applications
        .iter()
        .find(|application| application.id == relay.provider_id)
        .ok_or_else(|| CliError::UnknownChannelApplication(relay.provider_id.clone()))?;
    let message = store
        .messages()?
        .into_iter()
        .find(|message| message.message_id.as_str() == message_id)
        .ok_or_else(|| CliError::UnknownChannelMessage(message_id.to_owned()))?;
    if message.proposal.channel_id != relay.source_channel_id {
        return Err(CliError::RelaySourceMismatch);
    }
    if let Some(previous) = store.relay_attempts()?.into_iter().find(|attempt| {
        attempt.message_id == message.message_id
            && attempt.relay_id == relay.id
            && attempt.outcome == RelayAttemptOutcome::Delivered
    }) {
        let skipped = RelayAttempt::new(
            head.commit_id.clone(),
            head.snapshot.graph_id.clone(),
            relay,
            application,
            previous.environment_commit_id,
            message.message_id,
            RelayAttemptOutcome::SkippedAlreadyDelivered,
            None,
            false,
            None,
            None,
            format!("delivery already retained as {}", previous.attempt_id),
        );
        store.retain_relay_attempt(skipped.clone())?;
        return Ok(skipped);
    }

    let environment_store = LocalEnvironmentStore::default_for_workspace(workspace);
    let environment = environment_store.load()?;
    let environment_head = environment.head().ok_or(CliError::NoEnvironmentHead)?;
    let capability = environment_head
        .snapshot
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == application.environment_capability_id)
        .ok_or_else(|| {
            CliError::UnadmittedChannelApplication(application.environment_capability_id.clone())
        })?;
    if capability.availability != Availability::Available
        || capability.resolved_location.as_deref() != Some(application.executable_path.as_str())
        || application
            .executable_version
            .as_deref()
            .is_some_and(|version| capability.version.as_deref() != Some(version))
        || capability.content_digest.as_deref() != Some(application.executable_digest.as_str())
    {
        return Err(CliError::ChannelApplicationDrift(application.id.clone()));
    }
    let executable = PathBuf::from(&application.executable_path);
    let metadata =
        fs::symlink_metadata(&executable).map_err(|source| CliError::RelayExecutable {
            path: executable.clone(),
            source,
        })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::RelayExecutableType(executable));
    }
    let relay_args = application
        .relay_argv
        .iter()
        .map(|argument| match argument.as_str() {
            "{target}" => OsString::from(&relay.target_channel_locator),
            "{message}" => OsString::from(message.relay_payload()),
            _ => OsString::from(argument),
        })
        .collect();
    let output = run_bounded(&CommandRequest {
        program: PathBuf::from(&application.executable_path),
        args: relay_args,
        cwd: workspace.to_owned(),
        timeout: Duration::from_millis(application.timeout_ms),
        max_capture_bytes: application.max_output_bytes,
        environment: Vec::new(),
    })?;
    let outcome = if output.status.success() && !output.timed_out && !output.overflowed {
        RelayAttemptOutcome::Delivered
    } else {
        RelayAttemptOutcome::Failed
    };
    let detail = if output.timed_out {
        "relay process exceeded its admitted timeout".to_owned()
    } else if output.overflowed {
        "relay process exceeded its admitted output limit".to_owned()
    } else {
        output.status.code().map_or_else(
            || "relay process ended without an exit code".to_owned(),
            |code| format!("relay process exited with {code}"),
        )
    };
    let attempt = RelayAttempt::new(
        head.commit_id,
        head.snapshot.graph_id,
        relay,
        application,
        environment_head.commit_id.clone(),
        message.message_id,
        outcome,
        output.status.code(),
        output.timed_out,
        relay_output_digest("stdout", &output.stdout),
        relay_output_digest("stderr", &output.stderr),
        detail,
    );
    store.retain_relay_attempt(attempt.clone())?;
    Ok(attempt)
}

fn channel_apply(
    store: &LocalChannelStore,
    workspace: &Path,
    args: ChannelApplyArgs,
) -> Result<ExitCode, CliError> {
    let input_path = workspace.join(&args.graph);
    let metadata = fs::symlink_metadata(&input_path).map_err(|source| CliError::ChannelInput {
        path: input_path.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::ChannelInputType(input_path));
    }
    let canonical = input_path
        .canonicalize()
        .map_err(|source| CliError::ChannelInput {
            path: input_path.clone(),
            source,
        })?;
    if !canonical.starts_with(workspace) {
        return Err(CliError::ChannelInputEscape(canonical));
    }
    if metadata.len() > MAX_CHANNEL_GRAPH_INPUT_BYTES {
        return Err(CliError::ChannelInputLimit(MAX_CHANNEL_GRAPH_INPUT_BYTES));
    }
    let mut bytes = Vec::new();
    File::open(&canonical)
        .map_err(|source| CliError::ChannelInput {
            path: canonical.clone(),
            source,
        })?
        .take(MAX_CHANNEL_GRAPH_INPUT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::ChannelInput {
            path: canonical.clone(),
            source,
        })?;
    if bytes.len() as u64 > MAX_CHANNEL_GRAPH_INPUT_BYTES {
        return Err(CliError::ChannelInputLimit(MAX_CHANNEL_GRAPH_INPUT_BYTES));
    }
    let graph: ChannelGraph = serde_saphyr::from_slice(&bytes)?;
    let relative = canonical
        .strip_prefix(workspace)
        .expect("workspace containment was checked");
    let relative = relative
        .to_str()
        .ok_or_else(|| CliError::ChannelInputEncoding(canonical.clone()))?;
    let locator = format!("worktree:///{relative}");
    let result = store.apply(graph, ChannelGraphSource::worktree(locator, &bytes))?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_channel_apply(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn read_workspace_channel_input(
    workspace: &Path,
    relative: &Path,
    limit: u64,
) -> Result<Vec<u8>, CliError> {
    if relative_path_escapes(relative) || relative.is_absolute() {
        return Err(CliError::ChannelInputEscape(relative.to_owned()));
    }
    let input_path = workspace.join(relative);
    let metadata = fs::symlink_metadata(&input_path).map_err(|source| CliError::ChannelInput {
        path: input_path.clone(),
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::ChannelInputType(input_path));
    }
    let canonical = input_path
        .canonicalize()
        .map_err(|source| CliError::ChannelInput {
            path: input_path,
            source,
        })?;
    if !canonical.starts_with(workspace) {
        return Err(CliError::ChannelInputEscape(canonical));
    }
    if metadata.len() > limit {
        return Err(CliError::ChannelInputLimit(limit));
    }
    let mut bytes = Vec::new();
    File::open(&canonical)
        .map_err(|source| CliError::ChannelInput {
            path: canonical.clone(),
            source,
        })?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::ChannelInput {
            path: canonical,
            source,
        })?;
    if bytes.len() as u64 > limit {
        return Err(CliError::ChannelInputLimit(limit));
    }
    Ok(bytes)
}

fn journal_command(args: JournalArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let store = match args.state_dir {
        Some(path) if path.is_absolute() => LocalJournalStore::new(path),
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => LocalJournalStore::new(workspace.join(path)),
        None => LocalJournalStore::default_for_workspace(&workspace),
    };
    match args.command {
        JournalCommand::Add(command) => journal_add(&store, &workspace, command),
        JournalCommand::List(command) => journal_list(&store, command),
    }
}

fn journal_add(
    store: &LocalJournalStore,
    workspace: &Path,
    args: JournalAddArgs,
) -> Result<ExitCode, CliError> {
    let proposal_path = workspace.join(&args.proposal);
    let metadata =
        fs::symlink_metadata(&proposal_path).map_err(|source| CliError::JournalInput {
            path: proposal_path.clone(),
            source,
        })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CliError::JournalInputType(proposal_path));
    }
    let canonical = proposal_path
        .canonicalize()
        .map_err(|source| CliError::JournalInput {
            path: proposal_path.clone(),
            source,
        })?;
    if !canonical.starts_with(workspace) {
        return Err(CliError::JournalInputEscape(canonical));
    }
    if metadata.len() > MAX_JOURNAL_PROPOSAL_BYTES {
        return Err(CliError::JournalInputLimit(MAX_JOURNAL_PROPOSAL_BYTES));
    }
    let mut bytes = Vec::new();
    File::open(&canonical)
        .map_err(|source| CliError::JournalInput {
            path: canonical.clone(),
            source,
        })?
        .take(MAX_JOURNAL_PROPOSAL_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::JournalInput {
            path: canonical.clone(),
            source,
        })?;
    if bytes.len() as u64 > MAX_JOURNAL_PROPOSAL_BYTES {
        return Err(CliError::JournalInputLimit(MAX_JOURNAL_PROPOSAL_BYTES));
    }
    let proposal: JournalEntryProposal = serde_saphyr::from_slice(&bytes)?;
    if proposal.author.kind != JournalAuthorKind::Agent {
        return Err(CliError::JournalCliAuthor);
    }
    let admitted_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let admission = store.admit(proposal, &admitted_at)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &admission)?,
        WorkloadOutputFormat::Table => write_journal_admission(&mut stdout, &admission)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn journal_list(store: &LocalJournalStore, args: JournalListArgs) -> Result<ExitCode, CliError> {
    let log = store.load()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &log)?,
        WorkloadOutputFormat::Table => write_journal_log(&mut stdout, &log)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn ui_command(args: UiArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let state_directory = match args.state_dir {
        Some(path) if path.is_absolute() => path,
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => workspace.join(path),
        None => workspace.join(".rey").join("workloads"),
    };
    let journal_directory = match args.journal_state_dir {
        Some(path) if path.is_absolute() => path,
        Some(path) if relative_path_escapes(&path) => {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => workspace.join(path),
        None => workspace.join(".rey").join("journal"),
    };
    let server = ui::UiServer::bind(ui::UiServerConfig {
        workspace,
        state_directory,
        catalog_directory: args.catalog_dir,
        journal_directory,
        host: args.host,
        port: args.port,
    })?;
    let descriptor = server.descriptor();
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &descriptor)?,
        WorkloadOutputFormat::Table => write_ui_startup(&mut stdout, &descriptor)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    stdout.flush()?;
    if !descriptor.loopback_only {
        eprintln!(
            "rey: warning: UI is listening beyond loopback with unauthenticated Journal writes and exact workload approval enabled; protect access externally"
        );
    }
    server.serve()?;
    Ok(ExitCode::SUCCESS)
}

fn relative_path_escapes(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    })
}

fn env_command(args: EnvArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let store = match args.state_dir {
        Some(path) if path.is_absolute() => LocalEnvironmentStore::new(path),
        Some(path)
            if path.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            }) =>
        {
            return Err(CliError::StateDirectoryEscape(path));
        }
        Some(path) => LocalEnvironmentStore::new(workspace.join(path)),
        None => LocalEnvironmentStore::default_for_workspace(&workspace),
    };
    match args.command {
        EnvCommand::Status(command) => env_status(&store, &workspace, command),
        EnvCommand::Add(command) => env_add(&store, &workspace, command),
        EnvCommand::Commit(command) => env_commit(&store, &workspace, command),
        EnvCommand::Log(command) => env_log(&store, &workspace, command),
        EnvCommand::Diff(command) => env_diff(&store, &workspace, command),
    }
}

fn workloads(args: WorkloadsArgs) -> Result<ExitCode, CliError> {
    let workspace = args
        .workspace
        .canonicalize()
        .map_err(|source| CliError::Workspace {
            path: args.workspace.clone(),
            source,
        })?;
    if !workspace.is_dir() {
        return Err(CliError::WorkspaceDirectory(workspace));
    }
    let store = match args.state_dir {
        Some(path) if path.is_absolute() => LocalWorkloadStore::new(path),
        Some(path) => LocalWorkloadStore::new(workspace.join(path)),
        None => LocalWorkloadStore::default_for_workspace(&workspace),
    };
    match args.command {
        WorkloadsCommand::Create(command) => {
            if args.catalog != WorkloadCatalogSelection::Workspace {
                return Err(CliError::CreateRequiresWorkspaceCatalog);
            }
            workload_create(&store, &workspace, &args.catalog_dir, command)
        }
        WorkloadsCommand::List(command) => {
            workload_list(&store, &workspace, &args.catalog_dir, args.catalog, command)
        }
        WorkloadsCommand::Status(command) => match args.catalog {
            WorkloadCatalogSelection::Workspace => {
                if command.workload_id.is_some() {
                    return Err(CliError::WorkspaceStatusIsPortfolio);
                }
                workload_revision_status(&store, &workspace, &args.catalog_dir, command)
            }
            WorkloadCatalogSelection::Conformance => {
                workload_conformance_status(&store, &workspace, command)
            }
        },
        WorkloadsCommand::Add(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_add(&store, &workspace, &args.catalog_dir, command)
        }
        WorkloadsCommand::Commit(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_commit(&store, command)
        }
        WorkloadsCommand::Log(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_log(&store, command)
        }
        WorkloadsCommand::Diff(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_diff(&store, &workspace, &args.catalog_dir, command)
        }
        WorkloadsCommand::Test(command) => {
            let catalog = match args.catalog {
                WorkloadCatalogSelection::Workspace => {
                    if !command.staged {
                        return Err(CliError::WorkspaceTestRequiresIndex);
                    }
                    store.index_catalog()?
                }
                WorkloadCatalogSelection::Conformance => WorkloadCatalog::built_in_conformance()?,
            };
            workload_test(&store, &catalog, command)
        }
        WorkloadsCommand::Run(command) => {
            let catalog = match args.catalog {
                WorkloadCatalogSelection::Workspace => store.head_catalog()?,
                WorkloadCatalogSelection::Conformance => WorkloadCatalog::built_in_conformance()?,
            };
            workload_run(&store, &workspace, &catalog, command)
        }
        WorkloadsCommand::AdmitActivation(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_admit_activation(&store, &workspace, command)
        }
        WorkloadsCommand::ExecuteActivation(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_execute_activation(&store, &workspace, command)
        }
        WorkloadsCommand::VerifyActivation(command) => {
            require_workspace_admission_catalog(args.catalog)?;
            workload_verify_activation(&store, &workspace, command)
        }
    }
}

fn require_workspace_admission_catalog(catalog: WorkloadCatalogSelection) -> Result<(), CliError> {
    if catalog == WorkloadCatalogSelection::Workspace {
        Ok(())
    } else {
        Err(CliError::AdmissionRequiresWorkspaceCatalog)
    }
}

fn workload_create(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    args: WorkloadCreateArgs,
) -> Result<ExitCode, CliError> {
    let attention_binding = args
        .attention_row
        .as_deref()
        .map(|row_id| {
            let catalog = store.head_catalog()?;
            let state = store.load()?;
            let environment = retained_environment_snapshot(workspace)?;
            let git = retained_git_snapshot(workspace)?;
            let snapshot = derive_portfolio_snapshot(
                &catalog.definitions(),
                &state,
                environment.as_ref(),
                git.as_ref(),
            )?;
            let attention = WorkloadAttention::derive(&snapshot)?;
            let runtime = orient_portfolio_attention(&snapshot, &attention)?;
            WorkloadCreationAttentionBinding::from_runtime(&snapshot, &attention, &runtime, row_id)
                .map_err(CliError::from)
        })
        .transpose()?;
    let result = WorkloadCatalog::create_workspace_request(
        workspace,
        catalog_dir,
        &args.workload_id,
        args.title.as_deref(),
        args.intent.as_deref(),
        attention_binding,
    )?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_workload_create(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_admit_activation(
    store: &LocalWorkloadStore,
    workspace: &Path,
    args: WorkloadAdmitActivationArgs,
) -> Result<ExitCode, CliError> {
    let git_state = LocalGitStore::default_for_workspace(workspace).load()?;
    let activation = git_state.acknowledged_activation(&args.activation_id)?;
    let cursor = git_state
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    if cursor.snapshot_id != activation.target_snapshot_id
        || cursor.retained_evidence_id != activation.transition_id
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "activation target is not the current acknowledged Git cursor".to_owned(),
        )
        .into());
    }
    let workload_state = store.load()?;
    let workload_head = workload_state.commits.last().ok_or_else(|| {
        LocalWorkloadStateError::ActivationPrecondition(
            "workload HEAD is empty; activation cannot bind an unadmitted package".to_owned(),
        )
    })?;
    let catalog = store.head_catalog()?;
    let resolved = catalog
        .select(Some(&activation.workload_id))?
        .into_iter()
        .next()
        .ok_or(CliError::EmptyWorkloadCatalog)?;
    let environment =
        retained_environment_snapshot(workspace)?.ok_or(CliError::ActivationEnvironmentRequired)?;
    let admission = WorkloadActivationAdmission::new(
        activation,
        workload_head,
        &resolved.definition,
        environment.semantic_digest,
    )?;
    let admission = store.admit_activation(admission)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &admission)?,
        WorkloadOutputFormat::Table => {
            write_workload_activation_admission(&mut stdout, &admission)?
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_execute_activation(
    store: &LocalWorkloadStore,
    workspace: &Path,
    args: WorkloadExecuteActivationArgs,
) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let admission = state
        .activation_admissions
        .iter()
        .find(|admission| admission.admission_id.as_str() == args.admission_id)
        .cloned()
        .ok_or_else(|| {
            LocalWorkloadStateError::ActivationPrecondition(format!(
                "unknown retained activation admission {}",
                args.admission_id
            ))
        })?;
    if let Some(execution) = state
        .activation_executions
        .iter()
        .find(|execution| execution.admission_id == admission.admission_id)
    {
        write_workload_activation_execution_output(
            execution,
            &admission,
            true,
            args.format.resolve(),
        )?;
        return Ok(workload_activation_execution_exit(execution));
    }

    let git_state = LocalGitStore::default_for_workspace(workspace).load()?;
    let activation =
        git_state.acknowledged_activation(admission.activation.activation_id.as_str())?;
    let cursor = git_state
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    if activation != admission.activation
        || cursor.snapshot_id != admission.activation.target_snapshot_id
        || cursor.retained_evidence_id != admission.activation.transition_id
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "activation Git evidence is no longer the exact current acknowledged cursor".to_owned(),
        )
        .into());
    }

    let workload_head = state.commits.last().ok_or_else(|| {
        LocalWorkloadStateError::ActivationPrecondition(
            "workload HEAD is empty; activation execution cannot proceed".to_owned(),
        )
    })?;
    if workload_head.commit_id != admission.workload_head_commit_id
        || workload_head.snapshot.snapshot_revision != admission.workload_head_snapshot_id
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "workload HEAD changed after activation admission".to_owned(),
        )
        .into());
    }
    let catalog = store.head_catalog()?;
    let workload = catalog
        .select(Some(&admission.workload.id))?
        .into_iter()
        .next()
        .ok_or(CliError::EmptyWorkloadCatalog)?
        .definition;
    let declared_scenarios = workload
        .scenario_suite
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario.clone())
        .collect::<Vec<_>>();
    if workload.workload != admission.workload
        || workload.graph.graph != admission.graph
        || workload.scenario_suite.suite != admission.scenario_suite
        || workload.evaluator != admission.evaluator
        || declared_scenarios != admission.declared_scenarios
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "activation admission no longer matches the exact workload package".to_owned(),
        )
        .into());
    }
    let environment =
        retained_environment_snapshot(workspace)?.ok_or(CliError::ActivationEnvironmentRequired)?;
    if environment.semantic_digest != admission.capability_snapshot_id {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "retained capability snapshot changed after activation admission".to_owned(),
        )
        .into());
    }

    let mut coalesced = None;
    for source in &state.activation_executions {
        let source_admission = state
            .activation_admissions
            .iter()
            .find(|candidate| candidate.admission_id == source.admission_id)
            .ok_or(LocalWorkloadStateError::InvalidActivationExecution)?;
        if let Some(execution) =
            WorkloadActivationExecution::coalesce(&admission, source_admission, source)?
        {
            coalesced = Some(execution);
            break;
        }
    }
    let execution = if let Some(execution) = coalesced {
        execution
    } else {
        let result = execute_workload_scenario_selection_with_snapshot(
            &workload,
            &admission.selected_scenario_ids,
            environment.semantic_digest,
        )?;
        WorkloadActivationExecution::new(&admission, &workload, result)?
    };
    let execution = store.retain_activation_execution(execution)?;
    write_workload_activation_execution_output(
        &execution,
        &admission,
        false,
        args.format.resolve(),
    )?;
    Ok(workload_activation_execution_exit(&execution))
}

fn write_workload_activation_execution_output(
    execution: &WorkloadActivationExecution,
    admission: &WorkloadActivationAdmission,
    replayed: bool,
    format: WorkloadOutputFormat,
) -> Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    match format {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, execution)?,
        WorkloadOutputFormat::Table => {
            write_workload_activation_execution(&mut stdout, execution, admission, replayed)?
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(())
}

fn workload_activation_execution_exit(execution: &WorkloadActivationExecution) -> ExitCode {
    match execution.result.status {
        TestStatus::Passed => ExitCode::SUCCESS,
        TestStatus::Failed => ExitCode::from(2),
        TestStatus::Inconclusive => ExitCode::from(3),
    }
}

fn workload_verify_activation(
    store: &LocalWorkloadStore,
    workspace: &Path,
    args: WorkloadVerifyActivationArgs,
) -> Result<ExitCode, CliError> {
    if args.max_evidence_bytes == 0
        || args.max_evidence_bytes > MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES
    {
        return Err(CliError::InvalidRecomputationLimit {
            actual: args.max_evidence_bytes,
            max: MAX_WORKLOAD_RECOMPUTATION_EVIDENCE_BYTES,
        });
    }
    let state = store.load()?;
    let execution = state
        .activation_executions
        .iter()
        .find(|execution| execution.execution_id.as_str() == args.execution_id)
        .cloned()
        .ok_or_else(|| {
            LocalWorkloadStateError::ActivationPrecondition(format!(
                "unknown retained activation execution {}",
                args.execution_id
            ))
        })?;
    let admission = state
        .activation_admissions
        .iter()
        .find(|admission| admission.admission_id == execution.admission_id)
        .cloned()
        .ok_or(LocalWorkloadStateError::InvalidActivationExecution)?;
    if let Some(recomputation) = state
        .activation_recomputations
        .iter()
        .find(|recomputation| recomputation.execution_id == execution.execution_id)
    {
        write_workload_activation_recomputation_output(
            recomputation,
            &execution,
            &admission,
            true,
            args.format.resolve(),
        )?;
        return Ok(workload_activation_recomputation_exit(recomputation));
    }

    let git_state = LocalGitStore::default_for_workspace(workspace).load()?;
    let activation =
        git_state.acknowledged_activation(admission.activation.activation_id.as_str())?;
    let cursor = git_state
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    if activation != admission.activation
        || cursor.snapshot_id != admission.activation.target_snapshot_id
        || cursor.retained_evidence_id != admission.activation.transition_id
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "activation Git evidence is no longer the exact current acknowledged cursor".to_owned(),
        )
        .into());
    }

    let workload_head = state.commits.last().ok_or_else(|| {
        LocalWorkloadStateError::ActivationPrecondition(
            "workload HEAD is empty; full activation recomputation cannot proceed".to_owned(),
        )
    })?;
    if workload_head.commit_id != admission.workload_head_commit_id
        || workload_head.snapshot.snapshot_revision != admission.workload_head_snapshot_id
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "workload HEAD changed after activation admission".to_owned(),
        )
        .into());
    }
    let catalog = store.head_catalog()?;
    let workload = catalog
        .select(Some(&admission.workload.id))?
        .into_iter()
        .next()
        .ok_or(CliError::EmptyWorkloadCatalog)?
        .definition;
    let declared_scenarios = workload
        .scenario_suite
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario.clone())
        .collect::<Vec<_>>();
    if workload.workload != admission.workload
        || workload.graph.graph != admission.graph
        || workload.scenario_suite.suite != admission.scenario_suite
        || workload.evaluator != admission.evaluator
        || declared_scenarios != admission.declared_scenarios
    {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "activation admission no longer matches the exact workload package".to_owned(),
        )
        .into());
    }
    let environment =
        retained_environment_snapshot(workspace)?.ok_or(CliError::ActivationEnvironmentRequired)?;
    if environment.semantic_digest != admission.capability_snapshot_id {
        return Err(LocalWorkloadStateError::ActivationPrecondition(
            "retained capability snapshot changed after activation admission".to_owned(),
        )
        .into());
    }

    let all_scenario_ids = admission
        .declared_scenarios
        .iter()
        .map(|scenario| scenario.id.clone())
        .collect::<Vec<_>>();
    let full_result = execute_workload_scenario_selection_with_snapshot(
        &workload,
        &all_scenario_ids,
        environment.semantic_digest,
    )?;
    let recomputation = WorkloadActivationRecomputation::new(
        &execution,
        &admission,
        full_result,
        args.max_evidence_bytes,
    )?;
    let recomputation = store.retain_activation_recomputation(recomputation)?;
    write_workload_activation_recomputation_output(
        &recomputation,
        &execution,
        &admission,
        false,
        args.format.resolve(),
    )?;
    Ok(workload_activation_recomputation_exit(&recomputation))
}

fn write_workload_activation_recomputation_output(
    recomputation: &WorkloadActivationRecomputation,
    execution: &WorkloadActivationExecution,
    admission: &WorkloadActivationAdmission,
    replayed: bool,
    format: WorkloadOutputFormat,
) -> Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    match format {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, recomputation)?,
        WorkloadOutputFormat::Table => write_workload_activation_recomputation(
            &mut stdout,
            recomputation,
            execution,
            admission,
            replayed,
        )?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(())
}

fn workload_activation_recomputation_exit(
    recomputation: &WorkloadActivationRecomputation,
) -> ExitCode {
    match recomputation.assessment {
        WorkloadRecomputationAssessment::Equivalent => ExitCode::SUCCESS,
        WorkloadRecomputationAssessment::Different => ExitCode::from(2),
    }
}

fn current_workload_list(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
) -> Result<WorkloadList, CliError> {
    let mut catalog = store.head_catalog()?;
    let revision = store.status(workspace, catalog_dir)?;
    catalog.descriptor.workload_count = catalog
        .workloads
        .len()
        .max(revision.working.packages.len())
        .saturating_add(revision.drafts.len()) as u64;
    catalog.descriptor.draft_count = revision.drafts.len() as u64;
    let state = store.load()?;
    let summaries = catalog
        .workloads
        .iter()
        .map(|workload| {
            WorkloadSummary::derive_resolved(
                workload,
                state.record(&workload.definition.workload.id),
            )
        })
        .collect();
    let definitions = catalog.definitions();
    let environment = retained_environment_snapshot(workspace)?;
    let git = retained_git_snapshot(workspace)?;
    let snapshot =
        derive_portfolio_snapshot(&definitions, &state, environment.as_ref(), git.as_ref())?;
    let attention = WorkloadAttention::derive(&snapshot)?;
    let runtime = environment
        .as_ref()
        .map(|_| orient_portfolio_attention(&snapshot, &attention))
        .transpose()?;
    let list = WorkloadList::new(
        catalog.descriptor.clone(),
        summaries,
        revision.drafts.clone(),
        state.activation_admissions.clone(),
        state.activation_executions.clone(),
        attention,
        runtime,
    )
    .with_activation_recomputations(state.activation_recomputations.clone())
    .with_revision(revision);
    Ok(list)
}

fn workload_list(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    selection: WorkloadCatalogSelection,
    args: WorkloadOutputArgs,
) -> Result<ExitCode, CliError> {
    let list = match selection {
        WorkloadCatalogSelection::Workspace => {
            current_workload_list(store, workspace, catalog_dir)?
        }
        WorkloadCatalogSelection::Conformance => {
            let catalog = WorkloadCatalog::built_in_conformance()?;
            let state = store.load()?;
            let summaries = catalog
                .workloads
                .iter()
                .map(|workload| {
                    WorkloadSummary::derive_resolved(
                        workload,
                        state.record(&workload.definition.workload.id),
                    )
                })
                .collect();
            let definitions = catalog.definitions();
            let environment = retained_environment_snapshot(workspace)?;
            let git = retained_git_snapshot(workspace)?;
            let snapshot = derive_portfolio_snapshot(
                &definitions,
                &state,
                environment.as_ref(),
                git.as_ref(),
            )?;
            let attention = WorkloadAttention::derive(&snapshot)?;
            let runtime = environment
                .as_ref()
                .map(|_| orient_portfolio_attention(&snapshot, &attention))
                .transpose()?;
            WorkloadList::new(
                catalog.descriptor.clone(),
                summaries,
                catalog.drafts.clone(),
                Vec::new(),
                Vec::new(),
                attention,
                runtime,
            )
        }
    };
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &list)?,
        WorkloadOutputFormat::Table => {
            write_workload_list(&mut stdout, &list, TerminalStyle::stdout())?;
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_revision_status(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    args: WorkloadStatusArgs,
) -> Result<ExitCode, CliError> {
    let status = store.status(workspace, catalog_dir)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        WorkloadOutputFormat::Table => {
            write_workload_revision_status(&mut stdout, &status, TerminalStyle::stdout())?
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_conformance_status(
    store: &LocalWorkloadStore,
    workspace: &Path,
    args: WorkloadStatusArgs,
) -> Result<ExitCode, CliError> {
    let catalog = WorkloadCatalog::built_in_conformance()?;
    let state = store.load()?;
    let selected = catalog.select(args.workload_id.as_deref())?;
    let statuses = selected
        .into_iter()
        .map(|workload| {
            let record = state.record(&workload.definition.workload.id);
            WorkloadStatusView::new_resolved(workload, record)
        })
        .collect();
    let definitions = catalog.definitions();
    let environment = retained_environment_snapshot(workspace)?;
    let git = retained_git_snapshot(workspace)?;
    let snapshot =
        derive_portfolio_snapshot(&definitions, &state, environment.as_ref(), git.as_ref())?;
    let attention = WorkloadAttention::derive(&snapshot)?;
    let runtime = environment
        .as_ref()
        .map(|_| orient_portfolio_attention(&snapshot, &attention))
        .transpose()?;
    let batch = WorkloadStatusBatch::new(
        catalog.descriptor.clone(),
        statuses,
        Vec::new(),
        attention,
        runtime,
    );
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &batch)?,
        WorkloadOutputFormat::Table => write_workload_status(&mut stdout, &batch)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_add(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    args: WorkloadOutputArgs,
) -> Result<ExitCode, CliError> {
    let result = store.add(workspace, catalog_dir)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_workload_add(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_commit(
    store: &LocalWorkloadStore,
    args: WorkloadCommitArgs,
) -> Result<ExitCode, CliError> {
    let result = store.commit(args.message, None, None)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_workload_commit(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_log(store: &LocalWorkloadStore, args: WorkloadLogArgs) -> Result<ExitCode, CliError> {
    let log = store.log(args.max_count, args.patch)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &log)?,
        WorkloadOutputFormat::Table => write_workload_log(&mut stdout, &log)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_diff(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    args: WorkloadDiffArgs,
) -> Result<ExitCode, CliError> {
    let diff = store.diff(workspace, catalog_dir, args.staged)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        WorkloadOutputFormat::Table => write_workload_diff(&mut stdout, &diff)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_test(
    store: &LocalWorkloadStore,
    catalog: &WorkloadCatalog,
    args: WorkloadTestArgs,
) -> Result<ExitCode, CliError> {
    let mut state = store.load()?;
    let selected = catalog.select(args.workload_id.as_deref())?;
    if selected.is_empty() {
        return Err(CliError::EmptyWorkloadCatalog);
    }
    let definitions = selected
        .iter()
        .map(|workload| workload.definition.clone())
        .collect::<Vec<_>>();
    let capability_snapshot_id = workload_test_capability_snapshot(&definitions)?;
    let mut results = Vec::with_capacity(definitions.len());
    match args.format.resolve() {
        WorkloadOutputFormat::Json => {
            for resolved in &selected {
                let workload = &resolved.definition;
                let result = test_workload_with_observer_and_snapshot(
                    workload,
                    capability_snapshot_id.clone(),
                    |_| {},
                )?;
                state.retain_test(result.clone());
                results.push(result);
            }
        }
        WorkloadOutputFormat::Table => {
            let style = TerminalStyle::stdout();
            let mut stdout = io::stdout().lock();
            write_workload_test_plan(
                &mut stdout,
                &selected,
                &catalog.descriptor,
                args.workload_id.as_deref(),
                style,
            )?;
            for resolved in &selected {
                let workload = &resolved.definition;
                write_workload_test_start(&mut stdout, resolved, args.verbose, style)?;
                let scenario_total = workload.scenario_suite.scenarios.len();
                let mut scenario_index = 0;
                let mut render_error = None;
                let result = test_workload_with_observer_and_snapshot(
                    workload,
                    capability_snapshot_id.clone(),
                    |scenario| {
                        scenario_index += 1;
                        if render_error.is_none() {
                            render_error = write_workload_test_scenario(
                                &mut stdout,
                                workload,
                                scenario,
                                scenario_index,
                                scenario_total,
                                args.verbose,
                                style,
                            )
                            .and_then(|()| stdout.flush())
                            .err();
                        }
                    },
                );
                if let Some(error) = render_error {
                    return Err(CliError::Output(error));
                }
                let result = result?;
                write_workload_test_result(&mut stdout, &result, args.verbose, style)?;
                state.retain_test(result.clone());
                results.push(result);
            }
            state.refresh_index_qualification(&catalog.definitions());
            state.verify()?;
            store.save(&state)?;
            let batch = WorkloadTestBatch::new(
                catalog.descriptor.clone(),
                selected
                    .iter()
                    .map(|workload| workload.provenance.clone())
                    .collect(),
                results,
            );
            let exit_code = test_batch_exit(&batch);
            write_workload_test_summary(&mut stdout, &batch, style)?;
            return Ok(exit_code);
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    state.refresh_index_qualification(&catalog.definitions());
    state.verify()?;
    store.save(&state)?;
    let batch = WorkloadTestBatch::new(
        catalog.descriptor.clone(),
        selected
            .iter()
            .map(|workload| workload.provenance.clone())
            .collect(),
        results,
    );
    let exit_code = test_batch_exit(&batch);
    write_json_line(&mut io::stdout().lock(), &batch)?;
    Ok(exit_code)
}

fn workload_test_capability_snapshot(
    definitions: &[WorkloadDefinition],
) -> Result<SemanticDigest, CliError> {
    Ok(
        if definitions
            .iter()
            .any(|workload| workload.workload.id == BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID)
        {
            inspect_environment(&source_fixture_root(), DiscoveryLimits::default())?.semantic_digest
        } else if definitions
            .iter()
            .any(|workload| workload.workload.id == CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID)
        {
            SemanticHasher::new("rey.fixture.topography-capability-snapshot.v1").finish()
        } else {
            SemanticHasher::new("rey.no-mining-capability-snapshot.v1").finish()
        },
    )
}

fn admit_workload_files(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog_dir: &Path,
    message: String,
    expected_head: &str,
    expected_working: &str,
) -> Result<WorkloadCommitResult, CliError> {
    let status = store.status(workspace, catalog_dir)?;
    let head_matches = if expected_head == "EMPTY" {
        status.head.is_none()
    } else {
        status.head.as_ref().map(|commit| commit.commit_id.as_str()) == Some(expected_head)
    };
    if !head_matches {
        return Err(LocalWorkloadStateError::ApprovalPrecondition(
            "HEAD changed before admission".to_owned(),
        )
        .into());
    }
    if status.working.snapshot_revision.as_str() != expected_working {
        return Err(LocalWorkloadStateError::ApprovalPrecondition(
            "WORKING file snapshot changed before admission".to_owned(),
        )
        .into());
    }
    let added = store.add_expected(workspace, catalog_dir, Some(expected_working))?;
    let catalog = store.index_catalog()?;
    let definitions = catalog.definitions();
    if definitions.is_empty() {
        return Err(CliError::EmptyWorkloadCatalog);
    }
    let capability_snapshot_id = workload_test_capability_snapshot(&definitions)?;
    let mut results = Vec::with_capacity(definitions.len());
    for workload in &definitions {
        let result = test_workload_with_observer_and_snapshot(
            workload,
            capability_snapshot_id.clone(),
            |_| {},
        )?;
        results.push(result);
    }
    store.retain_index_tests(&added.snapshot.snapshot_revision, &definitions, results)?;
    Ok(store.commit(
        message,
        Some(expected_head),
        Some(added.snapshot.snapshot_revision.as_str()),
    )?)
}

fn workload_run(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog: &WorkloadCatalog,
    args: WorkloadRunArgs,
) -> Result<ExitCode, CliError> {
    let resolved = catalog
        .select(Some(&args.workload_id))?
        .into_iter()
        .next()
        .ok_or(CliError::EmptyWorkloadCatalog)?;
    let workload = resolved.definition;
    let mut state = store.load()?;
    let mut inputs = BTreeMap::new();
    if workload.workload.id == BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID {
        if args.input.is_some()
            || !args.sources.is_empty()
            || args.context_before != 0
            || args.context_after != 0
        {
            return Err(CliError::UnexpectedPortfolioInput);
        }
        let definitions = catalog.definitions();
        let environment = retained_environment_snapshot(workspace)?;
        let git = retained_git_snapshot(workspace)?;
        let snapshot =
            derive_portfolio_snapshot(&definitions, &state, environment.as_ref(), git.as_ref())?;
        inputs.insert(
            "portfolio".to_owned(),
            WorkloadValue::PortfolioSnapshot(Box::new(snapshot)),
        );
    } else if workload.workload.id == CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID {
        if args.input.is_some() {
            return Err(CliError::UnexpectedTopographyInput);
        }
        if args.sources.is_empty() {
            return Err(CliError::MissingTopographySeeds);
        }
        inputs.insert(
            "text".to_owned(),
            WorkloadValue::Utf8(
                args.sources
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
    } else {
        inputs.insert(
            "text".to_owned(),
            WorkloadValue::Utf8(args.input.ok_or(CliError::MissingWorkloadInput)?),
        );
    }
    let result = match fresh_qualification(&workload, state.record(&workload.workload.id)) {
        Some(qualification) if workload.workload.id == BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID => {
            if args.sources.is_empty() {
                return Err(CliError::MissingSourceFiles);
            }
            if args.max_matches == 0 {
                return Err(CliError::InvalidLimit);
            }
            let snapshot = inspect_environment(workspace, DiscoveryLimits::default())?;
            let mining_limits = MiningLimits {
                max_matches: args.max_matches,
                max_rows: args.max_matches,
                ..MiningLimits::default()
            };
            let source = SourceRunInput {
                root: workspace.to_owned(),
                relative_paths: args.sources,
                context_before: args.context_before,
                context_after: args.context_after,
                binding_limits: SourceBindingLimits::default(),
                mining_limits,
                capability_snapshot_id: snapshot.semantic_digest,
            };
            run_workload_with_source(&workload, qualification, inputs, &source)?
        }
        Some(qualification) if workload.workload.id == CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID => {
            if args.context_before != 0 || args.context_after != 0 {
                return Err(CliError::UnexpectedTopographyContext);
            }
            let snapshot = inspect_environment(workspace, DiscoveryLimits::default())?;
            let prior = state
                .record(&workload.workload.id)
                .and_then(|record| record.last_run.as_ref())
                .and_then(|run| run.topography.first())
                .cloned();
            let survey = TopographySurveyInput {
                root: workspace.to_owned(),
                relative_paths: args.sources,
                capability_snapshot_id: snapshot.semantic_digest,
                limits: TopographyLimits::default(),
                resolution_limits: ResolutionLimits::default(),
                prior,
            };
            run_workload_with_topography(&workload, qualification, inputs, &survey)?
        }
        Some(qualification) => {
            if !args.sources.is_empty() || args.context_before != 0 || args.context_after != 0 {
                return Err(CliError::UnexpectedSourceFiles);
            }
            run_workload(&workload, qualification, inputs)?
        }
        None => WorkloadRunResult::blocked(&workload, inputs),
    };
    let exit_code = match result.status {
        RunStatus::Passed => ExitCode::SUCCESS,
        RunStatus::Blocked => ExitCode::from(3),
    };
    state.retain_run(result.clone());
    state.verify()?;
    store.save(&state)?;
    let mut stdout = io::stdout().lock();
    let view = WorkloadRunView::new(catalog.descriptor.clone(), resolved.provenance, result);
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &view)?,
        WorkloadOutputFormat::Table => write_workload_run(&mut stdout, &view)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(exit_code)
}

fn retained_environment_snapshot(workspace: &Path) -> Result<Option<CapabilitySnapshot>, CliError> {
    let store = LocalEnvironmentStore::default_for_workspace(workspace);
    let history = store.load()?;
    let index = store.load_index(&history)?;
    Ok(index
        .map(|index| index.snapshot)
        .or_else(|| history.head().map(|commit| commit.snapshot.clone())))
}

fn retained_git_snapshot(workspace: &Path) -> Result<Option<rey_git::GitSnapshot>, CliError> {
    Ok(LocalGitStore::default_for_workspace(workspace)
        .load()?
        .cursor_snapshot)
}

fn test_batch_exit(batch: &WorkloadTestBatch) -> ExitCode {
    if batch
        .results
        .iter()
        .any(|result| result.status == TestStatus::Failed)
    {
        ExitCode::from(2)
    } else if batch
        .results
        .iter()
        .any(|result| result.status == TestStatus::Inconclusive)
    {
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    }
}

#[derive(Clone, Copy, Debug)]
struct TerminalStyle {
    enabled: bool,
}

impl TerminalStyle {
    fn stdout() -> Self {
        Self {
            enabled: io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
                && std::env::var_os("TERM").is_none_or(|term| term != "dumb"),
        }
    }

    fn paint(self, code: &str, value: &str) -> String {
        if self.enabled {
            format!("\u{1b}[{code}m{value}\u{1b}[0m")
        } else {
            value.to_owned()
        }
    }

    fn bold(self, value: &str) -> String {
        self.paint("1", value)
    }

    fn cyan_bold(self, value: &str) -> String {
        self.paint("1;36", value)
    }

    fn green(self, value: &str) -> String {
        self.paint("32", value)
    }

    fn yellow(self, value: &str) -> String {
        self.paint("33", value)
    }

    fn red(self, value: &str) -> String {
        self.paint("31", value)
    }

    fn dim(self, value: &str) -> String {
        self.paint("2", value)
    }

    fn admission_change(self, value: &str, staged: bool) -> String {
        if staged {
            self.green(value)
        } else {
            self.red(value)
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WorkloadPortfolioSummary {
    total: u64,
    drafts: u64,
    tested: u64,
    untested: u64,
    qualified: u64,
    failing: u64,
    inconclusive: u64,
    stale_workloads: u64,
    required_scenarios: u64,
    passed_scenarios: u64,
    evaluated_scenarios: u64,
    stale_scenarios: u64,
    optional_scenarios: u64,
    passed_runs: u64,
    blocked_runs: u64,
    unrun: u64,
}

impl WorkloadPortfolioSummary {
    fn derive(workloads: &[WorkloadSummary], drafts: usize) -> Self {
        let mut summary = Self {
            total: workloads.len().saturating_add(drafts) as u64,
            drafts: drafts as u64,
            ..Self::default()
        };
        for workload in workloads {
            match workload.qualification {
                rey::workloads::QualificationState::Untested => summary.untested += 1,
                rey::workloads::QualificationState::Qualified => {
                    summary.tested += 1;
                    summary.qualified += 1;
                }
                rey::workloads::QualificationState::Failing => {
                    summary.tested += 1;
                    summary.failing += 1;
                }
                rey::workloads::QualificationState::Inconclusive => {
                    summary.tested += 1;
                    summary.inconclusive += 1;
                }
                rey::workloads::QualificationState::Stale => {
                    summary.tested += 1;
                    summary.stale_workloads += 1;
                }
            }
            summary.required_scenarios += workload.required;
            summary.passed_scenarios += workload.passed;
            summary.evaluated_scenarios += workload.evaluated;
            summary.stale_scenarios += workload.stale;
            summary.optional_scenarios += workload.optional;
            match workload.last_run_status {
                Some(RunStatus::Passed) => summary.passed_runs += 1,
                Some(RunStatus::Blocked) => summary.blocked_runs += 1,
                None => summary.unrun += 1,
            }
        }
        summary
    }
}

fn write_ui_startup(
    output: &mut impl Write,
    descriptor: &ui::UiServerDescriptor,
) -> Result<(), CliError> {
    let style = TerminalStyle::stdout();
    writeln!(output)?;
    writeln!(output, "{}", style.bold("REY UI"))?;
    write_portfolio_field(output, "Status", &style.green("LISTENING"))?;
    write_portfolio_field(output, "Address", &descriptor.address)?;
    write_portfolio_field(output, "URL", &descriptor.url)?;
    write_portfolio_field(
        output,
        "Exposure",
        &if descriptor.loopback_only {
            style.green("LOOPBACK ONLY")
        } else {
            style.yellow("NETWORK EXPOSED · NO AUTHENTICATION")
        },
    )?;
    write_portfolio_field(output, "Application", "TANSTACK ROUTER · EMBEDDED")?;
    write_portfolio_field(output, "Grammar", "HIFI KINETIC · PRECISION")?;
    write_portfolio_field(
        output,
        "Data plane",
        "LIVE READS · JOURNAL WRITE · WORKLOAD APPROVAL",
    )?;
    write_portfolio_field(output, "Human entry", &descriptor.entry_route)?;
    write_portfolio_field(
        output,
        "Workload admission",
        "ENABLED · EXACT WORKING FILES → QUALIFIED INDEX → HEAD",
    )?;
    write_portfolio_field(
        output,
        "Revalidation",
        &format!(
            "{}ms · PASSIVE · NO REFRESH CONTROL",
            descriptor.live_refresh_interval_ms
        ),
    )?;
    write_portfolio_field(output, "Workspace", &descriptor.workspace)?;
    write_portfolio_field(output, "Catalog", &descriptor.catalog_root)?;
    write_portfolio_field(
        output,
        "API",
        "/api/v1/health · /api/v1/cadence · /api/v1/environment · /api/v1/journal · /api/v1/workloads · /api/v1/workloads/admit",
    )?;
    write_portfolio_field(output, "Grammar revision", &descriptor.grammar_revision)?;
    write_portfolio_field(
        output,
        "Implementation",
        &format!(
            "{} · {}",
            descriptor.source_repository.as_deref().unwrap_or("UNBOUND"),
            descriptor.implementation_revision
        ),
    )?;
    writeln!(output)?;
    writeln!(output, "  {}", style.dim("Press Ctrl-C to stop the server"))?;
    Ok(())
}

fn write_channel_list(output: &mut impl Write, status: &ChannelStatus) -> Result<(), CliError> {
    let snapshot = &status.working;
    let graph = &snapshot.graph;
    writeln!(output)?;
    writeln!(output, "CHANNEL GRAPH")?;
    write_portfolio_field(
        output,
        "Source",
        &format!(
            "{} · {}",
            snapshot.source.kind.label(),
            snapshot.source.locator
        ),
    )?;
    write_portfolio_field(output, "Snapshot", snapshot.snapshot_id.as_str())?;
    write_portfolio_field(output, "Graph", snapshot.graph_id.as_str())?;
    write_portfolio_field(
        output,
        "Inventory",
        &format!(
            "{} · {} · {} · {} · {} · {}",
            count_noun(graph.channels.len(), "channel"),
            count_noun(graph.subscriptions.len(), "subscription"),
            count_noun(graph.streams.len(), "stream"),
            count_noun(graph.applications.len(), "application"),
            count_noun(graph.relays.len(), "relay"),
            count_noun(graph.beacons.len(), "beacon")
        ),
    )?;

    writeln!(output)?;
    writeln!(output, "01 / CHANNELS")?;
    for channel in &graph.channels {
        writeln!(
            output,
            "  {}@{}  {}",
            channel.id, channel.revision, channel.name
        )?;
        writeln!(
            output,
            "    Scope                  {}{}",
            channel.scope.label(),
            if channel.broadcast_default {
                " · default broadcast"
            } else {
                ""
            }
        )?;
        writeln!(
            output,
            "    Observations           {}",
            channel
                .accepted_observation_kinds
                .iter()
                .map(|kind| kind.label())
                .collect::<Vec<_>>()
                .join(" · ")
        )?;
    }

    writeln!(output)?;
    writeln!(output, "02 / SUBSCRIPTIONS")?;
    for subscription in &graph.subscriptions {
        writeln!(
            output,
            "  {}@{}  channels {} · {} kinds · limit {}",
            subscription.id,
            subscription.revision,
            subscription.channel_ids.join(", "),
            subscription.observation_kinds.len(),
            subscription.limit
        )?;
    }

    writeln!(output)?;
    writeln!(output, "03 / FEED STREAMS")?;
    for (position, stream_id) in graph.layout.stream_ids.iter().enumerate() {
        let stream = graph
            .stream(stream_id)
            .expect("validated layout references exact streams");
        writeln!(
            output,
            "  {:02}  {}  {}@{} · lens {} · subscription {}",
            position + 1,
            stream.name,
            stream.id,
            stream.revision,
            stream.lens,
            stream.subscription_id
        )?;
    }
    writeln!(
        output,
        "  Layout                 {}@{} · {}",
        graph.layout.id,
        graph.layout.revision,
        graph.layout.stream_ids.join(" → ")
    )?;

    writeln!(output)?;
    writeln!(output, "04 / APPLICATIONS")?;
    if graph.applications.is_empty() {
        writeln!(output, "  none · no admitted communications application")?;
    } else {
        for application in &graph.applications {
            writeln!(
                output,
                "  {}@{}  {} · {} · timeout {}ms",
                application.id,
                application.revision,
                application.environment_capability_id,
                application.executable_path,
                application.timeout_ms,
            )?;
        }
    }

    writeln!(output)?;
    writeln!(output, "05 / RELAYS")?;
    if graph.relays.is_empty() {
        writeln!(output, "  none · transport not configured")?;
    } else {
        for relay in &graph.relays {
            writeln!(
                output,
                "  {}@{}  {} → {} · {} · {} hops",
                relay.id,
                relay.revision,
                relay.source_channel_id,
                relay.target_channel_locator,
                relay.provider_id,
                relay.hop_limit
            )?;
        }
    }
    writeln!(output)?;
    writeln!(output, "06 / POLLING BEACONS")?;
    if graph.beacons.is_empty() {
        writeln!(output, "  none · no relay polling configured")?;
    } else {
        for beacon in &graph.beacons {
            writeln!(
                output,
                "  {}@{}  {} · every {}s · batch {} · relays [{}]",
                beacon.id,
                beacon.revision,
                beacon.application_id,
                beacon.interval_seconds,
                beacon.batch_limit,
                beacon.relay_ids.join(", "),
            )?;
        }
    }
    Ok(())
}

fn write_channel_status(output: &mut impl Write, status: &ChannelStatus) -> Result<(), CliError> {
    let head = status.head_commit.as_ref().map_or_else(
        || "built-in (no commits yet)".to_owned(),
        |commit| format!("CHANNEL@{}", commit.sequence),
    );
    writeln!(output, "On channels {head}")?;
    write_channel_status_changes(
        output,
        &status.staged.changes,
        "Changes to be committed:",
        "  (use \"rey channels diff --staged\" to review)",
        TerminalStyle::stdout(),
        true,
    )?;
    write_channel_status_changes(
        output,
        &status.unstaged.changes,
        "Changes not staged for channel commit:",
        "  (use \"rey channels diff\" to review; \"rey channels add\" to stage)",
        TerminalStyle::stdout(),
        false,
    )?;
    writeln!(output)?;
    match status.state {
        ChannelWorkingState::Clean => {
            writeln!(output, "nothing to commit, channel working tree clean")?
        }
        ChannelWorkingState::Working => writeln!(
            output,
            "no changes added to channel commit (use `rey channels add` to stage)"
        )?,
        ChannelWorkingState::Staged => {
            writeln!(output, "changes staged in the channel admission index")?
        }
        ChannelWorkingState::Mixed => writeln!(
            output,
            "staged changes and unstaged channel changes are both present"
        )?,
    }
    Ok(())
}

fn write_channel_status_changes(
    output: &mut impl Write,
    changes: &[ChannelGraphChange],
    heading: &str,
    hint: &str,
    style: TerminalStyle,
    staged: bool,
) -> Result<(), CliError> {
    if changes.is_empty() {
        return Ok(());
    }
    writeln!(output)?;
    writeln!(output, "{heading}")?;
    writeln!(output, "{hint}")?;
    for change in changes {
        let line = format!(
            "{:<11} {}: {} · {}",
            format!("{}:", change.kind.label()),
            change.object_kind.label(),
            change.object_id,
            change.detail
        );
        let line = if staged {
            style.green(&line)
        } else {
            style.red(&line)
        };
        writeln!(output, "        {line}")?;
    }
    Ok(())
}

fn write_channel_add(output: &mut impl Write, result: &ChannelAddResult) -> Result<(), CliError> {
    writeln!(output, "CHANNEL INDEX")?;
    writeln!(output, "  Index                  {}", result.index.index_id)?;
    writeln!(
        output,
        "  Graph                  {}",
        result.index.snapshot.graph_id
    )?;
    writeln!(
        output,
        "  Selection              {} semantic changes staged",
        result.staged.summary.total
    )?;
    writeln!(
        output,
        "  Direction              {} → {}",
        result.staged.source_label, result.staged.target_label
    )?;
    Ok(())
}

fn write_channel_commit(
    output: &mut impl Write,
    result: &ChannelCommitResult,
) -> Result<(), CliError> {
    let commit = &result.commit;
    writeln!(
        output,
        "[CHANNEL@{} {}] {}",
        commit.sequence, commit.commit_id, commit.message
    )?;
    writeln!(
        output,
        " {} semantic changes · graph {}",
        commit.delta.summary.total, commit.snapshot.graph_id
    )?;
    Ok(())
}

fn write_channel_log(output: &mut impl Write, log: &ChannelLog) -> Result<(), CliError> {
    writeln!(output, "REY CHANNELS LOG")?;
    writeln!(
        output,
        "  History                {} total · {} shown · newest first",
        log.total_commits, log.selected_commits
    )?;
    if log.commits.is_empty() {
        writeln!(output)?;
        writeln!(output, "No channel commits.")?;
        return Ok(());
    }
    for commit in &log.commits {
        writeln!(output)?;
        writeln!(
            output,
            "commit CHANNEL@{} {}{}",
            commit.sequence,
            commit.commit_id,
            if log.head_commit_id.as_ref() == Some(&commit.commit_id) {
                " (HEAD)"
            } else {
                ""
            }
        )?;
        writeln!(
            output,
            "Date:   {}",
            DateTime::<Utc>::from_timestamp(commit.committed_at_unix, 0).map_or_else(
                || commit.committed_at_unix.to_string(),
                |value| value.to_rfc3339()
            )
        )?;
        writeln!(output)?;
        writeln!(output, "    {}", commit.message)?;
        writeln!(
            output,
            "    graph {} · {} semantic changes",
            commit.snapshot.graph_id, commit.delta.summary.total
        )?;
        if log.patch {
            let diff = ChannelDiff {
                schema: "rey.channel-diff.v1".to_owned(),
                source: commit.snapshot.clone(),
                target: commit.snapshot.clone(),
                delta: commit.delta.clone(),
            };
            for change in &diff.delta.changes {
                writeln!(
                    output,
                    "    {}  {} {} · {}",
                    channel_diff_marker(change.kind),
                    change.object_kind.label(),
                    change.object_id,
                    change.detail
                )?;
            }
        }
    }
    Ok(())
}

fn write_channel_message_admission(
    output: &mut impl Write,
    result: &ChannelMessageAdmission,
) -> Result<(), CliError> {
    writeln!(
        output,
        "{} channel message {}",
        if result.admitted {
            "Admitted"
        } else {
            "Already admitted"
        },
        result.message.message_id
    )?;
    writeln!(
        output,
        "  Sequence               {}",
        result.message.sequence
    )?;
    writeln!(
        output,
        "  Channel                {}",
        result.message.proposal.channel_id
    )?;
    writeln!(
        output,
        "  Kind                   {}",
        result.message.proposal.kind.label()
    )?;
    writeln!(
        output,
        "  Channel graph          {}",
        result.message.channel_graph_id
    )?;
    writeln!(
        output,
        "  Relay authority        none · relay remains an explicit command or beacon tick"
    )?;
    Ok(())
}

fn write_channel_messages(
    output: &mut impl Write,
    messages: &[ChannelMessage],
) -> Result<(), CliError> {
    writeln!(output, "ADMITTED CHANNEL MESSAGES")?;
    writeln!(output, "  {}", count_noun(messages.len(), "message"))?;
    for message in messages {
        writeln!(
            output,
            "  {:>4}  {}  {} · {}",
            message.sequence,
            message.message_id,
            message.proposal.channel_id,
            message.proposal.kind.label()
        )?;
    }
    Ok(())
}

fn write_channel_relay_attempt(
    output: &mut impl Write,
    attempt: &RelayAttempt,
) -> Result<(), CliError> {
    writeln!(output, "CHANNEL RELAY ATTEMPT")?;
    writeln!(output, "  Outcome                {:?}", attempt.outcome)?;
    writeln!(output, "  Attempt                {}", attempt.attempt_id)?;
    writeln!(output, "  Message                {}", attempt.message_id)?;
    writeln!(
        output,
        "  Relay                  {}@{}",
        attempt.relay_id, attempt.relay_revision
    )?;
    writeln!(
        output,
        "  Application            {} · {}",
        attempt.application_id, attempt.environment_capability_id
    )?;
    writeln!(
        output,
        "  Environment            {}",
        attempt.environment_commit_id
    )?;
    writeln!(
        output,
        "  Target                 {}",
        attempt.target_channel_locator
    )?;
    writeln!(output, "  Process                {}", attempt.detail)?;
    Ok(())
}

fn write_polling_beacon_tick(
    output: &mut impl Write,
    tick: &PollingBeaconTick,
) -> Result<(), CliError> {
    writeln!(output, "POLLING BEACON TICK")?;
    writeln!(
        output,
        "  Beacon                 {}@{}",
        tick.beacon_id, tick.beacon_revision
    )?;
    writeln!(output, "  Messages checked       {}", tick.checked_messages)?;
    writeln!(
        output,
        "  Relay attempts         {} · {} delivered · {} failed · {} skipped",
        tick.attempted, tick.delivered, tick.failed, tick.skipped
    )?;
    for attempt in &tick.attempts {
        writeln!(
            output,
            "  {:?}  {} · {} → {}",
            attempt.outcome, attempt.message_id, attempt.relay_id, attempt.target_channel_locator
        )?;
    }
    Ok(())
}

fn write_channel_diff(output: &mut impl Write, diff: &ChannelDiff) -> Result<(), CliError> {
    if diff.delta.changes.is_empty() {
        return Ok(());
    }
    writeln!(output)?;
    writeln!(
        output,
        "REY CHANNELS DIFF · {} → {}",
        diff.delta.source_label, diff.delta.target_label
    )?;
    write_portfolio_field(
        output,
        "Evidence",
        &format!("DIFFERENT · {} semantic changes", diff.delta.summary.total),
    )?;
    write_portfolio_field(
        output,
        "Graphs",
        &format!("{} → {}", diff.source.graph_id, diff.target.graph_id),
    )?;
    write_portfolio_field(
        output,
        "Snapshots",
        &format!("{} → {}", diff.source.snapshot_id, diff.target.snapshot_id),
    )?;
    write_portfolio_field(output, "Working source", &diff.target.source.locator)?;

    write_channel_diff_section(output, "01 / CHANNELS", &diff.delta.changes, |change| {
        change.object_kind == ChannelObjectKind::Channel
    })?;
    write_channel_diff_section(
        output,
        "02 / SUBSCRIPTIONS",
        &diff.delta.changes,
        |change| change.object_kind == ChannelObjectKind::Subscription,
    )?;
    write_channel_diff_section(output, "03 / FEED STREAMS", &diff.delta.changes, |change| {
        matches!(
            change.object_kind,
            ChannelObjectKind::Stream | ChannelObjectKind::Layout
        )
    })?;
    write_channel_diff_section(output, "04 / APPLICATIONS", &diff.delta.changes, |change| {
        change.object_kind == ChannelObjectKind::Application
    })?;
    write_channel_diff_section(output, "05 / RELAYS", &diff.delta.changes, |change| {
        change.object_kind == ChannelObjectKind::Relay
    })?;
    write_channel_diff_section(
        output,
        "06 / POLLING BEACONS",
        &diff.delta.changes,
        |change| change.object_kind == ChannelObjectKind::Beacon,
    )?;
    Ok(())
}

fn write_channel_diff_section(
    output: &mut impl Write,
    heading: &str,
    changes: &[ChannelGraphChange],
    include: impl Fn(&ChannelGraphChange) -> bool,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{heading}")?;
    let mut count = 0_u64;
    for change in changes.iter().filter(|change| include(change)) {
        count += 1;
        writeln!(
            output,
            "  {}  {} {} · {}",
            channel_diff_marker(change.kind),
            change.object_kind.label(),
            change.object_id,
            change.detail
        )?;
    }
    if count == 0 {
        writeln!(output, "  no changes")?;
    }
    Ok(())
}

const fn channel_diff_marker(kind: rey::channels::ChannelChangeKind) -> &'static str {
    match kind {
        rey::channels::ChannelChangeKind::Added => "+",
        rey::channels::ChannelChangeKind::Removed => "-",
        _ => "~",
    }
}

fn write_channel_apply(
    output: &mut impl Write,
    result: &ChannelApplyResult,
) -> Result<(), CliError> {
    if !result.applied {
        writeln!(output, "nothing to apply, channel working tree unchanged")?;
        return Ok(());
    }
    writeln!(output)?;
    writeln!(output, "CHANNEL GRAPH APPLIED")?;
    write_portfolio_field(output, "Source", &result.snapshot.source.locator)?;
    write_portfolio_field(
        output,
        "Working snapshot",
        result.snapshot.snapshot_id.as_str(),
    )?;
    write_portfolio_field(output, "Graph", result.snapshot.graph_id.as_str())?;
    write_portfolio_field(
        output,
        "Inventory",
        &format!(
            "{} · {} · {} · {}",
            count_noun(result.snapshot.graph.channels.len(), "channel"),
            count_noun(result.snapshot.graph.subscriptions.len(), "subscription"),
            count_noun(result.snapshot.graph.streams.len(), "stream"),
            count_noun(result.snapshot.graph.relays.len(), "relay")
        ),
    )?;
    write_portfolio_field(
        output,
        "Changes",
        &format!(
            "{} semantic changes · {} renamed · {} moved · {} retargeted",
            result.delta.summary.total,
            result.delta.summary.renamed,
            result.delta.summary.moved,
            result.delta.summary.retargeted
        ),
    )?;
    Ok(())
}

fn count_noun(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn write_journal_admission(
    output: &mut impl Write,
    admission: &JournalAdmission,
) -> Result<(), CliError> {
    let entry = &admission.entry;
    writeln!(output)?;
    writeln!(
        output,
        "{}",
        if admission.admitted {
            "JOURNAL ENTRY ADMITTED"
        } else {
            "JOURNAL ENTRY ALREADY ADMITTED"
        }
    )?;
    write_portfolio_field(output, "Entry", &format!("J@{}", entry.sequence))?;
    write_portfolio_field(output, "Title", &entry.title)?;
    write_portfolio_field(
        output,
        "Author",
        &format!(
            "{} / {}",
            journal_author_kind(entry.author.kind),
            entry.author.id
        ),
    )?;
    write_portfolio_field(output, "Coordinate", &entry.binding.coordinate)?;
    write_portfolio_field(output, "Scale", &entry.binding.scale.to_string())?;
    write_portfolio_field(output, "Document", &format!("/journal/{}", entry.slug()))?;
    write_portfolio_field(output, "Blocks", &entry.blocks.len().to_string())?;
    write_portfolio_field(
        output,
        "Broadsheet",
        &format!(
            "{} columns · {}",
            entry.layout.columns,
            count_noun(entry.layout.bands.len(), "band")
        ),
    )?;
    write_portfolio_field(
        output,
        "Revision",
        entry
            .supersedes
            .as_ref()
            .map_or("root", |identity| identity.as_str()),
    )?;
    write_portfolio_field(output, "Identity", entry.entry_id.as_str())?;
    writeln!(output)?;
    Ok(())
}

fn write_journal_log(output: &mut impl Write, log: &JournalLog) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "JOURNAL")?;
    write_portfolio_field(
        output,
        "Retained",
        &format!("{} entries", log.entries.len()),
    )?;
    write_portfolio_field(output, "Identity", log.log_id.as_str())?;
    if log.entries.is_empty() {
        writeln!(output)?;
        writeln!(output, "No journal entries.")?;
        return Ok(());
    }
    for entry in &log.entries {
        writeln!(output)?;
        writeln!(
            output,
            "J@{} {} · {} / {}",
            entry.sequence,
            entry.title,
            journal_author_kind(entry.author.kind),
            entry.author.id
        )?;
        writeln!(
            output,
            "  {} · scale {}",
            entry.binding.coordinate, entry.binding.scale
        )?;
        writeln!(output, "  /journal/{}", entry.slug())?;
        writeln!(
            output,
            "  {} · {} / {} · {}",
            entry.admitted_at,
            count_noun(entry.blocks.len(), "cell"),
            count_noun(entry.layout.bands.len(), "band"),
            entry.entry_id
        )?;
        if let Some(supersedes) = &entry.supersedes {
            writeln!(output, "  supersedes {supersedes}")?;
        }
        for band in &entry.layout.bands {
            let cells = band
                .cells
                .iter()
                .map(|cell| {
                    let kind = entry
                        .blocks
                        .iter()
                        .find(|block| block.id() == cell.block_id)
                        .map_or("missing", journal_block_kind);
                    format!("{}:{kind} {}/12", cell.block_id, cell.span)
                })
                .collect::<Vec<_>>()
                .join(" | ");
            writeln!(output, "  [{}] {cells}", band.id)?;
        }
    }
    writeln!(output)?;
    Ok(())
}

fn journal_block_kind(block: &JournalBlock) -> &'static str {
    match block {
        JournalBlock::Prose { .. } => "prose",
        JournalBlock::Explore { .. } => "explore",
        JournalBlock::Query { .. } => "query",
        JournalBlock::Frame { .. } => "frame",
        JournalBlock::Diff { .. } => "diff",
        JournalBlock::Action { .. } => "action",
    }
}

fn journal_author_kind(kind: JournalAuthorKind) -> &'static str {
    match kind {
        JournalAuthorKind::Human => "human",
        JournalAuthorKind::Agent => "agent",
        JournalAuthorKind::System => "system",
    }
}

fn write_git_status(output: &mut impl Write, status: &GitOperatorStatus) -> Result<(), CliError> {
    writeln!(output, "GIT ACTIVATION STATUS")?;
    write_git_snapshot_fields(output, &status.observed_snapshot)?;
    match &status.state.cursor {
        Some(cursor) => {
            write_portfolio_field(output, "Cursor", cursor.cursor_id.as_str())?;
            write_portfolio_field(output, "Cursor snapshot", cursor.snapshot_id.as_str())?;
            write_portfolio_field(
                output,
                "Observed delta",
                if status.changed_since_cursor == Some(true) {
                    "CHANGED · poll required"
                } else {
                    "UNCHANGED"
                },
            )?;
        }
        None => {
            write_portfolio_field(output, "Cursor", "UNINITIALIZED")?;
            write_portfolio_field(output, "Observed delta", "UNKNOWN · no retained baseline")?;
        }
    }
    if let Some(pending) = &status.state.pending {
        write_portfolio_field(
            output,
            "Pending transition",
            pending.transition.transition_id.as_str(),
        )?;
        write_portfolio_field(output, "Pending state", "AWAITING EVIDENCE ACK")?;
        write_portfolio_field(
            output,
            "Activation proposals",
            &pending.proposals.len().to_string(),
        )?;
    } else {
        write_portfolio_field(output, "Pending transition", "none")?;
    }
    write_portfolio_field(
        output,
        "Retained transitions",
        &status.state.retained_polls.len().to_string(),
    )?;
    write_portfolio_field(
        output,
        "Retained cadence ticks",
        &status.state.cadence_ticks.len().to_string(),
    )?;
    if let Some(tick) = status.state.cadence_ticks.last() {
        write_portfolio_field(
            output,
            "Latest cadence",
            &format!(
                "#{} · {} · {}",
                tick.sequence,
                if tick.changed { "CHANGED" } else { "NO CHANGE" },
                if tick.complete { "complete" } else { "partial" }
            ),
        )?;
        write_portfolio_field(output, "Latest tick", tick.tick_id.as_str())?;
    }
    write_portfolio_field(
        output,
        "Retained watch receipts",
        &status.state.watch_receipts.len().to_string(),
    )?;
    if let Some(receipt) = status.state.watch_receipts.last() {
        write_portfolio_field(
            output,
            "Latest watch stop",
            &format!("{} · {}", receipt.stop_reason.as_str(), receipt.watch_id),
        )?;
    }
    let receipted_ticks = status
        .state
        .watch_receipts
        .iter()
        .map(|receipt| receipt.tick_ids.len())
        .sum::<usize>();
    write_portfolio_field(
        output,
        "Unreceipted ticks",
        &(status.state.cadence_ticks.len() - receipted_ticks).to_string(),
    )?;
    write_portfolio_field(output, "Repository authority", &status.repository_authority)?;
    write_portfolio_field(output, "Next", &status.next)?;
    Ok(())
}

fn write_git_initialized(output: &mut impl Write, state: &LocalGitState) -> Result<(), CliError> {
    let cursor = state
        .cursor
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    let snapshot = state
        .cursor_snapshot
        .as_ref()
        .ok_or(LocalGitStateError::Uninitialized)?;
    writeln!(output, "GIT CURSOR INITIALIZED")?;
    write_git_snapshot_fields(output, snapshot)?;
    write_portfolio_field(output, "Cursor", cursor.cursor_id.as_str())?;
    write_portfolio_field(
        output,
        "Retained evidence",
        cursor.retained_evidence_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Authority",
        "baseline only · no activation · no execution",
    )?;
    write_portfolio_field(
        output,
        "Next",
        "rey git poll [--trigger FILE] or rey git watch [--trigger FILE]",
    )?;
    Ok(())
}

fn write_git_poll(output: &mut impl Write, outcome: &GitPollOutcome) -> Result<(), CliError> {
    let transition = &outcome.record.transition;
    writeln!(output, "GIT POLL TRANSITION")?;
    write_portfolio_field(output, "Transition", transition.transition_id.as_str())?;
    write_portfolio_field(
        output,
        "Source snapshot",
        transition.source_snapshot_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Target snapshot",
        transition.target_snapshot_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "HEAD movement",
        &format!(
            "{} · {}",
            transition.head_movement.as_str(),
            if transition.head_complete {
                "complete"
            } else {
                "incomplete"
            }
        ),
    )?;
    write_portfolio_field(
        output,
        "Watched ref changes",
        if transition.watched_ref_changes.is_empty() {
            "typed empty"
        } else {
            "see below"
        },
    )?;
    for change in &transition.watched_ref_changes {
        writeln!(
            output,
            "    {} · {} → {} · {} · {}",
            change.ref_name,
            change.source_oid.as_deref().unwrap_or("ABSENT"),
            change.target_oid.as_deref().unwrap_or("ABSENT"),
            change.movement.as_str(),
            if change.complete {
                "complete"
            } else {
                "incomplete"
            }
        )?;
    }
    write_portfolio_field(
        output,
        "Events",
        if transition.events.is_empty() {
            "typed empty"
        } else {
            "see below"
        },
    )?;
    for event in &transition.events {
        writeln!(output, "    {}", event.as_str())?;
    }
    write_portfolio_field(
        output,
        "Semantic index",
        &format!(
            "{} → {} · {}",
            transition
                .source_index_digest
                .as_ref()
                .map_or("ABSENT", SemanticDigest::as_str),
            transition
                .target_index_digest
                .as_ref()
                .map_or("ABSENT", SemanticDigest::as_str),
            if transition.source_index_complete && transition.target_index_complete {
                "complete"
            } else {
                "partial"
            }
        ),
    )?;
    for omission in &transition.omissions {
        writeln!(output, "    omission: {omission}")?;
    }
    write_portfolio_field(
        output,
        "Triggers",
        &outcome.record.triggers.len().to_string(),
    )?;
    write_portfolio_field(
        output,
        "Activation proposals",
        &outcome.record.proposals.len().to_string(),
    )?;
    for proposal in &outcome.record.proposals {
        writeln!(
            output,
            "    {} · {}@{} · {} scenario selections",
            proposal.activation_id,
            proposal.workload_id,
            proposal.graph.revision,
            proposal.scenario_ids.len()
        )?;
        if !proposal.matched_ref_names.is_empty() {
            writeln!(
                output,
                "      matched refs: {}",
                proposal.matched_ref_names.join(", ")
            )?;
        }
        writeln!(output, "      authority: {}", proposal.authority)?;
    }
    write_portfolio_field(
        output,
        "Poll state",
        if outcome.retained {
            "AWAITING EVIDENCE ACK"
        } else {
            "NO CHANGE · cursor unchanged"
        },
    )?;
    if outcome.retained {
        write_portfolio_field(
            output,
            "Next",
            &format!("rey git ack {}", transition.transition_id),
        )?;
    }
    Ok(())
}

fn write_git_watch(output: &mut impl Write, outcome: &GitWatchOutcome) -> Result<(), CliError> {
    writeln!(output, "GIT WATCH")?;
    write_portfolio_field(output, "Watch", outcome.watch_id.as_str())?;
    write_portfolio_field(
        output,
        "Bounds",
        &format!(
            "{} iterations · {} ms cadence · {} ms elapsed",
            outcome.max_iterations, outcome.interval_ms, outcome.max_elapsed_ms
        ),
    )?;
    write_portfolio_field(
        output,
        "Observed elapsed",
        &format!("{} ms", outcome.elapsed_ms),
    )?;
    write_portfolio_field(output, "Retained ticks", &outcome.ticks.len().to_string())?;
    for tick in &outcome.ticks {
        writeln!(
            output,
            "    #{} · {} · {} · {}",
            tick.sequence,
            if tick.changed { "CHANGED" } else { "NO CHANGE" },
            if tick.complete { "complete" } else { "partial" },
            tick.tick_id
        )?;
        writeln!(
            output,
            "      observed {} at unix-ms {}",
            tick.observed_snapshot_id, tick.observed_at_unix_ms
        )?;
        if let Some(transition_id) = &tick.retained_transition_id {
            writeln!(output, "      pending transition {transition_id}")?;
        }
        if !tick.activation_ids.is_empty() {
            writeln!(
                output,
                "      {} activation proposal(s)",
                tick.activation_ids.len()
            )?;
        }
        for omission in &tick.omissions {
            writeln!(output, "      omission: {omission}")?;
        }
    }
    write_portfolio_field(output, "Stop", outcome.stop_reason.as_str())?;
    write_portfolio_field(
        output,
        "Pending transition",
        outcome
            .pending_transition_id
            .as_ref()
            .map_or("none", SemanticDigest::as_str),
    )?;
    write_portfolio_field(output, "Authority", &outcome.authority)?;
    let next = outcome.pending_transition_id.as_ref().map_or_else(
        || "No transition is pending; another bounded watch must be explicit".to_owned(),
        |transition_id| format!("rey git ack {transition_id}"),
    );
    write_portfolio_field(output, "Next", &next)?;
    Ok(())
}

fn write_git_acknowledgement(
    output: &mut impl Write,
    result: &GitAcknowledgement,
) -> Result<(), CliError> {
    writeln!(output, "GIT CURSOR ADVANCED")?;
    write_portfolio_field(
        output,
        "Transition",
        result.acknowledged_transition_id.as_str(),
    )?;
    write_portfolio_field(output, "Cursor", result.cursor.cursor_id.as_str())?;
    write_portfolio_field(output, "Snapshot", result.cursor.snapshot_id.as_str())?;
    write_portfolio_field(
        output,
        "Retained transitions",
        &result.retained_transition_count.to_string(),
    )?;
    write_portfolio_field(output, "Authority", &result.authority)?;
    Ok(())
}

fn write_workload_activation_admission(
    output: &mut impl Write,
    admission: &WorkloadActivationAdmission,
) -> Result<(), CliError> {
    writeln!(output, "WORKLOAD ACTIVATION ADMITTED")?;
    write_portfolio_field(output, "Admission", admission.admission_id.as_str())?;
    write_portfolio_field(
        output,
        "Activation",
        admission.activation.activation_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Git transition",
        admission.activation.transition_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Git target",
        admission.activation.target_snapshot_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Workload HEAD",
        &format!(
            "{} · snapshot {}",
            admission.workload_head_commit_id, admission.workload_head_snapshot_id
        ),
    )?;
    write_portfolio_field(
        output,
        "Workload",
        &format!("{}@{}", admission.workload.id, admission.workload.revision),
    )?;
    write_portfolio_field(
        output,
        "Graph",
        &format!("{}@{}", admission.graph.id, admission.graph.revision),
    )?;
    write_portfolio_field(
        output,
        "Scenario suite",
        &format!(
            "{}@{} · {} selected",
            admission.scenario_suite.id,
            admission.scenario_suite.revision,
            admission.selected_scenario_ids.len()
        ),
    )?;
    write_portfolio_field(
        output,
        "Contracts",
        &format!(
            "workload {} · graph {} · suite {} · evaluator {}",
            admission.workload.semantic_digest,
            admission.graph.semantic_digest,
            admission.scenario_suite.semantic_digest,
            admission.evaluator.semantic_digest,
        ),
    )?;
    for scenario_id in &admission.selected_scenario_ids {
        writeln!(output, "    {scenario_id}")?;
    }
    write_portfolio_field(
        output,
        "Capabilities",
        admission.capability_snapshot_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Completeness",
        if admission.activation.complete {
            "complete"
        } else {
            "partial · see omissions"
        },
    )?;
    for omission in &admission.activation.omissions {
        writeln!(output, "    omission: {omission}")?;
    }
    write_portfolio_field(
        output,
        "Runtime budget",
        &format!(
            "{} scenarios · {} action · {} evidence bytes",
            admission.effective_budget.max_scenarios,
            admission.effective_budget.max_actions,
            admission.effective_budget.max_evidence_bytes,
        ),
    )?;
    write_portfolio_field(output, "Authority", &admission.authority)?;
    write_portfolio_field(
        output,
        "Next",
        "runtime scheduling must revalidate these preconditions; no execution has occurred",
    )?;
    Ok(())
}

fn write_workload_activation_execution(
    output: &mut impl Write,
    execution: &WorkloadActivationExecution,
    admission: &WorkloadActivationAdmission,
    replayed: bool,
) -> Result<(), CliError> {
    writeln!(output, "WORKLOAD ACTIVATION EXECUTION")?;
    let receipt = if replayed {
        "retained result replayed · graph was not executed again".to_owned()
    } else if let Some(source) = &execution.source_execution_id {
        format!("coalesced with retained execution {source} · graph was not executed again")
    } else {
        "new execution retained".to_owned()
    };
    write_portfolio_field(output, "Receipt", &receipt)?;
    write_portfolio_field(output, "Execution", execution.execution_id.as_str())?;
    write_portfolio_field(output, "Admission", execution.admission_id.as_str())?;
    write_portfolio_field(output, "Activation", execution.activation_id.as_str())?;
    if let Some(source) = &execution.source_execution_id {
        write_portfolio_field(output, "Coalesced source", source.as_str())?;
    }
    write_portfolio_field(
        output,
        "Git evidence",
        &format!(
            "{} → {} · transition {}",
            admission.activation.source_snapshot_id,
            admission.activation.target_snapshot_id,
            admission.activation.transition_id
        ),
    )?;
    write_portfolio_field(
        output,
        "Workload",
        &format!(
            "{}@{} · graph {}@{}",
            execution.result.workload.id,
            execution.result.workload.revision,
            execution.result.graph.id,
            execution.result.graph.revision
        ),
    )?;
    write_portfolio_field(
        output,
        "Contracts",
        &format!(
            "workload {} · graph {} · suite {} · evaluator {}",
            execution.result.workload.semantic_digest,
            execution.result.graph.semantic_digest,
            execution.result.scenario_suite.semantic_digest,
            execution.result.evaluator.semantic_digest,
        ),
    )?;
    write_portfolio_field(
        output,
        "Status",
        match execution.result.status {
            TestStatus::Passed => "PASSED",
            TestStatus::Failed => "FAILED",
            TestStatus::Inconclusive => "INCONCLUSIVE",
        },
    )?;
    write_portfolio_field(
        output,
        "Scenarios",
        &format!(
            "{} selected · {} evaluated",
            execution.result.selected_scenario_ids.len(),
            execution.result.summary.selected
        ),
    )?;
    for scenario in &execution.result.scenarios {
        writeln!(
            output,
            "    {} · {} · execution {}",
            scenario.scenario.id,
            match scenario.evaluation {
                ScenarioEvaluation::Passed => "PASSED",
                ScenarioEvaluation::Failed => "FAILED",
                ScenarioEvaluation::Inconclusive => "INCONCLUSIVE",
            },
            scenario.execution_id
        )?;
        for delta in &scenario.deltas {
            write_scenario_assertion(output, delta, TerminalStyle::stdout())?;
            writeln!(
                output,
                "      delta {} · output {} · {}",
                delta.delta_id,
                delta.inputs.output_id,
                match delta.assessment {
                    DeltaAssessment::Equal => "EQUAL",
                    DeltaAssessment::Different => "DIFFERENT",
                    DeltaAssessment::Inconclusive => "INCONCLUSIVE",
                }
            )?;
        }
        for mining in &scenario.mining {
            write_source_mining_assertions(output, mining, 2, TerminalStyle::stdout())?;
            writeln!(
                output,
                "      mining {} · relation delta {}",
                mining.execution.evidence.result.result_id, mining.relation_delta.delta_id
            )?;
        }
        for patch in &scenario.topography {
            write_topography_assertion(output, patch, 2, TerminalStyle::stdout())?;
            writeln!(output, "      topography patch {}", patch.patch_id)?;
        }
        for attention in &scenario.attention {
            writeln!(output, "      attention {}", attention.attention_id)?;
        }
    }
    write_portfolio_field(
        output,
        "Evidence budget",
        &format!(
            "{} / {} bytes",
            execution.evidence_bytes, admission.effective_budget.max_evidence_bytes
        ),
    )?;
    write_portfolio_field(
        output,
        "Capabilities",
        execution.result.capability_snapshot_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Completeness",
        if admission.activation.complete {
            "Git trigger evidence complete"
        } else {
            "Git trigger evidence partial · see admission omissions"
        },
    )?;
    for omission in &admission.activation.omissions {
        writeln!(output, "    omission: {omission}")?;
    }
    write_portfolio_field(output, "Authority", &execution.authority)?;
    write_portfolio_field(
        output,
        "Qualification",
        "unchanged · selected scenario evidence cannot qualify the workload",
    )?;
    Ok(())
}

fn write_workload_activation_recomputation(
    output: &mut impl Write,
    recomputation: &WorkloadActivationRecomputation,
    execution: &WorkloadActivationExecution,
    admission: &WorkloadActivationAdmission,
    replayed: bool,
) -> Result<(), CliError> {
    writeln!(output, "WORKLOAD ACTIVATION FULL RECOMPUTATION")?;
    write_portfolio_field(
        output,
        "Receipt",
        if replayed {
            "retained proof replayed · scenarios were not executed again"
        } else {
            "new full recomputation proof retained"
        },
    )?;
    write_portfolio_field(
        output,
        "Assessment",
        match recomputation.assessment {
            WorkloadRecomputationAssessment::Equivalent => "EQUIVALENT",
            WorkloadRecomputationAssessment::Different => "DIFFERENT",
        },
    )?;
    write_portfolio_field(
        output,
        "Recomputation",
        recomputation.recomputation_id.as_str(),
    )?;
    write_portfolio_field(output, "Execution", recomputation.execution_id.as_str())?;
    write_portfolio_field(output, "Admission", recomputation.admission_id.as_str())?;
    write_portfolio_field(
        output,
        "Results",
        &format!(
            "selected {} · full {}",
            recomputation.selected_result_id, recomputation.full_result.result_id
        ),
    )?;
    write_portfolio_field(
        output,
        "Git evidence",
        &format!(
            "{} → {} · transition {}",
            admission.activation.source_snapshot_id,
            admission.activation.target_snapshot_id,
            admission.activation.transition_id
        ),
    )?;
    write_portfolio_field(
        output,
        "Workload",
        &format!(
            "{}@{} · graph {}@{}",
            execution.result.workload.id,
            execution.result.workload.revision,
            execution.result.graph.id,
            execution.result.graph.revision
        ),
    )?;
    write_portfolio_field(
        output,
        "Scenario scope",
        &format!(
            "{} selected compared · {} fully recomputed",
            recomputation.comparisons.len(),
            recomputation.full_result.scenarios.len()
        ),
    )?;
    for comparison in &recomputation.comparisons {
        writeln!(
            output,
            "    {} · {}",
            comparison.scenario_id,
            if comparison.equivalent {
                "EQUIVALENT"
            } else {
                "DIFFERENT"
            }
        )?;
        writeln!(
            output,
            "      selected execution {} · full execution {}",
            comparison.selected_execution_id, comparison.full_execution_id
        )?;
    }
    write_portfolio_field(
        output,
        "Full status",
        match recomputation.full_result.status {
            TestStatus::Passed => "PASSED",
            TestStatus::Failed => "FAILED",
            TestStatus::Inconclusive => "INCONCLUSIVE",
        },
    )?;
    write_portfolio_field(
        output,
        "Evidence budget",
        &format!(
            "{} / {} bytes",
            recomputation.full_evidence_bytes, recomputation.max_evidence_bytes
        ),
    )?;
    write_portfolio_field(
        output,
        "Capabilities",
        recomputation.full_result.capability_snapshot_id.as_str(),
    )?;
    write_portfolio_field(output, "Authority", &recomputation.authority)?;
    write_portfolio_field(
        output,
        "Qualification",
        "qualification unchanged · full recomputation is comparison evidence only",
    )?;
    Ok(())
}

fn write_git_snapshot_fields(
    output: &mut impl Write,
    snapshot: &rey_git::GitSnapshot,
) -> Result<(), CliError> {
    write_portfolio_field(output, "Repository", snapshot.repository_id.as_str())?;
    write_portfolio_field(output, "Snapshot", snapshot.snapshot_id.as_str())?;
    write_portfolio_field(
        output,
        "HEAD",
        &format!(
            "{} · {}",
            snapshot.head.symbolic_ref.as_deref().unwrap_or("DETACHED"),
            snapshot.head.commit_oid.as_deref().unwrap_or("UNBORN")
        ),
    )?;
    write_portfolio_field(
        output,
        "Watched refs",
        if snapshot.watched_refs.is_empty() {
            "none"
        } else {
            "see below"
        },
    )?;
    for watched in &snapshot.watched_refs {
        writeln!(
            output,
            "    {} · {}",
            watched.name,
            watched.target_oid.as_deref().unwrap_or("ABSENT")
        )?;
    }
    write_portfolio_field(
        output,
        "Index",
        snapshot.index.as_ref().map_or("ABSENT · bare", |index| {
            if index.complete {
                "PRESENT · complete"
            } else {
                "PRESENT · partial"
            }
        }),
    )?;
    write_portfolio_field(
        output,
        "Repository read",
        "bounded direct argv · no hooks · no optional locks · no network",
    )?;
    Ok(())
}

fn write_workload_create(
    output: &mut impl Write,
    result: &WorkloadCreateResult,
) -> Result<(), CliError> {
    let style = TerminalStyle::stdout();
    writeln!(output, "Execution path: {}", style.cyan_bold("LOCAL STATE"))?;
    writeln!(output, "Mode: {}", style.cyan_bold("APPLY"))?;
    writeln!(
        output,
        "Stage: {}",
        style.bold("CREATE REQUEST → AWAIT CODING HARNESS")
    )?;
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD CREATION"))?;
    write_portfolio_field(output, "Workload", &result.draft.request.workload_id)?;
    write_portfolio_field(output, "Request", result.draft.request.request_id.as_str())?;
    if let Some(attention) = &result.draft.request.attention {
        write_portfolio_field(
            output,
            "Attention row",
            &format!(
                "{} · {} · {}",
                attention.attention_row_id,
                attention.reason.as_str(),
                attention.subject_id,
            ),
        )?;
        write_portfolio_field(
            output,
            "Portfolio",
            attention.portfolio_snapshot_id.as_str(),
        )?;
        write_portfolio_field(
            output,
            "Environment",
            attention.environment_snapshot_id.as_str(),
        )?;
        write_portfolio_field(output, "Frontier", attention.frontier_id.as_str())?;
        write_portfolio_field(output, "Frontier row", attention.frontier_row_id.as_str())?;
        write_portfolio_field(
            output,
            "Scheduling",
            attention.scheduling_decision_id.as_str(),
        )?;
        write_portfolio_field(
            output,
            "Reasoning surface",
            attention.reasoning_surface_id.as_str(),
        )?;
        write_portfolio_field(
            output,
            "Permitted action",
            &attention.admissible_action_ids.join(", "),
        )?;
        write_portfolio_field(output, "Current package", "ABSENT · CREATE")?;
        write_portfolio_field(
            output,
            "Failing delta refs",
            if attention.delta_ids.is_empty() {
                "0 · typed empty"
            } else {
                "present"
            },
        )?;
        write_portfolio_field(
            output,
            "Surface bounds",
            &format!(
                "{} rows · {} delta refs · {} evidence refs · {} action refs · {} evidence bytes · {} retrieval iterations",
                attention.surface_limits.max_rows,
                attention.surface_limits.max_delta_refs,
                attention.surface_limits.max_evidence_refs,
                attention.surface_limits.max_action_refs,
                attention.surface_limits.max_total_evidence_bytes,
                attention.surface_limits.max_retrieval_iterations,
            ),
        )?;
    }
    write_portfolio_field(output, "Created", &result.created_files.join(" · "))?;
    write_portfolio_field(output, "Admission", &style.yellow("AWAITING HARNESS"))?;
    write_portfolio_field(output, "Graph", &style.dim("MISSING"))?;
    write_portfolio_field(output, "Scenario oracle", &style.dim("NOT ADMITTED"))?;
    writeln!(output)?;
    writeln!(output, "{}", style.bold("AGENT INSTRUCTIONS"))?;
    for (index, instruction) in result.instructions.iter().enumerate() {
        writeln!(output, "  {}. {instruction}", index + 1)?;
    }
    writeln!(output)?;
    write_portfolio_field(output, "Further action required", "YES")?;
    write_portfolio_field(output, "Next", &result.next)?;
    Ok(())
}

fn write_workload_revision_status(
    output: &mut impl Write,
    status: &WorkloadRevisionStatus,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(
        output,
        "On workload {}",
        status.head.as_ref().map_or_else(
            || "no commits yet".to_owned(),
            |commit| format!("WORKLOAD@{}", commit.sequence)
        )
    )?;
    if !status.drafts.is_empty() {
        write_portfolio_field(output, "Admission state", "AWAITING HARNESS")?;
    }
    if status.unstaged.assessment == DeltaAssessment::Different {
        write_portfolio_field(output, "Admission state", "WORKING")?;
    }
    if status.index.is_some() {
        write_portfolio_field(
            output,
            "Admission state",
            if status.commit_ready {
                "INDEX QUALIFIED"
            } else {
                "INDEX UNQUALIFIED"
            },
        )?;
    }
    if status.state == WorkloadWorkingState::Clean
        && status.drafts.is_empty()
        && status.head.is_some()
    {
        write_portfolio_field(output, "Admission state", "HEAD")?;
    }
    if let Some(ignore) = &status.working.ignore {
        writeln!(
            output,
            "Ignore file    {} · {} rules · {} working objects omitted · {}",
            ignore.source,
            ignore.rules.len(),
            ignore.ignored,
            ignore.source_digest,
        )?;
        for omission in &ignore.omissions {
            writeln!(
                output,
                "  ignored:      {}: {} · {} matches · line {}",
                omission.rule.kind,
                omission.rule.pattern,
                omission.matched,
                omission.rule.source_line,
            )?;
        }
    }
    if status.state == WorkloadWorkingState::Clean && status.drafts.is_empty() {
        writeln!(output)?;
        writeln!(output, "nothing to admit, working workload catalog clean")?;
        return Ok(());
    }
    if status.staged.assessment == DeltaAssessment::Different {
        writeln!(output)?;
        writeln!(output, "Changes staged for workload admission:")?;
        writeln!(
            output,
            "  (review with \"rey workloads diff --staged\"; approve in Rey UI or with \"rey workloads commit\")"
        )?;
        write_workload_change_lines(output, &status.staged, true, style)?;
    }
    if status.unstaged.assessment == DeltaAssessment::Different {
        writeln!(output)?;
        writeln!(output, "Changes not staged for workload admission:")?;
        writeln!(
            output,
            "  (use \"rey workloads diff\" to review; admit the exact file snapshot in Rey UI, or use \"rey workloads add\" for CLI staging)"
        )?;
        write_workload_change_lines(output, &status.unstaged, false, style)?;
    }
    if !status.drafts.is_empty() {
        writeln!(output)?;
        writeln!(output, "Agentic workload requests awaiting packages:")?;
        for draft in &status.drafts {
            writeln!(
                output,
                "        requested: workload: {}",
                draft.request.workload_id
            )?;
        }
    }
    writeln!(output)?;
    if status.commit_ready {
        writeln!(
            output,
            "staged workload INDEX is qualified and awaiting human approval"
        )?;
    } else if status.index.is_some() && !status.qualification_omissions.is_empty() {
        writeln!(
            output,
            "staged workload INDEX is not ready (use `rey workloads test --staged`)"
        )?;
        for omission in &status.qualification_omissions {
            writeln!(output, "  {omission}")?;
        }
    } else if status.unstaged.assessment == DeltaAssessment::Different {
        writeln!(
            output,
            "incoming WORKING files are ready for human review in `rey ui`; use `rey workloads add` only for the explicit CLI staging path"
        )?;
    }
    Ok(())
}

fn write_workload_change_lines(
    output: &mut impl Write,
    changes: &WorkloadChangeSet,
    staged: bool,
    style: TerminalStyle,
) -> io::Result<()> {
    for change in &changes.changes {
        let label = match change.change_kind {
            WorkloadChangeKind::Inserted => "new:",
            WorkloadChangeKind::Deleted => "deleted:",
            WorkloadChangeKind::Modified => "modified:",
        };
        write_admission_status_entry(
            output,
            label,
            &format!("workload: {}", change.workload_id),
            staged,
            style,
        )?;
    }
    Ok(())
}

fn write_workload_diff(output: &mut impl Write, diff: &WorkloadChangeSet) -> Result<(), CliError> {
    writeln!(output, "WORKLOAD CHANGE SET")?;
    writeln!(
        output,
        "  Comparison             {} → {}",
        diff.source_label, diff.target_label
    )?;
    writeln!(
        output,
        "  Assessment             {} · +{} -{} ~{}",
        scene_assessment(diff.assessment),
        diff.inserted,
        diff.deleted,
        diff.modified
    )?;
    writeln!(
        output,
        "  Source revision         {}",
        diff.source_revision
            .as_ref()
            .map_or("EMPTY", SemanticDigest::as_str)
    )?;
    writeln!(
        output,
        "  Target revision         {}",
        diff.target_revision
            .as_ref()
            .map_or("EMPTY", SemanticDigest::as_str)
    )?;
    for change in &diff.changes {
        let symbol = match change.change_kind {
            WorkloadChangeKind::Inserted => '+',
            WorkloadChangeKind::Deleted => '-',
            WorkloadChangeKind::Modified => '~',
        };
        writeln!(output, "  {symbol} workload {}", change.workload_id)?;
    }
    Ok(())
}

fn write_workload_add(output: &mut impl Write, result: &WorkloadAddResult) -> Result<(), CliError> {
    writeln!(output, "WORKLOAD INDEX")?;
    writeln!(
        output,
        "  Snapshot               {}",
        result.snapshot.snapshot_revision
    )?;
    writeln!(
        output,
        "  Packages               {}",
        result.snapshot.packages.len()
    )?;
    writeln!(
        output,
        "  Selection              {} workload changes {}",
        result.delta.changes.len(),
        if result.staged {
            "staged"
        } else {
            "verified unchanged"
        }
    )?;
    writeln!(
        output,
        "  Authority              frozen candidate only · not admitted · not runnable"
    )?;
    Ok(())
}

fn write_workload_commit(
    output: &mut impl Write,
    result: &WorkloadCommitResult,
) -> Result<(), CliError> {
    writeln!(
        output,
        "[WORKLOAD@{} {}] {}",
        result.commit.sequence, result.commit.commit_id, result.commit.message
    )?;
    writeln!(
        output,
        " admission complete · snapshot {} · {} workloads · {} qualifications",
        result.commit.snapshot.snapshot_revision,
        result.commit.snapshot.packages.len(),
        result.commit.qualification_ids.len()
    )?;
    writeln!(
        output,
        " {} workload changes · +{} -{} ~{}",
        result.delta.changes.len(),
        result.delta.inserted,
        result.delta.deleted,
        result.delta.modified
    )?;
    Ok(())
}

fn write_workload_log(output: &mut impl Write, log: &WorkloadLog) -> Result<(), CliError> {
    writeln!(output, "REY WORKLOAD LOG")?;
    writeln!(
        output,
        "  History                {} total · {} shown · newest first",
        log.total_commits, log.selected_commits
    )?;
    for (index, commit) in log.commits.iter().enumerate() {
        writeln!(output)?;
        writeln!(
            output,
            "commit WORKLOAD@{} {}{}",
            commit.sequence,
            commit.commit_id,
            if index == 0 { " (HEAD)" } else { "" }
        )?;
        writeln!(
            output,
            "Parent: {}",
            commit
                .parent_commit_id
                .as_ref()
                .map_or("EMPTY", SemanticDigest::as_str)
        )?;
        writeln!(
            output,
            "Date:   {}",
            format_workload_commit_date(commit.committed_at_unix)
        )?;
        writeln!(output)?;
        writeln!(output, "    {}", commit.message)?;
        writeln!(output)?;
        writeln!(
            output,
            "  Snapshot               {} · {} workloads",
            commit.snapshot.snapshot_revision,
            commit.snapshot.packages.len()
        )?;
        writeln!(
            output,
            "  Qualifications         {} exact passing records",
            commit.qualification_ids.len()
        )?;
        if log.patch {
            let parent = log.commits.get(index + 1).map(|parent| &parent.snapshot);
            let diff = WorkloadChangeSet::derive(
                if parent.is_some() { "HEAD^" } else { "EMPTY" },
                parent,
                &format!("WORKLOAD@{}", commit.sequence),
                Some(&commit.snapshot),
            );
            writeln!(output)?;
            write_workload_diff(output, &diff)?;
        }
    }
    Ok(())
}

fn format_workload_commit_date(committed_at_unix: i64) -> String {
    DateTime::<Utc>::from_timestamp(committed_at_unix, 0).map_or_else(
        || "invalid timestamp".to_owned(),
        |date| date.format("%a %b %e %H:%M:%S %Y %z").to_string(),
    )
}

fn write_workload_list(
    output: &mut impl Write,
    list: &WorkloadList,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let portfolio = WorkloadPortfolioSummary::derive(&list.workloads, list.drafts.len());
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD PORTFOLIO"))?;
    write_portfolio_field(
        output,
        "Catalog",
        &format!(
            "{} · {} admitted · {} draft{}",
            list.catalog.kind.label(),
            list.catalog.admitted_count,
            list.catalog.draft_count,
            list.catalog
                .root
                .as_ref()
                .map_or_else(String::new, |root| format!(" · root {root}")),
        ),
    )?;
    write_portfolio_field(
        output,
        "Admission",
        &format!(
            "{} accepted · {} awaiting coding harness",
            list.catalog.admitted_count, list.catalog.draft_count,
        ),
    )?;
    write_portfolio_field(
        output,
        "Runtime admissions",
        &format!(
            "{} Git activation{} · {} executed · {} full recomputation proof{}",
            list.activation_admissions.len(),
            if list.activation_admissions.len() == 1 {
                ""
            } else {
                "s"
            },
            list.activation_executions.len(),
            list.activation_recomputations.len(),
            if list.activation_recomputations.len() == 1 {
                ""
            } else {
                "s"
            },
        ),
    )?;
    write_portfolio_field(
        output,
        "Qualification",
        &format!(
            "{}/{} qualified · {} failing · {} inconclusive · {} stale",
            portfolio.qualified,
            list.catalog.admitted_count,
            portfolio.failing,
            portfolio.inconclusive,
            portfolio.stale_workloads,
        ),
    )?;
    write_portfolio_field(
        output,
        "Scenarios",
        &format!(
            "{}/{} passing · {}/{} evaluated · {} stale · {} optional",
            portfolio.passed_scenarios,
            portfolio.required_scenarios,
            portfolio.evaluated_scenarios,
            portfolio.required_scenarios,
            portfolio.stale_scenarios,
            portfolio.optional_scenarios,
        ),
    )?;
    write_portfolio_field(
        output,
        "Runs",
        &format!(
            "{} passed · {} blocked · {} not run",
            portfolio.passed_runs, portfolio.blocked_runs, portfolio.unrun,
        ),
    )?;
    write_portfolio_field(
        output,
        "Inventory",
        &format!(
            "{} total · {} admitted · {} draft · {} tested · {} untested",
            portfolio.total,
            list.catalog.admitted_count,
            portfolio.drafts,
            portfolio.tested,
            portfolio.untested,
        ),
    )?;
    let mining_workloads = list
        .workloads
        .iter()
        .filter(|workload| workload.mining_operations > 0)
        .count();
    let mining_results = list
        .workloads
        .iter()
        .map(|workload| workload.mining_results)
        .sum::<u64>();
    let incomplete_mining = list
        .workloads
        .iter()
        .map(|workload| workload.incomplete_mining_results)
        .sum::<u64>();
    write_portfolio_field(
        output,
        "Mining",
        &format!(
            "{mining_workloads} workloads · {mining_results} retained results · {incomplete_mining} incomplete"
        ),
    )?;
    let topography_results = list
        .workloads
        .iter()
        .map(|workload| workload.topography_results)
        .sum::<u64>();
    let topography_frontier = list
        .workloads
        .iter()
        .map(|workload| workload.topography_frontier_rows)
        .sum::<u64>();
    write_portfolio_field(
        output,
        "Topography",
        &format!(
            "{topography_results} retained patches · {topography_frontier} unresolved boundary rows"
        ),
    )?;
    if let Some(atlas) = &list.semantic_atlas {
        write_portfolio_field(
            output,
            "Semantic atlas",
            &format!(
                "{} · {} regions in {} world clusters · {}",
                atlas.atlas_revision,
                atlas.regions.len(),
                atlas.clusters.len(),
                if atlas.complete {
                    "COMPLETE"
                } else {
                    "BOUNDED"
                },
            ),
        )?;
        write_portfolio_field(
            output,
            "World coordinates",
            "synthetic semantic longitude/latitude · not Earth CRS84 · reclustered only when admitted source revisions change",
        )?;
        write_portfolio_field(
            output,
            "Atlas compiler",
            &format!(
                "{}@{} · zoom selects retained LOD and never reclusters",
                atlas.compiler.id, atlas.compiler.revision,
            ),
        )?;
    }
    write_portfolio_field(
        output,
        "Attention",
        &format!(
            "{} refine · {} retest · {} create · {} blocked · {} policy excluded",
            list.attention.summary.refine,
            list.attention.summary.retest,
            list.attention.summary.create,
            list.attention.summary.blocked,
            list.attention.summary.policy_excluded,
        ),
    )?;
    write_portfolio_field(
        output,
        "Coverage",
        &format!(
            "{} mapped surfaces · {} owned · {} unowned",
            list.attention.summary.surfaces,
            list.attention.summary.owned_surfaces,
            list.attention.summary.unowned_surfaces,
        ),
    )?;
    write_attention_frontier(output, &list.attention, style)?;
    write_runtime_frontier(output, list.runtime.as_ref(), style)?;
    write_activation_admissions(
        output,
        &list.activation_admissions,
        &list.activation_executions,
        &list.activation_recomputations,
        style,
    )?;
    if list.workloads.is_empty() && list.drafts.is_empty() {
        writeln!(output, "  {}", style.dim("No workloads found"))?;
        return Ok(());
    }

    for (index, workload) in list.workloads.iter().enumerate() {
        writeln!(output)?;
        writeln!(output, "{}", style.bold(&workload.workload.id))?;
        write_portfolio_field(output, "Purpose", &workload.title)?;
        if let Some(provenance) = &workload.provenance {
            write_portfolio_field(
                output,
                "Origin",
                &format!("{} · {}", provenance.origin.label(), provenance.source),
            )?;
            if let Some(generation) = &provenance.generation {
                write_portfolio_field(
                    output,
                    "Generator",
                    &format!(
                        "{} · {}@{} · graph + scenario suite",
                        generation.kind.label(),
                        generation.producer,
                        generation.producer_revision,
                    ),
                )?;
            }
            if let Some(source_digest) = &provenance.source_digest {
                write_portfolio_field(output, "Package revision", source_digest.as_str())?;
            }
            write_portfolio_field(output, "Scenario oracle", "FROZEN AT ADMISSION")?;
        }
        write_workload_ownership(output, workload)?;
        write_portfolio_field(output, "Journey", &render_journey(workload, style))?;
        write_portfolio_field(
            output,
            "Scenario conformance",
            &render_scenario_conformance(workload, style),
        )?;
        write_portfolio_field(
            output,
            "Evaluation",
            &format!(
                "{} passed · {} failed · {} inconclusive · {} stale · {} optional",
                workload.passed,
                workload.failed,
                workload.inconclusive,
                workload.stale,
                workload.optional,
            ),
        )?;
        write_portfolio_field(
            output,
            "Qualification",
            &render_qualification(workload, style),
        )?;
        write_portfolio_field(
            output,
            "Graph",
            &format!(
                "{}@{}",
                workload.candidate_graph.id, workload.candidate_graph.revision
            ),
        )?;
        write_portfolio_field(
            output,
            "Operations",
            &workload
                .operations
                .iter()
                .map(|operation| format!("{}@{}", operation.id, operation.revision))
                .collect::<Vec<_>>()
                .join(" → "),
        )?;
        if workload.mining_operations > 0 {
            write_portfolio_field(
                output,
                "Mining evidence",
                &if workload.mining_results == 0 {
                    style.dim("not evaluated")
                } else {
                    format!(
                        "{} results · {} complete · {} incomplete · {} relation deltas · {} reasoning surfaces",
                        workload.mining_results,
                        workload.complete_mining_results,
                        workload.incomplete_mining_results,
                        workload.relation_deltas,
                        workload.reasoning_surfaces,
                    )
                },
            )?;
        }
        if workload.attention_results > 0 {
            write_portfolio_field(
                output,
                "Portfolio evidence",
                &format!(
                    "{} retained attention results · {} attention rows",
                    workload.attention_results, workload.attention_rows,
                ),
            )?;
        }
        if workload.topography_results > 0 {
            write_portfolio_field(
                output,
                "Topography",
                &format!(
                    "{} patches · revision {} · {} frontier rows",
                    workload.topography_results,
                    workload
                        .topography_revision
                        .as_ref()
                        .map_or("missing", SemanticDigest::as_str),
                    workload.topography_frontier_rows,
                ),
            )?;
            if let Some(coverage) = &workload.topography_coverage {
                write_portfolio_field(
                    output,
                    "Survey coverage",
                    &format!(
                        "{}/{} seeds · {} candidates · {} resolved · {} missing · {} omitted",
                        coverage.surveyed_seeds,
                        coverage.requested_seeds,
                        coverage.candidates,
                        coverage.resolved_candidates,
                        coverage.missing_seeds,
                        coverage.omitted_seeds,
                    ),
                )?;
            }
            if let Some(packet) = &workload.topography_projection {
                write_portfolio_field(
                    output,
                    "Projection engine",
                    &format!(
                        "{} · {}@{} · {} objects · {} validity regions · {}",
                        packet.packet_id,
                        packet.projection_basis.contract.id,
                        packet.projection_basis.contract.revision,
                        packet.objects.len(),
                        packet.validity.len(),
                        if packet.complete {
                            "COMPLETE"
                        } else {
                            "BOUNDED"
                        },
                    ),
                )?;
            }
        }
        write_portfolio_field(
            output,
            "Candidate",
            workload.candidate_graph.semantic_digest.as_str(),
        )?;
        write_portfolio_field(
            output,
            "Qualified",
            &workload.qualified_graph.as_ref().map_or_else(
                || style.dim("none"),
                |graph| graph.semantic_digest.to_string(),
            ),
        )?;
        write_portfolio_field(
            output,
            "Test evidence",
            &workload.last_test_result_id.as_ref().map_or_else(
                || style.dim("none"),
                |result_id| {
                    format!(
                        "{} · {}",
                        result_id,
                        render_freshness(workload.freshness, style)
                    )
                },
            ),
        )?;
        write_portfolio_field(
            output,
            "Last run",
            &render_last_run(workload.last_run_status, style),
        )?;
        if index + 1 < list.workloads.len() || !list.drafts.is_empty() {
            writeln!(
                output,
                "{}",
                style.dim("  ────────────────────────────────────────────────────────────")
            )?;
        }
    }
    for (index, draft) in list.drafts.iter().enumerate() {
        writeln!(output)?;
        write_workload_draft(output, draft, style)?;
        if index + 1 < list.drafts.len() {
            writeln!(
                output,
                "{}",
                style.dim("  ────────────────────────────────────────────────────────────")
            )?;
        }
    }
    Ok(())
}

fn write_activation_admissions(
    output: &mut impl Write,
    admissions: &[WorkloadActivationAdmission],
    executions: &[WorkloadActivationExecution],
    recomputations: &[WorkloadActivationRecomputation],
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{}", style.bold("RUNTIME ADMISSIONS"))?;
    if admissions.is_empty() {
        writeln!(output, "  {}", style.dim("No admitted Git activations"))?;
        return Ok(());
    }
    for admission in admissions {
        let execution = executions
            .iter()
            .find(|execution| execution.admission_id == admission.admission_id);
        let recomputation = execution.and_then(|execution| {
            recomputations
                .iter()
                .find(|recomputation| recomputation.execution_id == execution.execution_id)
        });
        writeln!(
            output,
            "  {} · {} · {} scenarios · {}",
            admission.admission_id,
            admission.workload.id,
            admission.selected_scenario_ids.len(),
            execution.map_or("ADMITTED", |execution| {
                if execution.source_execution_id.is_some() {
                    "COALESCED"
                } else {
                    "EXECUTED"
                }
            }),
        )?;
        writeln!(
            output,
            "    activation {} · transition {} · Git {}",
            admission.activation.activation_id,
            admission.activation.transition_id,
            admission.activation.target_snapshot_id,
        )?;
        if let Some(execution) = execution {
            writeln!(
                output,
                "    execution {} · {:?} · {} evidence bytes · qualification unchanged",
                execution.execution_id, execution.result.status, execution.evidence_bytes,
            )?;
            if let Some(source) = &execution.source_execution_id {
                writeln!(output, "    reused execution {source} · graph not rerun")?;
            }
            if let Some(recomputation) = recomputation {
                writeln!(
                    output,
                    "    FULL {} · proof {} · {} fully recomputed · qualification unchanged",
                    recomputation.assessment.as_str().to_uppercase(),
                    recomputation.recomputation_id,
                    recomputation.full_result.scenarios.len(),
                )?;
            }
        } else {
            writeln!(
                output,
                "    workload HEAD {} · capabilities {} · revalidate before execution",
                admission.workload_head_commit_id, admission.capability_snapshot_id,
            )?;
        }
    }
    Ok(())
}

fn write_workload_draft(
    output: &mut impl Write,
    draft: &WorkloadDraft,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output, "{}", style.bold(&draft.request.workload_id))?;
    write_portfolio_field(output, "Purpose", &draft.request.title)?;
    if let Some(intent) = &draft.request.intent {
        write_portfolio_field(output, "Intent", intent)?;
    }
    write_portfolio_field(
        output,
        "Origin",
        &format!("WORKLOAD CREATION REQUEST · {}", draft.source),
    )?;
    write_portfolio_field(output, "Journey", &style.cyan_bold("HYDRATE"))?;
    write_portfolio_field(output, "Generator", "CODING HARNESS · pending")?;
    write_portfolio_field(
        output,
        "Request revision",
        draft.request.request_id.as_str(),
    )?;
    if let Some(attention) = &draft.request.attention {
        write_portfolio_field(
            output,
            "Attention row",
            &format!(
                "{} · {} · {}",
                attention.attention_row_id,
                attention.reason.as_str(),
                attention.subject_id,
            ),
        )?;
        write_portfolio_field(
            output,
            "Reasoning surface",
            attention.reasoning_surface_id.as_str(),
        )?;
    }
    write_portfolio_field(output, "Source revision", draft.source_digest.as_str())?;
    write_portfolio_field(output, "Graph", &style.dim("MISSING"))?;
    write_portfolio_field(output, "Scenario oracle", &style.dim("NOT ADMITTED"))?;
    write_portfolio_field(output, "Admission", &style.yellow("AWAITING HARNESS"))?;
    write_portfolio_field(
        output,
        "Next",
        &format!("Materialize {}", draft.request.target_package),
    )?;
    Ok(())
}

fn write_portfolio_field(
    output: &mut impl Write,
    label: &str,
    value: &str,
) -> Result<(), CliError> {
    writeln!(output, "  {label:<22} {value}")?;
    Ok(())
}

fn write_workload_ownership(
    output: &mut impl Write,
    workload: &WorkloadSummary,
) -> Result<(), CliError> {
    if workload.owned_surfaces.is_empty() {
        write_portfolio_field(output, "Ownership", "no surfaces declared")?;
    } else {
        write_portfolio_field(
            output,
            "Ownership",
            &format!(
                "{} bounded surface declarations",
                workload.owned_surfaces.len()
            ),
        )?;
        for surface in &workload.owned_surfaces {
            write_portfolio_field(
                output,
                "Owned surface",
                &format!(
                    "{} · revision {} · capabilities {}",
                    surface.surface_id,
                    surface.source_revision,
                    if surface.required_capability_ids.is_empty() {
                        "none".to_owned()
                    } else {
                        surface.required_capability_ids.join(", ")
                    },
                ),
            )?;
        }
    }
    if workload.git_dependencies.is_empty() {
        write_portfolio_field(output, "Git dependencies", "none declared")?;
    }
    for dependency in &workload.git_dependencies {
        write_portfolio_field(
            output,
            "Git dependency",
            &format!(
                "{} · {} · repository {} · worktree {} · ref {} · revision {}",
                dependency.dependency_id,
                dependency.kind.as_str(),
                dependency.repository_id,
                dependency.worktree_id.as_deref().unwrap_or("absent"),
                dependency.symbolic_ref.as_deref().unwrap_or("detached"),
                dependency.source_revision,
            ),
        )?;
    }
    Ok(())
}

fn write_attention_frontier(
    output: &mut impl Write,
    attention: &WorkloadAttention,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{}", style.bold("ATTENTION FRONTIER"))?;
    if attention.rows.is_empty() {
        writeln!(
            output,
            "  {}",
            style.green("No unresolved portfolio attention")
        )?;
        return Ok(());
    }
    for row in &attention.rows {
        let action = match row.readiness {
            rey_runtime::AttentionReadiness::Ready => {
                style.cyan_bold(&row.action.as_str().to_uppercase())
            }
            rey_runtime::AttentionReadiness::Blocked => {
                style.yellow(&row.action.as_str().to_uppercase())
            }
            rey_runtime::AttentionReadiness::Excluded => {
                style.dim(&row.action.as_str().to_uppercase())
            }
        };
        writeln!(
            output,
            "  {action:<16} {} · {} · {} · priority {} · cost {}",
            row.subject_id,
            row.reason.as_str(),
            row.readiness.as_str(),
            row.priority,
            row.estimated_cost_units,
        )?;
        if !row.evidence_ids.is_empty() || !row.dependency_ids.is_empty() {
            writeln!(
                output,
                "    evidence {} · dependencies {}",
                if row.evidence_ids.is_empty() {
                    "none".to_owned()
                } else {
                    row.evidence_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                if row.dependency_ids.is_empty() {
                    "none".to_owned()
                } else {
                    row.dependency_ids.join(", ")
                },
            )?;
        }
    }
    Ok(())
}

fn write_runtime_frontier(
    output: &mut impl Write,
    runtime: Option<&PortfolioReasoningEvidence>,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{}", style.bold("RUNTIME FRONTIER"))?;
    let Some(runtime) = runtime else {
        writeln!(
            output,
            "  {}",
            style.yellow("Unavailable · no retained environment snapshot")
        )?;
        return Ok(());
    };
    let frontier = &runtime.frontier;
    write_portfolio_field(
        output,
        "Frontier",
        &format!(
            "{} · {:?} · {} schedulable rows",
            frontier.frontier_id,
            frontier.assessment,
            frontier.rows.len(),
        ),
    )?;
    write_portfolio_field(output, "Attention trace", frontier.inputs.trace_id.as_str())?;
    write_portfolio_field(
        output,
        "Portfolio snapshot",
        frontier.inputs.committed_record_id.as_str(),
    )?;
    write_portfolio_field(
        output,
        "Environment",
        frontier.inputs.capability_snapshot_id.as_str(),
    )?;
    for row in &frontier.rows {
        writeln!(
            output,
            "  {:<22} {} · {} · priority {} · cost {}",
            "Ready work", row.entity_kind, row.entity_id, row.priority, row.estimated_cost_units,
        )?;
        writeln!(output, "  {:<22} {}", "Frontier row", row.row_id)?;
        for claim in &row.claim_ids {
            writeln!(output, "  {:<22} {claim}", "Attention claim")?;
        }
    }
    write_portfolio_field(
        output,
        "Scheduling",
        &format!(
            "{} · {:?} · {} selected · cost {}/{}",
            runtime.scheduling.decision_id,
            runtime.scheduling.outcome,
            runtime.scheduling.selected.len(),
            runtime.scheduling.selected_cost_units,
            runtime.scheduling.limits.max_total_cost_units,
        ),
    )?;
    if let Some(surface) = &runtime.surface {
        write_portfolio_field(
            output,
            "Reasoning surface",
            &format!(
                "{} · {:?} · {} rows · {} evidence · {} actions",
                surface.surface_id,
                surface.completeness,
                surface.rows.len(),
                surface.evidence.len(),
                surface.admissible_actions.len(),
            ),
        )?;
        write_portfolio_field(
            output,
            "Surface budget",
            &format!(
                "{} rows · {} evidence bytes · {} retrieval iterations",
                surface.limits.max_rows,
                surface.limits.max_total_evidence_bytes,
                surface.limits.max_retrieval_iterations,
            ),
        )?;
    } else {
        write_portfolio_field(
            output,
            "Reasoning surface",
            "not produced · no work selected",
        )?;
    }
    write_portfolio_field(
        output,
        "Progress",
        "not derived · no prior runtime frontier",
    )?;
    write_portfolio_field(
        output,
        "Proof",
        "not derived · no evaluated runtime transition",
    )?;
    Ok(())
}

fn render_journey(summary: &WorkloadSummary, style: TerminalStyle) -> String {
    match summary.qualification {
        rey::workloads::QualificationState::Untested => style.cyan_bold("TEST"),
        rey::workloads::QualificationState::Failing => style.red("REVISE GRAPH"),
        rey::workloads::QualificationState::Inconclusive => style.yellow("RESTORE EVIDENCE"),
        rey::workloads::QualificationState::Stale => style.yellow("RETEST"),
        rey::workloads::QualificationState::Qualified => match summary.last_run_status {
            Some(RunStatus::Passed) => style.green("RUN COMPLETE"),
            Some(RunStatus::Blocked) | None => style.cyan_bold("RUN READY"),
        },
    }
}

fn render_scenario_conformance(summary: &WorkloadSummary, style: TerminalStyle) -> String {
    let percent = scenario_percent(summary.passed, summary.required);
    let raw_bar = score_bar(percent, 20);
    let bar = match summary.qualification {
        rey::workloads::QualificationState::Qualified => style.green(&raw_bar),
        rey::workloads::QualificationState::Failing => style.red(&raw_bar),
        rey::workloads::QualificationState::Inconclusive
        | rey::workloads::QualificationState::Stale => style.yellow(&raw_bar),
        rey::workloads::QualificationState::Untested => style.dim(&raw_bar),
    };
    format!(
        "{bar}  {percent:>3}%  {}/{} passing · {}/{} evaluated",
        summary.passed, summary.required, summary.evaluated, summary.required,
    )
}

fn scenario_percent(passed: u64, required: u64) -> u64 {
    passed
        .saturating_mul(100)
        .saturating_add(required / 2)
        .checked_div(required)
        .unwrap_or(0)
}

fn score_bar(percent: u64, width: u64) -> String {
    let bounded = percent.min(100);
    let filled = bounded.saturating_mul(width).saturating_add(50) / 100;
    format!(
        "{}{}",
        "█".repeat(filled as usize),
        "░".repeat(width.saturating_sub(filled) as usize)
    )
}

fn render_qualification(summary: &WorkloadSummary, style: TerminalStyle) -> String {
    match summary.qualification {
        rey::workloads::QualificationState::Qualified => style.green("QUALIFIED"),
        rey::workloads::QualificationState::Failing => style.red("FAILING"),
        rey::workloads::QualificationState::Inconclusive => style.yellow("INCONCLUSIVE"),
        rey::workloads::QualificationState::Stale => style.yellow("STALE"),
        rey::workloads::QualificationState::Untested => style.dim("UNTESTED"),
    }
}

fn render_freshness(freshness: rey::workloads::WorkloadFreshness, style: TerminalStyle) -> String {
    match freshness {
        rey::workloads::WorkloadFreshness::Fresh => style.green("fresh"),
        rey::workloads::WorkloadFreshness::Stale => style.yellow("stale"),
        rey::workloads::WorkloadFreshness::Untested => style.dim("untested"),
    }
}

fn render_last_run(status: Option<RunStatus>, style: TerminalStyle) -> String {
    match status {
        Some(RunStatus::Passed) => style.green("passed"),
        Some(RunStatus::Blocked) => style.yellow("blocked"),
        None => style.dim("not run"),
    }
}

fn write_workload_status(
    output: &mut impl Write,
    batch: &WorkloadStatusBatch,
) -> Result<(), CliError> {
    let style = TerminalStyle::stdout();
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD STATUS"))?;
    write_portfolio_field(
        output,
        "Catalog",
        &format!(
            "{} · {} admitted · {} draft · {}",
            batch.catalog.kind.label(),
            batch.catalog.admitted_count,
            batch.catalog.draft_count,
            batch.catalog.root.as_deref().unwrap_or("compiled"),
        ),
    )?;
    for (index, status) in batch.statuses.iter().enumerate() {
        writeln!(output)?;
        let summary = &status.summary;
        writeln!(output, "{}", style.bold(&summary.workload.id))?;
        write_portfolio_field(output, "Purpose", &summary.title)?;
        if let Some(provenance) = &summary.provenance {
            write_portfolio_field(
                output,
                "Origin",
                &format!("{} · {}", provenance.origin.label(), provenance.source),
            )?;
            if let Some(generation) = &provenance.generation {
                write_portfolio_field(
                    output,
                    "Generator",
                    &format!(
                        "{} · {}@{} · graph + scenario suite",
                        generation.kind.label(),
                        generation.producer,
                        generation.producer_revision,
                    ),
                )?;
            }
            if let Some(source_digest) = &provenance.source_digest {
                write_portfolio_field(output, "Package revision", source_digest.as_str())?;
            }
            write_portfolio_field(output, "Scenario oracle", "FROZEN AT ADMISSION")?;
        }
        write_workload_ownership(output, summary)?;
        write_portfolio_field(output, "Journey", &render_journey(summary, style))?;
        write_portfolio_field(
            output,
            "Scenario conformance",
            &render_scenario_conformance(summary, style),
        )?;
        write_portfolio_field(
            output,
            "Evaluation",
            &format!(
                "{} passed · {} failed · {} inconclusive · {} stale",
                summary.passed, summary.failed, summary.inconclusive, summary.stale,
            ),
        )?;
        write_portfolio_field(
            output,
            "Qualification",
            &format!(
                "{} · {}",
                render_qualification(summary, style),
                render_freshness(summary.freshness, style),
            ),
        )?;
        write_portfolio_field(
            output,
            "Candidate graph",
            &format!(
                "{}@{} · {}",
                summary.candidate_graph.id,
                summary.candidate_graph.revision,
                summary.candidate_graph.semantic_digest,
            ),
        )?;
        if summary.topography_results > 0 {
            write_portfolio_field(
                output,
                "Topography revision",
                summary
                    .topography_revision
                    .as_ref()
                    .map_or("missing", SemanticDigest::as_str),
            )?;
            if let Some(coverage) = &summary.topography_coverage {
                write_portfolio_field(
                    output,
                    "Topography coverage",
                    &format!(
                        "{}/{} seeds surveyed · {} empty · {} missing · {} omitted · {}/{} unique candidates resolved · {} frontier",
                        coverage.surveyed_seeds,
                        coverage.requested_seeds,
                        coverage.surveyed_empty_seeds,
                        coverage.missing_seeds,
                        coverage.omitted_seeds,
                        coverage.resolved_candidates,
                        coverage.unique_candidates,
                        summary.topography_frontier_rows,
                    ),
                )?;
            }
        }
        if let Some(result) = &status.last_test {
            writeln!(output)?;
            writeln!(output, "{}", style.bold("RETAINED TEST EVIDENCE"))?;
            write_test_detail(output, result)?;
        } else {
            write_portfolio_field(output, "Test evidence", &style.dim("none"))?;
        }
        if let Some(result) = &status.last_run {
            write_portfolio_field(
                output,
                "Last run",
                &format!(
                    "{:?} · {} · {}",
                    result.status, result.stop_reason, result.run_id,
                ),
            )?;
        } else {
            write_portfolio_field(output, "Last run", &style.dim("none"))?;
        }
        if index + 1 < batch.statuses.len() || !batch.drafts.is_empty() {
            writeln!(
                output,
                "{}",
                style.dim("  ────────────────────────────────────────────────────────────")
            )?;
        }
    }
    for (index, draft) in batch.drafts.iter().enumerate() {
        writeln!(output)?;
        write_workload_draft(output, draft, style)?;
        writeln!(output)?;
        writeln!(output, "{}", style.bold("AGENT INSTRUCTIONS"))?;
        for (instruction_index, instruction) in draft.request.requirements.iter().enumerate() {
            writeln!(output, "  {}. {instruction}", instruction_index + 1)?;
        }
        if index + 1 < batch.drafts.len() {
            writeln!(
                output,
                "{}",
                style.dim("  ────────────────────────────────────────────────────────────")
            )?;
        }
    }
    writeln!(output)?;
    write_attention_frontier(output, &batch.attention, TerminalStyle::stdout())?;
    write_runtime_frontier(output, batch.runtime.as_ref(), TerminalStyle::stdout())?;
    Ok(())
}

fn write_workload_test_plan(
    output: &mut impl Write,
    workloads: &[ResolvedWorkload],
    catalog: &WorkloadCatalogDescriptor,
    selected_workload: Option<&str>,
    style: TerminalStyle,
) -> io::Result<()> {
    let scope = selected_workload.map_or_else(
        || format!("all workloads · {}", workloads.len()),
        |id| format!("{id} · 1 workload"),
    );
    writeln!(
        output,
        "{} · {}",
        style.bold("WORKLOAD TEST"),
        style.bold(catalog.root.as_deref().unwrap_or("compiled catalog"))
    )?;
    writeln!(output, "  Scope       {scope}")?;
    writeln!(
        output,
        "  Execution   {} · read-only graph + probes · retain local results",
        style.cyan_bold("LOCAL")
    )?;
    writeln!(output, "  Assertions  {}", style.bold("EXPECTED → ACTUAL"))?;
    writeln!(output, "  Catalog     {}", catalog.kind.label())?;
    Ok(())
}

fn write_workload_test_start(
    output: &mut impl Write,
    resolved: &ResolvedWorkload,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let workload = &resolved.definition;
    let assertion_count = workload
        .scenario_suite
        .scenarios
        .iter()
        .map(|scenario| {
            scenario.expected_outputs.len()
                + usize::from(scenario.source_search.is_some()).saturating_mul(2)
                + usize::from(scenario.topography_survey.is_some())
        })
        .sum::<usize>();
    let graph_path = workload
        .graph
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<Vec<_>>()
        .join(" → ");
    writeln!(output)?;
    writeln!(
        output,
        "{} · {} scenarios · {} assertions",
        style.bold(&workload.workload.id),
        workload.scenario_suite.scenarios.len(),
        assertion_count,
    )?;
    writeln!(
        output,
        "  Graph       {} · deterministic serial",
        style.bold(&graph_path)
    )?;
    writeln!(
        output,
        "  Source      {} · {}",
        resolved.provenance.source,
        resolved.provenance.origin.label(),
    )?;
    if verbosity >= 2 {
        writeln!(
            output,
            "  Admission   typed DAG {} · scenario oracle FROZEN",
            style.green("VERIFIED")
        )?;
    }
    if verbosity >= 2
        && let Some(generation) = &resolved.provenance.generation
    {
        writeln!(
            output,
            "  Generation  {} · {}@{} · graph + scenario suite",
            style.cyan_bold(generation.kind.label()),
            generation.producer,
            generation.producer_revision,
        )?;
    }
    if verbosity >= 2
        && workload
            .graph
            .nodes
            .iter()
            .any(|node| node.operation.id.starts_with("rey.source-search."))
    {
        writeln!(
            output,
            "  Mining      {} · explicit local corpus · bounded read-only probe",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "  Operation   rey.source-search.literal-utf8@1 → rey.source-matches.v1 → ordered UTF-8 text"
        )?;
    }
    if verbosity >= 2
        && workload
            .graph
            .nodes
            .iter()
            .any(|node| node.operation.id == "rey.context-anchor-survey.locate")
    {
        writeln!(
            output,
            "  Topography  {} · explicit local seeds · bounded read-only survey",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "  Operation   rey.context-anchor-survey.locate@1 → rey.topography-patch.v1 → ordered UTF-8 evidence"
        )?;
    }
    if verbosity >= 2
        && workload
            .graph
            .nodes
            .iter()
            .any(|node| node.operation.id == "rey.portfolio.attention.derive")
    {
        writeln!(
            output,
            "  Portfolio   {} · retained catalog/environment inputs · bounded typed relation",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "  Operation   rey.portfolio.attention.derive@1 → rey.workload-attention.v1 → ordered UTF-8 text"
        )?;
    }
    if verbosity >= 2 {
        writeln!(
            output,
            "  Workload    {}@{} · {}",
            workload.workload.id, workload.workload.revision, workload.workload.semantic_digest
        )?;
        writeln!(
            output,
            "  Graph id    {}@{} · {}",
            workload.graph.graph.id,
            workload.graph.graph.revision,
            workload.graph.graph.semantic_digest
        )?;
        writeln!(
            output,
            "  Suite       {}@{} · {}",
            workload.scenario_suite.suite.id,
            workload.scenario_suite.suite.revision,
            workload.scenario_suite.suite.semantic_digest
        )?;
        writeln!(
            output,
            "  Evaluator   {}@{} · {}",
            workload.evaluator.id, workload.evaluator.revision, workload.evaluator.semantic_digest
        )?;
    }
    Ok(())
}

fn write_workload_test_scenario(
    output: &mut impl Write,
    workload: &WorkloadDefinition,
    scenario: &ScenarioResult,
    index: usize,
    total: usize,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let equal_outputs = scenario
        .deltas
        .iter()
        .filter(|delta| delta.assessment == DeltaAssessment::Equal)
        .count();
    let equal_relations = scenario
        .mining
        .iter()
        .filter(|evidence| evidence.relation_delta.assessment == DeltaAssessment::Equal)
        .count();
    let complete_mining = scenario
        .mining
        .iter()
        .filter(|evidence| {
            evidence.execution.evidence.result.completeness == MiningCompleteness::Complete
        })
        .count();
    let complete_topography = scenario
        .topography
        .iter()
        .filter(|patch| patch.complete)
        .count();
    let assertion_total =
        scenario.deltas.len() + scenario.mining.len().saturating_mul(2) + scenario.topography.len();
    let assertions_satisfied =
        equal_outputs + equal_relations + complete_mining + complete_topography;
    let label = match scenario.evaluation {
        ScenarioEvaluation::Passed => style.green("PASS"),
        ScenarioEvaluation::Failed => style.red("FAIL"),
        ScenarioEvaluation::Inconclusive => style.yellow("INCONCLUSIVE"),
    };
    let prefix = format!("{}.scenario.", workload.workload.id);
    let scenario_id = scenario
        .scenario
        .id
        .strip_prefix(&prefix)
        .unwrap_or(&scenario.scenario.id);
    writeln!(
        output,
        "  {label} {:02}/{:02} {} · {}/{} assertions satisfied · {}",
        index,
        total,
        scenario_id,
        assertions_satisfied,
        assertion_total,
        if scenario.required {
            "required"
        } else {
            "optional"
        }
    )?;
    let passing = scenario.evaluation == ScenarioEvaluation::Passed;
    if passing && verbosity == 0 {
        return Ok(());
    }
    writeln!(output, "    Assertions (EXPECTED → ACTUAL)")?;
    for delta in &scenario.deltas {
        if verbosity >= 1 || delta.assessment != DeltaAssessment::Equal {
            write_scenario_assertion(output, delta, style)?;
        }
    }
    for mining in &scenario.mining {
        write_source_mining_assertions(output, mining, verbosity, style)?;
    }
    for patch in &scenario.topography {
        write_topography_assertion(output, patch, verbosity, style)?;
    }
    if verbosity >= 2 {
        writeln!(output, "    Evidence (exact)")?;
        for delta in &scenario.deltas {
            write_scenario_delta(output, workload, scenario, delta, verbosity, style)?;
        }
        for mining in &scenario.mining {
            write_source_mining_evidence(output, mining, verbosity, style)?;
        }
        for patch in &scenario.topography {
            write_topography_evidence(output, patch, verbosity, style)?;
        }
        for attention in &scenario.attention {
            write_portfolio_attention_evidence(output, attention, verbosity, style)?;
        }
    }
    Ok(())
}

fn write_scenario_assertion(
    output: &mut impl Write,
    delta: &ScenarioOutputDelta,
    style: TerminalStyle,
) -> io::Result<()> {
    writeln!(
        output,
        "      {} output.{} · {}",
        assertion_marker(delta.assessment, style),
        delta.inputs.output_id,
        assertion_assessment(delta.assessment, style),
    )?;
    writeln!(
        output,
        "        EXPECTED {}",
        summarize_assertion_text(
            &delta.expected,
            delta.text_delta.source_line_count,
            delta.text_delta.inputs.source_artifact_id.as_str(),
        )
    )?;
    writeln!(
        output,
        "        ACTUAL   {}",
        summarize_assertion_text(
            &delta.observed,
            delta.text_delta.target_line_count,
            delta.text_delta.inputs.target_artifact_id.as_str(),
        )
    )?;
    if delta.assessment == DeltaAssessment::Different {
        write_text_delta_hunks(output, delta, style)?;
    }
    Ok(())
}

fn summarize_assertion_text(value: &str, lines: u64, artifact_id: &str) -> String {
    if !value.contains('\n') && value.chars().count() <= 96 {
        format!("{value:?}")
    } else {
        format!(
            "{lines} {} · {} bytes · {}",
            if lines == 1 { "line" } else { "lines" },
            value.len(),
            short_artifact_id(artifact_id),
        )
    }
}

fn short_artifact_id(value: &str) -> String {
    let prefix_len = value.find(':').map_or(0, |index| index + 1);
    let end = value.len().min(prefix_len.saturating_add(12));
    if end < value.len() {
        format!("{}…", &value[..end])
    } else {
        value.to_owned()
    }
}

fn assertion_marker(assessment: DeltaAssessment, style: TerminalStyle) -> String {
    match assessment {
        DeltaAssessment::Equal => style.green("="),
        DeltaAssessment::Different => style.red("!"),
        DeltaAssessment::Inconclusive => style.yellow("?"),
    }
}

fn assertion_assessment(assessment: DeltaAssessment, style: TerminalStyle) -> String {
    match assessment {
        DeltaAssessment::Equal => style.green("EQUAL"),
        DeltaAssessment::Different => style.red("DIFFERENT"),
        DeltaAssessment::Inconclusive => style.yellow("INCONCLUSIVE"),
    }
}

fn write_text_delta_hunks(
    output: &mut impl Write,
    delta: &ScenarioOutputDelta,
    style: TerminalStyle,
) -> io::Result<()> {
    for hunk in &delta.text_delta.hunks {
        writeln!(
            output,
            "        @@ -{},{} +{},{} @@",
            hunk.source_start_line,
            hunk.source_line_count,
            hunk.target_start_line,
            hunk.target_line_count
        )?;
        for line in &hunk.lines {
            let text = line.text.strip_suffix('\n').unwrap_or(&line.text);
            match line.kind {
                TextLineKind::Context => writeln!(output, "         {text}")?,
                TextLineKind::Delete => {
                    writeln!(output, "        {}", style.red(&format!("- {text}")))?
                }
                TextLineKind::Insert => {
                    writeln!(output, "        {}", style.green(&format!("+ {text}")))?
                }
            }
        }
    }
    Ok(())
}

fn write_source_mining_assertions(
    output: &mut impl Write,
    mining: &rey_runtime::MiningScenarioEvidence,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let relation = &mining.relation_delta;
    if verbosity >= 1 || relation.assessment != DeltaAssessment::Equal {
        writeln!(
            output,
            "      {} source.matches · {}",
            assertion_marker(relation.assessment, style),
            assertion_assessment(relation.assessment, style),
        )?;
        writeln!(
            output,
            "        EXPECTED {} typed rows",
            relation.summary.expected_rows
        )?;
        writeln!(
            output,
            "        ACTUAL   {} typed rows · +{} -{} ~{}",
            relation.summary.observed_rows,
            relation.summary.inserted,
            relation.summary.deleted,
            relation.summary.modified,
        )?;
        if relation.assessment != DeltaAssessment::Equal {
            write_source_match_changes(output, relation, style)?;
        }
    }

    let result = &mining.execution.evidence.result;
    let complete = result.completeness == MiningCompleteness::Complete;
    if verbosity >= 1 || !complete {
        let (marker, assessment) = if complete {
            (style.green("="), style.green("EQUAL"))
        } else {
            (style.yellow("?"), style.yellow("INCONCLUSIVE"))
        };
        writeln!(output, "      {marker} source.complete · {assessment}")?;
        writeln!(output, "        EXPECTED complete")?;
        writeln!(
            output,
            "        ACTUAL   {} · {} files · {} matches · {} bytes",
            result.completeness.as_str(),
            result.consumption.files,
            result.consumption.matches,
            result.consumption.bytes_read,
        )?;
        for omission in &result.omissions {
            writeln!(
                output,
                "        ? {} · {} omitted · {}",
                mining_omission_label(omission.kind),
                omission.omitted_count,
                omission.reason,
            )?;
        }
    }
    Ok(())
}

fn write_source_match_changes(
    output: &mut impl Write,
    relation: &rey_diff::SourceMatchDelta,
    style: TerminalStyle,
) -> io::Result<()> {
    writeln!(
        output,
        "        @@ EXPECTED source matches → ACTUAL source matches @@"
    )?;
    for change in &relation.changes {
        let path = change
            .observed
            .as_ref()
            .map(|row| row.path_display.as_str())
            .or_else(|| {
                change
                    .expected
                    .as_ref()
                    .map(|row| row.path.display.as_str())
            })
            .unwrap_or("<unknown>");
        let line = change
            .observed
            .as_ref()
            .map(|row| row.start_line)
            .or_else(|| change.expected.as_ref().map(|row| row.start_line))
            .unwrap_or(0);
        writeln!(
            output,
            "        @@ {path}:{line} bytes {}-{} @@",
            change.key.start_byte, change.key.end_byte,
        )?;
        if let Some(expected) = &change.expected {
            writeln!(
                output,
                "        {}",
                style.red(&format!("- {:?}", expected.matched_text))
            )?;
        }
        if let Some(actual) = &change.observed {
            writeln!(
                output,
                "        {}",
                style.green(&format!("+ {:?}", actual.matched_text))
            )?;
        }
    }
    Ok(())
}

fn write_topography_assertion(
    output: &mut impl Write,
    patch: &TopographyPatch,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    if verbosity == 0 && patch.complete {
        return Ok(());
    }
    let (marker, assessment) = if patch.complete {
        (style.green("="), style.green("EQUAL"))
    } else {
        (style.yellow("?"), style.yellow("INCONCLUSIVE"))
    };
    writeln!(output, "      {marker} topography.complete · {assessment}")?;
    writeln!(output, "        EXPECTED complete")?;
    writeln!(
        output,
        "        ACTUAL   {} · seeds {}/{} · candidates {}/{} resolved · patch +{} -{} ~{}",
        if patch.complete {
            "complete"
        } else {
            "bounded"
        },
        patch.coverage.surveyed_seeds,
        patch.coverage.requested_seeds,
        patch.coverage.resolved_candidates,
        patch.coverage.unique_candidates,
        patch.delta.inserted,
        patch.delta.deleted,
        patch.delta.modified,
    )?;
    for omission in &patch.omissions {
        writeln!(
            output,
            "        ? {} · {} · {} omitted · {}",
            omission.kind, omission.subject, omission.omitted_count, omission.reason,
        )?;
    }
    Ok(())
}

fn write_topography_evidence(
    output: &mut impl Write,
    patch: &TopographyPatch,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    const ROW_LIMIT: usize = 24;
    let projection = ProjectionPacket::from_topography_patch(patch).map_err(io::Error::other)?;

    writeln!(
        output,
        "         Topography patch: {} seeds · {} candidates · {} anchors · {} edges · {} frontier · {}",
        patch.coverage.requested_seeds,
        patch.coverage.candidates,
        patch.anchors.len(),
        patch.edges.len(),
        patch.frontier.len(),
        if patch.complete {
            style.green("COMPLETE")
        } else {
            style.yellow("BOUNDED")
        },
    )?;
    writeln!(
        output,
        "         Coverage: {}/{} surveyed · {} empty · {} missing · {} omitted · {}/{} unique candidates resolved",
        patch.coverage.surveyed_seeds,
        patch.coverage.requested_seeds,
        patch.coverage.surveyed_empty_seeds,
        patch.coverage.missing_seeds,
        patch.coverage.omitted_seeds,
        patch.coverage.resolved_candidates,
        patch.coverage.unique_candidates,
    )?;
    writeln!(
        output,
        "         Directed patch: {} → {} · +{} -{} ~{}",
        patch.delta.source_revision,
        patch.delta.target_revision,
        patch.delta.inserted,
        patch.delta.deleted,
        patch.delta.modified,
    )?;
    let surveyed_regions = patch
        .regions
        .iter()
        .filter(|region| matches!(region.state.as_str(), "surveyed" | "surveyed_empty"))
        .count();
    let sampled_conditions = patch
        .seeds
        .iter()
        .map(|seed| seed.candidate_count)
        .sum::<u64>();
    let precipitation_stations = patch
        .seeds
        .iter()
        .filter(|seed| seed.candidate_count > 0)
        .count();
    let omitted_conditions = patch
        .omissions
        .iter()
        .map(|omission| omission.omitted_count)
        .sum::<u64>();
    writeln!(
        output,
        "         World geometry: 1 admitted chart · {surveyed_regions} surveyed regions · {} unresolved probe horizons · unexplored extent has no inferred boundary",
        patch.frontier.len(),
    )?;
    writeln!(
        output,
        "         Survey atmosphere: {sampled_conditions} admitted candidate conditions at {precipitation_stations} sampled stations · {} unresolved boundary fronts · {omitted_conditions} omitted conditions",
        patch.frontier.len(),
    )?;
    writeln!(
        output,
        "         Natural-feature basis: {} anchor stations shape relief · {} retained seed edges remain inspector provenance and are not rendered as relief or paths",
        patch.anchors.len(),
        patch.edges.len(),
    )?;
    writeln!(
        output,
        "         Hydrology projection: rainfall and downslope accumulation may carve displayed streams, rivers, and erosion · no discovered or built path claim",
    )?;
    writeln!(
        output,
        "         Projection packet: {} · {}@{} · {} objects · {} validity regions · {} field channels · {} layers · {}",
        projection.packet_id,
        projection.projection_basis.contract.id,
        projection.projection_basis.contract.revision,
        projection.objects.len(),
        projection.validity.len(),
        projection.field_channels.len(),
        projection.layers.len(),
        if projection.complete {
            style.green("COMPLETE")
        } else {
            style.yellow("BOUNDED")
        },
    )?;
    writeln!(
        output,
        "         Projection boundary: {} retained source relationships excluded from terrain geometry · synthetic distance is not language or semantic distance",
        projection.excluded_source_relationships,
    )?;
    for seed in &patch.seeds {
        writeln!(
            output,
            "         SEED {:<18} {:<15} {} candidates · {}",
            seed.path,
            seed.state.as_str(),
            seed.candidate_count,
            seed.detail,
        )?;
    }
    for resolution in patch.resolutions.iter().take(ROW_LIMIT) {
        writeln!(
            output,
            "         LOCATOR {:<28} {:<12} {}",
            resolution.candidate,
            resolution.status.as_str(),
            resolution.coordinate.as_ref().map_or_else(
                || resolution.detail.clone(),
                |coordinate| coordinate.coordinate.clone(),
            ),
        )?;
    }
    write_topography_projection_fold(
        output,
        "locator resolutions",
        patch.resolutions.len(),
        ROW_LIMIT,
        style,
    )?;
    for omission in &patch.omissions {
        writeln!(
            output,
            "         OMISSION {} · {} · count {} · {}",
            omission.kind, omission.subject, omission.omitted_count, omission.reason,
        )?;
    }
    if verbosity >= 1 {
        for anchor in patch.anchors.iter().take(ROW_LIMIT) {
            writeln!(
                output,
                "         ANCHOR {:<18} {} · {}",
                anchor.kind.as_str(),
                anchor.label,
                anchor.coordinate.coordinate,
            )?;
        }
        write_topography_projection_fold(output, "anchors", patch.anchors.len(), ROW_LIMIT, style)?;
        for edge in patch.edges.iter().take(ROW_LIMIT) {
            writeln!(
                output,
                "         EDGE {:<10} {} → {} · {}",
                edge.kind.as_str(),
                edge.source_coordinate,
                edge.target_coordinate,
                edge.locator,
            )?;
        }
        write_topography_projection_fold(output, "edges", patch.edges.len(), ROW_LIMIT, style)?;
        for region in patch.regions.iter().take(ROW_LIMIT) {
            writeln!(
                output,
                "         REGION {:<14} {} · {}",
                region.state.as_str(),
                region.coordinate,
                region.detail,
            )?;
        }
        write_topography_projection_fold(output, "regions", patch.regions.len(), ROW_LIMIT, style)?;
        for row in patch.frontier.iter().take(ROW_LIMIT) {
            writeln!(
                output,
                "         PROBE {:<12} {} · {} · {}",
                row.status.as_str(),
                row.locator,
                topography_probe_action(row.status.as_str()),
                row.reason,
            )?;
        }
        write_topography_projection_fold(
            output,
            "frontier rows",
            patch.frontier.len(),
            ROW_LIMIT,
            style,
        )?;
    }
    if verbosity >= 2 {
        writeln!(
            output,
            "         {}",
            style.dim("Exact topography bindings:")
        )?;
        for (label, value) in [
            ("patch", patch.patch_id.as_str()),
            ("topography", patch.topography_revision.as_str()),
            ("prior", patch.prior_topography_revision.as_str()),
            ("delta", patch.delta.delta_id.as_str()),
            ("campaign", patch.campaign_id.as_str()),
            ("execution", patch.execution_id.as_str()),
            ("capability", patch.capability_snapshot_id.as_str()),
        ] {
            write_test_binding(output, label, value)?;
        }
        write_test_binding(
            output,
            "operation",
            &format!(
                "{}@{} · {}",
                patch.operation.id, patch.operation.revision, patch.operation.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "implementation",
            &format!(
                "{}@{} · {}",
                patch.implementation.id,
                patch.implementation.revision,
                patch.implementation.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "provider",
            &format!(
                "{}@{} · {}",
                patch.provider.id, patch.provider.revision, patch.provider.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "limits",
            &format!(
                "seeds={} seed_bytes={} total_bytes={} candidates={} anchors={} edges={} regions={} frontier={} omissions={}",
                patch.limits.max_seeds,
                patch.limits.max_seed_bytes,
                patch.limits.max_total_bytes,
                patch.limits.max_candidates,
                patch.limits.max_anchors,
                patch.limits.max_edges,
                patch.limits.max_regions,
                patch.limits.max_frontier,
                patch.limits.max_omissions,
            ),
        )?;
        for lineage in &patch.lineage {
            write_test_binding(
                output,
                "lineage",
                &format!(
                    "{} · {} · {}",
                    lineage.kind, lineage.identity, lineage.revision
                ),
            )?;
        }
        write_projection_packet_evidence(output, &projection, style)?;
    }
    Ok(())
}

fn write_projection_packet_evidence(
    output: &mut impl Write,
    packet: &ProjectionPacket,
    style: TerminalStyle,
) -> io::Result<()> {
    const ROW_LIMIT: usize = 24;
    writeln!(
        output,
        "         {}",
        style.dim("Exact projection bindings:")
    )?;
    for (label, value) in [
        ("packet", packet.packet_id.to_string()),
        ("source patch", packet.source_patch_id.to_string()),
        (
            "source topography",
            packet.source_topography_revision.to_string(),
        ),
        (
            "basis",
            format!(
                "{}@{} · {}",
                packet.projection_basis.contract.id,
                packet.projection_basis.contract.revision,
                packet.projection_basis.contract.semantic_digest,
            ),
        ),
        (
            "scene compiler",
            format!(
                "{}@{} · {}",
                packet.scene_compiler.id,
                packet.scene_compiler.revision,
                packet.scene_compiler.semantic_digest,
            ),
        ),
        (
            "extent",
            format!(
                "{}×{} {}",
                packet.extent.width, packet.extent.height, packet.extent.unit,
            ),
        ),
        (
            "terrain program",
            format!(
                "{}@{} · seed {} · {} bands · {}",
                packet.terrain_program.evaluator.id,
                packet.terrain_program.evaluator.revision,
                packet.terrain_program.seed,
                packet.terrain_program.bands.len(),
                packet.terrain_program.detail_rule,
            ),
        ),
        (
            "working set",
            format!(
                "≤{}×{} · ≤{} cells · ≤{} bytes · target {} px/sample · {}",
                packet.terrain_program.working_set.max_columns,
                packet.terrain_program.working_set.max_rows,
                packet.terrain_program.working_set.max_cells,
                packet.terrain_program.working_set.max_bytes,
                packet
                    .terrain_program
                    .working_set
                    .target_sample_spacing_pixels,
                packet.terrain_program.working_set.recenter_rule,
            ),
        ),
        (
            "distance",
            packet.projection_basis.distance_semantics.clone(),
        ),
        ("distortion", packet.projection_basis.distortion.clone()),
        (
            "limits",
            format!(
                "anchors={} frontier={} validity={} terrain_bands={} working_set_cells={} working_set_bytes={} contours={} features={} labels={}",
                packet.limits.max_anchor_objects,
                packet.limits.max_frontier_objects,
                packet.limits.max_validity_regions,
                packet.limits.max_terrain_bands,
                packet.limits.max_working_set_cells,
                packet.limits.max_working_set_bytes,
                packet.limits.max_contours,
                packet.limits.max_natural_features,
                packet.limits.max_labels,
            ),
        ),
    ] {
        write_test_binding(output, label, &value)?;
    }
    for band in &packet.terrain_program.bands {
        write_test_binding(
            output,
            "terrain band",
            &format!(
                "{} · wavelength {} · amplitude {:.3} · {} octave(s) · ≥{} samples/wavelength · {}",
                band.band_id,
                band.wavelength_scene_units,
                band.amplitude_microunits as f64 / 1_000_000.0,
                band.octaves,
                band.minimum_samples_per_wavelength,
                band.detail_authority,
            ),
        )?;
    }
    for channel in &packet.field_channels {
        write_test_binding(
            output,
            "field",
            &format!(
                "{} · {} · {} · {}@{} · source {}",
                channel.id,
                channel.kind.as_str(),
                channel.units,
                channel.implementation.id,
                channel.implementation.revision,
                channel.source_revision,
            ),
        )?;
    }
    for region in packet.validity.iter().take(ROW_LIMIT) {
        write_test_binding(
            output,
            "validity",
            &format!(
                "{} · {} · {}",
                region.state.as_str(),
                region.coordinate,
                region.source_revision,
            ),
        )?;
    }
    write_topography_projection_fold(
        output,
        "validity regions",
        packet.validity.len(),
        ROW_LIMIT,
        style,
    )?;
    for layer in &packet.layers {
        write_test_binding(
            output,
            "layer",
            &format!(
                "{} · {} · {}",
                layer.id,
                layer.authority.as_str(),
                layer.source_revision,
            ),
        )?;
    }
    for item in &packet.degradation {
        write_test_binding(
            output,
            "degradation",
            &format!(
                "{} · count {} · {}",
                item.kind, item.omitted_count, item.reason
            ),
        )?;
    }
    for omission in packet.omissions.iter().take(ROW_LIMIT) {
        write_test_binding(
            output,
            "projection omission",
            &format!(
                "{} · {} · count {} · {}",
                omission.kind, omission.subject, omission.omitted_count, omission.reason,
            ),
        )?;
    }
    write_topography_projection_fold(
        output,
        "projection omissions",
        packet.omissions.len(),
        ROW_LIMIT,
        style,
    )?;
    for lineage in packet.lineage.iter().take(ROW_LIMIT) {
        write_test_binding(
            output,
            "projection lineage",
            &format!(
                "{} · {} · {}",
                lineage.kind, lineage.identity, lineage.revision,
            ),
        )?;
    }
    write_topography_projection_fold(
        output,
        "projection lineage rows",
        packet.lineage.len(),
        ROW_LIMIT,
        style,
    )?;
    Ok(())
}

fn topography_probe_action(status: &str) -> &'static str {
    match status {
        "truncated" => "expand declared survey bound",
        "stale" => "revalidate source revision",
        "unsupported" => "admit a resolver capability",
        "unauthorized" => "obtain explicit read authority",
        "malformed" => "curate the locator",
        "missing" => "verify absence or repair reference",
        _ => "admit mining separately",
    }
}

fn write_topography_projection_fold(
    output: &mut impl Write,
    label: &str,
    total: usize,
    displayed: usize,
    style: TerminalStyle,
) -> io::Result<()> {
    if total > displayed {
        writeln!(
            output,
            "         {}",
            style.dim(&format!(
                "CLI projection folds {} additional {label}; structured output retains all rows",
                total - displayed
            )),
        )?;
    }
    Ok(())
}

fn write_portfolio_attention_evidence(
    output: &mut impl Write,
    attention: &WorkloadAttention,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    writeln!(
        output,
        "         Portfolio attention: {} rows · {} refine · {} retest · {} create · {} blocked · {} excluded",
        attention.rows.len(),
        attention.summary.refine,
        attention.summary.retest,
        attention.summary.create,
        attention.summary.blocked,
        attention.summary.policy_excluded,
    )?;
    if attention.rows.is_empty() {
        writeln!(
            output,
            "         {}",
            style.green("No unresolved portfolio attention")
        )?;
    }
    for row in &attention.rows {
        writeln!(
            output,
            "         {} {:<8} {} · {} · priority {} · cost {}",
            match row.readiness {
                rey_runtime::AttentionReadiness::Ready => "+",
                rey_runtime::AttentionReadiness::Blocked => "!",
                rey_runtime::AttentionReadiness::Excluded => "~",
            },
            row.action.as_str(),
            row.subject_id,
            row.reason.as_str(),
            row.priority,
            row.estimated_cost_units,
        )?;
        if verbosity >= 2 {
            write_test_binding(output, "attention", row.row_id.as_str())?;
            write_test_binding(output, "readiness", row.readiness.as_str())?;
            for evidence in &row.evidence_ids {
                write_test_binding(output, "evidence", evidence.as_str())?;
            }
            for dependency in &row.dependency_ids {
                write_test_binding(output, "dependency", dependency)?;
            }
        }
    }
    if verbosity >= 2 {
        write_test_binding(output, "relation", attention.attention_id.as_str())?;
        write_test_binding(output, "snapshot", attention.source_snapshot_id.as_str())?;
        write_test_binding(
            output,
            "derivation",
            &format!(
                "{}@{} · {}",
                attention.derivation.id,
                attention.derivation.revision,
                attention.derivation.semantic_digest,
            ),
        )?;
    }
    Ok(())
}

fn write_scenario_delta(
    output: &mut impl Write,
    workload: &WorkloadDefinition,
    scenario: &ScenarioResult,
    delta: &ScenarioOutputDelta,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let passing = delta.assessment == DeltaAssessment::Equal;
    let label = if passing {
        style.green("Match")
    } else {
        style.red("Delta")
    };
    if verbosity >= 2 {
        writeln!(
            output,
            "         {label} ({} · output {}):",
            delta.delta_id, delta.inputs.output_id
        )?;
        writeln!(output, "         {}", style.dim("Exact bindings:"))?;
        write_test_binding(
            output,
            "workload",
            &format!(
                "{}@{} · {}",
                workload.workload.id, workload.workload.revision, workload.workload.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "graph",
            &format!(
                "{}@{} · {}",
                workload.graph.graph.id,
                workload.graph.graph.revision,
                workload.graph.graph.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "scenario",
            &format!(
                "{}@{} · {}",
                scenario.scenario.id, scenario.scenario.revision, scenario.scenario.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "evaluator",
            &format!(
                "{}@{} · {}",
                workload.evaluator.id,
                workload.evaluator.revision,
                workload.evaluator.semantic_digest
            ),
        )?;
        write_test_binding(output, "execution", scenario.execution_id.as_str())?;
        write_test_binding(output, "delta", delta.delta_id.as_str())?;
        let projection = text_patch_projection();
        write_test_binding(
            output,
            "text view",
            &format!(
                "{}@{} · {}",
                projection.id, projection.revision, projection.semantic_digest
            ),
        )?;
    } else {
        writeln!(
            output,
            "         {label} (output {}):",
            delta.inputs.output_id
        )?;
    }
    writeln!(output, "         @@ {} · utf8 @@", delta.inputs.output_id)?;
    match delta.assessment {
        DeltaAssessment::Equal => {
            writeln!(output, "            {:?}", delta.expected)?;
        }
        DeltaAssessment::Different => {
            for hunk in &delta.text_delta.hunks {
                writeln!(
                    output,
                    "         @@ -{},{} +{},{} @@",
                    hunk.source_start_line,
                    hunk.source_line_count,
                    hunk.target_start_line,
                    hunk.target_line_count
                )?;
                for line in &hunk.lines {
                    let text = line.text.strip_suffix('\n').unwrap_or(&line.text);
                    match line.kind {
                        TextLineKind::Context => writeln!(output, "          {text}")?,
                        TextLineKind::Delete => {
                            writeln!(output, "         {}", style.red(&format!("- {text}")))?
                        }
                        TextLineKind::Insert => {
                            writeln!(output, "         {}", style.green(&format!("+ {text}")))?
                        }
                    }
                }
            }
        }
        DeltaAssessment::Inconclusive => {
            writeln!(
                output,
                "         {}",
                style.yellow(&format!(
                    "? EXPECTED {:?} · OBSERVED {:?}",
                    delta.expected, delta.observed
                ))
            )?;
        }
    }
    Ok(())
}

fn write_source_mining_evidence(
    output: &mut impl Write,
    mining: &rey_runtime::MiningScenarioEvidence,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let evidence = &mining.execution.evidence;
    let result = &evidence.result;
    let relation = &mining.relation_delta;
    let completeness = match result.completeness {
        MiningCompleteness::Complete => style.green("COMPLETE"),
        MiningCompleteness::Partial | MiningCompleteness::Truncated => {
            style.yellow(result.completeness.as_str().to_uppercase().as_str())
        }
        MiningCompleteness::Unsupported
        | MiningCompleteness::Unavailable
        | MiningCompleteness::Failed => {
            style.red(result.completeness.as_str().to_uppercase().as_str())
        }
    };
    let assessment = match relation.assessment {
        DeltaAssessment::Equal => style.green("EQUAL"),
        DeltaAssessment::Different => style.red("DIFFERENT"),
        DeltaAssessment::Inconclusive => style.yellow("INCONCLUSIVE"),
    };
    writeln!(
        output,
        "         Mining result: {completeness} · {} files read · {} matches · {} bytes read",
        result.consumption.files, result.consumption.matches, result.consumption.bytes_read
    )?;
    writeln!(
        output,
        "         Match relation: {assessment} · {}/{} rows equal · {} inserted · {} deleted · {} modified",
        relation.summary.equal_rows,
        relation
            .summary
            .expected_rows
            .max(relation.summary.observed_rows),
        relation.summary.inserted,
        relation.summary.deleted,
        relation.summary.modified,
    )?;
    for omission in &result.omissions {
        writeln!(
            output,
            "         {} {} · {} omitted · {}",
            style.yellow("OMISSION"),
            mining_omission_label(omission.kind),
            omission.omitted_count,
            omission.reason
        )?;
    }
    if relation.assessment != DeltaAssessment::Equal {
        writeln!(
            output,
            "         @@ expected source matches → observed source matches @@"
        )?;
        for change in &relation.changes {
            let (marker, label) = match change.kind {
                SourceMatchChangeKind::Inserted => ("+", "OBSERVED"),
                SourceMatchChangeKind::Deleted => ("-", "EXPECTED"),
                SourceMatchChangeKind::Modified => ("~", "CHANGED"),
            };
            let path = change
                .observed
                .as_ref()
                .map(|row| row.path_display.as_str())
                .or_else(|| {
                    change
                        .expected
                        .as_ref()
                        .map(|row| row.path.display.as_str())
                })
                .unwrap_or("<unknown>");
            writeln!(
                output,
                "         {marker} {label:<8} {path}:{} bytes {}-{}{}",
                change
                    .observed
                    .as_ref()
                    .map(|row| row.start_line)
                    .or_else(|| change.expected.as_ref().map(|row| row.start_line))
                    .unwrap_or(0),
                change.key.start_byte,
                change.key.end_byte,
                if change.changed_fields.is_empty() {
                    String::new()
                } else {
                    format!(" · fields {}", change.changed_fields.join(","))
                }
            )?;
            if let Some(expected) = &change.expected {
                writeln!(output, "           - EXPECTED {:?}", expected.matched_text)?;
            }
            if let Some(observed) = &change.observed {
                writeln!(output, "           + OBSERVED {:?}", observed.matched_text)?;
            }
        }
    }
    if verbosity >= 1 {
        writeln!(output, "         Matches:")?;
        if relation.observed.is_empty() {
            writeln!(
                output,
                "           (typed empty rey.source-matches relation)"
            )?;
        }
        for row in &relation.observed {
            writeln!(
                output,
                "           {}:{}:{}-{}  {:?}",
                row.path_display,
                row.start_line,
                row.start_byte_in_line,
                row.end_byte_in_line,
                row.matched_text
            )?;
            for (offset, line) in row.context_text.lines().enumerate() {
                writeln!(
                    output,
                    "             {:>5} │ {}",
                    row.start_line.saturating_add(offset as u64),
                    line
                )?;
            }
        }
        writeln!(
            output,
            "         Limits: files {} · matches {} · rows {} · bytes {} · depth {} · time {}ms",
            mining.execution.request.effective_limits.max_files,
            mining.execution.request.effective_limits.max_matches,
            mining.execution.request.effective_limits.max_rows,
            mining.execution.request.effective_limits.max_bytes,
            mining.execution.request.effective_limits.max_depth,
            mining.execution.request.effective_limits.max_time_ms,
        )?;
    }
    if verbosity >= 2 {
        writeln!(output, "         Exact mining bindings:")?;
        write_test_binding(
            output,
            "operation",
            &format!(
                "{}@{} · {}",
                result.operation.id, result.operation.revision, result.operation.semantic_digest
            ),
        )?;
        write_test_binding(
            output,
            "provider",
            &format!(
                "{}@{} · {}",
                result.provider.id, result.provider.revision, result.provider.semantic_digest
            ),
        )?;
        write_test_binding(output, "capability", result.capability_snapshot_id.as_str())?;
        write_test_binding(
            output,
            "corpus",
            mining.execution.corpus.binding_id.as_str(),
        )?;
        write_test_binding(
            output,
            "request",
            mining.execution.request.request_id.as_str(),
        )?;
        write_test_binding(output, "result", result.result_id.as_str())?;
        write_test_binding(output, "relation", relation.delta_id.as_str())?;
        let projection = source_match_table_projection();
        write_test_binding(
            output,
            "match view",
            &format!(
                "{}@{} · {}",
                projection.id, projection.revision, projection.semantic_digest
            ),
        )?;
        for row in &relation.observed {
            write_test_binding(output, "source", row.source_artifact_id.as_str())?;
            write_test_binding(output, "match", row.match_id.as_str())?;
            write_test_binding(output, "context", &row.context_ref)?;
        }
        if let Some(reasoning) = &mining.reasoning {
            writeln!(output, "         Delta-directed reasoning:")?;
            write_test_binding(output, "frontier", reasoning.frontier.frontier_id.as_str())?;
            write_test_binding(
                output,
                "scheduled",
                &format!(
                    "{} · {} work row selected",
                    reasoning.scheduling.decision_id,
                    reasoning.scheduling.selected.len()
                ),
            )?;
            write_test_binding(output, "surface", reasoning.surface.surface_id.as_str())?;
        }
    }
    Ok(())
}

fn mining_omission_label(kind: rey_mining::MiningOmissionKind) -> &'static str {
    use rey_mining::MiningOmissionKind;

    match kind {
        MiningOmissionKind::FileLimit => "file_limit",
        MiningOmissionKind::RowLimit => "row_limit",
        MiningOmissionKind::MatchLimit => "match_limit",
        MiningOmissionKind::NodeLimit => "node_limit",
        MiningOmissionKind::EdgeLimit => "edge_limit",
        MiningOmissionKind::DepthLimit => "depth_limit",
        MiningOmissionKind::ByteLimit => "byte_limit",
        MiningOmissionKind::TimeLimit => "time_limit",
        MiningOmissionKind::ProviderUnavailable => "provider_unavailable",
        MiningOmissionKind::Unsupported => "unsupported",
        MiningOmissionKind::ExecutionFailed => "execution_failed",
        MiningOmissionKind::SourceDrift => "source_drift",
        MiningOmissionKind::MalformedInput => "malformed_input",
    }
}

fn write_test_binding(output: &mut impl Write, label: &str, value: &str) -> io::Result<()> {
    writeln!(output, "           {label:<11} {value}")
}

fn write_workload_test_result(
    output: &mut impl Write,
    result: &WorkloadTestResult,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let status = match result.status {
        TestStatus::Passed => style.green("QUALIFIED"),
        TestStatus::Failed => style.red("GAPS FOUND"),
        TestStatus::Inconclusive => style.yellow("INCONCLUSIVE"),
    };
    writeln!(
        output,
        "  Result      {status} · {}/{} required scenarios passing · {}/{} evaluated",
        result.summary.passed,
        result.summary.required,
        result.summary.evaluated,
        result.summary.required
    )?;
    if verbosity >= 1 {
        writeln!(
            output,
            "  Qualification {}",
            if result.qualification.is_some() {
                "issued"
            } else {
                "not issued"
            }
        )?;
    }
    if verbosity >= 2 {
        writeln!(output, "  Stop reason   {}", result.stop_reason)?;
        writeln!(output, "  Test result   {}", result.result_id)?;
        if let Some(qualification) = &result.qualification {
            writeln!(
                output,
                "  Qualification artifact {}",
                qualification.qualification_id
            )?;
        }
    }
    Ok(())
}

fn write_workload_test_summary(
    output: &mut impl Write,
    batch: &WorkloadTestBatch,
    style: TerminalStyle,
) -> io::Result<()> {
    let workloads = batch.results.len() as u64;
    let qualified = batch
        .results
        .iter()
        .filter(|result| result.status == TestStatus::Passed)
        .count() as u64;
    let failed = batch
        .results
        .iter()
        .filter(|result| result.status == TestStatus::Failed)
        .count() as u64;
    let inconclusive = batch
        .results
        .iter()
        .filter(|result| result.status == TestStatus::Inconclusive)
        .count() as u64;
    let required = batch
        .results
        .iter()
        .map(|result| result.summary.required)
        .sum::<u64>();
    let passed = batch
        .results
        .iter()
        .map(|result| result.summary.passed)
        .sum::<u64>();
    let evaluated = batch
        .results
        .iter()
        .map(|result| result.summary.evaluated)
        .sum::<u64>();
    let mut equal_deltas = 0_u64;
    let mut different_deltas = 0_u64;
    let mut inconclusive_deltas = 0_u64;
    for delta in batch
        .results
        .iter()
        .flat_map(|result| &result.scenarios)
        .flat_map(|scenario| &scenario.deltas)
    {
        match delta.assessment {
            DeltaAssessment::Equal => equal_deltas = equal_deltas.saturating_add(1),
            DeltaAssessment::Different => {
                different_deltas = different_deltas.saturating_add(1);
            }
            DeltaAssessment::Inconclusive => {
                inconclusive_deltas = inconclusive_deltas.saturating_add(1);
            }
        }
    }
    for delta in batch
        .results
        .iter()
        .flat_map(|result| &result.scenarios)
        .flat_map(|scenario| &scenario.mining)
        .map(|evidence| &evidence.relation_delta)
    {
        match delta.assessment {
            DeltaAssessment::Equal => equal_deltas = equal_deltas.saturating_add(1),
            DeltaAssessment::Different => {
                different_deltas = different_deltas.saturating_add(1);
            }
            DeltaAssessment::Inconclusive => {
                inconclusive_deltas = inconclusive_deltas.saturating_add(1);
            }
        }
    }
    let result = if failed > 0 {
        style.red("GAPS FOUND")
    } else if inconclusive > 0 {
        style.yellow("INCONCLUSIVE")
    } else {
        style.green("QUALIFIED")
    };
    let workload_percent = scenario_percent(qualified, workloads);
    let scenario_passing_percent = scenario_percent(passed, required);
    let scenario_evaluated_percent = scenario_percent(evaluated, required);
    writeln!(output)?;
    writeln!(output, "{}", style.bold("TEST SUMMARY"))?;
    writeln!(
        output,
        "  {:<20} {}  {:>3}%  {}/{} qualified",
        "Workloads",
        score_bar(workload_percent, 20),
        workload_percent,
        qualified,
        workloads
    )?;
    writeln!(
        output,
        "  {:<20} {}  {:>3}%  {}/{} passing",
        "Required scenarios",
        score_bar(scenario_passing_percent, 20),
        scenario_passing_percent,
        passed,
        required
    )?;
    writeln!(
        output,
        "  {:<20} {}  {:>3}%  {}/{} evaluated",
        "Evaluation",
        score_bar(scenario_evaluated_percent, 20),
        scenario_evaluated_percent,
        evaluated,
        required
    )?;
    writeln!(output, "  Result               {result}")?;
    writeln!(
        output,
        "  Workloads            {qualified}/{workloads} qualified · {failed} with gaps · {inconclusive} inconclusive"
    )?;
    writeln!(
        output,
        "  Required scenarios   {passed}/{required} passing · {evaluated}/{required} evaluated"
    )?;
    writeln!(
        output,
        "  Output deltas        {equal_deltas} equal · {different_deltas} different · {inconclusive_deltas} inconclusive"
    )?;
    writeln!(
        output,
        "  Qualifications       {qualified} issued · results retained locally"
    )?;
    Ok(())
}

fn write_test_detail(output: &mut impl Write, result: &WorkloadTestResult) -> Result<(), CliError> {
    writeln!(
        output,
        "test={} workload={} status={:?} passed={} failed={} inconclusive={} evaluated={} reason={}",
        result.result_id,
        result.workload.id,
        result.status,
        result.summary.passed,
        result.summary.failed,
        result.summary.inconclusive,
        result.summary.evaluated,
        result.stop_reason
    )?;
    for scenario in &result.scenarios {
        writeln!(
            output,
            "  scenario={} required={} evaluation={}",
            scenario.scenario.id,
            scenario.required,
            match scenario.evaluation {
                ScenarioEvaluation::Passed => "passed",
                ScenarioEvaluation::Failed => "failed",
                ScenarioEvaluation::Inconclusive => "inconclusive",
            }
        )?;
        for delta in &scenario.deltas {
            if delta.assessment == rey_diff::DeltaAssessment::Different {
                writeln!(
                    output,
                    "    delta={} output={} expected={} observed={}",
                    delta.delta_id,
                    delta.inputs.output_id,
                    json_cell(&delta.expected)?,
                    json_cell(&delta.observed)?
                )?;
            }
        }
        for mining in &scenario.mining {
            write_source_mining_evidence(output, mining, 2, TerminalStyle { enabled: false })?;
        }
        for patch in &scenario.topography {
            write_topography_evidence(output, patch, 2, TerminalStyle { enabled: false })?;
        }
    }
    if let Some(qualification) = &result.qualification {
        writeln!(
            output,
            "qualification={} graph={}",
            qualification.qualification_id, qualification.graph.semantic_digest
        )?;
    }
    Ok(())
}

fn write_workload_run(output: &mut impl Write, view: &WorkloadRunView) -> Result<(), CliError> {
    let result = &view.result;
    let style = TerminalStyle::stdout();
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD RUN"))?;
    write_portfolio_field(
        output,
        "Catalog",
        &format!(
            "{} · {}",
            view.catalog.kind.label(),
            view.catalog.root.as_deref().unwrap_or("compiled"),
        ),
    )?;
    write_portfolio_field(output, "Workload", &result.workload.id)?;
    write_portfolio_field(
        output,
        "Origin",
        &format!(
            "{} · {}",
            view.provenance.origin.label(),
            view.provenance.source,
        ),
    )?;
    if let Some(generation) = &view.provenance.generation {
        write_portfolio_field(
            output,
            "Generator",
            &format!(
                "{} · {}@{} · oracle FROZEN",
                generation.kind.label(),
                generation.producer,
                generation.producer_revision,
            ),
        )?;
    }
    write_portfolio_field(
        output,
        "Graph",
        &format!(
            "{}@{} · {}",
            result.graph.id, result.graph.revision, result.graph.semantic_digest,
        ),
    )?;
    write_portfolio_field(
        output,
        "Result",
        &match result.status {
            RunStatus::Passed => style.green("PASSED"),
            RunStatus::Blocked => style.yellow("BLOCKED"),
        },
    )?;
    write_portfolio_field(output, "Stop reason", &result.stop_reason)?;
    write_portfolio_field(output, "Run evidence", result.run_id.as_str())?;
    write_portfolio_field(output, "Node order", &result.node_order.join(" → "))?;
    if result.mining.is_empty() && result.topography.is_empty() && result.attention.is_empty() {
        write_portfolio_field(output, "Outputs", &json_cell(&result.outputs)?)?;
    } else if let Some(WorkloadValue::Utf8(value)) = result.outputs.get("text")
        && !result.attention.is_empty()
    {
        writeln!(
            output,
            "output=text · {} canonical attention lines · {} bytes",
            value.lines().count(),
            value.len()
        )?;
    } else if let Some(WorkloadValue::Utf8(value)) = result.outputs.get("text") {
        writeln!(
            output,
            "output=text · {} canonical match lines · {} bytes",
            value.lines().count(),
            value.len()
        )?;
    }
    for mining in &result.mining {
        let evidence = &mining.evidence;
        writeln!(
            output,
            "mining={} completeness={} operation={}@{} provider={}@{} files={} matches={} bytes_read={}",
            evidence.result.result_id,
            evidence.result.completeness.as_str(),
            evidence.result.operation.id,
            evidence.result.operation.revision,
            evidence.result.provider.id,
            evidence.result.provider.revision,
            evidence.result.consumption.files,
            evidence.result.consumption.matches,
            evidence.result.consumption.bytes_read,
        )?;
        let projection = source_match_table_projection();
        writeln!(
            output,
            "bindings corpus={} request={} capability={} view={}@{}:{}",
            mining.corpus.binding_id,
            mining.request.request_id,
            mining.request.capability_snapshot_id,
            projection.id,
            projection.revision,
            projection.semantic_digest,
        )?;
        if evidence.matches.is_empty() {
            writeln!(output, "matches=(typed empty rey.source-matches relation)")?;
        }
        for row in &evidence.matches {
            let context = evidence
                .contexts
                .iter()
                .find(|context| context.artifact_id == row.context_artifact_id);
            writeln!(
                output,
                "match={} source={} location={}:{}:{}-{} text={:?}",
                row.match_id,
                row.source_artifact_id,
                row.path.display,
                row.start_line,
                row.start_byte_in_line,
                row.end_byte_in_line,
                row.matched_text,
            )?;
            writeln!(output, "  context={}", row.context_ref)?;
            if let Some(context) = context {
                for (offset, line) in context.text.lines().enumerate() {
                    writeln!(
                        output,
                        "  {:>5} │ {}",
                        row.context_start_line.saturating_add(offset as u64),
                        line
                    )?;
                }
            }
        }
        for omission in &evidence.result.omissions {
            writeln!(
                output,
                "omission={:?} count={} reason={}",
                omission.kind, omission.omitted_count, omission.reason
            )?;
        }
    }
    for patch in &result.topography {
        write_topography_evidence(output, patch, 2, TerminalStyle { enabled: false })?;
    }
    for attention in &result.attention {
        write_portfolio_attention_evidence(output, attention, 2, TerminalStyle { enabled: false })?;
    }
    Ok(())
}

fn env_status(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    args: EnvStatusArgs,
) -> Result<ExitCode, CliError> {
    let status = rey::current_environment_status(
        store,
        workspace,
        args.discovery.limits()?,
        args.discovery.map.as_deref(),
        args.max_changes,
    )?;
    let mut stdout = io::stdout().lock();
    match args.format {
        EnvHistoryOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        EnvHistoryOutputFormat::Table => {
            write_env_status(&mut stdout, &status, TerminalStyle::stdout())?
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn env_add(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    args: EnvAddArgs,
) -> Result<ExitCode, CliError> {
    if args.patch && args.format == EnvHistoryOutputFormat::Json {
        return Err(CliError::PatchFormat);
    }
    let history = store.load()?;
    let previous_index = store.load_index(&history)?;
    let working = inspect_environment_with_mapping(
        workspace,
        args.discovery.limits()?,
        args.discovery.map.as_deref(),
        EnvironmentMapLimits::default(),
    )?;
    let before = effective_index_snapshot(&history, previous_index.as_ref(), &working)?;
    if before.semantic_digest == working.semantic_digest {
        return Err(LocalEnvironmentHistoryError::NothingToAdd.into());
    }
    let mut initial =
        EnvironmentStatus::derive(&history, previous_index, working.clone(), args.max_changes)?;
    initial.apply_ignore_projection(workspace)?;
    let (candidate, selected_count) = if args.patch {
        let selected = select_capability_changes(&initial.unstaged_delta, &initial.operator)?;
        (
            stage_selected_capabilities(&before, &working, &selected)?,
            selected.len() as u64,
        )
    } else {
        (working.clone(), initial.unstaged_delta.changes.len() as u64)
    };
    let head_snapshot_id = history.head().map(|head| &head.snapshot.semantic_digest);
    let index = if head_snapshot_id == Some(&candidate.semantic_digest) {
        store.clear_index()?;
        None
    } else {
        let index = EnvironmentAdmissionIndex::new(&history, candidate)?;
        store.save_index(&history, &index)?;
        Some(index)
    };
    let mut status = EnvironmentStatus::derive(&history, index.clone(), working, args.max_changes)?;
    status.apply_ignore_projection(workspace)?;
    let result = EnvironmentAddResult::new(index, status, selected_count);
    let mut stdout = io::stdout().lock();
    match args.format {
        EnvHistoryOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        EnvHistoryOutputFormat::Table => {
            write_env_add(&mut stdout, store, &result, TerminalStyle::stdout())?
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn env_commit(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    args: EnvCommitArgs,
) -> Result<ExitCode, CliError> {
    let mut history = store.load()?;
    let index = store
        .load_index(&history)?
        .ok_or(LocalEnvironmentHistoryError::NothingStaged)?;
    let mut status = EnvironmentStatus::derive(
        &history,
        Some(index.clone()),
        index.snapshot.clone(),
        args.max_changes,
    )?;
    status.apply_ignore_projection(workspace)?;
    let commit = history.commit(args.message, index.snapshot)?;
    store.save(&history)?;
    store.clear_index()?;
    let result = EnvironmentCommitResult::new(commit, status.staged_delta);
    match args.format {
        EnvHistoryOutputFormat::Json => {
            let mut stdout = io::stdout().lock();
            write_json_line(&mut stdout, &result)?;
        }
        EnvHistoryOutputFormat::Table => {}
    }
    Ok(ExitCode::SUCCESS)
}

fn env_diff(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    args: EnvDiffArgs,
) -> Result<ExitCode, CliError> {
    let history = store.load()?;
    let index = store.load_index(&history)?;
    let snapshot = inspect_environment_with_mapping(
        workspace,
        args.discovery.limits()?,
        args.discovery.map.as_deref(),
        EnvironmentMapLimits::default(),
    )?;
    let mode = if args.staged {
        EnvironmentDiffMode::Staged
    } else {
        EnvironmentDiffMode::Unstaged
    };
    let mut status = EnvironmentStatus::derive(&history, index, snapshot, args.max_changes)?;
    status.apply_ignore_projection(workspace)?;
    let operator = status.operator.clone();
    let diff = EnvironmentDiff::from_status(status, mode);
    let mut stdout = io::stdout().lock();
    match args.format {
        EnvHistoryOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        EnvHistoryOutputFormat::Table => write_env_diff(
            &mut stdout,
            workspace,
            &diff,
            &operator,
            TerminalStyle::stdout(),
        )?,
    }
    Ok(ExitCode::SUCCESS)
}

fn env_log(
    store: &LocalEnvironmentStore,
    workspace: &Path,
    args: EnvLogArgs,
) -> Result<ExitCode, CliError> {
    let history = store.load()?;
    let log = EnvironmentLog::derive(&history, args.max_count, args.max_changes, args.patch)?;
    let mut stdout = io::stdout().lock();
    match args.format {
        EnvHistoryOutputFormat::Json => write_json_line(&mut stdout, &log)?,
        EnvHistoryOutputFormat::Table => {
            let projections = environment_log_projections(&history, &log)?;
            write_env_log(
                &mut stdout,
                workspace,
                &log,
                &projections,
                TerminalStyle::stdout(),
            )?
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn environment_log_projections(
    history: &LocalEnvironmentHistory,
    log: &EnvironmentLog,
) -> Result<Vec<EnvironmentOperatorProjection>, LocalEnvironmentHistoryError> {
    log.entries
        .iter()
        .map(|entry| {
            let source = if entry.commit.sequence == 1 {
                None
            } else {
                history
                    .commits
                    .get(entry.commit.sequence as usize - 2)
                    .map(|commit| &commit.snapshot)
            };
            EnvironmentOperatorProjection::derive_transition(
                source,
                &entry.commit.snapshot,
                entry.delta.source_label.clone(),
                entry.delta.target_label.clone(),
            )
        })
        .collect()
}

fn select_capability_changes(
    delta: &CapabilityDelta,
    projection: &EnvironmentOperatorProjection,
) -> Result<std::collections::BTreeSet<rey_diff::CapabilityKey>, CliError> {
    if delta.changes.is_empty() {
        return Err(LocalEnvironmentHistoryError::NothingToAdd.into());
    }
    let style = TerminalStyle::stdout();
    let mut output = io::stdout().lock();
    let mut input = io::stdin().lock();
    let mut selected = std::collections::BTreeSet::new();
    let mut stage_all = false;
    writeln!(output)?;
    writeln!(output, "{}", style.cyan_bold("ENVIRONMENT ADMISSION PATCH"))?;
    writeln!(
        output,
        "  Working tree           INDEX → WORKING · {} hunks",
        delta.changes.len()
    )?;
    writeln!(
        output,
        "  Selection              y stage · n skip · q quit · a all · d done · ? help"
    )?;
    for (position, change) in delta.changes.iter().enumerate() {
        if stage_all {
            selected.insert(change.key.clone());
            continue;
        }
        writeln!(output)?;
        writeln!(
            output,
            "Hunk {}/{}",
            position.saturating_add(1),
            delta.changes.len()
        )?;
        write_environment_admission_hunk(&mut output, change, projection, style)?;
        loop {
            write!(output, "Stage this hunk [y,n,q,a,d,?]? ")?;
            output.flush()?;
            let mut answer = String::new();
            if input.read_line(&mut answer)? == 0 {
                writeln!(output)?;
                return if selected.is_empty() {
                    Err(LocalEnvironmentHistoryError::EmptyPatchSelection.into())
                } else {
                    Ok(selected)
                };
            }
            match answer.trim() {
                "y" | "yes" => {
                    selected.insert(change.key.clone());
                    break;
                }
                "n" | "no" => break,
                "q" | "quit" | "d" | "none" => {
                    return if selected.is_empty() {
                        Err(LocalEnvironmentHistoryError::EmptyPatchSelection.into())
                    } else {
                        Ok(selected)
                    };
                }
                "a" | "all" => {
                    selected.insert(change.key.clone());
                    stage_all = true;
                    break;
                }
                "?" => writeln!(
                    output,
                    "  y stage this hunk; n skip; q quit; a stage this and all remaining; d leave this and all remaining unstaged"
                )?,
                _ => writeln!(output, "  expected y, n, q, a, d, or ?")?,
            }
        }
    }
    if selected.is_empty() {
        Err(LocalEnvironmentHistoryError::EmptyPatchSelection.into())
    } else {
        Ok(selected)
    }
}

fn write_environment_admission_hunk(
    output: &mut impl Write,
    change: &CapabilityChange,
    projection: &EnvironmentOperatorProjection,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let direction = EnvironmentProjectionDirection::IndexToWorking;
    let capability_kind = change
        .after
        .as_ref()
        .or(change.before.as_ref())
        .map(|record| record.capability_kind.as_str());
    let object_id = environment_change_object_id(change);

    if change.key.capability_id == "git.repository.inspect" {
        write_environment_hunk_header(output, "git", "repository", change.kind, style)?;
        let marker = match change.kind {
            CapabilityChangeKind::Inserted => style.green("+"),
            CapabilityChangeKind::Deleted => style.red("-"),
            CapabilityChangeKind::Modified => style.yellow("~"),
        };
        writeln!(
            output,
            "  {marker} Git repository state: HEAD + semantic index"
        )?;
        writeln!(
            output,
            "      Scope                Git state belongs to cadence and workload activation, not environment admission"
        )?;
        return Ok(());
    }

    match capability_kind {
        Some("environment_seed" | "environment_variable") => {
            if let Some(variable) = projection
                .variables
                .iter()
                .find(|variable| variable.object_id == object_id)
            {
                write_environment_hunk_header(
                    output,
                    "variable",
                    &variable.object_id,
                    change.kind,
                    style,
                )?;
                write_environment_variable_diff(output, variable, direction, style)?;
                return Ok(());
            }
        }
        Some("identity_probe" | "potential_executable") => {
            if let Some(application) = projection.applications.iter().find(|application| {
                application.object_id == object_id
                    || direction
                        .target(application)
                        .or_else(|| direction.source(application))
                        .is_some_and(|observation| {
                            observation
                                .potential_capabilities
                                .contains(&change.key.capability_id)
                        })
            }) {
                write_environment_hunk_header(
                    output,
                    "application",
                    &application.object_id,
                    change.kind,
                    style,
                )?;
                write_environment_application_diff(output, application, direction, style)?;
                return Ok(());
            }
        }
        Some("input_file") => {
            if let Some(input) = projection
                .inputs
                .iter()
                .find(|input| input.object_id == object_id)
            {
                write_environment_hunk_header(
                    output,
                    "input",
                    &input.object_id,
                    change.kind,
                    style,
                )?;
                write_environment_input_diff(output, input, direction, style)?;
                return Ok(());
            }
        }
        Some("environment_edge") => {
            if let Some(reference) = projection
                .references
                .iter()
                .find(|reference| reference.object_id == object_id)
            {
                write_environment_hunk_header(
                    output,
                    "reference",
                    &reference.object_id,
                    change.kind,
                    style,
                )?;
                write_environment_reference_diff(output, reference, direction, style)?;
                return Ok(());
            }
        }
        _ => {}
    }

    write_environment_hunk_header(
        output,
        "capability",
        &format!(
            "{}@{}/{}",
            change.key.provider_id, change.key.provider_revision, change.key.capability_id
        ),
        change.kind,
        style,
    )?;
    write_capability_change(output, change, style)?;
    Ok(())
}

fn environment_change_object_id(change: &CapabilityChange) -> String {
    if let Some(name) = change.key.capability_id.strip_prefix("env.seed.") {
        return format!("seed-{name}");
    }
    change
        .key
        .capability_id
        .strip_prefix("env.mapping.node.")
        .unwrap_or(&change.key.capability_id)
        .to_owned()
}

fn write_environment_hunk_header(
    output: &mut impl Write,
    kind: &str,
    object_id: &str,
    change: CapabilityChangeKind,
    style: TerminalStyle,
) -> io::Result<()> {
    let path = format!("environment/{kind}/{object_id}");
    writeln!(output, "diff --rey a/{path} b/{path}")?;
    writeln!(
        output,
        "{}",
        style.dim(&format!(
            "@@ INDEX → WORKING · {}",
            capability_change_kind_label(change)
        ))
    )
}

const fn capability_change_kind_label(change: CapabilityChangeKind) -> &'static str {
    match change {
        CapabilityChangeKind::Inserted => "new",
        CapabilityChangeKind::Deleted => "deleted",
        CapabilityChangeKind::Modified => "modified",
    }
}

fn write_environment_application_diff(
    output: &mut impl Write,
    application: &EnvironmentObjectStatus<EnvironmentApplicationObservation>,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> io::Result<()> {
    match direction.change(application) {
        EnvironmentObjectChange::Unchanged => {}
        EnvironmentObjectChange::Inserted => {
            if let Some(observation) = direction.target(application) {
                write_environment_application(output, observation, &style.green("+"), None)?;
            }
        }
        EnvironmentObjectChange::Deleted => {
            if let Some(observation) = direction.source(application) {
                write_environment_application(output, observation, &style.red("-"), None)?;
            }
        }
        EnvironmentObjectChange::Modified => {
            if let Some(observation) = direction.source(application) {
                write_environment_application(
                    output,
                    observation,
                    &style.red("-"),
                    Some("before"),
                )?;
            }
            if let Some(observation) = direction.target(application) {
                write_environment_application(
                    output,
                    observation,
                    &style.green("+"),
                    Some("after"),
                )?;
            }
        }
    }
    Ok(())
}

fn write_env_status(
    output: &mut impl Write,
    status: &EnvironmentStatus,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let projection = &status.operator;
    let head = status.head_sequence.map_or_else(
        || "no commits yet".to_owned(),
        |sequence| format!("ENV@{sequence}"),
    );
    writeln!(output, "On environment {head}")?;
    if let Some(ignore) = &status.ignored {
        writeln!(
            output,
            "Ignore file    {} · {} rules · {} working objects omitted · {}",
            ignore.source,
            ignore.rules.len(),
            ignore.ignored,
            ignore.source_digest,
        )?;
        for omission in &ignore.omissions {
            writeln!(
                output,
                "  ignored:      {}: {} · {} matches · line {}",
                omission.rule.kind,
                omission.rule.pattern,
                omission.matched,
                omission.rule.source_line,
            )?;
        }
    }

    write_environment_status_changes(
        output,
        projection,
        EnvironmentProjectionDirection::HeadToIndex,
        &status.staged_delta,
        "Changes to be committed:",
        "  (use \"rey env diff --staged\" to review)",
        true,
        style,
    )?;
    write_environment_status_changes(
        output,
        projection,
        EnvironmentProjectionDirection::IndexToWorking,
        &status.unstaged_delta,
        "Changes not staged for environment commit:",
        "  (use \"rey env diff\" to review; \"rey env add\" or \"rey env add -p\" to stage)",
        false,
        style,
    )?;

    writeln!(output)?;
    match status.state {
        EnvironmentWorkingState::Unborn => writeln!(
            output,
            "No environment commits yet. Use `rey env add` to begin tracking this environment."
        )?,
        EnvironmentWorkingState::Clean => {
            writeln!(output, "nothing to commit, working environment clean")?
        }
        EnvironmentWorkingState::Changed => writeln!(
            output,
            "no changes added to environment commit (use `rey env add` or `rey env add -p`)"
        )?,
        EnvironmentWorkingState::Staged => {
            writeln!(output, "changes staged in the environment admission index")?
        }
        EnvironmentWorkingState::Mixed => writeln!(
            output,
            "staged changes and unstaged environment drift are both present"
        )?,
        EnvironmentWorkingState::Inconclusive => writeln!(
            output,
            "working evidence is incomplete; Rey cannot establish a clean environment"
        )?,
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_environment_status_changes(
    output: &mut impl Write,
    projection: &EnvironmentOperatorProjection,
    direction: EnvironmentProjectionDirection,
    delta: &CapabilityDelta,
    heading: &str,
    hint: &str,
    staged: bool,
    style: TerminalStyle,
) -> io::Result<()> {
    if delta.changes.is_empty() {
        return Ok(());
    }
    writeln!(output)?;
    writeln!(output, "{heading}")?;
    writeln!(output, "{hint}")?;

    for variable in projection
        .variables
        .iter()
        .filter(|object| direction.change(object) != EnvironmentObjectChange::Unchanged)
    {
        let observation = direction
            .target(variable)
            .or_else(|| direction.source(variable));
        if let Some(observation) = observation {
            write_environment_status_entry(
                output,
                direction.change(variable),
                &format!("environment variable: {}", observation.name),
                staged,
                style,
            )?;
        }
    }
    for application in projection
        .applications
        .iter()
        .filter(|object| direction.change(object) != EnvironmentObjectChange::Unchanged)
    {
        let observation = direction
            .target(application)
            .or_else(|| direction.source(application));
        if let Some(observation) = observation {
            write_environment_status_entry(
                output,
                direction.change(application),
                &format!("application: {}", observation.name),
                staged,
                style,
            )?;
        }
    }
    for input in projection
        .inputs
        .iter()
        .filter(|object| direction.change(object) != EnvironmentObjectChange::Unchanged)
    {
        let observation = direction.target(input).or_else(|| direction.source(input));
        if let Some(observation) = observation {
            write_environment_status_entry(
                output,
                direction.change(input),
                &format!("input: {}", observation.path),
                staged,
                style,
            )?;
        }
    }
    for reference in projection
        .references
        .iter()
        .filter(|object| direction.change(object) != EnvironmentObjectChange::Unchanged)
    {
        let observation = direction
            .target(reference)
            .or_else(|| direction.source(reference));
        if let Some(observation) = observation {
            write_environment_status_entry(
                output,
                direction.change(reference),
                &format!(
                    "reference: {} --{}--> {}",
                    observation.from, observation.relation, observation.to
                ),
                staged,
                style,
            )?;
        }
    }

    for change in delta
        .changes
        .iter()
        .filter(|change| !environment_change_is_projected(change, projection, direction))
    {
        write_environment_status_entry(
            output,
            environment_capability_object_change(change.kind),
            &environment_capability_status_description(change),
            staged,
            style,
        )?;
    }
    Ok(())
}

fn environment_change_is_projected(
    change: &CapabilityChange,
    projection: &EnvironmentOperatorProjection,
    direction: EnvironmentProjectionDirection,
) -> bool {
    let Some(record) = change.after.as_ref().or(change.before.as_ref()) else {
        return false;
    };
    let object_id = environment_change_object_id(change);
    match record.capability_kind.as_str() {
        "environment_seed" | "environment_variable" => projection.variables.iter().any(|object| {
            object.object_id == object_id
                && direction.change(object) != EnvironmentObjectChange::Unchanged
        }),
        "identity_probe" | "potential_executable" => projection.applications.iter().any(|object| {
            direction.change(object) != EnvironmentObjectChange::Unchanged
                && (object.object_id == object_id
                    || direction
                        .target(object)
                        .or_else(|| direction.source(object))
                        .is_some_and(|observation| {
                            observation
                                .potential_capabilities
                                .contains(&change.key.capability_id)
                        }))
        }),
        "input_file" => projection.inputs.iter().any(|object| {
            object.object_id == object_id
                && direction.change(object) != EnvironmentObjectChange::Unchanged
        }),
        "environment_edge" => projection.references.iter().any(|object| {
            object.object_id == object_id
                && direction.change(object) != EnvironmentObjectChange::Unchanged
        }),
        _ => false,
    }
}

const fn environment_capability_object_change(
    change: CapabilityChangeKind,
) -> EnvironmentObjectChange {
    match change {
        CapabilityChangeKind::Inserted => EnvironmentObjectChange::Inserted,
        CapabilityChangeKind::Deleted => EnvironmentObjectChange::Deleted,
        CapabilityChangeKind::Modified => EnvironmentObjectChange::Modified,
    }
}

fn environment_capability_status_description(change: &CapabilityChange) -> String {
    let record = change.after.as_ref().or(change.before.as_ref());
    let capability_id = change.key.capability_id.as_str();
    let label = match capability_id {
        "frame.arrow-stream" => "typed interchange: Arrow stream frames",
        "git.repository.inspect" => "Git repository state: HEAD + semantic index",
        "source.search.literal-utf8" => "mining capability: literal UTF-8 source search",
        "workspace.metadata" => "context surface: workspace metadata",
        "tool.git.identity" => "application capability: Git identity probe",
        "tool.ripgrep.identity" => "application capability: ripgrep identity probe",
        "env.mapping.graph" => "reasoning map",
        _ => record.map_or("capability", |record| {
            match record.capability_kind.as_str() {
                "context_surface" => "context surface",
                "source_mining" => "mining capability",
                "typed_frame" => "typed interchange",
                "environment_map" => "reasoning map",
                _ => "capability",
            }
        }),
    };
    format!("{label} ({capability_id})")
}

fn write_environment_status_entry(
    output: &mut impl Write,
    change: EnvironmentObjectChange,
    description: &str,
    staged: bool,
    style: TerminalStyle,
) -> io::Result<()> {
    write_admission_status_entry(
        output,
        environment_object_change_label(change),
        description,
        staged,
        style,
    )
}

fn write_admission_status_entry(
    output: &mut impl Write,
    change_label: &str,
    description: &str,
    staged: bool,
    style: TerminalStyle,
) -> io::Result<()> {
    let line = format!("{change_label:<10} {description}");
    writeln!(output, "        {}", style.admission_change(&line, staged))
}

const fn environment_object_change_label(change: EnvironmentObjectChange) -> &'static str {
    match change {
        EnvironmentObjectChange::Unchanged => "unchanged:",
        EnvironmentObjectChange::Inserted => "new:",
        EnvironmentObjectChange::Deleted => "deleted:",
        EnvironmentObjectChange::Modified => "modified:",
    }
}

#[derive(Clone, Copy)]
enum EnvironmentProjectionDirection {
    HeadToIndex,
    IndexToWorking,
    HeadToWorking,
}

impl EnvironmentProjectionDirection {
    fn source<T>(self, object: &EnvironmentObjectStatus<T>) -> Option<&T> {
        match self {
            Self::HeadToIndex | Self::HeadToWorking => object.head.as_ref(),
            Self::IndexToWorking => object.index.as_ref(),
        }
    }

    fn target<T>(self, object: &EnvironmentObjectStatus<T>) -> Option<&T> {
        match self {
            Self::HeadToIndex => object.index.as_ref(),
            Self::IndexToWorking | Self::HeadToWorking => object.working.as_ref(),
        }
    }

    const fn change<T>(self, object: &EnvironmentObjectStatus<T>) -> EnvironmentObjectChange {
        match self {
            Self::HeadToIndex => object.changes.head_to_index,
            Self::IndexToWorking => object.changes.index_to_working,
            Self::HeadToWorking => object.changes.head_to_working,
        }
    }

    fn includes<T>(self, object: &EnvironmentObjectStatus<T>) -> bool {
        self.source(object).is_some() || self.target(object).is_some()
    }
}

fn write_environment_variable_diff(
    output: &mut impl Write,
    variable: &EnvironmentObjectStatus<EnvironmentVariableObservation>,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> io::Result<()> {
    let source = direction.source(variable);
    let target = direction.target(variable);
    match direction.change(variable) {
        EnvironmentObjectChange::Unchanged => {
            if let Some(observation) = target.or(source) {
                writeln!(
                    output,
                    "{}",
                    style.dim(&format!("  {}", environment_variable_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Inserted => {
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("+ {}", environment_variable_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Deleted => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("- {}", environment_variable_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Modified => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("- {}", environment_variable_line(observation)))
                )?;
            }
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("+ {}", environment_variable_line(observation)))
                )?;
            }
        }
    }
    Ok(())
}

fn environment_variable_line(observation: &EnvironmentVariableObservation) -> String {
    let value = match observation.availability {
        Availability::Unavailable => "<unset>".to_owned(),
        Availability::Error => format!(
            "<error:{}>",
            observation
                .error_code
                .as_deref()
                .unwrap_or("observation_failed")
        ),
        Availability::Available => match observation.capture {
            VariableCapture::Value => observation
                .value
                .as_deref()
                .map(escape_environment_value)
                .unwrap_or_else(|| "<invalid:missing-value>".to_owned()),
            VariableCapture::Digest => observation
                .value_digest
                .as_deref()
                .map(compact_digest)
                .map(|digest| format!("<digest:{digest}>"))
                .unwrap_or_else(|| "<present>".to_owned()),
            VariableCapture::Presence => {
                if observation.sensitive {
                    "<present:redacted>".to_owned()
                } else {
                    "<present>".to_owned()
                }
            }
        },
    };
    format!("{}={value}", observation.name)
}

fn escape_environment_value(value: &str) -> String {
    let escaped = value
        .chars()
        .flat_map(char::escape_default)
        .collect::<String>();
    let count = escaped.chars().count();
    if count <= ENVIRONMENT_VALUE_DISPLAY_CHARS {
        return escaped;
    }
    let prefix_length = 112;
    let suffix_length = 42;
    let prefix = escaped.chars().take(prefix_length).collect::<String>();
    let suffix = escaped
        .chars()
        .skip(count.saturating_sub(suffix_length))
        .collect::<String>();
    format!(
        "{prefix}…<{} chars omitted>…{suffix}",
        count.saturating_sub(prefix_length + suffix_length)
    )
}

fn compact_digest(value: &str) -> String {
    if value.len() <= 22 {
        value.to_owned()
    } else {
        format!("{}…{}", &value[..12], &value[value.len() - 6..])
    }
}

fn write_environment_application_planes(
    output: &mut impl Write,
    projection: &EnvironmentOperatorProjection,
    direction: EnvironmentProjectionDirection,
    search_label: &str,
    search_snapshot: &SemanticDigest,
    style: TerminalStyle,
) -> io::Result<()> {
    let desired = projection
        .applications
        .iter()
        .filter(|application| direction.target(application).is_some())
        .collect::<Vec<_>>();
    let applications_found =
        environment_application_count(&projection.applications, direction, Availability::Available);
    let applications_not_found = environment_application_count(
        &projection.applications,
        direction,
        Availability::Unavailable,
    );
    let application_errors =
        environment_application_count(&projection.applications, direction, Availability::Error);
    let changed_applications = environment_plane_changed_count(&projection.applications, direction);
    let removed_applications = projection
        .applications
        .iter()
        .filter(|application| {
            direction.target(application).is_none() && direction.source(application).is_some()
        })
        .count() as u64;

    writeln!(output)?;
    writeln!(output, "{}", style.bold("02 / BOUNDED SEARCH"))?;
    writeln!(
        output,
        "{}",
        style.bold(&format!("DESIRED INVENTORY · {} declared", desired.len()))
    )?;
    let inventory = match direction {
        EnvironmentProjectionDirection::HeadToIndex => {
            projection.application_inventory.index.as_ref()
        }
        EnvironmentProjectionDirection::IndexToWorking
        | EnvironmentProjectionDirection::HeadToWorking => {
            projection.application_inventory.working.as_ref()
        }
    };
    match inventory {
        Some(inventory) => writeln!(
            output,
            "  Record                 {} @ {}",
            inventory.source_path,
            compact_digest(&inventory.inventory_id)
        )?,
        None => writeln!(
            output,
            "  Record                 none · no desired applications"
        )?,
    }
    if desired.is_empty() {
        writeln!(output, "  NONE")?;
    } else {
        for application in &desired {
            let observation = direction
                .target(application)
                .expect("desired application has a target declaration");
            let requirement = if observation.required {
                "required"
            } else {
                "optional"
            };
            let capabilities = if observation.potential_capabilities.is_empty() {
                "no desired capabilities".to_owned()
            } else {
                observation.potential_capabilities.join(" · ")
            };
            writeln!(
                output,
                "  {:<22} {} · {requirement} · {capabilities}",
                application.object_id, observation.name
            )?;
            writeln!(
                output,
                "    Purpose              {}",
                observation.purpose.as_deref().unwrap_or("not declared")
            )?;
        }
    }

    writeln!(output)?;
    writeln!(
        output,
        "{}",
        style.bold(&format!(
            "SEARCH RECORD · {search_label} @ {}",
            compact_digest(&search_snapshot.to_string())
        ))
    )?;
    writeln!(
        output,
        "  Method                 declared adapters · bounded PATH resolution · fixed identity probes only"
    )?;
    writeln!(
        output,
        "APPLICATIONS · {} searched · {applications_found} found · {applications_not_found} not found · {application_errors} errors · {changed_applications} changed",
        desired.len()
    )?;
    write_application_group(
        output,
        "FOUND",
        applications_found,
        &projection.applications,
        Some(Availability::Available),
        direction,
        style,
    )?;
    write_application_group(
        output,
        "SEARCHED, NOT FOUND",
        applications_not_found,
        &projection.applications,
        Some(Availability::Unavailable),
        direction,
        style,
    )?;
    if application_errors > 0 {
        write_application_group(
            output,
            "OBSERVATION ERRORS",
            application_errors,
            &projection.applications,
            Some(Availability::Error),
            direction,
            style,
        )?;
    }
    if removed_applications > 0 {
        write_application_group(
            output,
            "NO LONGER SEARCHED",
            removed_applications,
            &projection.applications,
            None,
            direction,
            style,
        )?;
    }
    Ok(())
}

fn write_application_group(
    output: &mut impl Write,
    label: &str,
    count: u64,
    applications: &[EnvironmentObjectStatus<EnvironmentApplicationObservation>],
    availability: Option<Availability>,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> io::Result<()> {
    writeln!(output, "  {label} {count}")?;
    for application in applications.iter().filter(|application| {
        if let Some(availability) = availability {
            direction
                .target(application)
                .is_some_and(|working| working.availability == availability)
        } else {
            direction.target(application).is_none() && direction.source(application).is_some()
        }
    }) {
        let selected_change = direction.change(application);
        let source = direction.source(application);
        let target = direction.target(application);
        match selected_change {
            EnvironmentObjectChange::Unchanged => {
                let observation = target.or(source).expect(
                    "included environment application has at least one selected observation",
                );
                let marker = match observation.availability {
                    Availability::Available => style.green("+"),
                    Availability::Unavailable => style.yellow("?"),
                    Availability::Error => style.red("!"),
                };
                write_environment_application(output, observation, &marker, None)?;
            }
            EnvironmentObjectChange::Inserted => {
                if let Some(observation) = target {
                    write_environment_application(
                        output,
                        observation,
                        &style.green("+"),
                        Some("inserted"),
                    )?;
                }
            }
            EnvironmentObjectChange::Deleted => {
                if let Some(observation) = source {
                    write_environment_application(
                        output,
                        observation,
                        &style.red("-"),
                        Some("deleted"),
                    )?;
                }
            }
            EnvironmentObjectChange::Modified => {
                if let Some(observation) = source {
                    write_environment_application(
                        output,
                        observation,
                        &style.red("-"),
                        Some("before"),
                    )?;
                }
                if let Some(observation) = target {
                    write_environment_application(
                        output,
                        observation,
                        &style.green("+"),
                        Some("modified"),
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn write_environment_application(
    output: &mut impl Write,
    observation: &EnvironmentApplicationObservation,
    marker: &str,
    change: Option<&str>,
) -> io::Result<()> {
    let location = observation
        .resolved_path
        .as_deref()
        .unwrap_or("not resolved");
    let change = change.map_or_else(String::new, |change| format!(" · {change}"));
    writeln!(
        output,
        "    {marker} {:<16} {} · {} PATH entries{change}",
        observation.name, location, observation.searched_path_count
    )
}

fn write_env_add(
    output: &mut impl Write,
    store: &LocalEnvironmentStore,
    result: &EnvironmentAddResult,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{}", style.cyan_bold("ENVIRONMENT ADMISSION"))?;
    match &result.index {
        Some(index) => {
            writeln!(output, "  Index                  {}", index.index_id)?;
            writeln!(
                output,
                "  Snapshot               {}",
                index.snapshot.semantic_digest
            )?;
        }
        None => writeln!(
            output,
            "  Index                  matches HEAD · retained index cleared"
        )?,
    }
    writeln!(
        output,
        "  Selection              {} capability changes admitted",
        result.staged_changes
    )?;
    writeln!(
        output,
        "  Commit delta           {} changes · {}",
        result.staged_delta.changes.len(),
        delta_assessment_label(result.staged_delta.summary.assessment, style)
    )?;
    writeln!(
        output,
        "  Working delta          {} changes remain unstaged · {}",
        result.remaining_changes,
        delta_assessment_label(result.unstaged_delta.summary.assessment, style)
    )?;
    writeln!(
        output,
        "  Retention              {} · local only",
        store.index_path().display()
    )?;
    Ok(())
}

fn write_env_log(
    output: &mut impl Write,
    workspace: &Path,
    log: &EnvironmentLog,
    projections: &[EnvironmentOperatorProjection],
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "{}", style.cyan_bold("REY ENV LOG"))?;
    writeln!(output, "  Workspace              {}", workspace.display())?;
    writeln!(
        output,
        "  History                {} total · {} shown · newest first",
        log.total_commits, log.selected_commits
    )?;
    if log.entries.is_empty() {
        writeln!(output)?;
        writeln!(output, "No environment commits.")?;
        return Ok(());
    }
    for (entry, projection) in log.entries.iter().zip(projections) {
        let commit = &entry.commit;
        let head = log.head_commit_id.as_ref() == Some(&commit.commit_id);
        let direction = EnvironmentProjectionDirection::HeadToWorking;
        let variable_count = environment_plane_target_count(&projection.variables, direction);
        let application_count = environment_plane_target_count(&projection.applications, direction);
        let input_count = environment_plane_target_count(&projection.inputs, direction);
        let reference_count = environment_plane_target_count(&projection.references, direction);
        let changed_variables = environment_plane_changed_count(&projection.variables, direction);
        let changed_applications =
            environment_plane_changed_count(&projection.applications, direction);
        let changed_inputs = environment_plane_changed_count(&projection.inputs, direction);
        let changed_references = environment_plane_changed_count(&projection.references, direction);
        let changed_capabilities = entry.delta.changes.len() as u64;
        writeln!(output)?;
        writeln!(
            output,
            "{}{}",
            style.bold(&format!(
                "commit ENV@{} {}",
                commit.sequence, commit.commit_id
            )),
            if head { " (HEAD)" } else { "" }
        )?;
        writeln!(
            output,
            "Parent: {}",
            commit.parent_commit_id.as_ref().map_or_else(
                || "EMPTY".to_owned(),
                |parent| format!("ENV@{} {parent}", commit.sequence.saturating_sub(1))
            )
        )?;
        writeln!(output, "Date:   {}", format_environment_commit_date(commit))?;
        writeln!(output)?;
        for line in commit.message.lines() {
            writeln!(output, "    {line}")?;
        }
        writeln!(output)?;
        writeln!(
            output,
            "  Evidence               {} → {} · {} · {} authoritative capability {}",
            entry.delta.source_label,
            entry.delta.target_label,
            delta_assessment_label(entry.delta.summary.assessment, style),
            changed_capabilities,
            plural(changed_capabilities, "change", "changes")
        )?;
        writeln!(
            output,
            "  Environment            {} {} · {} {} · {} {} · {} {} · {}",
            variable_count,
            plural(variable_count, "variable", "variables"),
            application_count,
            plural(application_count, "application", "applications"),
            input_count,
            plural(input_count, "input", "inputs"),
            reference_count,
            plural(reference_count, "reference", "references"),
            if commit.snapshot.complete {
                "complete"
            } else {
                "incomplete"
            }
        )?;
        writeln!(
            output,
            "  Changes                {} {} · {} {} · {} {} · {} {}",
            changed_variables,
            plural(changed_variables, "variable", "variables"),
            changed_applications,
            plural(changed_applications, "application", "applications"),
            changed_inputs,
            plural(changed_inputs, "input", "inputs"),
            changed_references,
            plural(changed_references, "reference", "references")
        )?;
        match environment_snapshot_mapping(&commit.snapshot) {
            Some((source_path, schema)) => writeln!(
                output,
                "  Reasoning map          {} · {}",
                source_path, schema
            )?,
            None => writeln!(
                output,
                "  Reasoning map          none · process discovery only"
            )?,
        }
        if log.patch {
            write_environment_transition_planes(
                output,
                &entry.delta,
                projection,
                direction,
                style,
            )?;
        }
    }
    Ok(())
}

fn format_environment_commit_date(commit: &EnvironmentCommit) -> String {
    DateTime::<Utc>::from_timestamp(commit.committed_at_unix, 0).map_or_else(
        || "invalid timestamp".to_owned(),
        |date| date.format("%a %b %e %H:%M:%S %Y %z").to_string(),
    )
}

fn environment_snapshot_mapping(snapshot: &CapabilitySnapshot) -> Option<(&str, &str)> {
    snapshot
        .capabilities
        .iter()
        .filter(|capability| {
            capability.provider_id == "rey.env-map"
                && capability.capability_kind == "environment_map"
        })
        .max_by_key(|capability| capability.provider_revision)
        .and_then(|capability| {
            Some((
                capability.resolved_location.as_deref()?,
                capability.version.as_deref()?,
            ))
        })
}

fn write_env_diff(
    output: &mut impl Write,
    workspace: &Path,
    diff: &EnvironmentDiff,
    projection: &EnvironmentOperatorProjection,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let (view, direction) = match diff.mode {
        EnvironmentDiffMode::Unstaged => {
            ("UNSTAGED", EnvironmentProjectionDirection::IndexToWorking)
        }
        EnvironmentDiffMode::Staged => ("STAGED", EnvironmentProjectionDirection::HeadToIndex),
    };
    let changed_capabilities = diff.delta.changes.len() as u64;
    writeln!(output)?;
    writeln!(
        output,
        "{}",
        style.cyan_bold(&format!(
            "REY ENV DIFF · {} → {}",
            diff.delta.source_label, diff.delta.target_label
        ))
    )?;
    writeln!(output, "  View                   {view}")?;
    writeln!(output, "  Workspace              {}", workspace.display())?;
    writeln!(
        output,
        "  Evidence               {} · {} authoritative capability {}",
        delta_assessment_label(diff.delta.summary.assessment, style),
        changed_capabilities,
        plural(changed_capabilities, "change", "changes")
    )?;
    write_environment_transition_planes(output, &diff.delta, projection, direction, style)
}

fn write_environment_transition_planes(
    output: &mut impl Write,
    delta: &CapabilityDelta,
    projection: &EnvironmentOperatorProjection,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let variable_count = environment_plane_count(&projection.variables, direction);
    let changed_variables = environment_plane_changed_count(&projection.variables, direction);
    writeln!(output)?;
    writeln!(output, "{}", style.bold("01 / DIRECTED TEXT"))?;
    writeln!(
        output,
        "Environment variables · {variable_count} tracked · {changed_variables} changed"
    )?;
    writeln!(
        output,
        "{}",
        style.dim(&format!(
            "@@ {} → {}",
            delta.source_label, delta.target_label
        ))
    )?;
    if variable_count == 0 {
        writeln!(
            output,
            "  (no process seeds or explicit mapping variables observed)"
        )?;
    } else {
        for variable in projection
            .variables
            .iter()
            .filter(|variable| direction.includes(variable))
        {
            write_environment_variable_diff(output, variable, direction, style)?;
        }
    }

    write_environment_application_planes(
        output,
        projection,
        direction,
        &delta.target_label,
        &delta.target_snapshot,
        style,
    )?;

    let input_count = environment_plane_count(&projection.inputs, direction);
    let changed_inputs = environment_plane_changed_count(&projection.inputs, direction);
    let reference_count = environment_plane_count(&projection.references, direction);
    let changed_references = environment_plane_changed_count(&projection.references, direction);
    writeln!(output)?;
    writeln!(output, "{}", style.bold("03"))?;
    writeln!(output, "{}", style.bold("REFERENCE PLANE"))?;
    writeln!(output, "Inputs and topology")?;
    writeln!(
        output,
        "  INPUTS · {input_count} tracked · {changed_inputs} changed"
    )?;
    if input_count == 0 {
        writeln!(output, "    NONE")?;
    } else {
        for input in projection
            .inputs
            .iter()
            .filter(|input| direction.includes(input))
        {
            write_environment_input_diff(output, input, direction, style)?;
        }
    }
    writeln!(
        output,
        "  TOPOLOGY · {reference_count} declared edges · {changed_references} changed"
    )?;
    if reference_count == 0 {
        writeln!(output, "    NONE")?;
    } else {
        for reference in projection
            .references
            .iter()
            .filter(|reference| direction.includes(reference))
        {
            write_environment_reference_diff(output, reference, direction, style)?;
        }
    }
    Ok(())
}

fn environment_plane_count<T>(
    objects: &[EnvironmentObjectStatus<T>],
    direction: EnvironmentProjectionDirection,
) -> u64 {
    objects
        .iter()
        .filter(|object| direction.includes(object))
        .count() as u64
}

fn environment_plane_target_count<T>(
    objects: &[EnvironmentObjectStatus<T>],
    direction: EnvironmentProjectionDirection,
) -> u64 {
    objects
        .iter()
        .filter(|object| direction.target(object).is_some())
        .count() as u64
}

fn environment_plane_changed_count<T>(
    objects: &[EnvironmentObjectStatus<T>],
    direction: EnvironmentProjectionDirection,
) -> u64 {
    objects
        .iter()
        .filter(|object| {
            direction.includes(object)
                && direction.change(object) != EnvironmentObjectChange::Unchanged
        })
        .count() as u64
}

fn environment_application_count(
    applications: &[EnvironmentObjectStatus<EnvironmentApplicationObservation>],
    direction: EnvironmentProjectionDirection,
    availability: Availability,
) -> u64 {
    applications
        .iter()
        .filter(|application| {
            direction
                .target(application)
                .is_some_and(|application| application.availability == availability)
        })
        .count() as u64
}

fn write_environment_input_diff(
    output: &mut impl Write,
    input: &EnvironmentObjectStatus<EnvironmentInputObservation>,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> io::Result<()> {
    let source = direction.source(input);
    let target = direction.target(input);
    match direction.change(input) {
        EnvironmentObjectChange::Unchanged => {
            if let Some(observation) = target.or(source) {
                writeln!(
                    output,
                    "{}",
                    style.dim(&format!("    {}", environment_input_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Inserted => {
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("  + {}", environment_input_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Deleted => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("  - {}", environment_input_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Modified => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("  - {}", environment_input_line(observation)))
                )?;
            }
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("  + {}", environment_input_line(observation)))
                )?;
            }
        }
    }
    Ok(())
}

fn environment_input_line(observation: &EnvironmentInputObservation) -> String {
    let digest = observation
        .content_digest
        .as_deref()
        .map(compact_digest)
        .unwrap_or_else(|| "unbound".to_owned());
    let requirement = if observation.required {
        "required"
    } else {
        "optional"
    };
    format!(
        "{} · {requirement} · {} · {} bytes · {}",
        observation.path,
        digest,
        observation.byte_length.unwrap_or(0),
        environment_availability_label(observation.availability)
    )
}

fn write_environment_reference_diff(
    output: &mut impl Write,
    reference: &EnvironmentObjectStatus<EnvironmentReferenceObservation>,
    direction: EnvironmentProjectionDirection,
    style: TerminalStyle,
) -> io::Result<()> {
    let source = direction.source(reference);
    let target = direction.target(reference);
    match direction.change(reference) {
        EnvironmentObjectChange::Unchanged => {
            if let Some(observation) = target.or(source) {
                writeln!(
                    output,
                    "{}",
                    style.dim(&format!("    {}", environment_reference_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Inserted => {
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("  + {}", environment_reference_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Deleted => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("  - {}", environment_reference_line(observation)))
                )?;
            }
        }
        EnvironmentObjectChange::Modified => {
            if let Some(observation) = source {
                writeln!(
                    output,
                    "{}",
                    style.red(&format!("  - {}", environment_reference_line(observation)))
                )?;
            }
            if let Some(observation) = target {
                writeln!(
                    output,
                    "{}",
                    style.green(&format!("  + {}", environment_reference_line(observation)))
                )?;
            }
        }
    }
    Ok(())
}

fn environment_reference_line(observation: &EnvironmentReferenceObservation) -> String {
    format!(
        "{} --{}--> {}",
        observation.from, observation.relation, observation.to
    )
}

const fn environment_availability_label(availability: Availability) -> &'static str {
    match availability {
        Availability::Available => "available",
        Availability::Unavailable => "unavailable",
        Availability::Error => "error",
    }
}

const fn plural<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

fn write_capability_change(
    output: &mut impl Write,
    change: &CapabilityChange,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let (marker, kind) = match change.kind {
        CapabilityChangeKind::Inserted => (style.green("+"), "inserted"),
        CapabilityChangeKind::Deleted => (style.red("-"), "deleted"),
        CapabilityChangeKind::Modified => (style.yellow("~"), "modified"),
    };
    writeln!(
        output,
        "  {marker} {}@{} / {} ({kind})",
        change.key.provider_id, change.key.provider_revision, change.key.capability_id
    )?;
    match change.kind {
        CapabilityChangeKind::Inserted => {
            write_capability_record_projection(output, "after", change.after.as_ref())?;
        }
        CapabilityChangeKind::Deleted => {
            write_capability_record_projection(output, "before", change.before.as_ref())?;
        }
        CapabilityChangeKind::Modified => {
            for field in &change.changed_fields {
                if field == "provenance" || field == "error_detail" {
                    writeln!(
                        output,
                        "      {field}: changed · structured value omitted; inspect with `rey env diff --format json`"
                    )?;
                    continue;
                }
                writeln!(
                    output,
                    "      {field}: {} → {}",
                    capability_field(change.before.as_ref(), field)?,
                    capability_field(change.after.as_ref(), field)?
                )?;
            }
        }
    }
    Ok(())
}

fn write_capability_record_projection(
    output: &mut impl Write,
    side: &str,
    record: Option<&CapabilitySemanticRecord>,
) -> Result<(), CliError> {
    let Some(record) = record else {
        writeln!(output, "      {side}: null")?;
        return Ok(());
    };
    writeln!(
        output,
        "      {side}: kind={} · availability={} · trust={}",
        record.capability_kind,
        record.availability.as_str(),
        record.trust_class.as_str()
    )?;
    writeln!(
        output,
        "      version={} · location={} · content={}",
        record.version.as_deref().unwrap_or("null"),
        record.resolved_location.as_deref().unwrap_or("null"),
        record.content_digest.as_deref().unwrap_or("null")
    )?;
    writeln!(
        output,
        "      operations={} · enforced_limits={} · unsupported_limits={}",
        serde_json::to_string(&record.operations)?,
        serde_json::to_string(&record.enforced_limits)?,
        serde_json::to_string(&record.unsupported_limits)?
    )?;
    Ok(())
}

fn capability_field(
    record: Option<&CapabilitySemanticRecord>,
    field: &str,
) -> Result<String, CliError> {
    let Some(record) = record else {
        return Ok("null".to_owned());
    };
    let value = serde_json::to_value(record)?;
    let rendered = value
        .get(field)
        .map_or_else(|| "<missing>".to_owned(), serde_json::Value::to_string);
    const MAX_CAPABILITY_FIELD_CHARS: usize = 180;
    if rendered.chars().count() <= MAX_CAPABILITY_FIELD_CHARS {
        return Ok(rendered);
    }
    let prefix = rendered.chars().take(140).collect::<String>();
    Ok(format!(
        "{prefix}…<{} chars omitted>",
        rendered.chars().count().saturating_sub(140)
    ))
}

fn write_editor_generate(
    output: &mut impl Write,
    result: &EditorGenerateResult,
) -> Result<(), CliError> {
    writeln!(
        output,
        "{} deterministic terrain source {}",
        if result.changed {
            "Generated"
        } else {
            "Verified"
        },
        result.source.source_id
    )?;
    writeln!(output, "project      {}", result.project_path)?;
    writeln!(
        output,
        "bootstrap    {}",
        if result.project_created {
            "created scene project"
        } else {
            "existing scene project"
        }
    )?;
    writeln!(output, "output       {}", result.output_path)?;
    writeln!(
        output,
        "generator    {} · seed {}",
        result.recipe.generator, result.recipe.seed
    )?;
    writeln!(
        output,
        "bounds       [{:.6}, {:.6}] → [{:.6}, {:.6}]",
        result.recipe.bounds.west,
        result.recipe.bounds.south,
        result.recipe.bounds.east,
        result.recipe.bounds.north
    )?;
    writeln!(
        output,
        "coverage     {} features · {} coordinate positions",
        result.feature_count, result.coordinate_count
    )?;
    writeln!(
        output,
        "effects      uplift={:.3} · strength={:.3}±{:.3} · roughness={:.3}±{:.3} · falloff={:.3}",
        result.recipe.parameters.uplift_ratio,
        result.recipe.parameters.strength,
        result.recipe.parameters.strength_jitter,
        result.recipe.parameters.roughness,
        result.recipe.parameters.roughness_jitter,
        result.recipe.parameters.falloff
    )?;
    writeln!(
        output,
        "geometry     scale={:.3}..{:.3} · anisotropy={:.3} · orientation={:.1}°±{:.1}° · edge jitter={:.3}",
        result.recipe.parameters.scale_min,
        result.recipe.parameters.scale_max,
        result.recipe.parameters.anisotropy,
        result.recipe.parameters.orientation_degrees,
        result.recipe.parameters.orientation_jitter_degrees,
        result.recipe.parameters.edge_jitter
    )?;
    writeln!(
        output,
        "authority    generated WORKING candidate · run `rey editor diff`, then `rey editor add`"
    )?;
    Ok(())
}

fn write_editor_status(
    output: &mut impl Write,
    status: &EditorStatus,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let head = status.head.as_ref().map_or_else(
        || "no commits yet".to_owned(),
        |commit| format!("SCENE@{}", commit.sequence),
    );
    writeln!(output, "On scene {head}")?;

    if !status.initialized {
        writeln!(output)?;
        writeln!(output, "No scene project initialized.")?;
        writeln!(
            output,
            "Use `rey editor generate terrain --help` to create WORKING in `.rey/editor`."
        )?;
        return Ok(());
    }

    write_editor_status_changes(
        output,
        &status.staged,
        "Changes to be committed:",
        "  (use \"rey editor diff --staged\" to review)",
        true,
        style,
    )?;
    write_editor_status_changes(
        output,
        &status.unstaged,
        "Changes not staged for scene commit:",
        "  (use \"rey editor diff\" to review; \"rey editor add\" to stage)",
        false,
        style,
    )?;

    writeln!(output)?;
    match status.state {
        rey::editor::EditorWorkingState::Clean => {
            writeln!(output, "nothing to commit, working scene clean")?
        }
        rey::editor::EditorWorkingState::Working => writeln!(
            output,
            "no changes added to scene commit (use `rey editor add` to stage)"
        )?,
        rey::editor::EditorWorkingState::Staged => {
            writeln!(output, "changes staged in the scene index")?
        }
        rey::editor::EditorWorkingState::Mixed => writeln!(
            output,
            "staged changes and unstaged scene changes are both present"
        )?,
    }
    Ok(())
}

fn write_editor_status_changes(
    output: &mut impl Write,
    changes: &SceneChangeSet,
    heading: &str,
    hint: &str,
    staged: bool,
    style: TerminalStyle,
) -> io::Result<()> {
    if changes.changes.is_empty() {
        return Ok(());
    }
    writeln!(output)?;
    writeln!(output, "{heading}")?;
    writeln!(output, "{hint}")?;

    for change in &changes.changes {
        let change_label = match change.change_kind {
            SceneChangeKind::Inserted => "new:",
            SceneChangeKind::Deleted => "deleted:",
            SceneChangeKind::Modified => "modified:",
        };
        let object_kind = match change.object_kind {
            SceneObjectKind::Source => "source",
            SceneObjectKind::Feature => "feature",
        };
        let line = format!("{change_label:<10} {object_kind}: {}", change.object_id);
        let line = if staged {
            style.green(&line)
        } else {
            style.red(&line)
        };
        writeln!(output, "        {line}")?;
    }
    Ok(())
}

fn write_editor_diff(output: &mut impl Write, diff: &SceneChangeSet) -> Result<(), CliError> {
    writeln!(output, "SCENE CHANGE SET")?;
    writeln!(
        output,
        "Direction      {} → {}",
        diff.source_label, diff.target_label
    )?;
    writeln!(
        output,
        "Assessment     {} · +{} -{} ~{}",
        scene_assessment(diff.assessment),
        diff.inserted,
        diff.deleted,
        diff.modified
    )?;
    writeln!(
        output,
        "Source         {}",
        diff.source_revision
            .as_ref()
            .map_or("none", SemanticDigest::as_str)
    )?;
    writeln!(
        output,
        "Target         {}",
        diff.target_revision
            .as_ref()
            .map_or("none", SemanticDigest::as_str)
    )?;
    const HUMAN_CHANGE_LIMIT: usize = 64;
    for change in diff.changes.iter().take(HUMAN_CHANGE_LIMIT) {
        let marker = match change.change_kind {
            SceneChangeKind::Inserted => '+',
            SceneChangeKind::Deleted => '-',
            SceneChangeKind::Modified => '~',
        };
        let kind = match change.object_kind {
            SceneObjectKind::Source => "source",
            SceneObjectKind::Feature => "feature",
        };
        writeln!(output, "{marker} {kind:<7} {}", change.object_id)?;
    }
    if diff.changes.len() > HUMAN_CHANGE_LIMIT {
        writeln!(
            output,
            "… {} additional changes retained in structured output",
            diff.changes.len() - HUMAN_CHANGE_LIMIT
        )?;
    }
    if diff.changes.is_empty() {
        writeln!(output, "No semantic scene changes")?;
    }
    Ok(())
}

fn write_editor_add(output: &mut impl Write, result: &EditorAddResult) -> Result<(), CliError> {
    writeln!(output)?;
    writeln!(output, "SCENE INDEX")?;
    writeln!(
        output,
        "  Snapshot               {}",
        result.snapshot.snapshot_revision
    )?;
    writeln!(
        output,
        "  Scene                  {} sources · {} features · {} markers · {} positions",
        result.snapshot.coverage.sources,
        result.snapshot.coverage.features,
        result.snapshot.coverage.markers,
        result.snapshot.coverage.coordinates
    )?;
    writeln!(
        output,
        "  Selection              {} scene changes {}",
        result.delta.changes.len(),
        if result.staged {
            "staged"
        } else {
            "verified unchanged"
        }
    )?;
    writeln!(
        output,
        "  Commit delta           {} · +{} -{} ~{}",
        scene_assessment(result.delta.assessment),
        result.delta.inserted,
        result.delta.deleted,
        result.delta.modified
    )?;
    writeln!(
        output,
        "  Authority              candidate only · native objects frozen · not admitted"
    )?;
    Ok(())
}

fn write_editor_commit(
    output: &mut impl Write,
    result: &EditorCommitResult,
) -> Result<(), CliError> {
    writeln!(
        output,
        "[SCENE@{} {}] {}",
        result.commit.sequence, result.commit.commit_id, result.commit.message
    )?;
    writeln!(
        output,
        " validation complete · snapshot {} · {} sources · {} features · {} omissions",
        result.package.snapshot.snapshot_revision,
        result.package.snapshot.coverage.sources,
        result.package.snapshot.coverage.features,
        result.package.snapshot.omissions.len()
    )?;
    writeln!(
        output,
        " {} scene changes · +{} -{} ~{}",
        result.package.change_set.changes.len(),
        result.package.change_set.inserted,
        result.package.change_set.deleted,
        result.package.change_set.modified
    )?;
    writeln!(
        output,
        " package {} · candidate only",
        result.package.package_id
    )?;
    writeln!(
        output,
        " admission {} · {} · admitted={} · /explore unchanged",
        result.admission_request.request_id,
        result.admission_request.status,
        result.admission_request.admitted
    )?;
    Ok(())
}

fn write_editor_log(
    output: &mut impl Write,
    log: &EditorLog,
    style: TerminalStyle,
) -> Result<(), CliError> {
    writeln!(output, "REY EDITOR LOG")?;
    writeln!(
        output,
        "  History                {} total · {} shown · newest first",
        log.total_commits, log.selected_commits
    )?;
    if log.entries.is_empty() {
        writeln!(output)?;
        writeln!(output, "No scene commits.")?;
        return Ok(());
    }
    for entry in &log.entries {
        let commit = &entry.commit;
        let package = &entry.package;
        let head = log.head_commit_id.as_ref() == Some(&commit.commit_id);
        writeln!(output)?;
        writeln!(
            output,
            "{}{}",
            style.bold(&format!(
                "commit SCENE@{} {}",
                commit.sequence, commit.commit_id
            )),
            if head { " (HEAD)" } else { "" }
        )?;
        writeln!(
            output,
            "Parent: {}",
            commit.parent_commit_id.as_ref().map_or_else(
                || "EMPTY".to_owned(),
                |parent| format!("SCENE@{} {parent}", commit.sequence.saturating_sub(1))
            )
        )?;
        writeln!(output, "Date:   {}", format_scene_commit_date(commit))?;
        writeln!(output)?;
        for line in commit.message.lines() {
            writeln!(output, "    {line}")?;
        }
        writeln!(output)?;
        writeln!(
            output,
            "  Scene                  {} sources · {} features · {} markers · {} positions",
            package.snapshot.coverage.sources,
            package.snapshot.coverage.features,
            package.snapshot.coverage.markers,
            package.snapshot.coverage.coordinates
        )?;
        writeln!(
            output,
            "  Snapshot               {}",
            package.snapshot.snapshot_revision
        )?;
        writeln!(
            output,
            "  Package                {} · candidate only · no admission claim",
            package.package_id
        )?;
        writeln!(
            output,
            "  Delta                  {} → {} · {} · +{} -{} ~{}",
            package.change_set.source_label,
            package.change_set.target_label,
            scene_assessment(package.change_set.assessment),
            package.change_set.inserted,
            package.change_set.deleted,
            package.change_set.modified
        )?;
        if log.patch {
            writeln!(output)?;
            write_editor_diff(output, &package.change_set)?;
        }
    }
    Ok(())
}

fn format_scene_commit_date(commit: &SceneCommit) -> String {
    DateTime::<Utc>::from_timestamp(commit.committed_at_unix, 0).map_or_else(
        || "invalid timestamp".to_owned(),
        |date| date.format("%a %b %e %H:%M:%S %Y %z").to_string(),
    )
}

const fn scene_assessment(assessment: DeltaAssessment) -> &'static str {
    match assessment {
        DeltaAssessment::Equal => "EQUAL",
        DeltaAssessment::Different => "DIFFERENT",
        DeltaAssessment::Inconclusive => "INCONCLUSIVE",
    }
}

fn delta_assessment_label(assessment: DeltaAssessment, style: TerminalStyle) -> String {
    match assessment {
        DeltaAssessment::Equal => style.green("EQUAL"),
        DeltaAssessment::Different => style.yellow("DIFFERENT"),
        DeltaAssessment::Inconclusive => style.red("INCONCLUSIVE"),
    }
}

fn write_json_line(output: &mut impl Write, value: &impl Serialize) -> Result<(), CliError> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    Ok(())
}

fn json_cell(value: &impl Serialize) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

#[derive(Debug, Error)]
enum CliError {
    #[error(
        "workloads create requires the workspace catalog; built-in conformance workloads are immutable"
    )]
    CreateRequiresWorkspaceCatalog,
    #[error("workload admission history exists only for the workspace catalog")]
    AdmissionRequiresWorkspaceCatalog,
    #[error("workspace workload qualification requires --staged and a nonempty INDEX")]
    WorkspaceTestRequiresIndex,
    #[error(
        "workspace workload status reports the complete HEAD, INDEX, and WORKING portfolio; omit the workload id"
    )]
    WorkspaceStatusIsPortfolio,
    #[error("limits must be greater than zero")]
    InvalidLimit,
    #[error(
        "full recomputation evidence limit {actual} is outside the supported range 1..={max} bytes"
    )]
    InvalidRecomputationLimit { actual: u64, max: u64 },
    #[error("source-mining runs require at least one workspace-relative --source path")]
    MissingSourceFiles,
    #[error("context-anchor-survey runs require at least one workspace-relative --source seed")]
    MissingTopographySeeds,
    #[error("text workload runs require --input")]
    MissingWorkloadInput,
    #[error("selected workload catalog contains no admitted workload packages")]
    EmptyWorkloadCatalog,
    #[error("portfolio-attention runs use retained inputs and reject --input or source options")]
    UnexpectedPortfolioInput,
    #[error("context-anchor-survey derives its input from --source seeds and rejects --input")]
    UnexpectedTopographyInput,
    #[error("source context windows are not valid for context-anchor-survey")]
    UnexpectedTopographyContext,
    #[error("--source and source-context options are only valid for a source-mining workload")]
    UnexpectedSourceFiles,
    #[error("--patch requires human table output")]
    PatchFormat,
    #[error("workspace {path} could not be resolved: {source}")]
    Workspace { path: PathBuf, source: io::Error },
    #[error("workspace {0} is not a directory")]
    WorkspaceDirectory(PathBuf),
    #[error("relative state directory {0} escapes the workspace boundary")]
    StateDirectoryEscape(PathBuf),
    #[error("Git executable is unavailable on the process-owned PATH")]
    GitUnavailable,
    #[error("explicit workspace contains no Git repository")]
    GitRepositoryAbsent,
    #[error("Git activation trigger {path} could not be read: {source}")]
    GitTriggerInput { path: PathBuf, source: io::Error },
    #[error("Git activation trigger must be a regular non-symlinked file: {0}")]
    GitTriggerInputType(PathBuf),
    #[error("Git activation trigger resolves outside the workspace: {0}")]
    GitTriggerInputEscape(PathBuf),
    #[error("Git activation trigger exceeds {0} bytes")]
    GitTriggerInputLimit(u64),
    #[error("workload activation admission requires an exact retained environment snapshot")]
    ActivationEnvironmentRequired,
    #[error("channel graph {path} could not be read: {source}")]
    ChannelInput { path: PathBuf, source: io::Error },
    #[error("channel graph must be a regular non-symlinked file: {0}")]
    ChannelInputType(PathBuf),
    #[error("channel graph resolves outside the workspace: {0}")]
    ChannelInputEscape(PathBuf),
    #[error("channel graph path is not valid UTF-8: {0}")]
    ChannelInputEncoding(PathBuf),
    #[error("channel graph exceeds {0} bytes")]
    ChannelInputLimit(u64),
    #[error("unknown admitted channel message {0}")]
    UnknownChannelMessage(String),
    #[error("unknown admitted relay {0}")]
    UnknownRelay(String),
    #[error("unknown admitted channel application {0}")]
    UnknownChannelApplication(String),
    #[error("unknown admitted polling beacon {0}")]
    UnknownBeacon(String),
    #[error("relay requires an admitted environment HEAD")]
    NoEnvironmentHead,
    #[error("environment HEAD does not admit communications capability {0}")]
    UnadmittedChannelApplication(String),
    #[error("admitted channel application {0} has drifted from environment HEAD")]
    ChannelApplicationDrift(String),
    #[error("admitted message channel does not match the relay source channel")]
    RelaySourceMismatch,
    #[error("relay executable {path} could not be inspected: {source}")]
    RelayExecutable { path: PathBuf, source: io::Error },
    #[error("relay executable must be a regular non-symlinked file: {0}")]
    RelayExecutableType(PathBuf),
    #[error("journal proposal {path} could not be read: {source}")]
    JournalInput { path: PathBuf, source: io::Error },
    #[error("journal proposal must be a regular non-symlinked file: {0}")]
    JournalInputType(PathBuf),
    #[error("journal proposal resolves outside the workspace: {0}")]
    JournalInputEscape(PathBuf),
    #[error("journal proposal exceeds {0} bytes")]
    JournalInputLimit(u64),
    #[error(
        "rey journal add accepts agent-authored entries; humans write through the loopback UI and system entries are derived"
    )]
    JournalCliAuthor,
    #[error(transparent)]
    Rey(#[from] ReyError),
    #[error(transparent)]
    Discovery(#[from] rey_environment::DiscoveryError),
    #[error(transparent)]
    Command(#[from] rey_environment::CommandError),
    #[error(transparent)]
    Delta(#[from] rey_diff::DeltaError),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
    #[error(transparent)]
    Portfolio(#[from] rey_runtime::PortfolioError),
    #[error(transparent)]
    Git(#[from] rey_git::GitError),
    #[error(transparent)]
    GitState(#[from] LocalGitStateError),
    #[error(transparent)]
    WorkloadState(#[from] LocalWorkloadStateError),
    #[error(transparent)]
    WorkloadCatalog(#[from] WorkloadCatalogError),
    #[error(transparent)]
    EnvironmentState(#[from] LocalEnvironmentHistoryError),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    ChannelGraph(#[from] ChannelGraphError),
    #[error(transparent)]
    Editor(#[from] EditorError),
    #[error(transparent)]
    Ui(#[from] ui::UiError),
    #[error("YAML input failed: {0}")]
    Yaml(#[from] serde_saphyr::Error),
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output failed: {0}")]
    Output(#[from] io::Error),
}

#[cfg(test)]
mod terminal_style_tests {
    use super::*;

    #[test]
    fn environment_and_workload_admission_rows_share_positional_colors() {
        let style = TerminalStyle { enabled: true };
        let changes = WorkloadChangeSet {
            schema: "rey.workload-change-set.v1".to_owned(),
            source_label: "INDEX".to_owned(),
            target_label: "WORKING".to_owned(),
            source_revision: None,
            target_revision: None,
            assessment: DeltaAssessment::Different,
            inserted: 1,
            deleted: 0,
            modified: 0,
            changes: vec![rey::workloads::WorkloadChange {
                workload_id: "alpha".to_owned(),
                change_kind: WorkloadChangeKind::Inserted,
                source_revision: None,
                target_revision: None,
            }],
        };

        for (staged, color) in [(true, "32"), (false, "31")] {
            let mut workload = Vec::new();
            write_workload_change_lines(&mut workload, &changes, staged, style).unwrap();
            assert_eq!(
                String::from_utf8(workload).unwrap(),
                format!("        \u{1b}[{color}mnew:       workload: alpha\u{1b}[0m\n")
            );

            let mut environment = Vec::new();
            write_environment_status_entry(
                &mut environment,
                EnvironmentObjectChange::Inserted,
                "environment variable: ALPHA",
                staged,
                style,
            )
            .unwrap();
            assert_eq!(
                String::from_utf8(environment).unwrap(),
                format!("        \u{1b}[{color}mnew:       environment variable: ALPHA\u{1b}[0m\n")
            );
        }
    }
}
