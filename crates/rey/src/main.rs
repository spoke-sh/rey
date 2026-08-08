#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use rey::{
    ReyError, inspect_environment,
    workloads::{
        LocalWorkloadStateError, LocalWorkloadStore, WorkloadList, WorkloadStatusBatch,
        WorkloadStatusView, WorkloadSummary, WorkloadTestBatch, fresh_qualification,
    },
};
use rey_diff::{CapabilityDelta, DeltaLimits, DeltaOptions, compare_capabilities};
use rey_environment::{CapabilitySnapshot, DiscoveryLimits};
use rey_git::GitLimits;
use rey_proof::{
    EvaluationOptions, LocalBundleLimits, ProofStatus, RequiredCapabilitiesClaim,
    RequiredCapabilityCertificate, VerificationStatus, create_local_proof_bundle,
    evaluate_required_capabilities, required_capability_evaluator, verify_local_proof_bundle,
    verify_required_capability_certificate,
};
use rey_runtime::{
    RunStatus, ScenarioEvaluation, TestStatus, WorkloadDefinition, WorkloadRunResult,
    WorkloadTestResult, WorkloadValue, built_in_workload, built_in_workloads, run_workload,
    test_workload,
};
use serde::Serialize;
use thiserror::Error;

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
    /// Inspect bounded local context capabilities without requiring Spoke.
    Environment(EnvironmentArgs),
    /// Inspect, test, qualify, and execute bounded compute graphs.
    Workloads(WorkloadsArgs),
}

#[derive(Debug, Args)]
struct WorkloadsArgs {
    /// Workspace used as the default local result-state boundary.
    #[arg(long, global = true, default_value = ".")]
    workspace: PathBuf,

    /// Explicit local result-state directory; relative paths resolve below the workspace.
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: WorkloadsCommand,
}

#[derive(Debug, Subcommand)]
enum WorkloadsCommand {
    /// List built-in workloads and retained scenario progress without executing them.
    List(WorkloadListArgs),
    /// Show workload definitions, retained deltas, qualification, and latest run.
    Status(WorkloadStatusArgs),
    /// Execute required scenarios and retain their typed output deltas.
    Test(WorkloadTestArgs),
    /// Execute an exactly qualified graph with caller-provided UTF-8 input.
    Run(WorkloadRunArgs),
}

