#![forbid(unsafe_code)]

use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use rey::{ReyError, inspect_environment};
use rey_environment::DiscoveryLimits;
use rey_git::GitLimits;
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
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rey: {error}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Environment(EnvironmentArgs {
            command: EnvironmentCommand::Inspect(args),
        }) => inspect(args),
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
    #[error(transparent)]
    Rey(#[from] ReyError),
    #[error(transparent)]
    Discovery(#[from] rey_environment::DiscoveryError),
    #[error(transparent)]
    Frame(#[from] rey_dataframe::FrameError),
    #[error("JSON output failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output failed: {0}")]
    Output(#[from] io::Error),
}
