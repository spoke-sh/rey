use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde::Serialize;
use thiserror::Error;

use crate::ui::{UiError, UiServer, UiServerDescriptor};
use crate::version;
use rey::channels::{ChannelGraphError, LocalChannelStore};

pub const AGENT_PROCESS_SCHEMA: &str = "rey.agent-process.v1";
pub const REY_PROCESS_SCHEMA: &str = "rey.process.v1";
pub const AGENT_TOPOLOGY_SCHEMA: &str = "rey.agent-topology.v1";
const ORCHESTRATOR_NODE_ID: &str = "rey.orchestrator";
const OPERATOR_SERVER_NODE_ID: &str = "rey.operator-http";
const GITHUB_INBOX_NODE_ID: &str = "rey.channel-github-inbox";
const SUPERVISION_POLL_INTERVAL_MS: u64 = 50;
const INGRESS_SCAN_INTERVAL_MS: u64 = 250;
const MAX_BACKGROUND_WORKERS: u64 = 2;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentProcessDescriptor {
    pub schema: String,
    pub state: String,
    pub process: ReyProcessDescriptor,
    pub topology: AgentTopologyDescriptor,
    pub operator: UiServerDescriptor,
    pub authority: String,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReyProcessDescriptor {
    pub schema: String,
    pub process_id: String,
    pub os_pid: u32,
    pub role: String,
    pub topology_node_id: String,
    pub invocation: String,
    pub lifecycle: String,
    pub shutdown: String,
    pub implementation_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentTopologyDescriptor {
    pub schema: String,
    pub root_node_id: String,
    pub nodes: Vec<AgentTopologyNode>,
    pub edges: Vec<AgentTopologyEdge>,
    pub max_background_workers: u64,
    pub supervision_poll_interval_ms: u64,
    pub agent_runtime_invocation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentTopologyNode {
    pub node_id: String,
    pub kind: String,
    pub parent_node_id: Option<String>,
    pub execution: String,
    pub lifecycle: String,
    pub state: String,
    pub restart_policy: String,
    pub authority: String,
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AgentTopologyEdge {
    pub source_node_id: String,
    pub target_node_id: String,
    pub relationship: String,
}

impl AgentProcessDescriptor {
    #[must_use]
    pub fn from_operator(operator: UiServerDescriptor) -> Self {
        let os_pid = std::process::id();
        let process_id = format!("local-process:{os_pid}");
        let process = ReyProcessDescriptor {
            schema: REY_PROCESS_SCHEMA.to_owned(),
            process_id,
            os_pid,
            role: "orchestrator".to_owned(),
            topology_node_id: ORCHESTRATOR_NODE_ID.to_owned(),
            invocation: "rey agent".to_owned(),
            lifecycle: "foreground; owns every in-process background worker".to_owned(),
            shutdown: "cooperative SIGINT/SIGTERM at a bounded worker boundary".to_owned(),
            implementation_revision: operator.implementation_revision.clone(),
        };
        let topology = AgentTopologyDescriptor {
            schema: AGENT_TOPOLOGY_SCHEMA.to_owned(),
            root_node_id: ORCHESTRATOR_NODE_ID.to_owned(),
            nodes: vec![
                AgentTopologyNode {
                    node_id: ORCHESTRATOR_NODE_ID.to_owned(),
                    kind: "rey_process".to_owned(),
                    parent_node_id: None,
                    execution: "os_process".to_owned(),
                    lifecycle: "foreground".to_owned(),
                    state: "running".to_owned(),
                    restart_policy: "external".to_owned(),
                    authority: "background_lifecycle_only; no workload or agent-runtime authority"
                        .to_owned(),
                    endpoint: None,
                },
                AgentTopologyNode {
                    node_id: OPERATOR_SERVER_NODE_ID.to_owned(),
                    kind: "background_work".to_owned(),
                    parent_node_id: Some(ORCHESTRATOR_NODE_ID.to_owned()),
                    execution: "supervised_thread".to_owned(),
                    lifecycle: "bound_to_rey_process".to_owned(),
                    state: "running".to_owned(),
                    restart_policy: "never; fail the Rey process closed".to_owned(),
                    authority: "operator HTTP projection and its declared bounded writes"
                        .to_owned(),
                    endpoint: Some(operator.url.clone()),
                },
                AgentTopologyNode {
                    node_id: GITHUB_INBOX_NODE_ID.to_owned(),
                    kind: "background_work".to_owned(),
                    parent_node_id: Some(ORCHESTRATOR_NODE_ID.to_owned()),
                    execution: "supervised_thread".to_owned(),
                    lifecycle: "bound_to_rey_process".to_owned(),
                    state: "running".to_owned(),
                    restart_policy: "next admitted cadence; no immediate retry".to_owned(),
                    authority: "poll exact committed GitHub inbox applications through the bounded channels poll contract".to_owned(),
                    endpoint: None,
                },
            ],
            edges: vec![
                AgentTopologyEdge {
                    source_node_id: ORCHESTRATOR_NODE_ID.to_owned(),
                    target_node_id: OPERATOR_SERVER_NODE_ID.to_owned(),
                    relationship: "supervises".to_owned(),
                },
                AgentTopologyEdge {
                    source_node_id: ORCHESTRATOR_NODE_ID.to_owned(),
                    target_node_id: GITHUB_INBOX_NODE_ID.to_owned(),
                    relationship: "supervises".to_owned(),
                },
            ],
            max_background_workers: MAX_BACKGROUND_WORKERS,
            supervision_poll_interval_ms: SUPERVISION_POLL_INTERVAL_MS,
            agent_runtime_invocation:
                "none; discovery, assignment, and execution authority remain separate".to_owned(),
        };
        Self {
            schema: AGENT_PROCESS_SCHEMA.to_owned(),
            state: "running".to_owned(),
            process,
            topology,
            operator,
            authority: "local orchestration, operator projection, and exact admitted GitHub Channel polling only".to_owned(),
            omissions: vec![
                "no autonomous workload scheduling".to_owned(),
                "no discovered agent runtime is invoked or assigned".to_owned(),
                "no inbound provider other than an exact admitted github.com gh application"
                    .to_owned(),
                "no restart, daemonization, multi-process fencing, or crash durability".to_owned(),
            ],
        }
    }
}

pub struct AgentOrchestrator {
    cancelled: Arc<AtomicBool>,
    operator_worker: thread::JoinHandle<Result<(), UiError>>,
    github_inbox_worker: thread::JoinHandle<Result<(), ChannelIngressError>>,
}

impl AgentOrchestrator {
    pub fn start(operator: UiServer) -> Result<Self, AgentError> {
        let ingress = ChannelIngressWorker::from_operator(&operator.descriptor())?;
        let cancelled = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&cancelled))
            .map_err(AgentError::Signal)?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&cancelled))
            .map_err(AgentError::Signal)?;

        let worker_cancelled = Arc::clone(&cancelled);
        let operator_worker = thread::Builder::new()
            .name(OPERATOR_SERVER_NODE_ID.to_owned())
            .spawn(move || operator.serve_until(worker_cancelled))
            .map_err(AgentError::Spawn)?;
        let ingress_cancelled = Arc::clone(&cancelled);
        let github_inbox_worker = match thread::Builder::new()
            .name(GITHUB_INBOX_NODE_ID.to_owned())
            .spawn(move || ingress.serve_until(ingress_cancelled))
        {
            Ok(worker) => worker,
            Err(error) => {
                cancelled.store(true, Ordering::Relaxed);
                let _ = operator_worker.join();
                return Err(AgentError::Spawn(error));
            }
        };
        log_info(format_args!(
            "Rey version {}; commit {}",
            version::VERSION,
            version::COMMIT_SHA
        ));
        log_info(format_args!("Started Rey process [{}]", std::process::id()));
        log_info(format_args!(
            "Agent startup complete; background workers {OPERATOR_SERVER_NODE_ID} and {GITHUB_INBOX_NODE_ID} are running"
        ));
        Ok(Self {
            cancelled,
            operator_worker,
            github_inbox_worker,
        })
    }

    pub fn wait(self) -> Result<(), AgentError> {
        let mut operator_worker = Some(self.operator_worker);
        let mut github_inbox_worker = Some(self.github_inbox_worker);
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                log_info("Shutdown requested");
                log_info(format_args!(
                    "Agent shutdown started; stopping background workers {OPERATOR_SERVER_NODE_ID} and {GITHUB_INBOX_NODE_ID}"
                ));
                let operator_result = finish_operator_worker(
                    operator_worker.take().expect("operator worker is present"),
                    true,
                );
                let ingress_result = finish_github_inbox_worker(
                    github_inbox_worker
                        .take()
                        .expect("GitHub inbox worker is present"),
                    true,
                );
                operator_result?;
                ingress_result?;
                log_info(format_args!(
                    "Finished Rey process [{}]",
                    std::process::id()
                ));
                return Ok(());
            }
            if operator_worker
                .as_ref()
                .is_some_and(thread::JoinHandle::is_finished)
            {
                self.cancelled.store(true, Ordering::Relaxed);
                let operator_result = finish_operator_worker(
                    operator_worker.take().expect("operator worker is present"),
                    false,
                );
                let ingress_result = finish_github_inbox_worker(
                    github_inbox_worker
                        .take()
                        .expect("GitHub inbox worker is present"),
                    true,
                );
                operator_result?;
                return ingress_result;
            }
            if github_inbox_worker
                .as_ref()
                .is_some_and(thread::JoinHandle::is_finished)
            {
                self.cancelled.store(true, Ordering::Relaxed);
                let ingress_result = finish_github_inbox_worker(
                    github_inbox_worker
                        .take()
                        .expect("GitHub inbox worker is present"),
                    false,
                );
                let operator_result = finish_operator_worker(
                    operator_worker.take().expect("operator worker is present"),
                    true,
                );
                ingress_result?;
                return operator_result;
            }
            thread::park_timeout(Duration::from_millis(SUPERVISION_POLL_INTERVAL_MS));
        }
    }
}