#[derive(Debug, Args)]
struct WorkloadListArgs {
    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadStatusArgs {
    /// Workload id; omit to show every built-in workload.
    workload_id: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadTestArgs {
    /// Workload id; omit to test every built-in workload.
    workload_id: Option<String>,

    /// Output representation; auto uses a table on a terminal and JSON when piped.
    #[arg(long, value_enum, default_value_t = WorkloadOutputFormat::Auto)]
    format: WorkloadOutputFormat,
}

#[derive(Debug, Args)]
struct WorkloadRunArgs {
    /// Exact built-in workload id.
    workload_id: String,

    /// UTF-8 value bound to the workload's `text` input.
    #[arg(long)]
    input: String,

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
struct EnvironmentArgs {
    #[command(subcommand)]
    command: EnvironmentCommand,
}

#[derive(Debug, Subcommand)]
enum EnvironmentCommand {
    /// Emit the frozen standalone capability snapshot.
    Inspect(InspectArgs),
    /// Compare two verified capability snapshots.
    Diff(DiffArgs),
    /// Evaluate required capabilities and emit a bound certificate.
    Prove(ProveArgs),
    /// Re-evaluate a certificate against current snapshots and contracts.
    Verify(VerifyArgs),
    /// Verify a retained local-only capability proof bundle.
    VerifyBundle(VerifyBundleArgs),
}

#[derive(Debug, Args)]
struct InspectArgs {
    /// Explicit workspace boundary to inspect.
    #[arg(long, default_value = ".")]
    workspace: PathBuf,

    /// Output representation; auto uses a table on a terminal and Arrow when piped.
    #[arg(long, value_enum, default_value_t = OutputFormat::Auto)]
    format: OutputFormat,

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

#[derive(Debug, Args)]
struct SnapshotPairArgs {
    /// JSON capability snapshot used as the source state.
    source: PathBuf,

    /// JSON capability snapshot used as the target state.
    target: PathBuf,

    /// Stable review label for the source side.
    #[arg(long, default_value = "SOURCE")]
    source_label: String,

    /// Stable review label for the target side.
    #[arg(long, default_value = "TARGET")]
    target_label: String,

    /// Maximum bytes read from each input document.
    #[arg(long, default_value_t = 4_194_304)]
    max_input_bytes: u64,

    /// Maximum capability rows admitted from each snapshot.
    #[arg(long, default_value_t = 4_096)]
    max_capabilities: u64,

    /// Maximum changes admitted to an authoritative delta.
    #[arg(long, default_value_t = 4_096)]
    max_changes: u64,
}

#[derive(Debug, Args)]
struct DiffArgs {
    #[command(flatten)]
    pair: SnapshotPairArgs,

    /// Delta projection to emit.
    #[arg(long, value_enum, default_value_t = DiffFormat::Structured)]
    diff_format: DiffFormat,

    /// Typed encoding for a structured delta.
    #[arg(long, value_enum, default_value_t = StructuredFormat::Json)]
    format: StructuredFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum DiffFormat {
    Structured,
    TabularDiff,
    Summary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum StructuredFormat {
    Json,
    Arrow,
}

#[derive(Debug, Args)]
struct ProveArgs {
    #[command(flatten)]
    pair: SnapshotPairArgs,

    /// Capability id that must be available in the target; repeat as needed.
    #[arg(long = "require-capability", required = true)]
    required_capabilities: Vec<String>,

    /// Publish a content-addressed local-only proof bundle at this new directory.
    #[arg(long)]
    bundle: Option<PathBuf>,

    /// Maximum bytes retained in one bundle artifact.
    #[arg(long, default_value_t = 16_777_216)]
    max_bundle_artifact_bytes: u64,

    /// Maximum logical bytes retained across bundle artifact roles.
    #[arg(long, default_value_t = 67_108_864)]
    max_bundle_bytes: u64,
}

#[derive(Debug, Args)]
struct VerifyArgs {
    /// Required-capability certificate JSON to verify.
    certificate: PathBuf,

    /// JSON capability snapshot used as the source state.
    source: PathBuf,

    /// JSON capability snapshot used as the target state.
    target: PathBuf,

    /// Maximum bytes read from each input document.
    #[arg(long, default_value_t = 4_194_304)]
    max_input_bytes: u64,

    /// Maximum capability rows admitted from each snapshot.
    #[arg(long, default_value_t = 4_096)]
    max_capabilities: u64,
}

#[derive(Debug, Args)]
struct VerifyBundleArgs {
    /// Local proof bundle directory to verify without following symlinked evidence.
    bundle: PathBuf,

    /// Maximum bytes admitted from one manifest or artifact.
    #[arg(long, default_value_t = 16_777_216)]
    max_artifact_bytes: u64,

    /// Maximum logical bytes admitted across artifact roles.
    #[arg(long, default_value_t = 67_108_864)]
    max_bundle_bytes: u64,

    /// Maximum capability rows admitted from each retained snapshot.
    #[arg(long, default_value_t = 4_096)]
    max_capabilities: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Auto,
    Table,
    Arrow,
    Json,
}

impl OutputFormat {
    fn resolve(self) -> Self {
        match (self, io::stdout().is_terminal()) {
            (Self::Auto, true) => Self::Table,
            (Self::Auto, false) => Self::Arrow,
            (selected, _) => selected,
        }
    }
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
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::Inspect(args),
        }) => {
            inspect(args)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::Diff(args),
        }) => {
            diff(args)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::Prove(args),
        }) => prove(args),
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::Verify(args),
        }) => verify(args),
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::VerifyBundle(args),
        }) => verify_bundle(args),
        Command::Workloads(args) => workloads(args),
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
        WorkloadsCommand::List(command) => workload_list(&store, command),
        WorkloadsCommand::Status(command) => workload_status(&store, command),
        WorkloadsCommand::Test(command) => workload_test(&store, command),
        WorkloadsCommand::Run(command) => workload_run(&store, command),
    }
}

