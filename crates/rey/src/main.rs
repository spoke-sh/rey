#![forbid(unsafe_code)]

mod ui;

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, BufRead, IsTerminal, Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use chrono::{DateTime, Utc};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use rey::{
    ReyError,
    channels::{
        ChannelApplyResult, ChannelDiff, ChannelGraph, ChannelGraphChange, ChannelGraphError,
        ChannelGraphSource, ChannelObjectKind, ChannelStatus, ChannelWorkingState,
        LocalChannelStore, MAX_CHANNEL_GRAPH_INPUT_BYTES,
    },
    editor::{
        EditorAddResult, EditorError, EditorImportResult, EditorPackageResult, EditorStatus,
        LocalEditorStore, SceneCandidateSnapshot, SceneChangeKind, SceneChangeSet, SceneObjectKind,
        ScenePackage, SceneSourceRole,
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
    inspect_environment, inspect_environment_with_mapping,
    journal::{
        JournalAdmission, JournalAuthorKind, JournalEntryProposal, JournalError, JournalLog,
        LocalJournalStore, MAX_JOURNAL_PROPOSAL_BYTES,
    },
    workloads::{
        LocalWorkloadStateError, LocalWorkloadStore, ResolvedWorkload, WorkloadCatalog,
        WorkloadCatalogDescriptor, WorkloadCatalogError, WorkloadCreateResult, WorkloadDraft,
        WorkloadList, WorkloadRunView, WorkloadStatusBatch, WorkloadStatusView, WorkloadSummary,
        WorkloadTestBatch, derive_portfolio_snapshot, derive_workload_attention,
        fresh_qualification,
    },
};
use rey_core::{SemanticDigest, SemanticHasher};
use rey_diff::{
    CapabilityChange, CapabilityChangeKind, CapabilityDelta, CapabilitySemanticRecord,
    DeltaAssessment, SCENARIO_OUTPUT_DELTA_SCHEMA, ScenarioOutputDelta, SourceMatchChangeKind,
    TextLineKind, source_match_table_projection, text_patch_projection,
};
use rey_environment::{
    Availability, CapabilitySnapshot, DiscoveryLimits, EnvironmentMapLimits, SourceBindingLimits,
    VariableCapture,
};
use rey_locator::ResolutionLimits;
use rey_mining::{
    MiningCompleteness, MiningLimits, ProjectionPacket, TopographyLimits, TopographyPatch,
};
use rey_runtime::{
    BUILT_IN_PORTFOLIO_ATTENTION_WORKLOAD_ID, BUILT_IN_SOURCE_SEARCH_WORKLOAD_ID,
    CONTEXT_ANCHOR_SURVEY_WORKLOAD_ID, RunStatus, ScenarioEvaluation, ScenarioResult,
    SourceRunInput, TestStatus, TopographySurveyInput, WorkloadAttention, WorkloadDefinition,
    WorkloadRunResult, WorkloadTestResult, WorkloadValue, run_workload, run_workload_with_source,
    run_workload_with_topography, source_fixture_root, test_workload_with_observer_and_snapshot,
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
    /// Build immutable read-first scene candidates for explicit Rey admission.
    Editor(EditorArgs),
    /// Inspect, test, qualify, and execute bounded compute graphs.
    Workloads(WorkloadsArgs),
    /// Read and admit bounded collaboration journal entries.
    Journal(JournalArgs),
    /// Serve the Rey operator interface.
    Ui(UiArgs),
}

#[derive(Debug, Args)]
struct EditorArgs {
    /// Workspace containing the project and all imported native sources.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local editor-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    /// Workspace-relative rey.editor-project.v1 JSON document.
    #[arg(long, global = true, default_value = "rey.scene.json")]
    project: PathBuf,

    #[command(subcommand)]
    command: EditorCommand,
}

#[derive(Debug, Subcommand)]
enum EditorCommand {
    /// Create an empty OGC CRS84 scene project without staging it.
    Init(EditorInitArgs),
    /// Validate and register one workspace-contained GeoJSON source in WORKING.
    Import(EditorImportArgs),
    /// Show PACKAGE, INDEX, and WORKING state without changing it.
    Status(EditorOutputArgs),
    /// Show INDEX to WORKING changes, or PACKAGE to INDEX with --staged.
    Diff(EditorDiffArgs),
    /// Stage the exact verified project and immutable native-source objects.
    Add(EditorOutputArgs),
    /// Validate all declared sources and render the resulting scene snapshot.
    Validate(EditorOutputArgs),
    /// Package exactly the staged index and emit an unadmitted workload request.
    Package(EditorOutputArgs),
    /// Inspect an immutable scene package by exact package identity.
    Inspect(EditorInspectArgs),
}

#[derive(Debug, Args)]
struct EditorInitArgs {
    /// Stable scene project identity.
    #[arg(long)]
    id: String,

    /// Human receipt or typed JSON project.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorImportArgs {
    /// Workspace-relative GeoJSON Feature or FeatureCollection.
    source: PathBuf,

    /// Stable identity for this native survey source.
    #[arg(long)]
    id: String,

    /// Semantic projection role; line features do not imply discovered paths.
    #[arg(long, value_enum, default_value_t = EditorRoleArg::Features)]
    role: EditorRoleArg,

    /// Human receipt or typed JSON result.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorOutputArgs {
    /// Human evidence or typed JSON contract.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorDiffArgs {
    /// Compare the current PACKAGE candidate with the INDEX.
    #[arg(long)]
    staged: bool,

    /// Human semantic changes or typed JSON change set.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct EditorInspectArgs {
    /// Exact blake3 scene package identity.
    package_id: String,

    /// Human package evidence or typed JSON package.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum EditorRoleArg {
    Features,
    Markers,
    TerrainControl,
    Hydrology,
    Boundary,
}

impl From<EditorRoleArg> for SceneSourceRole {
    fn from(value: EditorRoleArg) -> Self {
        match value {
            EditorRoleArg::Features => Self::Features,
            EditorRoleArg::Markers => Self::Markers,
            EditorRoleArg::TerrainControl => Self::TerrainControl,
            EditorRoleArg::Hydrology => Self::Hydrology,
            EditorRoleArg::Boundary => Self::Boundary,
        }
    }
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
    /// Show whether Channel WORKING differs from the built-in graph.
    Status(ChannelStatusArgs),
    /// Display the semantic BUILT-IN to WORKING Channel graph delta.
    Diff(ChannelDiffArgs),
    /// Validate a workspace-contained YAML graph and write Channel WORKING.
    Apply(ChannelApplyArgs),
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
    /// Human semantic patch or typed JSON envelope.
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
    /// Workspace-contained YAML proposal using rey.journal-entry-proposal.v1.
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
    #[arg(long, default_value = "workloads")]
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
    #[arg(long, global = true, default_value = "workloads")]
    catalog_dir: PathBuf,

    #[command(subcommand)]
    command: WorkloadsCommand,
}

#[derive(Debug, Subcommand)]
enum WorkloadsCommand {
    /// Create a strict workload request for an external coding harness.
    Create(WorkloadCreateArgs),
    /// List resolved workloads and retained scenario progress without executing them.
    List(WorkloadListArgs),
    /// Show workload definitions, retained deltas, qualification, and latest run.
    Status(WorkloadStatusArgs),
    /// Execute required scenarios and retain their typed output deltas.
    Test(WorkloadTestArgs),
    /// Execute an exactly qualified graph against explicit or retained inputs.
    Run(WorkloadRunArgs),
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

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadListArgs {
    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadStatusArgs {
    /// Workload id; omit to show every workload in the selected catalog.
    workload_id: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadTestArgs {
    /// Workload id; omit to test every workload in the selected catalog.
    workload_id: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,

    /// Render matching evidence; repeat as -vv for exact identity bindings.
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count)]
    verbose: u8,
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
        Command::Editor(args) => editor_command(args),
        Command::Workloads(args) => workloads(args),
        Command::Journal(args) => journal_command(args),
        Command::Ui(args) => ui_command(args),
    }
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
        EditorCommand::Init(command) => editor_init(&store, &args.project, command),
        EditorCommand::Import(command) => editor_import(&store, &args.project, command),
        EditorCommand::Status(command) => editor_status(&store, &args.project, command),
        EditorCommand::Diff(command) => editor_diff(&store, &args.project, command),
        EditorCommand::Add(command) => editor_add(&store, &args.project, command),
        EditorCommand::Validate(command) => editor_validate(&store, &args.project, command),
        EditorCommand::Package(command) => editor_package(&store, command),
        EditorCommand::Inspect(command) => editor_inspect(&store, command),
    }
}