fn log_info(message: impl std::fmt::Display) {
    eprintln!("INFO:     {message}");
}

fn log_error(message: impl std::fmt::Display) {
    eprintln!("ERROR:    {message}");
}

fn finish_operator_worker(
    worker: thread::JoinHandle<Result<(), UiError>>,
    cancelled: bool,
) -> Result<(), AgentError> {
    match worker.join() {
        Ok(Ok(())) if cancelled => {
            log_info(format_args!(
                "Agent shutdown complete; background worker {OPERATOR_SERVER_NODE_ID} stopped"
            ));
            Ok(())
        }
        Ok(Ok(())) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {OPERATOR_SERVER_NODE_ID} exited without a shutdown request"
            ));
            Err(AgentError::UnexpectedWorkerExit(
                OPERATOR_SERVER_NODE_ID.to_owned(),
            ))
        }
        Ok(Err(error)) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {OPERATOR_SERVER_NODE_ID}: {error}"
            ));
            Err(AgentError::Operator(error))
        }
        Err(_) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {OPERATOR_SERVER_NODE_ID} panicked"
            ));
            Err(AgentError::WorkerPanicked(
                OPERATOR_SERVER_NODE_ID.to_owned(),
            ))
        }
    }
}

fn finish_github_inbox_worker(
    worker: thread::JoinHandle<Result<(), ChannelIngressError>>,
    cancelled: bool,
) -> Result<(), AgentError> {
    match worker.join() {
        Ok(Ok(())) if cancelled => {
            log_info(format_args!(
                "Agent shutdown complete; background worker {GITHUB_INBOX_NODE_ID} stopped"
            ));
            Ok(())
        }
        Ok(Ok(())) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {GITHUB_INBOX_NODE_ID} exited without a shutdown request"
            ));
            Err(AgentError::UnexpectedWorkerExit(
                GITHUB_INBOX_NODE_ID.to_owned(),
            ))
        }
        Ok(Err(error)) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {GITHUB_INBOX_NODE_ID}: {error}"
            ));
            Err(AgentError::ChannelIngress(error))
        }
        Err(_) => {
            log_error(format_args!(
                "Agent lifecycle failed; background worker {GITHUB_INBOX_NODE_ID} panicked"
            ));
            Err(AgentError::WorkerPanicked(GITHUB_INBOX_NODE_ID.to_owned()))
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GitHubPollKey {
    channel_head_commit_id: String,
    application_id: String,
    application_revision: u64,
}

#[derive(Clone, Debug)]
struct AdmittedGitHubPoll {
    key: GitHubPollKey,
    interval: Duration,
}

struct ChannelIngressWorker {
    workspace: PathBuf,
    channel_directory: PathBuf,
    rey_executable: PathBuf,
}

impl ChannelIngressWorker {
    fn from_operator(operator: &UiServerDescriptor) -> Result<Self, ChannelIngressError> {
        Ok(Self {
            workspace: PathBuf::from(&operator.workspace),
            channel_directory: PathBuf::from(&operator.channel_root),
            rey_executable: std::env::current_exe().map_err(ChannelIngressError::Executable)?,
        })
    }

    fn serve_until(self, cancelled: Arc<AtomicBool>) -> Result<(), ChannelIngressError> {
        let store = LocalChannelStore::new(self.channel_directory.clone());
        let mut next_polls = BTreeMap::<GitHubPollKey, Instant>::new();
        while !cancelled.load(Ordering::Relaxed) {
            let admitted = Self::admitted_polls(&store)?;
            let current = admitted
                .iter()
                .map(|poll| poll.key.clone())
                .collect::<BTreeSet<_>>();
            next_polls.retain(|key, _| current.contains(key));

            for poll in admitted {
                if cancelled.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let now = Instant::now();
                if next_polls.get(&poll.key).is_some_and(|next| *next > now) {
                    continue;
                }
                if let Err(error) = self.run_poll(&poll.key) {
                    if cancelled.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    let still_current = Self::admitted_polls(&store)?
                        .iter()
                        .any(|candidate| candidate.key == poll.key);
                    if still_current {
                        return Err(error);
                    }
                    continue;
                }
                next_polls.insert(poll.key, Instant::now() + poll.interval);
            }
            thread::park_timeout(Duration::from_millis(INGRESS_SCAN_INTERVAL_MS));
        }
        Ok(())
    }

    fn admitted_polls(
        store: &LocalChannelStore,
    ) -> Result<Vec<AdmittedGitHubPoll>, ChannelIngressError> {
        let status = store.status()?;
        let Some(head) = status.head_commit else {
            return Ok(Vec::new());
        };
        Ok(head
            .snapshot
            .graph
            .applications
            .iter()
            .filter_map(|application| {
                application
                    .github_inbox
                    .as_ref()
                    .map(|inbox| AdmittedGitHubPoll {
                        key: GitHubPollKey {
                            channel_head_commit_id: head.commit_id.to_string(),
                            application_id: application.id.clone(),
                            application_revision: application.revision,
                        },
                        interval: Duration::from_secs(inbox.poll_interval_seconds),
                    })
            })
            .collect())
    }

    fn run_poll(&self, poll: &GitHubPollKey) -> Result<(), ChannelIngressError> {
        let output = Command::new(&self.rey_executable)
            .arg("channels")
            .arg("--workspace")
            .arg(&self.workspace)
            .arg("--state-dir")
            .arg(&self.channel_directory)
            .arg("poll")
            .arg(&poll.application_id)
            .arg("--format")
            .arg("json")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(ChannelIngressError::PollSpawn)?;
        if output.status.success() || output.status.code() == Some(3) {
            return Ok(());
        }
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = detail.trim().chars().take(512).collect::<String>();
        Err(ChannelIngressError::PollFailed {
            application_id: poll.application_id.clone(),
            status: output
                .status
                .code()
                .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
            detail,
        })
    }
}

#[derive(Debug, Error)]
pub enum ChannelIngressError {
    #[error("current Rey executable could not be resolved: {0}")]
    Executable(io::Error),
    #[error("Channel state could not be inspected: {0}")]
    Channel(#[from] ChannelGraphError),
    #[error("resident GitHub poll command could not be started: {0}")]
    PollSpawn(io::Error),
    #[error("resident GitHub poll for {application_id} exited with {status}: {detail}")]
    PollFailed {
        application_id: String,
        status: String,
        detail: String,
    },
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Rey agent signal handling could not be installed: {0}")]
    Signal(io::Error),
    #[error("Rey agent could not start its supervised operator worker: {0}")]
    Spawn(io::Error),
    #[error("supervised background worker {0} exited without a shutdown request")]
    UnexpectedWorkerExit(String),
    #[error("supervised background worker {0} panicked")]
    WorkerPanicked(String),
    #[error("supervised operator worker failed: {0}")]
    Operator(UiError),
    #[error("supervised GitHub Channel ingress worker failed: {0}")]
    ChannelIngress(#[from] ChannelIngressError),
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use tempfile::TempDir;

    use super::{AGENT_PROCESS_SCHEMA, AGENT_TOPOLOGY_SCHEMA, AgentProcessDescriptor};
    use crate::ui::{UiServer, UiServerConfig};

    #[test]
    fn descriptor_exposes_the_supervised_rey_process_topology() {
        let workspace = TempDir::new().unwrap();
        let server = UiServer::bind(UiServerConfig {
            workspace: workspace.path().to_owned(),
            state_directory: workspace.path().join(".rey/workloads"),
            catalog_directory: "sys".into(),
            journal_directory: workspace.path().join(".rey/journal"),
            channel_directory: workspace.path().join(".rey/channels"),
            conversation_directory: workspace.path().join(".rey/conversations"),
            host: "127.0.0.1".parse::<IpAddr>().unwrap(),
            port: 0,
        })
        .unwrap();
        let descriptor = AgentProcessDescriptor::from_operator(server.descriptor());

        assert_eq!(descriptor.schema, AGENT_PROCESS_SCHEMA);
        assert_eq!(descriptor.process.role, "orchestrator");
        assert_eq!(
            descriptor.process.topology_node_id,
            descriptor.topology.root_node_id
        );
        assert_eq!(descriptor.topology.schema, AGENT_TOPOLOGY_SCHEMA);
        assert_eq!(descriptor.topology.nodes.len(), 3);
        assert_eq!(descriptor.topology.edges.len(), 2);
        assert_eq!(descriptor.topology.edges[0].relationship, "supervises");
        assert_eq!(descriptor.topology.edges[1].relationship, "supervises");
        assert_eq!(descriptor.topology.max_background_workers, 2);
        assert_eq!(
            descriptor.topology.nodes[2].node_id,
            "rey.channel-github-inbox"
        );
        assert_eq!(descriptor.operator.schema, "rey.ui-server.v1");
    }
}
