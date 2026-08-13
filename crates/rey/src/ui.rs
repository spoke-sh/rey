use std::{
    env,
    io::{Cursor, Read},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rey::{
    channels::{
        ChannelGraph, ChannelGraphSource, ChannelStatus, LocalChannelStore,
        MAX_CHANNEL_GRAPH_INPUT_BYTES,
    },
    conversations::{
        CONVERSATION_MESSAGE_PROPOSAL_SCHEMA, ConversationMessageProposal, ConversationSource,
        DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT, LocalConversationStore,
        MAX_CONVERSATION_MESSAGE_INPUT_BYTES,
    },
    current_environment_status,
    env::LocalEnvironmentStore,
    journal::{
        JournalAdmission, JournalAuthor, JournalAuthorKind, JournalEntryProposal, JournalLog,
        LocalJournalStore, MAX_JOURNAL_PROPOSAL_BYTES,
    },
    journal_opportunities::{DEFAULT_JOURNAL_OPPORTUNITY_LIMIT, JournalOpportunitySurface},
    journal_queries::LocalJournalQueryStore,
    journal_seed::JournalSeed,
    observations::{DEFAULT_OBSERVATION_FRONTIER_LIMIT, LocalObservationStore},
    workload_evidence::{
        WorkloadEvidenceError, workload_delta_evidence, workload_evidence_catalog,
        workload_scenario_evidence,
    },
    workloads::LocalWorkloadStore,
};
use rey_core::SemanticDigest;
use rey_environment::{DiscoveryLimits, resolve_executable};
use rey_git::{GitInspector, GitLimits, GitRepositoryStatus};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const UI_SERVER_SCHEMA: &str = "rey.ui-server.v1";
const AGENT_HEALTH_SCHEMA: &str = "rey.agent-health.v1";
const UI_ERROR_SCHEMA: &str = "rey.ui-error.v1";
const UI_CADENCE_SCHEMA: &str = "rey.ui-cadence.v1";
const UI_JOURNAL_SCHEMA: &str = "rey.ui-journal.v2";
const UI_CHANNELS_SCHEMA: &str = "rey.ui-channels.v1";
const UI_CHANNEL_WORKING_WRITE_SCHEMA: &str = "rey.ui-channel-working-write.v1";
const UI_CONVERSATION_MESSAGE_WRITE_SCHEMA: &str = "rey.ui-conversation-message-write.v1";
const MAX_REQUEST_TARGET_BYTES: usize = 4_096;
const MAX_WORKLOAD_APPROVAL_BYTES: u64 = 16 * 1_024;
const LIVE_REFRESH_INTERVAL_MS: u64 = 5_000;
const CADENCE_GIT_COMMIT_LIMIT: usize = 24;
const CADENCE_ENVIRONMENT_COMMIT_LIMIT: usize = 24;
const HIFI_GRAMMAR_REVISION: &str = "git:058c6504fc10740360717e97e687fd77bef6a5c5";
const REY_IMPLEMENTATION_REVISION: &str = env!("REY_BUILD_REVISION");
const REQUEST_RECEIVE_POLL_INTERVAL_MS: u64 = 50;

include!(concat!(env!("OUT_DIR"), "/rey_ui_assets.rs"));

#[derive(Clone, Debug)]
pub struct UiServerConfig {
    pub workspace: PathBuf,
    pub state_directory: PathBuf,
    pub catalog_directory: PathBuf,
    pub journal_directory: PathBuf,
    pub channel_directory: PathBuf,
    pub conversation_directory: PathBuf,
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UiServerDescriptor {
    pub schema: String,
    pub address: String,
    pub url: String,
    pub host: String,
    pub port: u16,
    pub loopback_only: bool,
    pub read_only: bool,
    pub journal_write_enabled: bool,
    pub workload_admission_enabled: bool,
    pub channel_write_enabled: bool,
    pub conversation_write_enabled: bool,
    pub workspace: String,
    pub catalog_root: String,
    pub channel_root: String,
    pub conversation_root: String,
    pub application: String,
    pub grammar: String,
    pub theme: String,
    pub grammar_revision: String,
    pub entry_route: String,
    pub live_refresh_interval_ms: u64,
    pub source_repository: Option<String>,
    pub implementation_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiCadenceTick {
    id: String,
    kind: String,
    state: String,
    ordinal: String,
    title: String,
    detail: String,
    revision: String,
    parent_revisions: Vec<String>,
    occurred_at_unix: Option<i64>,
    publication: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiCadenceRepositoryState {
    id: String,
    working_tree_state: String,
    staged_entries: u64,
    unstaged_entries: u64,
    untracked_entries: u64,
    conflicted_entries: u64,
    push_state: String,
    branch: Option<String>,
    head_revision: Option<String>,
    upstream: Option<String>,
    upstream_revision: Option<String>,
    ahead: Option<u64>,
    behind: Option<u64>,
    comparison_basis: String,
    complete: bool,
    scope: String,
    omissions: Vec<String>,
}

impl From<GitRepositoryStatus> for UiCadenceRepositoryState {
    fn from(status: GitRepositoryStatus) -> Self {
        Self {
            id: status.status_id.to_string(),
            working_tree_state: status.working_tree.state,
            staged_entries: status.working_tree.staged_entries,
            unstaged_entries: status.working_tree.unstaged_entries,
            untracked_entries: status.working_tree.untracked_entries,
            conflicted_entries: status.working_tree.conflicted_entries,
            push_state: status.publication.state,
            branch: status.publication.branch,
            head_revision: status.publication.head_oid,
            upstream: status.publication.upstream,
            upstream_revision: status.publication.upstream_oid,
            ahead: status.publication.ahead,
            behind: status.publication.behind,
            comparison_basis: status.publication.comparison_basis,
            complete: status.complete,
            scope: status.scope,
            omissions: status.omissions,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiCadenceLane {
    id: String,
    label: String,
    clock: String,
    ordering: String,
    complete: bool,
    ticks: Vec<UiCadenceTick>,
    omissions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiCadenceSchedule {
    id: String,
    label: String,
    source: String,
    interval_ms: u64,
    activation: String,
    authority: String,
    retention: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiCadenceProjection {
    schema: String,
    ordering: String,
    source_repository: Option<String>,
    repository_state: Option<UiCadenceRepositoryState>,
    lanes: Vec<UiCadenceLane>,
    schedules: Vec<UiCadenceSchedule>,
    omissions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct UiJournalProjection {
    schema: String,
    write_enabled: bool,
    authority: String,
    log: JournalLog,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiChannelListener {
    address: String,
    loopback_only: bool,
    authentication: String,
    warning: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiChannelProjection {
    schema: String,
    write_enabled: bool,
    authority: String,
    listener: UiChannelListener,
    status: ChannelStatus,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UiChannelWorkingWrite {
    schema: String,
    expected_head_snapshot_id: SemanticDigest,
    expected_working_snapshot_id: SemanticDigest,
    graph: ChannelGraph,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UiWorkloadApproval {
    message: String,
    expected_head: String,
    expected_working: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UiConversationMessageWrite {
    schema: String,
    expected_log_id: SemanticDigest,
    session_id: SemanticDigest,
    body: String,
    reply_to: Option<SemanticDigest>,
}

pub struct UiServer {
    server: Server,
    config: UiServerConfig,
    descriptor: UiServerDescriptor,
}

impl UiServer {
    pub fn bind(config: UiServerConfig) -> Result<Self, UiError> {
        let requested = SocketAddr::new(config.host, config.port);
        let server = Server::http(requested).map_err(|source| UiError::Bind {
            address: requested,
            detail: source.to_string(),
        })?;
        let bound = server.server_addr().to_ip().ok_or(UiError::NonIpListener)?;
        let descriptor = UiServerDescriptor {
            schema: UI_SERVER_SCHEMA.to_owned(),
            address: bound.to_string(),
            url: format!("http://{bound}"),
            host: bound.ip().to_string(),
            port: bound.port(),
            loopback_only: bound.ip().is_loopback(),
            read_only: false,
            journal_write_enabled: true,
            workload_admission_enabled: true,
            channel_write_enabled: true,
            conversation_write_enabled: true,
            workspace: config.workspace.display().to_string(),
            catalog_root: config.catalog_directory.display().to_string(),
            channel_root: config.channel_directory.display().to_string(),
            conversation_root: config.conversation_directory.display().to_string(),
            application: "tanstack_router".to_owned(),
            grammar: "kinetic".to_owned(),
            theme: "precision".to_owned(),
            grammar_revision: HIFI_GRAMMAR_REVISION.to_owned(),
            entry_route: "/explore".to_owned(),
            live_refresh_interval_ms: LIVE_REFRESH_INTERVAL_MS,
            source_repository: None,
            implementation_revision: REY_IMPLEMENTATION_REVISION.to_owned(),
        };
        Ok(Self {
            server,
            config,
            descriptor,
        })
    }

    #[must_use]
    pub fn descriptor(&self) -> UiServerDescriptor {
        self.descriptor.clone()
    }

    pub fn serve_until(self, cancelled: Arc<AtomicBool>) -> Result<(), UiError> {
        while !cancelled.load(Ordering::Relaxed) {
            let Some(mut request) = self
                .server
                .recv_timeout(Duration::from_millis(REQUEST_RECEIVE_POLL_INTERVAL_MS))
                .map_err(UiError::Receive)?
            else {
                continue;
            };
            let response = self.route(&mut request);
            request.respond(response).map_err(UiError::Respond)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn serve_bounded(self, max_requests: Option<usize>) -> Result<(), UiError> {
        let mut served = 0_usize;
        loop {
            if max_requests.is_some_and(|limit| served >= limit) {
                return Ok(());
            }
            let mut request = self.server.recv().map_err(UiError::Receive)?;
            let response = self.route(&mut request);
            request.respond(response).map_err(UiError::Respond)?;
            served = served.saturating_add(1);
        }
    }

    fn route(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request.url().len() > MAX_REQUEST_TARGET_BYTES {
            return json_error(
                StatusCode(414),
                "request_target_limit",
                "request target exceeds the 4096-byte limit",
            );
        }
        let (path, query) = request
            .url()
            .split_once('?')
            .map_or((request.url(), None), |(path, query)| (path, Some(query)));
        let head = request.method() == &Method::Head;
        if request.method() == &Method::Post && path == "/api/v1/journal" {
            return self.admit_journal(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/workloads/admit" {
            return self.admit_workloads(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/channels/working" {
            return self.write_channel_working(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/conversations/messages" {
            return self.write_conversation_message(request);
        }
        if request.method() != &Method::Get && !head {
            return with_header(
                json_error(
                    StatusCode(405),
                    "method_not_allowed",
                    "this Rey UI route is read-only; use GET or HEAD",
                ),
                "Allow",
                "GET, HEAD",
            );
        }

        let response = match path {
            "/" => redirect_response("/explore"),
            "/api/v1/health" => self.health(),
            "/api/v1/agent" => self.agent(),
            "/api/v1/cadence" => self.cadence(),
            "/api/v1/channels" => self.channels(),
            "/api/v1/conversations" => self.conversations(),
            "/api/v1/environment" => self.environment(),
            "/api/v1/journal" => self.journal(),
            "/api/v1/journal/opportunities" => self.journal_opportunities(),
            "/api/v1/journal/queries" => self.journal_queries(),
            "/api/v1/journal/seed" => self.journal_seed(query),
            "/api/v1/observations" => self.observations(),
            "/api/v1/workloads" => self.workloads(),
            "/api/v1/workloads/evidence" => self.workload_evidence(),
            path if path.starts_with("/api/v1/workloads/") => self.exact_workload_evidence(path),
            path if path.starts_with("/api/") => json_error(
                StatusCode(404),
                "api_route_not_found",
                "no read-only Rey UI API route matches this target",
            ),
            path if path.starts_with("/assets/") => static_ui_asset(path).unwrap_or_else(|| {
                json_error(
                    StatusCode(404),
                    "static_asset_not_found",
                    "no embedded Rey UI asset matches this target",
                )
            }),
            _ => index_response(),
        };
        if head {
            response.with_data(Cursor::new(Vec::new()), Some(0))
        } else {
            response
        }
    }

    fn health(&self) -> Response<Cursor<Vec<u8>>> {
        let agent = crate::agent::AgentProcessDescriptor::from_operator(self.descriptor());
        json_response(
            StatusCode(200),
            &json!({
                "schema": AGENT_HEALTH_SCHEMA,
                "status": "ready",
                "agent": agent,
                "server": self.descriptor,
            }),
        )
    }

    fn agent(&self) -> Response<Cursor<Vec<u8>>> {
        json_response(
            StatusCode(200),
            &crate::agent::AgentProcessDescriptor::from_operator(self.descriptor()),
        )
    }

    fn workloads(&self) -> Response<Cursor<Vec<u8>>> {
        let result = {
            let store = LocalWorkloadStore::new(self.config.state_directory.clone());
            super::current_workload_list(
                &store,
                &self.config.workspace,
                &self.config.catalog_directory,
            )
            .map_err(|error| error.to_string())
        };
        match result {
            Ok(list) => json_response(StatusCode(200), &list),
            Err(detail) => json_error(StatusCode(500), "portfolio_unavailable", &detail),
        }
    }

    fn workload_evidence(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalWorkloadStore::new(self.config.state_directory.clone());
        let catalog = match store.head_catalog() {
            Ok(catalog) => catalog,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "workload_evidence_unavailable",
                    &error.to_string(),
                );
            }
        };
        let state = match store.load() {
            Ok(state) => state,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "workload_evidence_unavailable",
                    &error.to_string(),
                );
            }
        };
        let result = workload_evidence_catalog(&catalog, &state);
        match result {
            Ok(evidence) => json_response(StatusCode(200), &evidence),
            Err(error) => json_error(
                StatusCode(500),
                "workload_evidence_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn exact_workload_evidence(&self, path: &str) -> Response<Cursor<Vec<u8>>> {
        let Some(tail) = path.strip_prefix("/api/v1/workloads/") else {
            return json_error(
                StatusCode(404),
                "workload_evidence_route_not_found",
                "no exact workload evidence route matches this target",
            );
        };
        let segments = match tail
            .split('/')
            .map(decode_path_segment)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(segments) => segments,
            Err(detail) => {
                return json_error(StatusCode(400), "workload_evidence_route_invalid", detail);
            }
        };
        let store = LocalWorkloadStore::new(self.config.state_directory.clone());
        let catalog = match store.head_catalog() {
            Ok(catalog) => catalog,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "workload_evidence_unavailable",
                    &error.to_string(),
                );
            }
        };
        let state = match store.load() {
            Ok(state) => state,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "workload_evidence_unavailable",
                    &error.to_string(),
                );
            }
        };
        let result = match segments.as_slice() {
            [workload_id, kind, execution_id]
                if kind == "scenarios" && !workload_id.is_empty() && !execution_id.is_empty() =>
            {
                workload_scenario_evidence(&catalog, &state, workload_id, execution_id)
                    .map(|evidence| json_response(StatusCode(200), &evidence))
            }
            [workload_id, kind, delta_id]
                if kind == "deltas" && !workload_id.is_empty() && !delta_id.is_empty() =>
            {
                workload_delta_evidence(&catalog, &state, workload_id, delta_id)
                    .map(|evidence| json_response(StatusCode(200), &evidence))
            }
            _ => {
                return json_error(
                    StatusCode(404),
                    "workload_evidence_route_not_found",
                    "expected /api/v1/workloads/{workload-id}/scenarios/{execution-id} or /api/v1/workloads/{workload-id}/deltas/{delta-id}",
                );
            }
        };
        match result {
            Ok(response) => response,
            Err(
                error @ (WorkloadEvidenceError::UnknownWorkload(_)
                | WorkloadEvidenceError::EvidenceUnavailable(_)
                | WorkloadEvidenceError::UnknownScenario { .. }
                | WorkloadEvidenceError::UnknownDelta { .. }),
            ) => json_error(
                StatusCode(404),
                "workload_evidence_not_found",
                &error.to_string(),
            ),
            Err(error @ WorkloadEvidenceError::InvalidRetainedResult(_)) => json_error(
                StatusCode(500),
                "workload_evidence_invalid",
                &error.to_string(),
            ),
        }
    }

    fn channels(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalChannelStore::new(self.config.channel_directory.clone());
        match store.status() {
            Ok(status) => json_response(
                StatusCode(200),
                &UiChannelProjection {
                    schema: UI_CHANNELS_SCHEMA.to_owned(),
                    write_enabled: self.descriptor.channel_write_enabled,
                    authority: "unauthenticated_channel_working_write; no INDEX, HEAD, relay, or execution authority"
                        .to_owned(),
                    listener: UiChannelListener {
                        address: self.descriptor.address.clone(),
                        loopback_only: self.descriptor.loopback_only,
                        authentication: "none".to_owned(),
                        warning: if self.descriptor.loopback_only {
                            "any local client that can reach this listener may replace Channel WORKING"
                                .to_owned()
                        } else {
                            "any network client that can reach this listener may replace Channel WORKING without authentication"
                                .to_owned()
                        },
                    },
                    status,
                },
            ),
            Err(error) => json_error(
                StatusCode(500),
                "channel_status_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn conversations(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalConversationStore::new(self.config.conversation_directory.clone());
        match store.transcript(None, DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT) {
            Ok(transcript) => json_response(StatusCode(200), &transcript),
            Err(error) => json_error(
                StatusCode(500),
                "conversation_transcript_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn write_conversation_message(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request_header(request, "Content-Type") != Some("application/json") {
            return json_error(
                StatusCode(415),
                "conversation_message_content_type",
                "conversation message writes require Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_CONVERSATION_MESSAGE_INPUT_BYTES)
        {
            return json_error(
                StatusCode(413),
                "conversation_message_body_limit",
                "conversation message write exceeds the 65536-byte limit",
            );
        }
        let mut bytes = Vec::new();
        if let Err(error) = request
            .as_reader()
            .take(MAX_CONVERSATION_MESSAGE_INPUT_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return json_error(
                StatusCode(400),
                "conversation_message_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_CONVERSATION_MESSAGE_INPUT_BYTES {
            return json_error(
                StatusCode(413),
                "conversation_message_body_limit",
                "conversation message write exceeds the 65536-byte limit",
            );
        }
        let write: UiConversationMessageWrite = match serde_json::from_slice(&bytes) {
            Ok(write) => write,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "conversation_message_json_invalid",
                    &error.to_string(),
                );
            }
        };
        if write.schema != UI_CONVERSATION_MESSAGE_WRITE_SCHEMA {
            return json_error(
                StatusCode(422),
                "conversation_message_schema_invalid",
                "expected rey.ui-conversation-message-write.v1",
            );
        }
        let store = LocalConversationStore::new(self.config.conversation_directory.clone());
        let log = match store.load() {
            Ok(log) => log,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "conversation_transcript_unavailable",
                    &error.to_string(),
                );
            }
        };
        let session = match log.session(write.session_id.as_str()) {
            Ok(session) => session,
            Err(error) => {
                return json_error(
                    StatusCode(409),
                    "conversation_session_unavailable",
                    &error.to_string(),
                );
            }
        };
        let Some(author_id) = session.proposal.browser_writer_id.clone() else {
            return json_error(
                StatusCode(403),
                "conversation_browser_writer_unavailable",
                "the exact conversation session declares no human browser writer",
            );
        };
        let proposal = ConversationMessageProposal {
            schema: CONVERSATION_MESSAGE_PROPOSAL_SCHEMA.to_owned(),
            session_id: write.session_id,
            author_id,
            body: write.body,
            reply_to: write.reply_to,
        };
        let source = ConversationSource::from_bytes(
            format!(
                "rey-ui://{}/conversations/{}",
                self.descriptor.address, proposal.session_id
            ),
            &bytes,
        );
        match store.admit_message_if_log(
            proposal,
            source,
            chrono::Utc::now().timestamp(),
            &write.expected_log_id,
        ) {
            Ok(result) => json_response(StatusCode(201), &result),
            Err(error) => json_error(
                StatusCode(409),
                "conversation_message_rejected",
                &error.to_string(),
            ),
        }
    }

    fn observations(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalObservationStore::new(self.config.channel_directory.clone());
        match store
            .load()
            .and_then(|log| log.frontier(DEFAULT_OBSERVATION_FRONTIER_LIMIT))
        {
            Ok(frontier) => json_response(StatusCode(200), &frontier),
            Err(error) => json_error(
                StatusCode(500),
                "observation_frontier_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn journal_seed(&self, query: Option<&str>) -> Response<Cursor<Vec<u8>>> {
        let observation_ids = match journal_seed_observation_ids(query) {
            Ok(observation_ids) => observation_ids,
            Err(detail) => {
                return json_error(StatusCode(400), "journal_seed_query_invalid", &detail);
            }
        };
        let store = LocalObservationStore::new(self.config.channel_directory.clone());
        let log = match store.load() {
            Ok(log) => log,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "observation_frontier_unavailable",
                    &error.to_string(),
                );
            }
        };
        match JournalSeed::from_log(
            &log,
            &observation_ids,
            JournalAuthor {
                kind: JournalAuthorKind::Human,
                id: "operator".to_owned(),
            },
        ) {
            Ok(seed) => json_response(StatusCode(200), &seed),
            Err(error) => json_error(StatusCode(422), "journal_seed_rejected", &error.to_string()),
        }
    }

    fn write_channel_working(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request_header(request, "Content-Type") != Some("application/json") {
            return json_error(
                StatusCode(415),
                "channel_working_content_type",
                "Channel WORKING writes require Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_CHANNEL_GRAPH_INPUT_BYTES)
        {
            return json_error(
                StatusCode(413),
                "channel_working_body_limit",
                "Channel WORKING write exceeds the 1048576-byte limit",
            );
        }
        let mut bytes = Vec::new();
        if let Err(error) = request
            .as_reader()
            .take(MAX_CHANNEL_GRAPH_INPUT_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return json_error(
                StatusCode(400),
                "channel_working_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_CHANNEL_GRAPH_INPUT_BYTES {
            return json_error(
                StatusCode(413),
                "channel_working_body_limit",
                "Channel WORKING write exceeds the 1048576-byte limit",
            );
        }
        let write: UiChannelWorkingWrite = match serde_json::from_slice(&bytes) {
            Ok(write) => write,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "channel_working_invalid",
                    &error.to_string(),
                );
            }
        };
        if write.schema != UI_CHANNEL_WORKING_WRITE_SCHEMA {
            return json_error(
                StatusCode(400),
                "channel_working_schema",
                "expected schema rey.ui-channel-working-write.v1",
            );
        }
        let graph_bytes = match serde_json::to_vec(&write.graph) {
            Ok(bytes) => bytes,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "channel_working_invalid",
                    &error.to_string(),
                );
            }
        };
        let store = LocalChannelStore::new(self.config.channel_directory.clone());
        match store.apply_if_current(
            write.graph,
            ChannelGraphSource::worktree("ui:///channels/working".to_owned(), &graph_bytes),
            &write.expected_head_snapshot_id,
            &write.expected_working_snapshot_id,
        ) {
            Ok(result) => json_response(
                if result.applied {
                    StatusCode(201)
                } else {
                    StatusCode(200)
                },
                &result,
            ),
            Err(error) => json_error(
                StatusCode(409),
                "channel_working_rejected",
                &error.to_string(),
            ),
        }
    }

    fn admit_workloads(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request_header(request, "Content-Type") != Some("application/json") {
            return json_error(
                StatusCode(415),
                "workload_admission_content_type",
                "workload admission requires Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_WORKLOAD_APPROVAL_BYTES)
        {
            return json_error(
                StatusCode(413),
                "workload_admission_body_limit",
                "workload approval exceeds the 16384-byte limit",
            );
        }
        let mut bytes = Vec::new();
        if let Err(error) = request
            .as_reader()
            .take(MAX_WORKLOAD_APPROVAL_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return json_error(
                StatusCode(400),
                "workload_admission_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_WORKLOAD_APPROVAL_BYTES {
            return json_error(
                StatusCode(413),
                "workload_admission_body_limit",
                "workload approval exceeds the 16384-byte limit",
            );
        }
        let approval: UiWorkloadApproval = match serde_json::from_slice(&bytes) {
            Ok(approval) => approval,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "workload_admission_invalid",
                    &error.to_string(),
                );
            }
        };
        let store = LocalWorkloadStore::new(self.config.state_directory.clone());
        match super::admit_workload_files(
            &store,
            &self.config.workspace,
            &self.config.catalog_directory,
            approval.message,
            &approval.expected_head,
            &approval.expected_working,
        ) {
            Ok(result) => json_response(StatusCode(201), &result),
            Err(error) => json_error(
                StatusCode(409),
                "workload_admission_rejected",
                &error.to_string(),
            ),
        }
    }

    fn journal(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalJournalStore::new(self.config.journal_directory.clone());
        match store.load() {
            Ok(log) => json_response(
                StatusCode(200),
                &UiJournalProjection {
                    schema: UI_JOURNAL_SCHEMA.to_owned(),
                    write_enabled: self.descriptor.journal_write_enabled,
                    authority: "unauthenticated_journal_admission".to_owned(),
                    log,
                },
            ),
            Err(error) => json_error(StatusCode(500), "journal_unavailable", &error.to_string()),
        }
    }

    fn journal_opportunities(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalJournalStore::new(self.config.journal_directory.clone());
        let log = match store.load() {
            Ok(log) => log,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "journal_opportunities_unavailable",
                    &error.to_string(),
                );
            }
        };
        match JournalOpportunitySurface::derive(&log, DEFAULT_JOURNAL_OPPORTUNITY_LIMIT) {
            Ok(surface) => json_response(StatusCode(200), &surface),
            Err(error) => json_error(
                StatusCode(500),
                "journal_opportunities_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn journal_queries(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalJournalQueryStore::new(self.config.journal_directory.clone());
        match store.load() {
            Ok(state) => json_response(StatusCode(200), &state),
            Err(error) => json_error(
                StatusCode(500),
                "journal_queries_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn admit_journal(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        let content_type = request_header(request, "Content-Type");
        if content_type != Some("application/json") {
            return json_error(
                StatusCode(415),
                "journal_content_type",
                "journal writes require Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_JOURNAL_PROPOSAL_BYTES)
        {
            return json_error(
                StatusCode(413),
                "journal_body_limit",
                "journal proposal exceeds the 1048576-byte limit",
            );
        }
        let mut bytes = Vec::new();
        let read = request
            .as_reader()
            .take(MAX_JOURNAL_PROPOSAL_BYTES.saturating_add(1))
            .read_to_end(&mut bytes);
        if let Err(error) = read {
            return json_error(
                StatusCode(400),
                "journal_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_JOURNAL_PROPOSAL_BYTES {
            return json_error(
                StatusCode(413),
                "journal_body_limit",
                "journal proposal exceeds the 1048576-byte limit",
            );
        }
        let proposal: JournalEntryProposal = match serde_json::from_slice(&bytes) {
            Ok(proposal) => proposal,
            Err(error) => {
                return json_error(StatusCode(400), "journal_json_invalid", &error.to_string());
            }
        };
        if proposal.author.kind != JournalAuthorKind::Human {
            return json_error(
                StatusCode(422),
                "journal_author_invalid",
                "the human UI admission endpoint accepts only human-authored entries; agents use rey journal add",
            );
        }
        let admitted_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let store = LocalJournalStore::new(self.config.journal_directory.clone());
        match store.admit(proposal, &admitted_at) {
            Ok(admission) => journal_admission_response(&admission),
            Err(error) => json_error(
                StatusCode(422),
                "journal_admission_rejected",
                &error.to_string(),
            ),
        }
    }

    fn environment(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalEnvironmentStore::default_for_workspace(&self.config.workspace);
        match current_environment_status(
            &store,
            &self.config.workspace,
            DiscoveryLimits::default(),
            None,
            4_096,
        ) {
            Ok(status) => json_response(StatusCode(200), &status),
            Err(error) => json_error(
                StatusCode(500),
                "environment_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn cadence(&self) -> Response<Cursor<Vec<u8>>> {
        match self.cadence_projection() {
            Ok(cadence) => json_response(StatusCode(200), &cadence),
            Err(detail) => json_error(StatusCode(500), "cadence_unavailable", &detail),
        }
    }

    fn cadence_projection(&self) -> Result<UiCadenceProjection, String> {
        let environment_store =
            LocalEnvironmentStore::default_for_workspace(&self.config.workspace);
        let history = environment_store
            .load()
            .map_err(|error| error.to_string())?;
        let index = environment_store
            .load_index(&history)
            .map_err(|error| error.to_string())?;
        let environment_total = history.commits.len();
        let mut environment_ticks = Vec::new();
        if let Some(index) = index {
            environment_ticks.push(UiCadenceTick {
                id: index.index_id.to_string(),
                kind: "rey_admission".to_owned(),
                state: "staged".to_owned(),
                ordinal: "INDEX".to_owned(),
                title: "Environment admission index".to_owned(),
                detail: format!(
                    "{} capabilities staged against {}",
                    index.snapshot.capabilities.len(),
                    index
                        .base_commit_id
                        .as_ref()
                        .map_or("EMPTY", |revision| revision.as_str())
                ),
                revision: index.index_id.to_string(),
                parent_revisions: index
                    .base_commit_id
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                occurred_at_unix: None,
                publication: None,
            });
        }
        environment_ticks.extend(
            history
                .commits
                .iter()
                .rev()
                .take(CADENCE_ENVIRONMENT_COMMIT_LIMIT)
                .map(|commit| UiCadenceTick {
                    id: commit.commit_id.to_string(),
                    kind: "rey_admission".to_owned(),
                    state: "committed".to_owned(),
                    ordinal: format!("ENV@{}", commit.sequence),
                    title: commit.message.clone(),
                    detail: format!(
                        "{} capabilities · {} snapshot",
                        commit.snapshot.capabilities.len(),
                        if commit.snapshot.complete {
                            "complete"
                        } else {
                            "incomplete"
                        }
                    ),
                    revision: commit.commit_id.to_string(),
                    parent_revisions: commit
                        .parent_commit_id
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                    occurred_at_unix: None,
                    publication: None,
                }),
        );
        let environment_complete = environment_total <= CADENCE_ENVIRONMENT_COMMIT_LIMIT;
        let environment_omissions = if environment_complete {
            Vec::new()
        } else {
            vec![format!(
                "{} older environment admissions omitted",
                environment_total - CADENCE_ENVIRONMENT_COMMIT_LIMIT
            )]
        };

        let mut projection_omissions = Vec::new();
        let source_repository = None;
        let mut repository_state: Option<UiCadenceRepositoryState> = None;
        let git_lane = match env::var_os("PATH")
            .map(|path| env::split_paths(&path).collect::<Vec<_>>())
            .and_then(|paths| resolve_executable("git", &paths))
        {
            Some(git_program) => {
                let inspector = GitInspector {
                    git_program,
                    workspace: self.config.workspace.clone(),
                    limits: GitLimits::default(),
                };
                match inspector.inspect_repository_status() {
                    Ok(Some(status)) => repository_state = Some(status.into()),
                    Ok(None) => projection_omissions
                        .push("Git repository state is absent for this workspace".to_owned()),
                    Err(error) => projection_omissions
                        .push(format!("Git repository state inspection failed: {error}")),
                }
                match inspector.inspect_recent_commits(CADENCE_GIT_COMMIT_LIMIT) {
                    Ok(Some(sequence)) => {
                        let commit_oids = sequence
                            .commits
                            .iter()
                            .map(|commit| commit.commit_oid.clone())
                            .collect::<Vec<_>>();
                        let upstream_revision = repository_state
                            .as_ref()
                            .and_then(|state| state.upstream_revision.as_deref());
                        let publications = match inspector
                            .inspect_commit_publication(&commit_oids, upstream_revision)
                        {
                            Ok(Some(states)) => states,
                            Ok(None) => vec!["unknown".to_owned(); commit_oids.len()],
                            Err(error) => {
                                projection_omissions.push(format!(
                                    "Git commit publication inspection failed: {error}"
                                ));
                                vec!["unknown".to_owned(); commit_oids.len()]
                            }
                        };
                        UiCadenceLane {
                            id: sequence.sequence_id.to_string(),
                            label: "Git commits".to_owned(),
                            clock: "reachable_head_history".to_owned(),
                            ordering: "newest_first".to_owned(),
                            complete: sequence.complete,
                            ticks: sequence
                                .commits
                                .iter()
                                .enumerate()
                                .map(|(index, commit)| UiCadenceTick {
                                    id: format!("git:{}", commit.commit_oid),
                                    kind: "git_commit".to_owned(),
                                    state: "observed".to_owned(),
                                    ordinal: if index == 0 {
                                        "HEAD".to_owned()
                                    } else {
                                        format!("HEAD~{index}")
                                    },
                                    title: commit.subject.clone(),
                                    detail: format!(
                                        "{} parent{} · {}",
                                        commit.parent_oids.len(),
                                        if commit.parent_oids.len() == 1 {
                                            ""
                                        } else {
                                            "s"
                                        },
                                        sequence.object_format
                                    ),
                                    revision: commit.commit_oid.clone(),
                                    parent_revisions: commit.parent_oids.clone(),
                                    occurred_at_unix: Some(commit.committed_at_unix),
                                    publication: publications.get(index).cloned(),
                                })
                                .collect(),
                            omissions: sequence.omissions,
                        }
                    }
                    Ok(None) => UiCadenceLane {
                        id: "git:absent".to_owned(),
                        label: "Git commits".to_owned(),
                        clock: "reachable_head_history".to_owned(),
                        ordering: "newest_first".to_owned(),
                        complete: false,
                        ticks: Vec::new(),
                        omissions: vec!["workspace is not a bounded Git repository".to_owned()],
                    },
                    Err(error) => {
                        projection_omissions
                            .push(format!("Git cadence inspection failed: {error}"));
                        UiCadenceLane {
                            id: "git:error".to_owned(),
                            label: "Git commits".to_owned(),
                            clock: "reachable_head_history".to_owned(),
                            ordering: "newest_first".to_owned(),
                            complete: false,
                            ticks: Vec::new(),
                            omissions: vec![error.to_string()],
                        }
                    }
                }
            }
            None => UiCadenceLane {
                id: "git:unavailable".to_owned(),
                label: "Git commits".to_owned(),
                clock: "reachable_head_history".to_owned(),
                ordering: "newest_first".to_owned(),
                complete: false,
                ticks: Vec::new(),
                omissions: vec!["git executable was not found on the declared PATH".to_owned()],
            },
        };
        let environment_lane = UiCadenceLane {
            id: history.head().map_or_else(
                || "environment:unborn".to_owned(),
                |head| head.commit_id.to_string(),
            ),
            label: "Rey admissions".to_owned(),
            clock: "environment_sequence".to_owned(),
            ordering: "newest_first".to_owned(),
            complete: environment_complete,
            ticks: environment_ticks,
            omissions: environment_omissions,
        };
        projection_omissions.push(
            "Git and Rey admission clocks have no proven total ordering; environment v1 commits retain sequence but no wall time"
                .to_owned(),
        );

        Ok(UiCadenceProjection {
            schema: UI_CADENCE_SCHEMA.to_owned(),
            ordering: "partial".to_owned(),
            source_repository,
            repository_state,
            lanes: vec![git_lane, environment_lane],
            schedules: vec![
                UiCadenceSchedule {
                    id: "ui.portfolio.passive-revalidation".to_owned(),
                    label: "Portfolio scan".to_owned(),
                    source: "/api/v1/workloads".to_owned(),
                    interval_ms: LIVE_REFRESH_INTERVAL_MS,
                    activation: "application_mounted".to_owned(),
                    authority: "mounted_browser_projection".to_owned(),
                    retention: "last_good_document".to_owned(),
                },
                UiCadenceSchedule {
                    id: "ui.environment.passive-revalidation".to_owned(),
                    label: "Environment scan".to_owned(),
                    source: "/api/v1/environment".to_owned(),
                    interval_ms: LIVE_REFRESH_INTERVAL_MS,
                    activation: "environment_route_mounted".to_owned(),
                    authority: "mounted_browser_projection".to_owned(),
                    retention: "last_good_document".to_owned(),
                },
                UiCadenceSchedule {
                    id: "ui.channels.passive-revalidation".to_owned(),
                    label: "Channel-backed Feed scan".to_owned(),
                    source: "/api/v1/channels".to_owned(),
                    interval_ms: LIVE_REFRESH_INTERVAL_MS,
                    activation: "feed_route_mounted".to_owned(),
                    authority: "mounted_browser_projection".to_owned(),
                    retention: "last_good_document".to_owned(),
                },
                UiCadenceSchedule {
                    id: "ui.observations.passive-revalidation".to_owned(),
                    label: "Observation frontier scan".to_owned(),
                    source: "/api/v1/observations".to_owned(),
                    interval_ms: LIVE_REFRESH_INTERVAL_MS,
                    activation: "application_or_feed_mounted".to_owned(),
                    authority: "mounted_browser_projection".to_owned(),
                    retention: "last_good_document".to_owned(),
                },
                UiCadenceSchedule {
                    id: "ui.cadence.passive-revalidation".to_owned(),
                    label: "Cadence scan".to_owned(),
                    source: "/api/v1/cadence".to_owned(),
                    interval_ms: LIVE_REFRESH_INTERVAL_MS,
                    activation: "cadence_route_mounted".to_owned(),
                    authority: "mounted_browser_projection".to_owned(),
                    retention: "last_good_document".to_owned(),
                },
            ],
            omissions: projection_omissions,
        })
    }
}

fn decode_path_segment(value: &str) -> Result<String, &'static str> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("exact workload evidence paths contain a truncated percent escape");
            }
            let high = hex_digit(bytes[index + 1])
                .ok_or("exact workload evidence paths contain an invalid percent escape")?;
            let low = hex_digit(bytes[index + 2])
                .ok_or("exact workload evidence paths contain an invalid percent escape")?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded)
        .map_err(|_| "exact workload evidence path segments must be UTF-8")?;
    if decoded.is_empty() || decoded.contains(['/', '\0']) {
        return Err("exact workload evidence path segments must be non-empty resource identities");
    }
    Ok(decoded)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn index_response() -> Response<Cursor<Vec<u8>>> {
    let response = static_response(INDEX_HTML, "text/html; charset=utf-8");
    with_header(
        response,
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
    )
}

fn redirect_response(location: &str) -> Response<Cursor<Vec<u8>>> {
    let response = Response::from_data(Vec::new()).with_status_code(StatusCode(307));
    let response = with_header(response, "Location", location);
    with_common_headers(response, "no-cache")
}

fn static_response(bytes: &[u8], content_type: &str) -> Response<Cursor<Vec<u8>>> {
    let response = Response::from_data(bytes.to_vec()).with_status_code(StatusCode(200));
    let response = with_header(response, "Content-Type", content_type);
    with_common_headers(response, "no-cache")
}

fn static_ui_asset(path: &str) -> Option<Response<Cursor<Vec<u8>>>> {
    STATIC_UI_ASSETS
        .iter()
        .find(|(asset_path, _, _)| *asset_path == path)
        .map(|(_, bytes, content_type)| static_response(bytes, content_type))
}

fn json_response(value_status: StatusCode, value: &impl Serialize) -> Response<Cursor<Vec<u8>>> {
    match serde_json::to_vec(value) {
        Ok(bytes) => {
            let response = Response::from_data(bytes).with_status_code(value_status);
            let response = with_header(response, "Content-Type", "application/json; charset=utf-8");
            with_common_headers(response, "no-store")
        }
        Err(error) => json_error(StatusCode(500), "json_encoding_failed", &error.to_string()),
    }
}

fn journal_admission_response(admission: &JournalAdmission) -> Response<Cursor<Vec<u8>>> {
    json_response(
        if admission.admitted {
            StatusCode(201)
        } else {
            StatusCode(200)
        },
        admission,
    )
}

fn journal_seed_observation_ids(query: Option<&str>) -> Result<Vec<String>, String> {
    let query = query.ok_or_else(|| "journal seed requires ?observations=id[,id]".to_owned())?;
    let mut value = None;
    for pair in query.split('&') {
        let (name, encoded) = pair
            .split_once('=')
            .ok_or_else(|| "journal seed query parameters require names and values".to_owned())?;
        if name != "observations" || value.is_some() {
            return Err("journal seed accepts exactly one observations parameter".to_owned());
        }
        value = Some(percent_decode_query(encoded)?);
    }
    let ids = value
        .ok_or_else(|| "journal seed requires an observations parameter".to_owned())?
        .split(',')
        .map(str::trim)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if ids.is_empty() || ids.iter().any(String::is_empty) {
        return Err("journal seed observation identities cannot be empty".to_owned());
    }
    Ok(ids)
}

fn percent_decode_query(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("journal seed query has invalid percent encoding".to_owned());
            }
            let high = query_hex(bytes[index + 1])?;
            let low = query_hex(bytes[index + 2])?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| "journal seed query must decode to UTF-8".to_owned())
}

fn query_hex(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("journal seed query has invalid percent encoding".to_owned()),
    }
}

fn request_header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.as_str())
}

fn json_error(status: StatusCode, category: &str, detail: &str) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&json!({
        "schema": UI_ERROR_SCHEMA,
        "category": category,
        "detail": detail,
    }))
    .unwrap_or_else(|_| {
        b"{\"schema\":\"rey.ui-error.v1\",\"category\":\"encoding_failed\"}".to_vec()
    });
    let response = Response::from_data(body).with_status_code(status);
    let response = with_header(response, "Content-Type", "application/json; charset=utf-8");
    with_common_headers(response, "no-store")
}

fn with_common_headers(
    response: Response<Cursor<Vec<u8>>>,
    cache_control: &str,
) -> Response<Cursor<Vec<u8>>> {
    let response = with_header(response, "Cache-Control", cache_control);
    let response = with_header(response, "X-Content-Type-Options", "nosniff");
    let response = with_header(response, "Referrer-Policy", "no-referrer");
    with_header(response, "Cross-Origin-Resource-Policy", "same-origin")
}

fn with_header(
    response: Response<Cursor<Vec<u8>>>,
    name: &str,
    value: &str,
) -> Response<Cursor<Vec<u8>>> {
    response.with_header(Header::from_bytes(name, value).expect("static HTTP header is valid"))
}

#[derive(Debug, Error)]
pub enum UiError {
    #[error("operator server could not bind {address}: {detail}")]
    Bind { address: SocketAddr, detail: String },
    #[error("operator listener did not resolve to an IP socket")]
    NonIpListener,
    #[error("operator request receive failed: {0}")]
    Receive(std::io::Error),
    #[error("operator response failed: {0}")]
    Respond(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpStream,
        thread,
    };

    use tempfile::TempDir;

    use super::{STATIC_UI_ASSETS, UiServer, UiServerConfig};
    use rey::{
        channels::LocalChannelStore,
        conversations::{
            ConversationSessionProposal, ConversationSource as TranscriptSource,
            LocalConversationStore,
        },
        observations::{LocalObservationStore, ObservationProposal, ObservationSource},
        workloads::LocalWorkloadStore,
    };

    #[test]
    fn server_admits_unauthenticated_journal_writes_and_serves_deep_links() {
        let workspace = TempDir::new().unwrap();
        let channel_directory = workspace.path().join(".rey/channels");
        let channel_store = LocalChannelStore::new(channel_directory.clone());
        let observation_store = LocalObservationStore::new(channel_directory);
        let observation_proposal: ObservationProposal = serde_json::from_value(serde_json::json!({
            "schema": "rey.observation.v1",
            "kind": "finding",
            "author": { "kind": "agent", "id": "codex" },
            "subject_locator": "rey+local://workload/context-anchor-survey?revision=1",
            "body": "The exact survey bearing remains unresolved.",
            "desired_delta": "Admit one bounded Journal synthesis.",
            "completeness": "complete",
            "omissions": [],
            "evidence": [],
            "supersedes": null
        }))
        .unwrap();
        let observation = observation_store
            .admit_and_broadcast(
                observation_proposal,
                ObservationSource::workspace_file(
                    "workspace://observation.yaml".to_owned(),
                    b"ui-observation",
                ),
                Vec::new(),
                None,
                &channel_store.status().unwrap().working,
                1,
            )
            .unwrap()
            .observation;
        let package_directory = workspace.path().join("sys/context-anchor-survey");
        fs::create_dir_all(&package_directory).unwrap();
        fs::write(
            package_directory.join("workload.yaml"),
            include_str!("../../../sys/context-anchor-survey/workload.yaml"),
        )
        .unwrap();
        let workload_store = LocalWorkloadStore::default_for_workspace(workspace.path());
        let working_revision = workload_store
            .status(workspace.path(), std::path::Path::new("sys"))
            .unwrap()
            .working
            .snapshot_revision;
        assert!(!workload_store.path().exists());
        let conversation_directory = workspace.path().join(".rey/conversations");
        let conversation_store = LocalConversationStore::new(conversation_directory.clone());
        let conversation_proposal: ConversationSessionProposal =
            serde_json::from_value(serde_json::json!({
                "schema": "rey.conversation-session-proposal.v1",
                "title": "Operator and agent conversation",
                "transport": {
                    "kind": "local_transcript",
                    "provider": "rey.local-transcript",
                    "provider_revision": "v1"
                },
                "participants": [
                    { "participant_id": "operator", "kind": "human", "label": "Operator" },
                    { "participant_id": "codex", "kind": "agent", "label": "Codex" }
                ],
                "writer_ids": ["operator", "codex"],
                "browser_writer_id": "operator"
            }))
            .unwrap();
        let conversation_source = serde_json::to_vec(&conversation_proposal).unwrap();
        conversation_store
            .admit_session(
                conversation_proposal,
                TranscriptSource::from_bytes(
                    "workspace://conversation-session.yaml".to_owned(),
                    &conversation_source,
                ),
                1,
            )
            .unwrap();
        let server = UiServer::bind(UiServerConfig {
            workspace: workspace.path().to_owned(),
            state_directory: workspace.path().join(".rey/workloads"),
            catalog_directory: "sys".into(),
            journal_directory: workspace.path().join(".rey/journal"),
            channel_directory: workspace.path().join(".rey/channels"),
            conversation_directory,
            host: "127.0.0.1".parse().unwrap(),
            port: 0,
        })
        .unwrap();
        let descriptor = server.descriptor();
        assert_eq!(descriptor.schema, "rey.ui-server.v1");
        assert!(descriptor.loopback_only);
        assert!(!descriptor.read_only);
        assert!(descriptor.journal_write_enabled);
        assert!(descriptor.workload_admission_enabled);
        assert!(descriptor.channel_write_enabled);
        assert!(descriptor.conversation_write_enabled);
        assert_eq!(descriptor.grammar, "kinetic");
        assert_eq!(descriptor.theme, "precision");
        assert_eq!(descriptor.source_repository, None);
        assert!(!descriptor.implementation_revision.is_empty());
        assert_eq!(
            descriptor.grammar_revision,
            "git:058c6504fc10740360717e97e687fd77bef6a5c5"
        );
        let address = descriptor.address.clone();
        let origin = descriptor.url.clone();
        let handle = thread::spawn(move || {
            server
                .serve_bounded(Some(39 + STATIC_UI_ASSETS.len()))
                .unwrap()
        });

        let health = request(&address, "GET /api/v1/health HTTP/1.1");
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains("\"schema\":\"rey.agent-health.v1\""));
        assert!(health.contains("\"schema\":\"rey.agent-process.v1\""));
        assert!(health.contains("\"loopback_only\":true"));

        let conversation = request(&address, "GET /api/v1/conversations HTTP/1.1");
        assert!(conversation.starts_with("HTTP/1.1 200"));
        let conversation: serde_json::Value =
            serde_json::from_str(response_body(&conversation)).unwrap();
        assert_eq!(conversation["availability"], "available");
        assert_eq!(conversation["browser_write_enabled"], true);
        assert_eq!(
            conversation["session"]["proposal"]["browser_writer_id"],
            "operator"
        );
        let conversation_write = serde_json::json!({
            "schema": "rey.ui-conversation-message-write.v1",
            "expected_log_id": conversation["log_id"],
            "session_id": conversation["session"]["session_id"],
            "body": "The operator admitted this exact local transcript message.",
            "reply_to": null
        })
        .to_string();
        let conversation_message = request_with_body(
            &address,
            "POST /api/v1/conversations/messages HTTP/1.1",
            &[("Content-Type", "application/json")],
            &conversation_write,
        );
        assert!(conversation_message.starts_with("HTTP/1.1 201"));
        assert!(conversation_message.contains("\"delivery\":\"not_attempted\""));
        assert!(conversation_message.contains("\"author_id\":\"operator\""));
        let stale_conversation_message = request_with_body(
            &address,
            "POST /api/v1/conversations/messages HTTP/1.1",
            &[("Content-Type", "application/json")],
            &conversation_write,
        );
        assert!(stale_conversation_message.starts_with("HTTP/1.1 409"));
        assert!(stale_conversation_message.contains("conversation log changed before append"));

        let workloads = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(workloads.starts_with("HTTP/1.1 200"));
        assert!(workloads.contains("\"schema\":\"rey.workload-list.v1\""));
        assert!(workloads.contains("\"state\":\"working\""));
        assert!(workloads.contains("\"index\":null"));

        let approval = serde_json::json!({
            "message": "Approve exact context survey",
            "expected_head": "EMPTY",
            "expected_working": working_revision,
        })
        .to_string();
        fs::write(
            package_directory.join("workload.yaml"),
            include_str!("../../../sys/context-anchor-survey/workload.yaml")
                .replace("Survey project context anchors", "Changed after review"),
        )
        .unwrap();
        let stale = request_with_body(
            &address,
            "POST /api/v1/workloads/admit HTTP/1.1",
            &[("Content-Type", "application/json")],
            &approval,
        );
        assert!(stale.starts_with("HTTP/1.1 409"));
        assert!(stale.contains("WORKING file snapshot changed before admission"));
        assert!(!workload_store.path().exists());
        fs::write(
            package_directory.join("workload.yaml"),
            include_str!("../../../sys/context-anchor-survey/workload.yaml"),
        )
        .unwrap();
        let approved = request_with_body(
            &address,
            "POST /api/v1/workloads/admit HTTP/1.1",
            &[("Content-Type", "application/json")],
            &approval,
        );
        assert!(approved.starts_with("HTTP/1.1 201"), "{approved}");
        assert!(approved.contains("\"schema\":\"rey.workload-commit-result.v1\""));
        assert!(approved.contains("\"sequence\":1"));

        let admitted = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(admitted.starts_with("HTTP/1.1 200"));
        assert!(admitted.contains("\"sequence\":1"));
        assert!(admitted.contains("\"index\":null"));
        assert!(admitted.contains("\"state\":\"clean\""));

        let evidence = request(&address, "GET /api/v1/workloads/evidence HTTP/1.1");
        assert!(evidence.starts_with("HTTP/1.1 200"));
        let evidence_json: serde_json::Value =
            serde_json::from_str(response_body(&evidence)).unwrap();
        assert_eq!(
            evidence_json["schema"],
            "rey.ui-workload-evidence-catalog.v1"
        );
        assert_eq!(evidence_json["workloads"][0]["freshness"], "fresh");
        let workload_id = evidence_json["workloads"][0]["workload_id"]
            .as_str()
            .unwrap();
        let execution_id = evidence_json["workloads"][0]["scenarios"][0]["execution_id"]
            .as_str()
            .unwrap();
        let delta_id = evidence_json["workloads"][0]["scenarios"][0]["deltas"][0]["delta_id"]
            .as_str()
            .unwrap();
        let encoded_execution = execution_id.replace(':', "%3A");
        let encoded_delta = delta_id.replace(':', "%3A");
        let scenario = request(
            &address,
            &format!("GET /api/v1/workloads/{workload_id}/scenarios/{encoded_execution} HTTP/1.1"),
        );
        assert!(scenario.starts_with("HTTP/1.1 200"));
        assert!(scenario.contains("\"schema\":\"rey.ui-workload-scenario-evidence.v1\""));
        assert!(scenario.contains("\"authority\":\"verified_retained_result_projection"));
        assert!(scenario.contains(execution_id));
        let scenario_head = request(
            &address,
            &format!("HEAD /api/v1/workloads/{workload_id}/scenarios/{encoded_execution} HTTP/1.1"),
        );
        assert!(scenario_head.starts_with("HTTP/1.1 200"));
        assert!(!scenario_head.contains(execution_id));
        let delta = request(
            &address,
            &format!("GET /api/v1/workloads/{workload_id}/deltas/{encoded_delta} HTTP/1.1"),
        );
        assert!(delta.starts_with("HTTP/1.1 200"));
        assert!(delta.contains("\"schema\":\"rey.ui-workload-delta-evidence.v1\""));
        assert!(delta.contains("\"kind\":\"scenario_output\""));
        assert!(delta.contains(delta_id));
        let topography_delta_id = evidence_json["workloads"][0]["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|scenario| scenario["deltas"].as_array().unwrap())
            .find(|delta| delta["kind"] == "topography_patch")
            .and_then(|delta| delta["delta_id"].as_str())
            .unwrap();
        let encoded_topography_delta = topography_delta_id.replace(':', "%3A");
        let topography_delta = request(
            &address,
            &format!(
                "GET /api/v1/workloads/{workload_id}/deltas/{encoded_topography_delta} HTTP/1.1"
            ),
        );
        assert!(topography_delta.starts_with("HTTP/1.1 200"));
        assert!(topography_delta.contains("\"kind\":\"topography_patch\""));
        assert!(topography_delta.contains(topography_delta_id));
        let unknown = request(
            &address,
            &format!(
                "GET /api/v1/workloads/{workload_id}/deltas/blake3%3Aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa HTTP/1.1"
            ),
        );
        assert!(unknown.starts_with("HTTP/1.1 404"));
        assert!(unknown.contains("workload_evidence_not_found"));
        let scenario_route = request(
            &address,
            &format!("GET /workloads/{workload_id}/scenarios/{encoded_execution} HTTP/1.1"),
        );
        assert!(scenario_route.starts_with("HTTP/1.1 200"));
        assert!(scenario_route.contains("<title>Rey / Explore</title>"));
        let delta_route = request(
            &address,
            &format!("GET /workloads/{workload_id}/deltas/{encoded_delta} HTTP/1.1"),
        );
        assert!(delta_route.starts_with("HTTP/1.1 200"));
        assert!(delta_route.contains("<title>Rey / Explore</title>"));

        let environment = request(&address, "GET /api/v1/environment HTTP/1.1");
        assert!(environment.starts_with("HTTP/1.1 200"));
        assert!(environment.contains("\"schema\":\"rey.environment-status.v2\""));
        assert!(
            environment
                .contains("\"operator\":{\"schema\":\"rey.environment-operator-projection.v2\"")
        );

        let cadence = request(&address, "GET /api/v1/cadence HTTP/1.1");
        assert!(cadence.starts_with("HTTP/1.1 200"));
        assert!(cadence.contains("\"schema\":\"rey.ui-cadence.v1\""));
        assert!(cadence.contains("\"ordering\":\"partial\""));
        assert!(cadence.contains("\"source_repository\":null"));
        assert!(cadence.contains("\"repository_state\":null"));
        assert!(cadence.contains("ui.portfolio.passive-revalidation"));
        assert!(cadence.contains("ui.channels.passive-revalidation"));
        assert!(cadence.contains("\"activation\":\"feed_route_mounted\""));
        assert!(cadence.contains("ui.observations.passive-revalidation"));
        assert!(cadence.contains("ui.cadence.passive-revalidation"));

        let observations = request(&address, "GET /api/v1/observations HTTP/1.1");
        assert!(observations.starts_with("HTTP/1.1 200"));
        assert!(observations.contains("\"schema\":\"rey.observation-frontier.v1\""));
        assert!(observations.contains("\"ordering\":\"observation_sequence_ascending\""));
        assert!(observations.contains("\"limit\":64"));
        assert!(observations.contains(observation.observation_id.as_str()));

        let seed = request(
            &address,
            &format!(
                "GET /api/v1/journal/seed?observations={} HTTP/1.1",
                observation.observation_id
            ),
        );
        assert!(seed.starts_with("HTTP/1.1 200"));
        assert!(seed.contains("\"schema\":\"rey.journal-seed.v1\""));
        assert!(seed.contains("\"kind\":\"human\",\"id\":\"operator\""));
        assert!(seed.contains(observation.observation_id.as_str()));
        assert!(!workspace.path().join(".rey/journal/journal.json").exists());

        let journal = request(&address, "GET /api/v1/journal HTTP/1.1");
        assert!(journal.starts_with("HTTP/1.1 200"));
        assert!(journal.contains("\"schema\":\"rey.ui-journal.v2\""));
        assert!(journal.contains("\"write_enabled\":true"));
        assert!(journal.contains("\"authority\":\"unauthenticated_journal_admission\""));
        assert!(journal.contains("\"entries\":[]"));

        let opportunities = request(&address, "GET /api/v1/journal/opportunities HTTP/1.1");
        assert!(opportunities.starts_with("HTTP/1.1 200"));
        assert!(opportunities.contains("\"schema\":\"rey.journal-opportunity-surface.v1\""));
        assert!(opportunities.contains("\"runtime_boundary\":\"requires_verified_selected_ready_create_attention_row_and_workload_admission\""));
        assert!(opportunities.contains("\"rows\":[]"));

        let queries = request(&address, "GET /api/v1/journal/queries HTTP/1.1");
        assert!(queries.starts_with("HTTP/1.1 200"));
        assert!(queries.contains("\"schema\":\"rey.journal-query-state.v1\""));
        assert!(queries.contains("\"admissions\":[]"));
        assert!(queries.contains("\"executions\":[]"));

        let proposal = serde_json::json!({
            "schema": "rey.journal-entry-proposal.v2",
            "title": "Bind the Journal",
            "author": { "kind": "human", "id": "operator" },
            "binding": {
                "coordinate": "rey+local://portfolio/current?revision=blake3%3Aportfolio",
                "scale": 0.68,
                "source_revision": "blake3:portfolio"
            },
            "layout": {
                "kind": "broadsheet",
                "columns": 12,
                "bands": [
                    {
                        "id": "lead",
                        "cells": [{ "block_id": "context", "span": 12 }]
                    },
                    {
                        "id": "bearing",
                        "cells": [{ "block_id": "next-bearing", "span": 12 }]
                    }
                ]
            },
            "blocks": [
                {
                    "kind": "prose",
                    "id": "context",
                    "document": [{ "kind": "paragraph", "text": "A retained human entry." }]
                },
                {
                    "kind": "action",
                    "id": "next-bearing",
                    "operation": "refine",
                    "desired_delta": "Close the bounded coverage gap.",
                    "evidence_ids": ["blake3:coverage"],
                    "dependency_ids": []
                }
            ]
        })
        .to_string();
        let cross_origin = request_with_body(
            &address,
            "POST /api/v1/journal HTTP/1.1",
            &[
                ("Content-Type", "application/json"),
                ("Origin", "http://not-rey.invalid"),
            ],
            &proposal,
        );
        assert!(cross_origin.starts_with("HTTP/1.1 201"));
        assert!(cross_origin.contains("\"admitted\":true"));

        let opportunities = request(&address, "HEAD /api/v1/journal/opportunities HTTP/1.1");
        assert!(opportunities.starts_with("HTTP/1.1 200"));
        assert!(opportunities.contains("Content-Length:"));
        assert!(!opportunities.contains("next-bearing"));
        let opportunities = request(&address, "GET /api/v1/journal/opportunities HTTP/1.1");
        assert!(opportunities.contains("\"block_id\":\"next-bearing\""));
        assert!(opportunities.contains("\"readiness\":\"authored_only\""));
        assert!(opportunities.contains("\"authority\":\"none\""));

        let agent_proposal = proposal.replace("\"kind\":\"human\"", "\"kind\":\"agent\"");
        let wrong_author = request_with_body(
            &address,
            "POST /api/v1/journal HTTP/1.1",
            &[
                ("Content-Type", "application/json"),
                ("Origin", origin.as_str()),
            ],
            &agent_proposal,
        );
        assert!(wrong_author.starts_with("HTTP/1.1 422"));
        assert!(wrong_author.contains("\"category\":\"journal_author_invalid\""));

        let admitted = request_with_body(
            &address,
            "POST /api/v1/journal HTTP/1.1",
            &[
                ("Content-Type", "application/json"),
                ("Origin", origin.as_str()),
            ],
            &proposal,
        );
        assert!(admitted.starts_with("HTTP/1.1 200"));
        assert!(admitted.contains("\"schema\":\"rey.journal-admission.v2\""));
        assert!(admitted.contains("\"admitted\":false"));
        assert!(admitted.contains("\"kind\":\"human\""));

        for required_asset in [
            "/assets/app.js",
            "/assets/agents.js",
            "/assets/explore.js",
            "/assets/react.js",
            "/assets/rolldown-runtime.js",
            "/assets/tanstack-router.js",
            "/assets/three.js",
        ] {
            assert!(
                STATIC_UI_ASSETS
                    .iter()
                    .any(|(path, _, _)| *path == required_asset),
                "{required_asset} was not embedded"
            );
        }
        let mut application = String::new();
        for (asset, _, content_type) in STATIC_UI_ASSETS {
            let response = request(&address, &format!("GET {asset} HTTP/1.1"));
            assert!(response.starts_with("HTTP/1.1 200"), "{asset}");
            assert!(response.contains(content_type), "{asset}");
            if content_type.starts_with("text/javascript") {
                application.push_str(response_body(&response));
            }
        }
        assert!(application.contains("01 / DIRECTED TEXT"));
        assert!(application.contains("02 / BOUNDED SEARCH"));
        assert!(application.contains("REFERENCE PLANE"));
        assert!(application.contains("Inputs and topology"));
        assert!(application.contains("RETAINED SEQUENCE"));
        assert!(application.contains("01 / JOURNAL"));
        assert!(application.contains("Supervised agent topology"));
        assert!(application.contains("02 / REY PROCESS"));
        assert!(application.contains("WRITE A JOURNAL ENTRY"));
        assert!(application.contains("HUMAN + AGENT · EXPLORE-BOUND"));
        assert!(application.contains("UNAUTHENTICATED · VALIDATED DOCUMENT ADMISSION"));
        assert!(application.contains("journal/$slug"));
        assert!(application.contains("JOURNAL / BROADSHEET"));
        assert!(application.contains("WORKING Δ"));
        assert!(application.contains("RECORD REVISION"));
        assert!(application.contains("QUERY AND ACTION CELLS DO NOT EXECUTE"));
        assert!(!application.contains("RECOMMENDATION BASIS"));
        assert!(application.contains("WORK LEDGER"));
        assert!(!application.contains("RETAINED RESULTS / NOT LIVE AGENT TELEMETRY"));
        assert!(application.contains("DESIRED INVENTORY"));
        assert!(application.contains("SEARCH RECORD"));
        assert!(application.contains("PROCESS SEEDS"));
        assert!(application.contains("./three-globe.js"));
        assert!(application.contains("./three-terrain.js"));
        assert!(application.contains("./three-webgpu.js"));
        assert!(application.contains("HISTORY / RUNTIME + COLLABORATION"));
        assert!(application.contains("Mailbox history"));
        assert!(application.contains("REY / AGENT / OPERATOR"));
        assert!(application.contains("data-communication-backdrop"));
        assert!(application.contains("No conversation session is admitted"));
        assert!(application.contains("NO AVAILABLE BROWSER WRITER · SEND DISABLED"));
        assert!(application.contains("DELIVERY / NOT ATTEMPTED"));
        assert!(application.contains("MAILBOX"));
        assert!(application.contains("INSPECT EVIDENCE"));
        assert!(application.contains("rey+local://"));
        assert!(application.contains("NO CURRENT OBJECT SATISFIES THIS IDENTITY"));
        assert!(application.contains("--kinetic-control-press-x"));
        assert!(application.contains("--kinetic-light-highlight"));
        assert!(application.contains("--kinetic-shadow-soft-y"));
        assert!(application.contains("WORKING TREE"));
        assert!(application.contains("PUSH RELATION"));
        assert!(application.contains("NO NETWORK FETCH"));
        assert!(application.contains("data-feed-stream"));
        assert!(application.contains("ADOPT INTO CHANNEL WORKING"));
        assert!(application.contains("data-feed-drag-identity"));
        assert!(application.contains("STREAM COORDINATE"));
        assert!(application.contains("ADD STREAM"));
        assert!(application.contains("APPLY LENS"));
        assert!(application.contains("Rename stream"));
        assert!(!application.contains("Channel operator index"));
        assert!(!application.contains("WRITE CHANNEL WORKING"));
        assert!(!application.contains("ALL LENS"));
        assert!(application.contains("Share an observation"));
        assert!(application.contains("ADMISSION CONTROL"));
        assert!(application.contains("ADMIT EXACT FILE SNAPSHOT"));
        assert!(application.contains("Display order is not causal order"));
        assert!(application.contains("data-kinetic-dense-table"));
        assert!(application.contains("Incoming workload revisions"));
        assert!(application.contains("Admitted workload HEAD"));
        assert!(application.contains("Workload creation requests"));
        assert!(application.contains("WORKLOAD / REVISION"));
        assert!(application.contains("MINING / ATTENTION"));
        assert!(application.contains("RUNTIME POSTURE"));
        assert!(application.contains("SCENARIO OUTCOMES"));
        assert!(application.contains("CONTENT IDENTITY"));
        assert!(application.contains("REQUEST POSTURE"));
        assert!(application.contains("REQUEST BINDINGS"));
        assert!(!application.contains("OPEN MECHANISM"));
        assert!(!application.contains("TICK → GRAPH → SCENARIO → DELTA → ATTENTION"));

        let stylesheet = request(&address, "GET /assets/app.css HTTP/1.1");
        assert!(stylesheet.starts_with("HTTP/1.1 200"));
        assert!(stylesheet.contains("text/css"));
        assert!(stylesheet.contains("@layer priority"));

        let root = request(&address, "GET / HTTP/1.1");
        assert!(root.starts_with("HTTP/1.1 307"));
        assert!(root.contains("Location: /explore"));

        let explore = request(&address, "GET /explore HTTP/1.1");
        assert!(explore.starts_with("HTTP/1.1 200"));
        assert!(explore.contains("Content-Security-Policy"));
        assert!(explore.contains("<title>Rey / Explore</title>"));

        let feed = request(&address, "GET /feed HTTP/1.1");
        assert!(feed.starts_with("HTTP/1.1 200"));
        assert!(feed.contains("<title>Rey / Explore</title>"));

        let environment = request(&address, "GET /environment HTTP/1.1");
        assert!(environment.starts_with("HTTP/1.1 200"));
        assert!(environment.contains("<title>Rey / Explore</title>"));

        let cadence = request(&address, "GET /cadence HTTP/1.1");
        assert!(cadence.starts_with("HTTP/1.1 200"));
        assert!(cadence.contains("<title>Rey / Explore</title>"));

        let agents = request(&address, "GET /agents HTTP/1.1");
        assert!(agents.starts_with("HTTP/1.1 200"));
        assert!(agents.contains("<title>Rey / Explore</title>"));

        let journal_new = request(&address, "GET /journal/new HTTP/1.1");
        assert!(journal_new.starts_with("HTTP/1.1 200"));
        assert!(journal_new.contains("<title>Rey / Explore</title>"));

        let journal_entry = request(&address, "GET /journal/j1-example HTTP/1.1");
        assert!(journal_entry.starts_with("HTTP/1.1 200"));
        assert!(journal_entry.contains("<title>Rey / Explore</title>"));

        let coordinate = request(
            &address,
            "GET /explore?coordinate=rey%2Blocal%3A%2F%2Fagent%2Fcodex%3Frevision%3Dgpt-5%26role%3Dcoding_harness&scale=1.46 HTTP/1.1",
        );
        assert!(coordinate.starts_with("HTTP/1.1 200"));
        assert!(coordinate.contains("<title>Rey / Explore</title>"));

        let rejected = request(&address, "POST /api/v1/workloads HTTP/1.1");
        assert!(rejected.starts_with("HTTP/1.1 405"));
        assert!(rejected.contains("\"category\":\"method_not_allowed\""));
        handle.join().unwrap();
    }

    #[test]
    fn server_projects_and_conditionally_writes_channel_working() {
        let workspace = TempDir::new().unwrap();
        let channel_directory = workspace.path().join(".rey/channels");
        let server = UiServer::bind(UiServerConfig {
            workspace: workspace.path().to_owned(),
            state_directory: workspace.path().join(".rey/workloads"),
            catalog_directory: "sys".into(),
            journal_directory: workspace.path().join(".rey/journal"),
            channel_directory: channel_directory.clone(),
            conversation_directory: workspace.path().join(".rey/conversations"),
            host: "127.0.0.1".parse().unwrap(),
            port: 0,
        })
        .unwrap();
        let address = server.descriptor().address;
        let handle = thread::spawn(move || server.serve_bounded(Some(6)).unwrap());

        let initial = request(&address, "GET /api/v1/channels HTTP/1.1");
        assert!(initial.starts_with("HTTP/1.1 200"));
        assert!(!channel_directory.exists());
        let initial: serde_json::Value = serde_json::from_str(response_body(&initial)).unwrap();
        assert_eq!(initial["schema"], "rey.ui-channels.v1");
        assert_eq!(initial["write_enabled"], true);
        assert_eq!(initial["listener"]["loopback_only"], true);
        assert_eq!(initial["listener"]["authentication"], "none");
        assert_eq!(initial["status"]["state"], "clean");
        let expected_head = initial["status"]["head"]["snapshot_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let expected_working = initial["status"]["working"]["snapshot_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut graph = initial["status"]["working"]["graph"].clone();
        let signals = graph["streams"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|stream| stream["id"] == "signals")
            .unwrap();
        signals["name"] = "Signal desk".into();
        signals["revision"] = 2.into();
        let write = serde_json::json!({
            "schema": "rey.ui-channel-working-write.v1",
            "expected_head_snapshot_id": expected_head,
            "expected_working_snapshot_id": expected_working,
            "graph": graph,
        })
        .to_string();

        let wrong_type = request_with_body(
            &address,
            "POST /api/v1/channels/working HTTP/1.1",
            &[],
            &write,
        );
        assert!(wrong_type.starts_with("HTTP/1.1 415"));
        assert!(!channel_directory.exists());

        let applied = request_with_body(
            &address,
            "POST /api/v1/channels/working HTTP/1.1",
            &[
                ("Content-Type", "application/json"),
                ("Origin", "http://not-rey.invalid"),
            ],
            &write,
        );
        assert!(applied.starts_with("HTTP/1.1 201"));
        assert!(applied.contains("\"schema\":\"rey.channel-apply-result.v1\""));
        assert!(applied.contains("\"applied\":true"));

        let current = request(&address, "GET /api/v1/channels HTTP/1.1");
        assert!(current.starts_with("HTTP/1.1 200"));
        assert!(current.contains("\"state\":\"working\""));
        assert!(current.contains("Signal desk"));
        assert!(current.contains("ui:///channels/working"));

        let stale = request_with_body(
            &address,
            "POST /api/v1/channels/working HTTP/1.1",
            &[("Content-Type", "application/json")],
            &write,
        );
        assert!(stale.starts_with("HTTP/1.1 409"));
        assert!(stale.contains("channel WORKING changed before the WORKING write"));

        let retained = request(&address, "GET /api/v1/channels HTTP/1.1");
        assert!(retained.starts_with("HTTP/1.1 200"));
        assert!(retained.contains("Signal desk"));
        handle.join().unwrap();
    }

    fn request(address: &str, request_line: &str) -> String {
        request_with_body(address, request_line, &[], "")
    }

    fn response_body(response: &str) -> &str {
        response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap()
    }

    fn request_with_body(
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
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let headers = std::str::from_utf8(&response[..header_end]).unwrap();
        let body = &response[header_end + 4..];
        let body = if headers
            .lines()
            .any(|line| line.eq_ignore_ascii_case("Transfer-Encoding: chunked"))
        {
            decode_chunked(body)
        } else {
            body.to_vec()
        };
        format!("{headers}\r\n\r\n{}", String::from_utf8(body).unwrap())
    }

    fn decode_chunked(encoded: &[u8]) -> Vec<u8> {
        let mut decoded = Vec::new();
        let mut cursor = 0;
        loop {
            let line_end = encoded[cursor..]
                .windows(2)
                .position(|window| window == b"\r\n")
                .map(|offset| cursor + offset)
                .unwrap();
            let size = std::str::from_utf8(&encoded[cursor..line_end])
                .unwrap()
                .split(';')
                .next()
                .and_then(|value| usize::from_str_radix(value.trim(), 16).ok())
                .unwrap();
            cursor = line_end + 2;
            if size == 0 {
                break;
            }
            decoded.extend_from_slice(&encoded[cursor..cursor + size]);
            cursor += size;
            assert_eq!(&encoded[cursor..cursor + 2], b"\r\n");
            cursor += 2;
        }
        decoded
    }
}