fn workload_list(store: &LocalWorkloadStore, args: WorkloadListArgs) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let summaries = built_in_workloads()?
        .iter()
        .map(|workload| WorkloadSummary::derive(workload, state.record(&workload.workload.id)))
        .collect();
    let list = WorkloadList::new(summaries);
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

fn workload_status(
    store: &LocalWorkloadStore,
    args: WorkloadStatusArgs,
) -> Result<ExitCode, CliError> {
    let state = store.load()?;
    let definitions = select_workloads(args.workload_id.as_deref())?;
    let statuses = definitions
        .into_iter()
        .map(|workload| {
            let record = state.record(&workload.workload.id);
            WorkloadStatusView::new(workload, record)
        })
        .collect();
    let batch = WorkloadStatusBatch::new(statuses);
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &batch)?,
        WorkloadOutputFormat::Table => write_workload_status(&mut stdout, &batch)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(ExitCode::SUCCESS)
}

fn workload_test(store: &LocalWorkloadStore, args: WorkloadTestArgs) -> Result<ExitCode, CliError> {
    let mut state = store.load()?;
    let definitions = select_workloads(args.workload_id.as_deref())?;
    let mut results = Vec::with_capacity(definitions.len());
    for workload in definitions {
        let result = test_workload(&workload)?;
        state.retain_test(result.clone());
        results.push(result);
    }
    state.verify()?;
    store.save(&state)?;
    let batch = WorkloadTestBatch::new(results);
    let exit_code = test_batch_exit(&batch);
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &batch)?,
        WorkloadOutputFormat::Table => write_workload_test_batch(&mut stdout, &batch)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(exit_code)
}

