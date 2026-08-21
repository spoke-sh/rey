use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use serde::Serialize;
use thiserror::Error;

use crate::scheduler::{ManagedScheduler, SchedulerError};
use crate::ui::{UiError, UiServer, UiServerDescriptor};
use crate::version;

pub const AGENT_PROCESS_SCHEMA: &str = "rey.agent-process.v2";
pub const REY_PROCESS_SCHEMA: &str = "rey.process.v1";
pub const AGENT_TOPOLOGY_SCHEMA: &str = "rey.agent-topology.v1";
const ORCHESTRATOR_NODE_ID: &str = "rey.orchestrator";
const OPERATOR_SERVER_NODE_ID: &str = "rey.operator-http";
const SCHEDULER_PROCESS_NODE_ID: &str = "rey.scheduler";
const SCHEDULER_BRIDGE_NODE_ID: &str = "rey.scheduler-event-bridge";
const SUPERVISION_POLL_INTERVAL_MS: u64 = 50;
const MAX_BACKGROUND_WORKERS: u64 = 3;

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
                    node_id: SCHEDULER_PROCESS_NODE_ID.to_owned(),
                    kind: "background_work".to_owned(),
                    parent_node_id: Some(ORCHESTRATOR_NODE_ID.to_owned()),
                    execution: "supervised_child_process".to_owned(),
                    lifecycle: "bound_to_rey_process".to_owned(),
                    state: "running".to_owned(),
                    restart_policy: "never; fail the Rey process closed".to_owned(),
                    authority: "bounded registered scans, retained schedule control, exact admitted provider polls, and semantic-change publication only".to_owned(),
                    endpoint: None,
                },
                AgentTopologyNode {
                    node_id: SCHEDULER_BRIDGE_NODE_ID.to_owned(),
                    kind: "background_work".to_owned(),
                    parent_node_id: Some(ORCHESTRATOR_NODE_ID.to_owned()),
                    execution: "supervised_thread".to_owned(),
                    lifecycle: "bound_to_scheduler_process".to_owned(),
                    state: "running".to_owned(),
                    restart_policy: "never; fail the Rey process closed".to_owned(),
                    authority: "typed scheduler IPC projection and runtime event publication only".to_owned(),
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
                    target_node_id: SCHEDULER_PROCESS_NODE_ID.to_owned(),
                    relationship: "supervises".to_owned(),
                },
                AgentTopologyEdge {
                    source_node_id: ORCHESTRATOR_NODE_ID.to_owned(),
                    target_node_id: SCHEDULER_BRIDGE_NODE_ID.to_owned(),
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
            authority: "local orchestration, operator projection, scheduler lifecycle, bounded scans, and exact admitted provider polling only".to_owned(),
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
    scheduler: ManagedScheduler,
}

impl AgentOrchestrator {
    pub fn start(operator: UiServer) -> Result<Self, AgentError> {
        let operator_descriptor = operator.descriptor();
        let scheduler_runtime = operator.scheduler_runtime();
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
        let scheduler = match ManagedScheduler::spawn(&operator_descriptor, scheduler_runtime) {
            Ok(scheduler) => scheduler,
            Err(error) => {
                cancelled.store(true, Ordering::Relaxed);
                let _ = operator_worker.join();
                return Err(AgentError::Scheduler(error));
            }
        };
        log_info(format_args!(
            "Rey version {}; commit {}",
            version::VERSION,
            version::COMMIT_SHA
        ));
        log_info(format_args!("Started Rey process [{}]", std::process::id()));
        log_info(format_args!(
            "Agent startup complete; background worker {OPERATOR_SERVER_NODE_ID} and child process {SCHEDULER_PROCESS_NODE_ID} are running"
        ));
        Ok(Self {
            cancelled,
            operator_worker,
            scheduler,
        })
    }

    pub fn wait(self) -> Result<(), AgentError> {
        let mut operator_worker = Some(self.operator_worker);
        let mut scheduler = Some(self.scheduler);
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                log_info("Shutdown requested");
                log_info(format_args!(
                    "Agent shutdown started; stopping {OPERATOR_SERVER_NODE_ID} and {SCHEDULER_PROCESS_NODE_ID}"
                ));
                let scheduler_result = scheduler
                    .take()
                    .expect("scheduler process is present")
                    .finish(true);
                let operator_result = finish_operator_worker(
                    operator_worker.take().expect("operator worker is present"),
                    true,
                );
                scheduler_result?;
                operator_result?;
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
                let scheduler_result = scheduler
                    .take()
                    .expect("scheduler process is present")
                    .finish(true);
                operator_result?;
                return scheduler_result.map_err(AgentError::Scheduler);
            }
            if scheduler
                .as_mut()
                .expect("scheduler process is present")
                .is_finished()?
            {
                self.cancelled.store(true, Ordering::Relaxed);
                let scheduler_result = scheduler
                    .take()
                    .expect("scheduler process is present")
                    .finish(false);
                let operator_result = finish_operator_worker(
                    operator_worker.take().expect("operator worker is present"),
                    true,
                );
                scheduler_result?;
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
    #[error("supervised scheduler failed: {0}")]
    Scheduler(#[from] SchedulerError),
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
        assert_eq!(descriptor.operator.schema, "rey.ui-server.v2");
    }
}