fn editor_init(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorInitArgs,
) -> Result<ExitCode, CliError> {
    let project = store.init_project(project_path, args.id)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &project)?,
        WorkloadOutputFormat::Table => {
            writeln!(
                stdout,
                "Initialized empty scene project in {}",
                project_path.display()
            )?;
            writeln!(stdout, "project      {}", project.project_id)?;
            writeln!(stdout, "schema       {}", project.schema)?;
            writeln!(stdout, "coordinates  OGC CRS84 · longitude/latitude")?;
            writeln!(
                stdout,
                "authority    WORKING only · nothing staged or admitted"
            )?;
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_import(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorImportArgs,
) -> Result<ExitCode, CliError> {
    let result = store.import_geojson(project_path, &args.source, args.id, args.role.into())?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_import(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_status(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorOutputArgs,
) -> Result<ExitCode, CliError> {
    let status = store.status(project_path)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &status)?,
        WorkloadOutputFormat::Table => write_editor_status(&mut stdout, &status)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_diff(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorDiffArgs,
) -> Result<ExitCode, CliError> {
    let diff = store.diff(project_path, args.staged)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        WorkloadOutputFormat::Table => write_editor_diff(&mut stdout, &diff)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_add(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorOutputArgs,
) -> Result<ExitCode, CliError> {
    let result = store.add(project_path)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_add(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_validate(
    store: &LocalEditorStore,
    project_path: &Path,
    args: EditorOutputArgs,
) -> Result<ExitCode, CliError> {
    let snapshot = store.validate(project_path)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &snapshot)?,
        WorkloadOutputFormat::Table => {
            writeln!(stdout, "SCENE VALIDATION · VERIFIED")?;
            write_editor_snapshot(&mut stdout, &snapshot)?;
            writeln!(
                stdout,
                "Admission       none · validation does not stage or admit"
            )?;
        }
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_package(store: &LocalEditorStore, args: EditorOutputArgs) -> Result<ExitCode, CliError> {
    let result = store.package()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_editor_package(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn editor_inspect(store: &LocalEditorStore, args: EditorInspectArgs) -> Result<ExitCode, CliError> {
    let package = store.inspect(&args.package_id)?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &package)?,
        WorkloadOutputFormat::Table => write_editor_package_inspection(&mut stdout, &package)?,
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
    let diff = store.diff()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &diff)?,
        WorkloadOutputFormat::Table => write_channel_diff(&mut stdout, &diff)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
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
            "rey: warning: UI is listening beyond loopback with unauthenticated Journal writes enabled; protect access externally"
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
        EnvCommand::Commit(command) => env_commit(&store, command),
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
    let catalog = match args.catalog {
        WorkloadCatalogSelection::Workspace => {
            WorkloadCatalog::load_workspace(&workspace, &args.catalog_dir)?
        }
        WorkloadCatalogSelection::Conformance => WorkloadCatalog::built_in_conformance()?,
    };
    match args.command {
        WorkloadsCommand::Create(command) => {
            if args.catalog != WorkloadCatalogSelection::Workspace {
                return Err(CliError::CreateRequiresWorkspaceCatalog);
            }
            workload_create(&workspace, &args.catalog_dir, command)
        }
        WorkloadsCommand::List(command) => workload_list(&store, &workspace, &catalog, command),
        WorkloadsCommand::Status(command) => workload_status(&store, &workspace, &catalog, command),
        WorkloadsCommand::Test(command) => workload_test(&store, &catalog, command),
        WorkloadsCommand::Run(command) => workload_run(&store, &workspace, &catalog, command),
    }
}

fn workload_create(
    workspace: &Path,
    catalog_dir: &Path,
    args: WorkloadCreateArgs,
) -> Result<ExitCode, CliError> {
    let result = WorkloadCatalog::create_workspace_request(
        workspace,
        catalog_dir,
        &args.workload_id,
        args.title.as_deref(),
        args.intent.as_deref(),
    )?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_workload_create(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_list(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog: &WorkloadCatalog,
    args: WorkloadListArgs,
) -> Result<ExitCode, CliError> {
    let list = current_workload_list(store, workspace, catalog)?;
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

fn current_workload_list(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog: &WorkloadCatalog,
) -> Result<WorkloadList, CliError> {
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
    let attention = derive_workload_attention(&definitions, &state, environment.as_ref())?;
    let list = WorkloadList::new(
        catalog.descriptor.clone(),
        summaries,
        catalog.drafts.clone(),
        attention,
    );
    Ok(list)
}

fn workload_status(
    store: &LocalWorkloadStore,
    workspace: &Path,
    catalog: &WorkloadCatalog,
    args: WorkloadStatusArgs,
) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let selected = match args.workload_id.as_deref() {
        Some(id)
            if catalog
                .drafts
                .iter()
                .any(|draft| draft.request.workload_id == id) =>
        {
            Vec::new()
        }
        _ => catalog.select(args.workload_id.as_deref())?,
    };
    let drafts = catalog.select_drafts(args.workload_id.as_deref());
    let statuses = selected
        .into_iter()
        .map(|workload| {
            let record = state.record(&workload.definition.workload.id);
            WorkloadStatusView::new_resolved(workload, record)
        })
        .collect();
    let definitions = catalog.definitions();
    let environment = retained_environment_snapshot(workspace)?;
    let attention = derive_workload_attention(&definitions, &state, environment.as_ref())?;
    let batch = WorkloadStatusBatch::new(catalog.descriptor.clone(), statuses, drafts, attention);
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &batch)?,
        WorkloadOutputFormat::Table => write_workload_status(&mut stdout, &batch)?,
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
    let capability_snapshot_id = if definitions
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
    };
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
        let snapshot = derive_portfolio_snapshot(&definitions, &state, environment.as_ref())?;
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
                rey::workloads::QualificationState::Untested => {
                    summary.untested = summary.untested.saturating_add(1);
                }
                rey::workloads::QualificationState::Qualified => {
                    summary.tested = summary.tested.saturating_add(1);
                    summary.qualified = summary.qualified.saturating_add(1);
                }
                rey::workloads::QualificationState::Failing => {
                    summary.tested = summary.tested.saturating_add(1);
                    summary.failing = summary.failing.saturating_add(1);
                }
                rey::workloads::QualificationState::Inconclusive => {
                    summary.tested = summary.tested.saturating_add(1);
                    summary.inconclusive = summary.inconclusive.saturating_add(1);
                }
                rey::workloads::QualificationState::Stale => {
                    summary.tested = summary.tested.saturating_add(1);
                    summary.stale_workloads = summary.stale_workloads.saturating_add(1);
                }
            }
            summary.required_scenarios =
                summary.required_scenarios.saturating_add(workload.required);
            summary.passed_scenarios = summary.passed_scenarios.saturating_add(workload.passed);
            summary.evaluated_scenarios = summary
                .evaluated_scenarios
                .saturating_add(workload.evaluated);
            summary.stale_scenarios = summary.stale_scenarios.saturating_add(workload.stale);
            summary.optional_scenarios =
                summary.optional_scenarios.saturating_add(workload.optional);
            match workload.last_run_status {
                Some(RunStatus::Passed) => {
                    summary.passed_runs = summary.passed_runs.saturating_add(1);
                }
                Some(RunStatus::Blocked) => {
                    summary.blocked_runs = summary.blocked_runs.saturating_add(1);
                }
                None => summary.unrun = summary.unrun.saturating_add(1),
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
        "LIVE READS · UNAUTHENTICATED JOURNAL WRITE",
    )?;
    write_portfolio_field(output, "Human entry", &descriptor.entry_route)?;
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
        "/api/v1/health · /api/v1/cadence · /api/v1/environment · /api/v1/journal · /api/v1/workloads",
    )?;
    write_portfolio_field(output, "Grammar revision", &descriptor.grammar_revision)?;
    write_portfolio_field(
        output,
        "Implementation",
        &format!(
            "{} · {}",
            descriptor.source_repository, descriptor.implementation_revision
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
            "{} · {} · {} · {}",
            count_noun(graph.channels.len(), "channel"),
            count_noun(graph.subscriptions.len(), "subscription"),
            count_noun(graph.streams.len(), "stream"),
            count_noun(graph.relays.len(), "relay")
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
    writeln!(output, "04 / RELAYS")?;
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
    Ok(())
}

fn write_channel_status(output: &mut impl Write, status: &ChannelStatus) -> Result<(), CliError> {
    writeln!(output, "On channels built-in")?;
    if status.state == ChannelWorkingState::Clean {
        writeln!(output)?;
        writeln!(output, "nothing to commit, channel working tree clean")?;
        return Ok(());
    }

    writeln!(output)?;
    writeln!(output, "Changes in channel working tree:")?;
    writeln!(output, "  (use \"rey channels diff\" to review)")?;
    for change in &status.delta.changes {
        writeln!(
            output,
            "        {:<11} {}: {} · {}",
            format!("{}:", change.kind.label()),
            change.object_kind.label(),
            change.object_id,
            change.detail
        )?;
    }
    writeln!(output)?;
    writeln!(output, "channel working tree differs from built-in")?;
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
    write_channel_diff_section(output, "04 / RELAYS", &diff.delta.changes, |change| {
        change.object_kind == ChannelObjectKind::Relay
    })?;
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
            match change.kind {
                rey::channels::ChannelChangeKind::Added => "+",
                rey::channels::ChannelChangeKind::Removed => "-",
                _ => "~",
            },
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
            "  {} · {} blocks · {}",
            entry.admitted_at,
            entry.blocks.len(),
            entry.entry_id
        )?;
    }
    writeln!(output)?;
    Ok(())
}

fn journal_author_kind(kind: JournalAuthorKind) -> &'static str {
    match kind {
        JournalAuthorKind::Human => "human",
        JournalAuthorKind::Agent => "agent",
        JournalAuthorKind::System => "system",
    }
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
    write_portfolio_field(output, "Created", &result.created_files.join(" · "))?;
    write_portfolio_field(
        output,
        "Admission",
        &style.yellow("AWAITING CODING HARNESS"),
    )?;
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
    write_portfolio_field(output, "Source revision", draft.source_digest.as_str())?;
    write_portfolio_field(output, "Graph", &style.dim("MISSING"))?;
    write_portfolio_field(output, "Scenario oracle", &style.dim("NOT ADMITTED"))?;
    write_portfolio_field(
        output,
        "Admission",
        &style.yellow("AWAITING CODING HARNESS"),
    )?;
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
    }
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
        || format!("ALL WORKLOADS ({})", workloads.len()),
        str::to_owned,
    );
    writeln!(output, "Execution path: {}", style.cyan_bold("LOCAL"))?;
    writeln!(
        output,
        "Mode: {}",
        style.cyan_bold("READ-ONLY GRAPH + PROBES · RETAIN LOCAL RESULTS")
    )?;
    writeln!(
        output,
        "Stage: {}",
        style.bold("EXECUTE SCENARIOS → MINE EVIDENCE → DIFF EXPECTED")
    )?;
    writeln!(output, "Scope: {}", style.bold(&scope))?;
    writeln!(
        output,
        "Catalog: {} · {}",
        style.bold(catalog.kind.label()),
        catalog.root.as_deref().unwrap_or("compiled")
    )?;
    writeln!(output)?;
    writeln!(
        output,
        "WORKLOAD CONFORMANCE EVALUATION · {}",
        style.bold(&scope)
    )?;
    writeln!(output, "Status: {}", style.cyan_bold("RUNNING"))?;
    writeln!(output, "Workloads queued: {}", workloads.len())?;
    writeln!(output)?;
    writeln!(
        output,
        "SCENARIOS · results render incrementally in declaration order"
    )?;
    Ok(())
}

fn write_workload_test_start(
    output: &mut impl Write,
    resolved: &ResolvedWorkload,
    verbosity: u8,
    style: TerminalStyle,
) -> io::Result<()> {
    let workload = &resolved.definition;
    let output_count = workload
        .scenario_suite
        .scenarios
        .iter()
        .map(|scenario| scenario.expected_outputs.len())
        .sum::<usize>();
    writeln!(output)?;
    writeln!(
        output,
        "WORKLOAD {} · {} scenarios · {} outputs",
        style.bold(&workload.workload.id),
        workload.scenario_suite.scenarios.len(),
        output_count,
    )?;
    writeln!(
        output,
        "Graph admission: {} · typed DAG {} · scenario oracle FROZEN",
        style.cyan_bold(resolved.provenance.origin.label()),
        style.green("VERIFIED")
    )?;
    if let Some(generation) = &resolved.provenance.generation {
        writeln!(
            output,
            "Generation: {} · {}@{} · graph + scenario suite",
            style.cyan_bold(generation.kind.label()),
            generation.producer,
            generation.producer_revision,
        )?;
        writeln!(output, "Package: {}", resolved.provenance.source)?;
    }
    if workload
        .graph
        .nodes
        .iter()
        .any(|node| node.operation.id.starts_with("rey.source-search."))
    {
        writeln!(
            output,
            "Mining admission: {} · explicit local corpus · bounded read-only probe",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "Mining operation: rey.source-search.literal-utf8@1 → rey.source-matches.v1 → ordered UTF-8 text"
        )?;
    }
    if workload
        .graph
        .nodes
        .iter()
        .any(|node| node.operation.id == "rey.context-anchor-survey.locate")
    {
        writeln!(
            output,
            "Topography admission: {} · explicit local seeds · bounded read-only survey",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "Survey operation: rey.context-anchor-survey.locate@1 → rey.topography-patch.v1 → ordered UTF-8 evidence"
        )?;
    }
    if workload
        .graph
        .nodes
        .iter()
        .any(|node| node.operation.id == "rey.portfolio.attention.derive")
    {
        writeln!(
            output,
            "Portfolio mining: {} · retained catalog/environment inputs · bounded typed relation",
            style.green("VERIFIED")
        )?;
        writeln!(
            output,
            "Attention operation: rey.portfolio.attention.derive@1 → rey.workload-attention.v1 → ordered UTF-8 text"
        )?;
    }
    if verbosity >= 1 {
        let node_count = workload.graph.nodes.len();
        writeln!(
            output,
            "Execution model: {} · {} {}",
            style.cyan_bold("DETERMINISTIC SERIAL"),
            node_count,
            if node_count == 1 { "node" } else { "nodes" }
        )?;
    }
    if verbosity >= 2 {
        writeln!(
            output,
            "Workload binding: {}@{} · {}",
            workload.workload.id, workload.workload.revision, workload.workload.semantic_digest
        )?;
        writeln!(
            output,
            "Graph binding: {}@{} · {}",
            workload.graph.graph.id,
            workload.graph.graph.revision,
            workload.graph.graph.semantic_digest
        )?;
        writeln!(
            output,
            "Scenario suite: {}@{} · {}",
            workload.scenario_suite.suite.id,
            workload.scenario_suite.suite.revision,
            workload.scenario_suite.suite.semantic_digest
        )?;
        writeln!(
            output,
            "Evaluator: {}@{} · {}",
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
    let equal = scenario
        .deltas
        .iter()
        .filter(|delta| delta.assessment == DeltaAssessment::Equal)
        .count();
    let equal_relations = scenario
        .mining
        .iter()
        .filter(|evidence| evidence.relation_delta.assessment == DeltaAssessment::Equal)
        .count();
    let evidence_total = scenario.deltas.len()
        + scenario.mining.len()
        + scenario.topography.len()
        + scenario.attention.len();
    let evidence_equal = equal
        + equal_relations
        + scenario
            .topography
            .iter()
            .filter(|patch| patch.complete)
            .count()
        + scenario.attention.len();
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
        "{label} {} · {:02}/{:02} {} · {}/{} {} equal · {}",
        workload.workload.id,
        index,
        total,
        scenario_id,
        evidence_equal,
        evidence_total,
        if scenario.mining.is_empty()
            && scenario.topography.is_empty()
            && scenario.attention.is_empty()
        {
            "outputs"
        } else {
            "evidence branches"
        },
        if scenario.required {
            "required"
        } else {
            "optional"
        }
    )?;
    if verbosity >= 1 {
        if !scenario.topography.is_empty() {
            writeln!(
                output,
                "     Evidence formats: {} (ordered utf8) · rey.topography-patch.v1 · rey.topography-patch-delta.v1",
                SCENARIO_OUTPUT_DELTA_SCHEMA
            )?;
        } else if scenario.mining.is_empty() && scenario.attention.is_empty() {
            writeln!(
                output,
                "     Evidence format: {} (utf8)",
                SCENARIO_OUTPUT_DELTA_SCHEMA
            )?;
        } else if !scenario.mining.is_empty() {
            writeln!(
                output,
                "     Evidence formats: {} (ordered utf8) · rey.source-match-delta.v1 (typed relation) · rey.mining-result.v1",
                SCENARIO_OUTPUT_DELTA_SCHEMA
            )?;
        } else {
            writeln!(
                output,
                "     Evidence formats: {} (ordered utf8) · rey.workload-attention.v1 (typed relation)",
                SCENARIO_OUTPUT_DELTA_SCHEMA
            )?;
        }
    }
    let passing = scenario.evaluation == ScenarioEvaluation::Passed;
    if passing && verbosity == 0 {
        return Ok(());
    }
    writeln!(
        output,
        "     {}:",
        if passing {
            "Evidence matches"
        } else {
            "Evidence deltas"
        }
    )?;
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
            "field pyramid",
            format!(
                "{} levels · {} cells · {} bytes allocated · {}",
                packet.field_pyramid.levels.len(),
                packet.field_pyramid.total_cells,
                packet.field_pyramid.total_bytes,
                packet.field_pyramid.stable_coordinate_rule,
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
                "anchors={} frontier={} validity={} levels={} cells/level={} bytes/level={} total_cells={} total_field_bytes={} contours={} features={} labels={}",
                packet.limits.max_anchor_objects,
                packet.limits.max_frontier_objects,
                packet.limits.max_validity_regions,
                packet.limits.max_field_levels,
                packet.limits.max_field_cells,
                packet.limits.max_field_bytes,
                packet.limits.max_total_field_cells,
                packet.limits.max_total_field_bytes,
                packet.limits.max_contours,
                packet.limits.max_natural_features,
                packet.limits.max_labels,
            ),
        ),
    ] {
        write_test_binding(output, label, &value)?;
    }
    for level in &packet.field_pyramid.levels {
        write_test_binding(
            output,
            "field level",
            &format!(
                "{} · {}×{} · stride {} · {} cells · {} bytes · {} · {}",
                level.level_id,
                level.columns,
                level.rows,
                level.sample_stride,
                level.cells,
                level.total_bytes,
                level.regimes.join("/"),
                level.detail_authority,
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
        "     Workload result: {status} · {}/{} scenarios passing · {}/{} evaluated",
        result.summary.passed,
        result.summary.required,
        result.summary.evaluated,
        result.summary.required
    )?;
    if verbosity >= 1 {
        writeln!(output, "     Stop reason: {}", result.stop_reason)?;
        writeln!(
            output,
            "     Qualification: {}",
            if result.qualification.is_some() {
                "issued"
            } else {
                "not issued"
            }
        )?;
    }
    if verbosity >= 2 {
        writeln!(output, "     Test result: {}", result.result_id)?;
        if let Some(qualification) = &result.qualification {
            writeln!(
                output,
                "     Qualification artifact: {}",
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
    writeln!(output, "{}", style.bold("PORTFOLIO CONFORMANCE"))?;
    writeln!(
        output,
        "  {:<22} {}  {:>3}%  {}/{} qualified",
        "Workloads",
        score_bar(workload_percent, 20),
        workload_percent,
        qualified,
        workloads
    )?;
    writeln!(
        output,
        "  {:<22} {}  {:>3}%  {}/{} passing",
        "Scenario conformance",
        score_bar(scenario_passing_percent, 20),
        scenario_passing_percent,
        passed,
        required
    )?;
    writeln!(
        output,
        "  {:<22} {}  {:>3}%  {}/{} evaluated",
        "Scenario evaluation",
        score_bar(scenario_evaluated_percent, 20),
        scenario_evaluated_percent,
        evaluated,
        required
    )?;
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD TEST SUMMARY"))?;
    writeln!(output, "  Result: {result}")?;
    writeln!(
        output,
        "  Workloads: {qualified}/{workloads} qualified · {failed} with gaps · {inconclusive} inconclusive"
    )?;
    writeln!(
        output,
        "  Scenarios: {passed}/{required} passing · {evaluated}/{required} evaluated"
    )?;
    writeln!(
        output,
        "  Deltas: {equal_deltas} equal · {different_deltas} different · {inconclusive_deltas} inconclusive"
    )?;
    writeln!(
        output,
        "  Qualifications: {qualified} issued · results retained locally"
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
    let initial =
        EnvironmentStatus::derive(&history, previous_index, working.clone(), args.max_changes)?;
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
    let status = EnvironmentStatus::derive(&history, index.clone(), working, args.max_changes)?;
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

fn env_commit(store: &LocalEnvironmentStore, args: EnvCommitArgs) -> Result<ExitCode, CliError> {
    let mut history = store.load()?;
    let index = store
        .load_index(&history)?
        .ok_or(LocalEnvironmentHistoryError::NothingStaged)?;
    let status = EnvironmentStatus::derive(
        &history,
        Some(index.clone()),
        index.snapshot.clone(),
        args.max_changes,
    )?;
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
    let status = EnvironmentStatus::derive(&history, index, snapshot, args.max_changes)?;
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
    let line = format!(
        "{:<10} {description}",
        environment_object_change_label(change)
    );
    let line = if staged {
        style.green(&line)
    } else {
        style.red(&line)
    };
    writeln!(output, "        {line}")
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

fn write_editor_import(
    output: &mut impl Write,
    result: &EditorImportResult,
) -> Result<(), CliError> {
    writeln!(
        output,
        "{} GeoJSON source {}",
        if result.imported {
            "Imported"
        } else {
            "Already registered"
        },
        result.source.source_id
    )?;
    writeln!(output, "project      {}", result.project_path)?;
    writeln!(output, "source       {}", result.source.path)?;
    writeln!(output, "role         {}", result.source.role.label())?;
    writeln!(
        output,
        "coverage     {} features · {} coordinate positions",
        result.feature_count, result.coordinate_count
    )?;
    writeln!(
        output,
        "authority    WORKING only · run `rey editor add` to stage"
    )?;
    Ok(())
}

fn write_editor_status(output: &mut impl Write, status: &EditorStatus) -> Result<(), CliError> {
    match &status.package {
        Some(package) => writeln!(output, "On scene package {}", package.package_id)?,
        None => writeln!(output, "No scene package yet")?,
    }
    writeln!(output)?;
    writeln!(
        output,
        "State          {}",
        match status.state {
            rey::editor::EditorWorkingState::Clean => "clean",
            rey::editor::EditorWorkingState::Working => "working changes",
            rey::editor::EditorWorkingState::Staged => "staged changes",
            rey::editor::EditorWorkingState::Mixed => "staged and working changes",
        }
    )?;
    writeln!(
        output,
        "PACKAGE→INDEX +{} -{} ~{} · {}",
        status.staged.inserted,
        status.staged.deleted,
        status.staged.modified,
        scene_assessment(status.staged.assessment)
    )?;
    writeln!(
        output,
        "INDEX→WORKING +{} -{} ~{} · {}",
        status.unstaged.inserted,
        status.unstaged.deleted,
        status.unstaged.modified,
        scene_assessment(status.unstaged.assessment)
    )?;
    write_editor_snapshot(output, &status.working)?;
    writeln!(output, "Admission       {}", status.admission_boundary)?;
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
    writeln!(
        output,
        "{} scene index",
        if result.staged {
            "Staged"
        } else {
            "Verified unchanged"
        }
    )?;
    write_editor_snapshot(output, &result.snapshot)?;
    writeln!(
        output,
        "PACKAGE→INDEX +{} -{} ~{}",
        result.delta.inserted, result.delta.deleted, result.delta.modified
    )?;
    writeln!(
        output,
        "Authority       staged candidate · native objects frozen · not admitted"
    )?;
    Ok(())
}

fn write_editor_snapshot(
    output: &mut impl Write,
    snapshot: &SceneCandidateSnapshot,
) -> Result<(), CliError> {
    writeln!(output, "Scene snapshot:")?;
    writeln!(output, "  schema          {}", snapshot.schema)?;
    writeln!(output, "  project         {}", snapshot.project_id)?;
    writeln!(output, "  revision        {}", snapshot.snapshot_revision)?;
    writeln!(
        output,
        "  coordinates     {} {} · {}",
        snapshot.coordinate_system.authority,
        snapshot.coordinate_system.code,
        snapshot.coordinate_system.axis_order
    )?;
    match &snapshot.bounds {
        Some(bounds) => writeln!(
            output,
            "  bounds          [{:.6}, {:.6}] → [{:.6}, {:.6}]",
            bounds.west, bounds.south, bounds.east, bounds.north
        )?,
        None => writeln!(output, "  bounds          empty · no geographic claim")?,
    }
    writeln!(
        output,
        "  coverage        {} sources · {} features · {} markers · {} positions",
        snapshot.coverage.sources,
        snapshot.coverage.features,
        snapshot.coverage.markers,
        snapshot.coverage.coordinates
    )?;
    writeln!(
        output,
        "  completeness    {} · {} omissions",
        if snapshot.complete {
            "complete"
        } else {
            "bounded"
        },
        snapshot.omissions.len()
    )?;
    writeln!(
        output,
        "  limits          sources={} · source_bytes={} · features={} · coordinates={} · properties={}/{} bytes",
        snapshot.limits.max_sources,
        snapshot.limits.max_source_bytes,
        snapshot.limits.max_features,
        snapshot.limits.max_coordinates,
        snapshot.limits.max_properties_per_feature,
        snapshot.limits.max_properties_bytes_per_feature
    )?;
    for source in &snapshot.sources {
        writeln!(
            output,
            "  SOURCE {} · {} · {} · {} features · {} positions",
            source.source_id,
            source.role.label(),
            source.artifact.content_digest,
            source.feature_count,
            source.coordinate_count
        )?;
        writeln!(
            output,
            "         worktree={} · object={} · {} bytes · {}",
            source.worktree_path,
            source.artifact.object_path,
            source.artifact.bytes,
            source.artifact.media_type
        )?;
    }
    const HUMAN_FEATURE_LIMIT: usize = 24;
    for feature in snapshot.features.iter().take(HUMAN_FEATURE_LIMIT) {
        write!(
            output,
            "  FEATURE {} · {} · {} · {} positions",
            feature.feature_id,
            feature.role.label(),
            feature.geometry_kind,
            feature.coordinate_count
        )?;
        if let Some(marker) = &feature.marker {
            write!(
                output,
                " · POI {:?} · zoom {}..{} · priority {}",
                marker.title, marker.min_zoom, marker.max_zoom, marker.collision_priority
            )?;
        }
        writeln!(output)?;
    }
    if snapshot.features.len() > HUMAN_FEATURE_LIMIT {
        writeln!(
            output,
            "  … {} additional features retained in structured output and native artifacts",
            snapshot.features.len() - HUMAN_FEATURE_LIMIT
        )?;
    }
    Ok(())
}

fn write_editor_package(
    output: &mut impl Write,
    result: &EditorPackageResult,
) -> Result<(), CliError> {
    writeln!(
        output,
        "{} immutable scene package",
        if result.created { "Created" } else { "Reused" }
    )?;
    write_editor_package_inspection(output, &result.package)?;
    writeln!(output, "Admission request:")?;
    writeln!(
        output,
        "  schema          {}",
        result.admission_request.schema
    )?;
    writeln!(
        output,
        "  request         {}",
        result.admission_request.request_id
    )?;
    writeln!(
        output,
        "  operation       {}",
        result.admission_request.requested_operation
    )?;
    writeln!(
        output,
        "  status          {} · admitted={} · /explore unchanged",
        result.admission_request.status, result.admission_request.admitted
    )?;
    Ok(())
}

fn write_editor_package_inspection(
    output: &mut impl Write,
    package: &ScenePackage,
) -> Result<(), CliError> {
    writeln!(output, "SCENE PACKAGE · CANDIDATE ONLY")?;
    writeln!(output, "Package         {}", package.package_id)?;
    writeln!(
        output,
        "Parent          {}",
        package
            .parent_package_id
            .as_ref()
            .map_or("none", SemanticDigest::as_str)
    )?;
    writeln!(
        output,
        "Authority       {} · no admission claim",
        package.admission_authority
    )?;
    writeln!(
        output,
        "Directed delta  PACKAGE → CANDIDATE · +{} -{} ~{}",
        package.change_set.inserted, package.change_set.deleted, package.change_set.modified
    )?;
    write_editor_snapshot(output, &package.snapshot)?;
    Ok(())
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
    #[error("limits must be greater than zero")]
    InvalidLimit,
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
    Delta(#[from] rey_diff::DeltaError),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
    #[error(transparent)]
    Portfolio(#[from] rey_runtime::PortfolioError),
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