fn workload_run(store: &LocalWorkloadStore, args: WorkloadRunArgs) -> Result<ExitCode, CliError> {
    let workload = built_in_workload(&args.workload_id)?;
    let mut state = store.load()?;
    let mut inputs = BTreeMap::new();
    inputs.insert("text".to_owned(), WorkloadValue::Utf8(args.input));
    let result = match fresh_qualification(&workload, state.record(&workload.workload.id)) {
        Some(qualification) => run_workload(&workload, qualification, inputs)?,
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
    match args.format.resolve() {
        WorkloadOutputFormat::Json => write_json_line(&mut stdout, &result)?,
        WorkloadOutputFormat::Table => write_workload_run(&mut stdout, &result)?,
        WorkloadOutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(exit_code)
}

fn select_workloads(workload_id: Option<&str>) -> Result<Vec<WorkloadDefinition>, CliError> {
    match workload_id {
        Some(workload_id) => Ok(vec![built_in_workload(workload_id)?]),
        None => Ok(built_in_workloads()?),
    }
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
    fn derive(workloads: &[WorkloadSummary]) -> Self {
        let mut summary = Self::default();
        for workload in workloads {
            summary.total = summary.total.saturating_add(1);
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

fn write_workload_list(
    output: &mut impl Write,
    list: &WorkloadList,
    style: TerminalStyle,
) -> Result<(), CliError> {
    let portfolio = WorkloadPortfolioSummary::derive(&list.workloads);
    writeln!(output)?;
    writeln!(output, "{}", style.bold("WORKLOAD PORTFOLIO"))?;
    write_portfolio_field(
        output,
        "Qualification",
        &format!(
            "{}/{} qualified · {} failing · {} inconclusive · {} stale",
            portfolio.qualified,
            portfolio.total,
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
            "{} total · {} tested · {} untested",
            portfolio.total, portfolio.tested, portfolio.untested,
        ),
    )?;
    if list.workloads.is_empty() {
        writeln!(output, "  {}", style.dim("No workloads found"))?;
        return Ok(());
    }

    for (index, workload) in list.workloads.iter().enumerate() {
        writeln!(output)?;
        writeln!(output, "{}", style.bold(&workload.workload.id))?;
        write_portfolio_field(output, "Purpose", &workload.title)?;
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
        if index + 1 < list.workloads.len() {
            writeln!(
                output,
                "{}",
                style.dim("  ────────────────────────────────────────────────────────────")
            )?;
        }
    }
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

fn progress_bar(summary: &WorkloadSummary) -> String {
    let mut bar = String::with_capacity(summary.required as usize + 2);
    bar.push('[');
    for _ in 0..summary.passed {
        bar.push('=');
    }
    for _ in 0..summary.failed {
        bar.push('!');
    }
    for _ in 0..summary.inconclusive.saturating_add(summary.stale) {
        bar.push('?');
    }
    let represented = summary
        .passed
        .saturating_add(summary.failed)
        .saturating_add(summary.inconclusive)
        .saturating_add(summary.stale);
    for _ in represented..summary.required {
        bar.push('.');
    }
    bar.push(']');
    bar
}

fn write_workload_status(
    output: &mut impl Write,
    batch: &WorkloadStatusBatch,
) -> Result<(), CliError> {
    for (index, status) in batch.statuses.iter().enumerate() {
        if index > 0 {
            writeln!(output)?;
        }
        let summary = &status.summary;
        writeln!(
            output,
            "workload={} title={}",
            summary.workload.id, summary.title
        )?;
        writeln!(
            output,
            "candidate_graph={}@{} {}",
            summary.candidate_graph.id,
            summary.candidate_graph.revision,
            summary.candidate_graph.semantic_digest
        )?;
        writeln!(
            output,
            "progress={} passed={} evaluated={} required={} qualification={} freshness={}",
            progress_bar(summary),
            summary.passed,
            summary.evaluated,
            summary.required,
            summary.qualification.as_str(),
            summary.freshness.as_str()
        )?;
        if let Some(result) = &status.last_test {
            write_test_detail(output, result)?;
        } else {
            writeln!(output, "test=none")?;
        }
        if let Some(result) = &status.last_run {
            writeln!(
                output,
                "run={} status={:?} reason={}",
                result.run_id, result.status, result.stop_reason
            )?;
        } else {
            writeln!(output, "run=none")?;
        }
    }
    Ok(())
}

fn write_workload_test_batch(
    output: &mut impl Write,
    batch: &WorkloadTestBatch,
) -> Result<(), CliError> {
    for (index, result) in batch.results.iter().enumerate() {
        if index > 0 {
            writeln!(output)?;
        }
        write_test_detail(output, result)?;
    }
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

fn write_workload_run(output: &mut impl Write, result: &WorkloadRunResult) -> Result<(), CliError> {
    writeln!(
        output,
        "run={} workload={} graph={} status={:?} reason={}",
        result.run_id,
        result.workload.id,
        result.graph.semantic_digest,
        result.status,
        result.stop_reason
    )?;
    writeln!(output, "node_order={}", json_cell(&result.node_order)?)?;
    writeln!(output, "outputs={}", json_cell(&result.outputs)?)?;
    Ok(())
}

fn inspect(args: InspectArgs) -> Result<(), CliError> {
    if args.total_timeout_ms == 0 || args.probe_timeout_ms == 0 || args.max_capture_bytes == 0 {
        return Err(CliError::InvalidLimit);
    }
    let snapshot = inspect_environment(
        &args.workspace,
        DiscoveryLimits {
            total_timeout_ms: args.total_timeout_ms,
            probe_timeout_ms: args.probe_timeout_ms,
            max_capture_bytes: args.max_capture_bytes,
            max_capabilities: 64,
        },
        GitLimits::default(),
    )?;
    let frame = snapshot.to_frame()?;
    let mut stdout = io::stdout().lock();
    match args.format.resolve() {
        OutputFormat::Arrow => stdout.write_all(&frame.to_arrow_stream()?)?,
        OutputFormat::Json => {
            serde_json::to_writer(&mut stdout, &snapshot)?;
            stdout.write_all(b"\n")?;
        }
        OutputFormat::Table => {
            writeln!(
                stdout,
                "snapshot={} complete={} profile={}",
                snapshot.semantic_digest, snapshot.complete, snapshot.profile
            )?;
            write_capability_table(&mut stdout, &snapshot.capabilities)?;
        }
        OutputFormat::Auto => unreachable!("auto output is resolved before rendering"),
    }
    Ok(())
}

fn diff(args: DiffArgs) -> Result<(), CliError> {
    if args.diff_format != DiffFormat::Structured && args.format != StructuredFormat::Json {
        return Err(CliError::FormatScope);
    }
    let (source, target) = load_pair(&args.pair)?;
    let delta = compare_pair(&source, &target, &args.pair)?;
    let mut stdout = io::stdout().lock();
    match (args.diff_format, args.format) {
        (DiffFormat::Structured, StructuredFormat::Json) => {
            write_json_line(&mut stdout, &delta)?;
        }
        (DiffFormat::Structured, StructuredFormat::Arrow) => {
            stdout.write_all(&delta.to_frame()?.to_arrow_stream()?)?;
        }
        (DiffFormat::TabularDiff, StructuredFormat::Json) => {
            stdout.write_all(&delta.to_tabular_diff()?)?;
        }
        (DiffFormat::Summary, StructuredFormat::Json) => {
            write_json_line(&mut stdout, &delta.summary)?;
        }
        (_, StructuredFormat::Arrow) => return Err(CliError::FormatScope),
    }
    Ok(())
}

fn prove(args: ProveArgs) -> Result<ExitCode, CliError> {
    let (source, target) = load_pair(&args.pair)?;
    let claim = RequiredCapabilitiesClaim::new(args.required_capabilities)?;
    let certificate = evaluate_required_capabilities(
        &source,
        &target,
        claim,
        EvaluationOptions {
            source_label: args.pair.source_label.clone(),
            target_label: args.pair.target_label.clone(),
            delta_limits: DeltaLimits {
                max_changes: args.pair.max_changes,
            },
            ..EvaluationOptions::default()
        },
    )?;
    if let Some(bundle) = &args.bundle {
        create_local_proof_bundle(
            bundle,
            &source,
            &target,
            &certificate,
            LocalBundleLimits {
                max_artifact_bytes: args.max_bundle_artifact_bytes,
                max_total_bytes: args.max_bundle_bytes,
                max_capabilities: args.pair.max_capabilities,
                ..LocalBundleLimits::default()
            },
        )?;
    }
    write_json_line(&mut io::stdout().lock(), &certificate)?;
    Ok(match certificate.status {
        ProofStatus::Passed => ExitCode::SUCCESS,
        ProofStatus::Failed => ExitCode::from(2),
        ProofStatus::Inconclusive => ExitCode::from(3),
    })
}

fn verify_bundle(args: VerifyBundleArgs) -> Result<ExitCode, CliError> {
    let verification = verify_local_proof_bundle(
        &args.bundle,
        LocalBundleLimits {
            max_artifact_bytes: args.max_artifact_bytes,
            max_total_bytes: args.max_bundle_bytes,
            max_capabilities: args.max_capabilities,
            ..LocalBundleLimits::default()
        },
    )?;
    write_json_line(&mut io::stdout().lock(), &verification)?;
    Ok(ExitCode::SUCCESS)
}

fn verify(args: VerifyArgs) -> Result<ExitCode, CliError> {
    if args.max_input_bytes == 0 || args.max_capabilities == 0 {
        return Err(CliError::InvalidLimit);
    }
    let certificate_bytes = read_bounded(&args.certificate, args.max_input_bytes)?;
    let certificate: RequiredCapabilityCertificate = serde_json::from_slice(&certificate_bytes)?;
    let source = load_snapshot(&args.source, args.max_input_bytes, args.max_capabilities)?;
    let target = load_snapshot(&args.target, args.max_input_bytes, args.max_capabilities)?;
    let verification = verify_required_capability_certificate(
        &certificate,
        &source,
        &target,
        required_capability_evaluator(),
    )?;
    write_json_line(&mut io::stdout().lock(), &verification)?;
    Ok(match verification.status {
        VerificationStatus::Verified => ExitCode::SUCCESS,
        VerificationStatus::Stale => ExitCode::from(4),
    })
}

fn load_pair(
    args: &SnapshotPairArgs,
) -> Result<(CapabilitySnapshot, CapabilitySnapshot), CliError> {
    validate_pair_limits(args)?;
    let source = load_snapshot(&args.source, args.max_input_bytes, args.max_capabilities)?;
    let target = load_snapshot(&args.target, args.max_input_bytes, args.max_capabilities)?;
    Ok((source, target))
}

fn compare_pair(
    source: &CapabilitySnapshot,
    target: &CapabilitySnapshot,
    args: &SnapshotPairArgs,
) -> Result<CapabilityDelta, CliError> {
    Ok(compare_capabilities(
        source,
        target,
        DeltaOptions {
            source_label: args.source_label.clone(),
            target_label: args.target_label.clone(),
            limits: DeltaLimits {
                max_changes: args.max_changes,
            },
            ..DeltaOptions::default()
        },
    )?)
}

fn validate_pair_limits(args: &SnapshotPairArgs) -> Result<(), CliError> {
    if args.max_input_bytes == 0 || args.max_capabilities == 0 || args.max_changes == 0 {
        return Err(CliError::InvalidLimit);
    }
    Ok(())
}

fn load_snapshot(
    path: &Path,
    max_input_bytes: u64,
    max_capabilities: u64,
) -> Result<CapabilitySnapshot, CliError> {
    let bytes = read_bounded(path, max_input_bytes)?;
    Ok(CapabilitySnapshot::from_json_slice(
        &bytes,
        max_capabilities,
    )?)
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, CliError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|source| CliError::Input {
            path: path.to_owned(),
            source,
        })?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CliError::Input {
            path: path.to_owned(),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(CliError::InputLimit {
            path: path.to_owned(),
            limit: max_bytes,
        });
    }
    Ok(bytes)
}

fn write_json_line(output: &mut impl Write, value: &impl Serialize) -> Result<(), CliError> {
    serde_json::to_writer(&mut *output, value)?;
    output.write_all(b"\n")?;
    Ok(())
}

fn write_capability_table(
    output: &mut impl Write,
    rows: &[rey_environment::CapabilityRecord],
) -> Result<(), CliError> {
    writeln!(
        output,
        "provider_id\tprovider_revision\tprovider_kind\tcapability_id\tcapability_kind\tresolved_location\tversion\tcontent_digest\tprovenance\tavailability\ttrust_class\toperations\tenforced_limits\tunsupported_limits\tobserved_at\terror_code\terror_detail"
    )?;
    for row in rows {
        let cells = [
            json_cell(&row.provider_id)?,
            json_cell(&row.provider_revision)?,
            json_cell(&row.provider_kind)?,
            json_cell(&row.capability_id)?,
            json_cell(&row.capability_kind)?,
            json_cell(&row.resolved_location)?,
            json_cell(&row.version)?,
            json_cell(&row.content_digest)?,
            json_cell(&row.provenance)?,
            json_cell(&row.availability)?,
            json_cell(&row.trust_class)?,
            json_cell(&row.operations)?,
            json_cell(&row.enforced_limits)?,
            json_cell(&row.unsupported_limits)?,
            json_cell(&row.observed_at)?,
            json_cell(&row.error_code)?,
            json_cell(&row.error_detail)?,
        ];
        writeln!(output, "{}", cells.join("\t"))?;
    }
    Ok(())
}

fn json_cell(value: &impl Serialize) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

#[derive(Debug, Error)]
enum CliError {
    #[error("limits must be greater than zero")]
    InvalidLimit,
    #[error("--format arrow is only valid with --diff-format structured")]
    FormatScope,
    #[error("input {path} could not be read: {source}")]
    Input { path: PathBuf, source: io::Error },
    #[error("input {path} exceeds the {limit}-byte limit")]
    InputLimit { path: PathBuf, limit: u64 },
    #[error("workspace {path} could not be resolved: {source}")]
    Workspace { path: PathBuf, source: io::Error },
    #[error("workspace {0} is not a directory")]
    WorkspaceDirectory(PathBuf),
    #[error(transparent)]
    Rey(#[from] ReyError),
    #[error(transparent)]
    Discovery(#[from] rey_environment::DiscoveryError),
    #[error(transparent)]
    Frame(#[from] rey_dataframe::FrameError),
    #[error(transparent)]
    Delta(#[from] rey_diff::DeltaError),
    #[error(transparent)]
    Proof(#[from] rey_proof::ProofError),
    #[error(transparent)]
    Bundle(#[from] rey_proof::LocalBundleError),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
    #[error(transparent)]
    WorkloadState(#[from] LocalWorkloadStateError),
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output failed: {0}")]
    Output(#[from] io::Error),
}
