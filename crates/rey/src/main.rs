#![forbid(unsafe_code)]

use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use rey::{ReyError, inspect_environment};
use rey_diff::{CapabilityDelta, DeltaLimits, DeltaOptions, compare_capabilities};
use rey_environment::{CapabilitySnapshot, DiscoveryLimits};
use rey_git::GitLimits;
use rey_proof::{
    EvaluationOptions, LocalBundleLimits, ProofStatus, RequiredCapabilitiesClaim,
    RequiredCapabilityCertificate, VerificationStatus, create_local_proof_bundle,
    evaluate_required_capabilities, required_capability_evaluator, verify_local_proof_bundle,
    verify_required_capability_certificate,
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
    }
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
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output failed: {0}")]
    Output(#[from] io::Error),
}
