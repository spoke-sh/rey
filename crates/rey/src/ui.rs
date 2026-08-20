use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{Cursor, Read, Write},
    net::{IpAddr, SocketAddr, TcpListener},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State, rejection::BytesRejection},
    http::{
        HeaderMap, HeaderName, HeaderValue, Method as AxumMethod, Request as AxumRequest,
        StatusCode as AxumStatusCode, Uri,
    },
    middleware::{self, Next},
    response::Response as AxumResponse,
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use flate2::{Compression, write::GzEncoder};
use rey::{
    channels::{
        ChannelGraph, ChannelGraphSource, ChannelMailboxProjection, ChannelObservationKind,
        ChannelStatus, LocalChannelStore, MAX_CHANNEL_GRAPH_INPUT_BYTES,
    },
    conversations::{
        CONVERSATION_MESSAGE_PROPOSAL_SCHEMA, ConversationMessageProposal, ConversationSource,
        DEFAULT_CONVERSATION_TRANSCRIPT_LIMIT, LocalConversationStore,
        MAX_CONVERSATION_MESSAGE_INPUT_BYTES,
    },
    current_environment_status,
    env::{
        EnvironmentApplicationObservation, EnvironmentCommit, EnvironmentObjectChange,
        EnvironmentOperatorProjection, LocalEnvironmentStore,
    },
    journal::{
        JournalAdmission, JournalAuthor, JournalAuthorKind, JournalEntryProposal, JournalLog,
        LocalJournalStore, MAX_JOURNAL_PROPOSAL_BYTES,
    },
    journal_opportunities::{DEFAULT_JOURNAL_OPPORTUNITY_LIMIT, JournalOpportunitySurface},
    journal_queries::LocalJournalQueryStore,
    journal_seed::JournalSeed,
    observations::{
        DEFAULT_OBSERVATION_FRONTIER_LIMIT, LocalObservationStore, ObservationAuthor,
        ObservationAuthorKind, ObservationCompleteness, ObservationProposal, ObservationSource,
    },
    workload_evidence::{
        WorkloadEvidenceError, workload_delta_evidence, workload_evidence_catalog,
        workload_scenario_evidence,
    },
    workloads::{LocalWorkloadStore, WorkloadCommit, WorkloadList},
};
use rey_core::{SemanticDigest, SemanticHasher};
use rey_environment::{DiscoveryLimits, resolve_executable};
use rey_git::{GitInspector, GitLimits, GitRepositoryStatus};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::{API_ROOT_PATH, API_ROUTES, ApiMethod, OPENAPI_PATH, SWAGGER_PATH, openapi};

const UI_SERVER_SCHEMA: &str = "rey.ui-server.v2";
const AGENT_HEALTH_SCHEMA: &str = "rey.agent-health.v2";
const UI_ERROR_SCHEMA: &str = "rey.api-error.v1";
const UI_CADENCE_SCHEMA: &str = "rey.ui-cadence.v1";
const UI_JOURNAL_SCHEMA: &str = "rey.ui-journal.v2";
const UI_CHANNELS_SCHEMA: &str = "rey.ui-channels.v1";
const UI_CHANNEL_WORKING_WRITE_SCHEMA: &str = "rey.ui-channel-working-write.v1";
const UI_GITHUB_POLL_WRITE_SCHEMA: &str = "rey.ui-github-poll-write.v1";
const UI_CONVERSATION_MESSAGE_WRITE_SCHEMA: &str = "rey.ui-conversation-message-write.v1";
const UI_OBSERVATION_WRITE_SCHEMA: &str = "rey.ui-observation-write.v1";
const UI_FEED_ADMISSIONS_SCHEMA: &str = "rey.ui-feed-admissions.v1";
const UI_REVALIDATION_SCHEMA: &str = "rey.ui-revalidation.v1";
const MAX_REQUEST_TARGET_BYTES: usize = 4_096;
const MAX_WORKLOAD_APPROVAL_BYTES: u64 = 16 * 1_024;
const MAX_UI_OBSERVATION_WRITE_BYTES: u64 = 32 * 1_024;
const MAX_UI_OBSERVATION_BODY_CHARS: usize = 500;
const MAX_UI_GITHUB_POLL_WRITE_BYTES: u64 = 4 * 1_024;
const MAX_REVALIDATION_SOURCE_BYTES: u64 = 128 * 1_024 * 1_024;
const MAX_REVALIDATION_SOURCE_ENTRIES: usize = 4_096;
const LIVE_REFRESH_INTERVAL_MS: u64 = 5_000;
const CADENCE_GIT_COMMIT_LIMIT: usize = 24;
const CADENCE_ENVIRONMENT_COMMIT_LIMIT: usize = 24;
const FEED_ADMISSION_LIMIT: usize = 64;
const FEED_ADMISSION_HISTORY_SCAN_LIMIT: usize = 256;
const HIFI_GRAMMAR_REVISION: &str = "git:058c6504fc10740360717e97e687fd77bef6a5c5";
const REY_IMPLEMENTATION_REVISION: &str = env!("REY_BUILD_REVISION");
const REQUEST_RECEIVE_POLL_INTERVAL_MS: u64 = 50;
const MAX_OPERATOR_REQUEST_BODY_BYTES: usize = MAX_JOURNAL_PROPOSAL_BYTES as usize;

include!(concat!(env!("OUT_DIR"), "/rey_ui_assets.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
enum Method {
    Get,
    Head,
    Post,
    Other,
}

impl From<AxumMethod> for Method {
    fn from(method: AxumMethod) -> Self {
        if method == AxumMethod::GET {
            Self::Get
        } else if method == AxumMethod::HEAD {
            Self::Head
        } else if method == AxumMethod::POST {
            Self::Post
        } else {
            Self::Other
        }
    }
}

struct Request {
    method: Method,
    target: String,
    headers: HeaderMap,
    body: Cursor<Vec<u8>>,
}

impl Request {
    fn new(method: AxumMethod, uri: Uri, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            method: method.into(),
            target: uri
                .path_and_query()
                .map_or_else(|| uri.path().to_owned(), ToString::to_string),
            headers,
            body: Cursor::new(body.to_vec()),
        }
    }

    fn url(&self) -> &str {
        &self.target
    }

    const fn method(&self) -> &Method {
        &self.method
    }

    fn body_length(&self) -> Option<usize> {
        Some(self.body.get_ref().len())
    }

    fn as_reader(&mut self) -> &mut Cursor<Vec<u8>> {
        &mut self.body
    }
}

#[derive(Clone, Copy)]
struct StatusCode(u16);

struct Header {
    name: HeaderName,
    value: HeaderValue,
}

impl Header {
    fn from_bytes(name: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Result<Self, ()> {
        Ok(Self {
            name: HeaderName::from_bytes(name.as_ref()).map_err(|_| ())?,
            value: HeaderValue::from_bytes(value.as_ref()).map_err(|_| ())?,
        })
    }
}

struct Response<R> {
    status: StatusCode,
    headers: HeaderMap,
    data: R,
}

impl Response<Cursor<Vec<u8>>> {
    fn from_data(data: Vec<u8>) -> Self {
        Self {
            status: StatusCode(200),
            headers: HeaderMap::new(),
            data: Cursor::new(data),
        }
    }

    const fn with_status_code(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    fn with_header(mut self, header: Header) -> Self {
        self.headers.insert(header.name, header.value);
        self
    }

    fn with_data(mut self, data: Cursor<Vec<u8>>, _length: Option<usize>) -> Self {
        self.data = data;
        self
    }

    fn into_axum(self) -> AxumResponse {
        let mut response = AxumResponse::new(Body::from(self.data.into_inner()));
        *response.status_mut() = AxumStatusCode::from_u16(self.status.0)
            .expect("internal HTTP response status is valid");
        *response.headers_mut() = self.headers;
        response
    }
}

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
    pub observation_write_enabled: bool,
    pub workload_admission_enabled: bool,
    pub channel_write_enabled: bool,
    pub conversation_write_enabled: bool,
    pub http_framework: String,
    pub api_root: String,
    pub openapi_document: String,
    pub swagger_ui: String,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiEnvironmentApplicationAdmission {
    name: String,
    change: EnvironmentObjectChange,
    availability: Option<String>,
    resolved_path: Option<String>,
    groups: Vec<String>,
}

impl UiEnvironmentApplicationAdmission {
    fn from_transition(
        change: EnvironmentObjectChange,
        source: Option<&EnvironmentApplicationObservation>,
        target: Option<&EnvironmentApplicationObservation>,
    ) -> Option<Self> {
        let observation = target.or(source)?;
        Some(Self {
            name: observation.name.clone(),
            change,
            availability: target.map(|application| application.availability.as_str().to_owned()),
            resolved_path: target.and_then(|application| application.resolved_path.clone()),
            groups: observation.groups.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiEnvironmentAdmissionChanges {
    variables: u64,
    applications: Vec<UiEnvironmentApplicationAdmission>,
    inputs: u64,
    references: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum UiFeedAdmission {
    Environment {
        commit: EnvironmentCommit,
        changes: UiEnvironmentAdmissionChanges,
    },
    Workload {
        commit: WorkloadCommit,
    },
}

impl UiFeedAdmission {
    fn committed_at_unix(&self) -> i64 {
        match self {
            Self::Environment { commit, .. } => commit.committed_at_unix,
            Self::Workload { commit } => commit.committed_at_unix,
        }
    }

    fn stable_identity(&self) -> (&str, &str) {
        match self {
            Self::Environment { commit, .. } => ("environment", commit.commit_id.as_str()),
            Self::Workload { commit } => ("workload", commit.commit_id.as_str()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiFeedAdmissions {
    schema: String,
    ordering: String,
    total_admissions: u64,
    selected_admissions: u64,
    complete: bool,
    admissions: Vec<UiFeedAdmission>,
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
    mailbox: ChannelMailboxProjection,
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
struct UiGitHubPollWrite {
    schema: String,
    expected_channel_head_commit_id: SemanticDigest,
    application_id: String,
    application_revision: u64,
    message_id: SemanticDigest,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UiObservationWrite {
    schema: String,
    kind: ChannelObservationKind,
    body: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct UiRevalidationCursor {
    schema: String,
    revision: SemanticDigest,
    poll_after_ms: u64,
    basis: String,
    source_entries: usize,
    source_bytes: u64,
    scope: Vec<String>,
    authority: String,
    omissions: Vec<String>,
}

pub struct UiServer {
    listener: TcpListener,
    config: UiServerConfig,
    descriptor: UiServerDescriptor,
    projection_cache: Mutex<UiProjectionCache>,
    workload_projection: Mutex<()>,
}

#[derive(Default)]
struct UiProjectionCache {
    workloads: Option<CachedJsonProjection>,
    workload_evidence: Option<CachedJsonProjection>,
}

struct CachedJsonProjection {
    source_revision: SemanticDigest,
    bytes: Vec<u8>,
    gzip_bytes: Vec<u8>,
}

#[derive(Clone)]
struct ServeControl {
    cancelled: Arc<AtomicBool>,
    served: Arc<AtomicUsize>,
    max_requests: Option<usize>,
}

impl ServeControl {
    async fn wait_for_shutdown(self) {
        loop {
            if self.cancelled.load(Ordering::Relaxed)
                || self
                    .max_requests
                    .is_some_and(|limit| self.served.load(Ordering::Relaxed) >= limit)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(REQUEST_RECEIVE_POLL_INTERVAL_MS)).await;
        }
    }
}

fn operator_router(server: Arc<UiServer>, control: ServeControl) -> Router {
    let mut router = Router::<Arc<UiServer>>::new();
    for route in API_ROUTES {
        let method_router = match route.method {
            ApiMethod::Read => get(dispatch_request),
            ApiMethod::Write => post(dispatch_request),
        };
        router = router.route(route.path, method_router);
    }
    router
        .route("/", get(dispatch_request))
        .merge(SwaggerUi::new(SWAGGER_PATH).external_url_unchecked(OPENAPI_PATH, openapi()))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(dispatch_request)
        .layer(DefaultBodyLimit::max(MAX_OPERATOR_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn_with_state(control, count_request))
        .with_state(server)
}

async fn method_not_allowed(uri: Uri) -> AxumResponse {
    let mut read = false;
    let mut write = false;
    for route in API_ROUTES
        .iter()
        .filter(|route| api_path_matches(route.path, uri.path()))
    {
        match route.method {
            ApiMethod::Read => read = true,
            ApiMethod::Write => write = true,
        }
    }
    let allowed = match (read, write) {
        (true, true) => "GET, HEAD, POST",
        (false, true) => "POST",
        _ => "GET, HEAD",
    };
    with_header(
        json_error(
            StatusCode(405),
            "method_not_allowed",
            &format!("this route accepts {allowed}"),
        ),
        "Allow",
        allowed,
    )
    .into_axum()
}

fn api_path_matches(pattern: &str, path: &str) -> bool {
    let pattern_segments = pattern.trim_matches('/').split('/').collect::<Vec<_>>();
    let path_segments = path.trim_matches('/').split('/').collect::<Vec<_>>();
    pattern_segments.len() == path_segments.len()
        && pattern_segments
            .iter()
            .zip(path_segments)
            .all(|(pattern, path)| {
                (!path.is_empty() && pattern.starts_with('{') && pattern.ends_with('}'))
                    || pattern == &path
            })
}

async fn dispatch_request(
    State(server): State<Arc<UiServer>>,
    method: AxumMethod,
    uri: Uri,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> AxumResponse {
    let body = match body {
        Ok(body) => body,
        Err(_) => {
            return json_error(
                StatusCode(413),
                "request_body_limit",
                "request body exceeds the 1048576-byte operator limit",
            )
            .into_axum();
        }
    };
    let mut request = Request::new(method, uri, headers, body);
    match tokio::task::spawn_blocking(move || server.route(&mut request)).await {
        Ok(response) => response.into_axum(),
        Err(error) => json_error(
            StatusCode(500),
            "request_handler_failed",
            &format!("operator request handler failed: {error}"),
        )
        .into_axum(),
    }
}

async fn count_request(
    State(control): State<ServeControl>,
    request: AxumRequest<Body>,
    next: Next,
) -> AxumResponse {
    let response = next.run(request).await;
    control.served.fetch_add(1, Ordering::Relaxed);
    response
}

impl UiServer {
    pub fn bind(config: UiServerConfig) -> Result<Self, UiError> {
        let requested = SocketAddr::new(config.host, config.port);
        let listener = TcpListener::bind(requested).map_err(|source| UiError::Bind {
            address: requested,
            detail: source.to_string(),
        })?;
        listener.set_nonblocking(true).map_err(UiError::Listener)?;
        let bound = listener.local_addr().map_err(UiError::Listener)?;
        let descriptor = UiServerDescriptor {
            schema: UI_SERVER_SCHEMA.to_owned(),
            address: bound.to_string(),
            url: format!("http://{bound}"),
            host: bound.ip().to_string(),
            port: bound.port(),
            loopback_only: bound.ip().is_loopback(),
            read_only: false,
            journal_write_enabled: true,
            observation_write_enabled: true,
            workload_admission_enabled: true,
            channel_write_enabled: true,
            conversation_write_enabled: true,
            http_framework: "axum".to_owned(),
            api_root: API_ROOT_PATH.to_owned(),
            openapi_document: OPENAPI_PATH.to_owned(),
            swagger_ui: "/api/docs/".to_owned(),
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
            listener,
            config,
            descriptor,
            projection_cache: Mutex::new(UiProjectionCache::default()),
            workload_projection: Mutex::new(()),
        })
    }

    #[must_use]
    pub fn descriptor(&self) -> UiServerDescriptor {
        self.descriptor.clone()
    }

    pub fn serve_until(self, cancelled: Arc<AtomicBool>) -> Result<(), UiError> {
        self.serve(cancelled, None)
    }

    #[cfg(test)]
    fn serve_bounded(self, max_requests: Option<usize>) -> Result<(), UiError> {
        self.serve(Arc::new(AtomicBool::new(false)), max_requests)
    }

    fn serve(self, cancelled: Arc<AtomicBool>, max_requests: Option<usize>) -> Result<(), UiError> {
        let listener = self.listener.try_clone().map_err(UiError::Listener)?;
        let server = Arc::new(self);
        let control = ServeControl {
            cancelled,
            served: Arc::new(AtomicUsize::new(0)),
            max_requests,
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(UiError::Runtime)?;
        runtime.block_on(async move {
            let listener =
                tokio::net::TcpListener::from_std(listener).map_err(UiError::Listener)?;
            let application = operator_router(server, control.clone());
            axum::serve(listener, application)
                .with_graceful_shutdown(control.wait_for_shutdown())
                .await
                .map_err(UiError::Serve)
        })
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
        let accepts_gzip = accepts_content_encoding(request, "gzip");
        if request.method() == &Method::Post && path == "/api/v1/journal" {
            return self.admit_journal(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/observations" {
            return self.admit_observation(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/workloads/admit" {
            return self.admit_workloads(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/channels/working" {
            return self.write_channel_working(request);
        }
        if request.method() == &Method::Post && path == "/api/v1/channels/poll" {
            return self.poll_github_mailbox(request);
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
            "/" => redirect_response(API_ROOT_PATH),
            "/api" => redirect_response("/api/docs/"),
            "/api/v1/health" => self.health(),
            "/api/v1/agent" => self.agent(),
            "/api/v1/revalidation" => self.revalidation(),
            "/api/v1/cadence" => self.cadence(),
            "/api/v1/channels" => self.channels(),
            "/api/v1/conversations" => self.conversations(),
            "/api/v1/environment" => self.environment(),
            "/api/v1/feed/admissions" => self.feed_admissions(),
            "/api/v1/journal" => self.journal(),
            "/api/v1/journal/opportunities" => self.journal_opportunities(),
            "/api/v1/journal/queries" => self.journal_queries(),
            "/api/v1/journal/seed" => self.journal_seed(query),
            "/api/v1/observations" => self.observations(),
            "/api/v1/workloads" => self.workloads(accepts_gzip),
            "/api/v1/workloads/admissions" => self.workload_admissions(),
            "/api/v1/workloads/evidence" => self.workload_evidence(accepts_gzip),
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

    fn revalidation(&self) -> Response<Cursor<Vec<u8>>> {
        match self.revalidation_cursor() {
            Ok(cursor) => json_response(StatusCode(200), &cursor),
            Err(detail) => json_error(StatusCode(500), "revalidation_unavailable", &detail),
        }
    }

    fn revalidation_cursor(&self) -> Result<UiRevalidationCursor, String> {
        let catalog = if self.config.catalog_directory.is_absolute() {
            self.config.catalog_directory.clone()
        } else {
            self.config.workspace.join(&self.config.catalog_directory)
        };
        let sources = [
            ("workloads", self.config.state_directory.clone()),
            ("catalog", catalog),
            (
                "environment",
                self.config.workspace.join(".rey").join("environment"),
            ),
            ("git", self.config.workspace.join(".rey").join("git")),
            ("channels", self.config.channel_directory.clone()),
            ("conversations", self.config.conversation_directory.clone()),
        ];
        let mut hasher = SemanticHasher::new(UI_REVALIDATION_SCHEMA);
        let mut scan = RevalidationScan::default();
        let mut scope = Vec::with_capacity(sources.len());
        for (label, root) in sources {
            scope.push(label.to_owned());
            hash_revalidation_source(&mut hasher, label, &root, &mut scan)?;
        }
        Ok(UiRevalidationCursor {
            schema: UI_REVALIDATION_SCHEMA.to_owned(),
            revision: hasher.finish(),
            poll_after_ms: LIVE_REFRESH_INTERVAL_MS,
            basis: "exact bounded source bytes; missing roots and non-regular entries are framed explicitly"
                .to_owned(),
            source_entries: scan.entries,
            source_bytes: scan.bytes,
            scope,
            authority: "change detection only; a changed cursor causes the browser to reload typed projections"
                .to_owned(),
            omissions: vec![
                "the cursor does not assess, admit, execute, schedule, or retain source state"
                    .to_owned(),
            ],
        })
    }

    fn workload_projection_revision(&self) -> Result<SemanticDigest, String> {
        let catalog = if self.config.catalog_directory.is_absolute() {
            self.config.catalog_directory.clone()
        } else {
            self.config.workspace.join(&self.config.catalog_directory)
        };
        let sources = [
            ("workloads", self.config.state_directory.clone()),
            ("catalog", catalog),
            ("ignore", self.config.workspace.join(".reyignore")),
            (
                "environment",
                self.config.workspace.join(".rey").join("environment"),
            ),
            ("git", self.config.workspace.join(".rey").join("git")),
        ];
        let mut hasher = SemanticHasher::new("rey.ui-workload-projection-revision.v1");
        let mut scan = RevalidationScan::default();
        for (label, root) in sources {
            hash_revalidation_source(&mut hasher, label, &root, &mut scan)?;
        }
        Ok(hasher.finish())
    }

    fn workloads(&self, accepts_gzip: bool) -> Response<Cursor<Vec<u8>>> {
        let mut source_revision = self.workload_projection_revision().ok();
        if let Some(revision) = &source_revision
            && let Ok(cache) = self.projection_cache.lock()
            && let Some(cached) = &cache.workloads
            && &cached.source_revision == revision
        {
            return cached_json_response(StatusCode(200), cached, accepts_gzip);
        }
        let _projection = match self.workload_projection.lock() {
            Ok(projection) => projection,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "portfolio_projection_unavailable",
                    &format!("workload projection coordinator is unavailable: {error}"),
                );
            }
        };
        source_revision = self.workload_projection_revision().ok();
        if let Some(revision) = &source_revision
            && let Ok(cache) = self.projection_cache.lock()
            && let Some(cached) = &cache.workloads
            && &cached.source_revision == revision
        {
            return cached_json_response(StatusCode(200), cached, accepts_gzip);
        }
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
            Ok(list) => self.cache_workloads(source_revision, &list, accepts_gzip),
            Err(detail) => json_error(StatusCode(500), "portfolio_unavailable", &detail),
        }
    }

    fn workload_evidence(&self, accepts_gzip: bool) -> Response<Cursor<Vec<u8>>> {
        let source_revision = self.workload_projection_revision().ok();
        if let Some(revision) = &source_revision
            && let Ok(cache) = self.projection_cache.lock()
            && let Some(cached) = &cache.workload_evidence
            && &cached.source_revision == revision
        {
            return cached_json_response(StatusCode(200), cached, accepts_gzip);
        }
        let store = LocalWorkloadStore::new(self.config.state_directory.clone());
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
        let catalog = match store.head_catalog_from_state(&state) {
            Ok(catalog) => catalog,
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
            Ok(evidence) => self.cache_workload_evidence(source_revision, &evidence, accepts_gzip),
            Err(error) => json_error(
                StatusCode(500),
                "workload_evidence_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn cache_workloads(
        &self,
        source_revision: Option<SemanticDigest>,
        list: &WorkloadList,
        accepts_gzip: bool,
    ) -> Response<Cursor<Vec<u8>>> {
        let projection = match compact_workload_transport(list) {
            Ok(projection) => projection,
            Err(detail) => {
                return json_error(StatusCode(500), "workload_transport_failed", &detail);
            }
        };
        self.cache_projection(
            source_revision,
            &projection,
            accepts_gzip,
            |cache, projection| {
                cache.workloads = Some(projection);
            },
        )
    }

    fn cache_workload_evidence(
        &self,
        source_revision: Option<SemanticDigest>,
        evidence: &impl Serialize,
        accepts_gzip: bool,
    ) -> Response<Cursor<Vec<u8>>> {
        self.cache_projection(
            source_revision,
            evidence,
            accepts_gzip,
            |cache, projection| {
                cache.workload_evidence = Some(projection);
            },
        )
    }

    fn cache_projection(
        &self,
        source_revision: Option<SemanticDigest>,
        value: &impl Serialize,
        accepts_gzip: bool,
        retain: impl FnOnce(&mut UiProjectionCache, CachedJsonProjection),
    ) -> Response<Cursor<Vec<u8>>> {
        let bytes = match serde_json::to_vec(value) {
            Ok(bytes) => bytes,
            Err(error) => {
                return json_error(StatusCode(500), "json_encoding_failed", &error.to_string());
            }
        };
        let gzip_bytes = match gzip_bytes(&bytes) {
            Ok(bytes) => bytes,
            Err(error) => {
                return json_error(StatusCode(500), "gzip_encoding_failed", &error.to_string());
            }
        };
        if let Some(source_revision) = source_revision
            && self
                .workload_projection_revision()
                .ok()
                .is_some_and(|revision| revision == source_revision)
            && let Ok(mut cache) = self.projection_cache.lock()
        {
            retain(
                &mut cache,
                CachedJsonProjection {
                    source_revision,
                    bytes: bytes.clone(),
                    gzip_bytes: gzip_bytes.clone(),
                },
            );
        }
        encoded_json_response(StatusCode(200), bytes, gzip_bytes, accepts_gzip)
    }

    fn workload_admissions(&self) -> Response<Cursor<Vec<u8>>> {
        let store = LocalWorkloadStore::new(self.config.state_directory.clone());
        match store.log(FEED_ADMISSION_LIMIT, false) {
            Ok(log) => json_response(StatusCode(200), &log),
            Err(error) => json_error(
                StatusCode(500),
                "workload_admissions_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn feed_admissions(&self) -> Response<Cursor<Vec<u8>>> {
        match self.feed_admissions_projection() {
            Ok(admissions) => json_response(StatusCode(200), &admissions),
            Err(detail) => json_error(StatusCode(500), "feed_admissions_unavailable", &detail),
        }
    }

    fn feed_admissions_projection(&self) -> Result<UiFeedAdmissions, String> {
        let environment_store =
            LocalEnvironmentStore::default_for_workspace(&self.config.workspace);
        let environment_history = environment_store
            .load()
            .map_err(|error| error.to_string())?;
        let workload_store = LocalWorkloadStore::new(self.config.state_directory.clone());
        let workload_log = workload_store
            .log(FEED_ADMISSION_HISTORY_SCAN_LIMIT, false)
            .map_err(|error| error.to_string())?;
        let total_admissions =
            environment_history.commits.len() as u64 + workload_log.total_commits;
        let mut admissions =
            Vec::with_capacity(environment_history.commits.len() + workload_log.commits.len());

        for (index, commit) in environment_history.commits.iter().enumerate() {
            let source = index
                .checked_sub(1)
                .and_then(|source_index| environment_history.commits.get(source_index))
                .map(|source_commit| &source_commit.snapshot);
            let projection = EnvironmentOperatorProjection::derive_transition(
                source,
                &commit.snapshot,
                index.checked_sub(1).map_or_else(
                    || "EMPTY".to_owned(),
                    |source_index| format!("ENV@{}", source_index + 1),
                ),
                format!("ENV@{}", commit.sequence),
            )
            .map_err(|error| error.to_string())?;
            let changed =
                |change: EnvironmentObjectChange| change != EnvironmentObjectChange::Unchanged;
            let applications = projection
                .applications
                .iter()
                .filter(|application| changed(application.changes.head_to_working))
                .filter_map(|application| {
                    UiEnvironmentApplicationAdmission::from_transition(
                        application.changes.head_to_working,
                        application.head.as_ref(),
                        application.working.as_ref(),
                    )
                })
                .collect();
            admissions.push(UiFeedAdmission::Environment {
                commit: commit.clone(),
                changes: UiEnvironmentAdmissionChanges {
                    variables: projection
                        .variables
                        .iter()
                        .filter(|variable| changed(variable.changes.head_to_working))
                        .count() as u64,
                    applications,
                    inputs: projection
                        .inputs
                        .iter()
                        .filter(|input| changed(input.changes.head_to_working))
                        .count() as u64,
                    references: projection
                        .references
                        .iter()
                        .filter(|reference| changed(reference.changes.head_to_working))
                        .count() as u64,
                },
            });
        }
        admissions.extend(
            workload_log
                .commits
                .into_iter()
                .map(|commit| UiFeedAdmission::Workload { commit }),
        );
        admissions.sort_by(|left, right| {
            right
                .committed_at_unix()
                .cmp(&left.committed_at_unix())
                .then_with(|| left.stable_identity().cmp(&right.stable_identity()))
        });
        admissions.truncate(FEED_ADMISSION_LIMIT);
        let selected_admissions = admissions.len() as u64;
        let complete = selected_admissions == total_admissions;
        let omissions = if complete {
            Vec::new()
        } else {
            vec![format!(
                "{} older committed admissions omitted",
                total_admissions - selected_admissions
            )]
        };
        Ok(UiFeedAdmissions {
            schema: UI_FEED_ADMISSIONS_SCHEMA.to_owned(),
            ordering: "committed_at_unix_desc_then_stable_identity".to_owned(),
            total_admissions,
            selected_admissions,
            complete,
            admissions,
            omissions,
        })
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
        let catalog = match store.head_catalog_from_state(&state) {
            Ok(catalog) => catalog,
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
        match store.status().and_then(|status| {
            let mailbox = store.mailbox(&status)?;
            Ok((status, mailbox))
        }) {
            Ok((status, mailbox)) => json_response(
                StatusCode(200),
                &UiChannelProjection {
                    schema: UI_CHANNELS_SCHEMA.to_owned(),
                    write_enabled: self.descriptor.channel_write_enabled,
                    authority: "unauthenticated_channel_working_write and exact clicked-message GitHub poll; no INDEX, HEAD, relay, provider mutation, or proof authority"
                        .to_owned(),
                    listener: UiChannelListener {
                        address: self.descriptor.address.clone(),
                        loopback_only: self.descriptor.loopback_only,
                        authentication: "none".to_owned(),
                        warning: if self.descriptor.loopback_only {
                            "any local client that can reach this listener may replace Channel WORKING or request an exact admitted GitHub poll"
                                .to_owned()
                        } else {
                            "any network client that can reach this listener may replace Channel WORKING or request an exact admitted GitHub poll without authentication"
                                .to_owned()
                        },
                    },
                    status,
                    mailbox,
                },
            ),
            Err(error) => json_error(
                StatusCode(500),
                "channel_status_unavailable",
                &error.to_string(),
            ),
        }
    }

    fn poll_github_mailbox(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request_header(request, "Content-Type") != Some("application/json") {
            return json_error(
                StatusCode(415),
                "github_poll_content_type",
                "GitHub poll requests require Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_UI_GITHUB_POLL_WRITE_BYTES)
        {
            return json_error(
                StatusCode(413),
                "github_poll_body_limit",
                "GitHub poll request exceeds the 4096-byte limit",
            );
        }
        let mut bytes = Vec::new();
        if let Err(error) = request
            .as_reader()
            .take(MAX_UI_GITHUB_POLL_WRITE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return json_error(
                StatusCode(400),
                "github_poll_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_UI_GITHUB_POLL_WRITE_BYTES {
            return json_error(
                StatusCode(413),
                "github_poll_body_limit",
                "GitHub poll request exceeds the 4096-byte limit",
            );
        }
        let write: UiGitHubPollWrite = match serde_json::from_slice(&bytes) {
            Ok(write) => write,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "github_poll_json_invalid",
                    &error.to_string(),
                );
            }
        };
        if write.schema != UI_GITHUB_POLL_WRITE_SCHEMA {
            return json_error(
                StatusCode(422),
                "github_poll_schema_invalid",
                "expected rey.ui-github-poll-write.v1",
            );
        }

        let store = LocalChannelStore::new(self.config.channel_directory.clone());
        let status = match store.status() {
            Ok(status) => status,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "channel_status_unavailable",
                    &error.to_string(),
                );
            }
        };
        let Some(head) = status.head_commit.as_ref() else {
            return json_error(
                StatusCode(409),
                "github_poll_head_unavailable",
                "GitHub poll requires an admitted Channel HEAD",
            );
        };
        if head.commit_id != write.expected_channel_head_commit_id {
            return json_error(
                StatusCode(409),
                "github_poll_head_stale",
                "clicked mailbox evidence does not bind the current Channel HEAD",
            );
        }
        let mailbox = match store.mailbox(&status) {
            Ok(mailbox) => mailbox,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "github_mailbox_unavailable",
                    &error.to_string(),
                );
            }
        };
        let Some(message) = mailbox
            .messages
            .iter()
            .find(|message| message.message_id == write.message_id)
        else {
            return self.channels();
        };
        if message.source.github_application()
            != Some((write.application_id.as_str(), write.application_revision))
        {
            return json_error(
                StatusCode(409),
                "github_poll_message_stale",
                "clicked mailbox evidence does not bind the requested GitHub application revision",
            );
        }
        let application_current = head.snapshot.graph.applications.iter().any(|application| {
            application.id == write.application_id
                && application.revision == write.application_revision
                && application.github_inbox.is_some()
        });
        if !application_current {
            return json_error(
                StatusCode(409),
                "github_poll_application_stale",
                "clicked mailbox evidence does not bind a current admitted GitHub application",
            );
        }

        let executable = match env::current_exe() {
            Ok(executable) => executable,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "github_poll_executable_unavailable",
                    &error.to_string(),
                );
            }
        };
        let output = match Command::new(executable)
            .arg("channels")
            .arg("--workspace")
            .arg(&self.config.workspace)
            .arg("--state-dir")
            .arg(&self.config.channel_directory)
            .arg("poll")
            .arg(&write.application_id)
            .arg("--format")
            .arg("json")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) => output,
            Err(error) => {
                return json_error(
                    StatusCode(502),
                    "github_poll_spawn_failed",
                    &error.to_string(),
                );
            }
        };
        if !output.status.success() && output.status.code() != Some(3) {
            let detail = String::from_utf8_lossy(&output.stderr)
                .trim()
                .chars()
                .take(512)
                .collect::<String>();
            return json_error(
                StatusCode(502),
                "github_poll_failed",
                &format!(
                    "GitHub poll exited with {}{}",
                    output
                        .status
                        .code()
                        .map_or_else(|| "signal".to_owned(), |code| code.to_string()),
                    if detail.is_empty() {
                        String::new()
                    } else {
                        format!(" · {detail}")
                    }
                ),
            );
        }
        self.channels()
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

    fn admit_observation(&self, request: &mut Request) -> Response<Cursor<Vec<u8>>> {
        if request_header(request, "Content-Type") != Some("application/json") {
            return json_error(
                StatusCode(415),
                "observation_content_type",
                "observation writes require Content-Type: application/json",
            );
        }
        if request
            .body_length()
            .is_some_and(|length| length as u64 > MAX_UI_OBSERVATION_WRITE_BYTES)
        {
            return json_error(
                StatusCode(413),
                "observation_body_limit",
                "observation write exceeds the 32768-byte limit",
            );
        }
        let mut bytes = Vec::new();
        if let Err(error) = request
            .as_reader()
            .take(MAX_UI_OBSERVATION_WRITE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return json_error(
                StatusCode(400),
                "observation_body_unreadable",
                &error.to_string(),
            );
        }
        if bytes.len() as u64 > MAX_UI_OBSERVATION_WRITE_BYTES {
            return json_error(
                StatusCode(413),
                "observation_body_limit",
                "observation write exceeds the 32768-byte limit",
            );
        }
        let write: UiObservationWrite = match serde_json::from_slice(&bytes) {
            Ok(write) => write,
            Err(error) => {
                return json_error(
                    StatusCode(400),
                    "observation_json_invalid",
                    &error.to_string(),
                );
            }
        };
        if write.schema != UI_OBSERVATION_WRITE_SCHEMA {
            return json_error(
                StatusCode(422),
                "observation_schema_invalid",
                "expected rey.ui-observation-write.v1",
            );
        }
        if write.body.chars().count() > MAX_UI_OBSERVATION_BODY_CHARS {
            return json_error(
                StatusCode(422),
                "observation_body_character_limit",
                "observation body exceeds the 500-character limit",
            );
        }
        let proposal = ObservationProposal {
            schema: rey::observations::OBSERVATION_PROPOSAL_SCHEMA.to_owned(),
            kind: write.kind,
            author: ObservationAuthor {
                kind: ObservationAuthorKind::Human,
                id: "operator".to_owned(),
            },
            subject_locator: "worktree:///".to_owned(),
            body: write.body,
            desired_delta: None,
            completeness: ObservationCompleteness::Partial,
            omissions: vec!["browser composer provides no exact evidence bindings".to_owned()],
            evidence: Vec::new(),
            supersedes: None,
        };
        let channel_store = LocalChannelStore::new(self.config.channel_directory.clone());
        let channel_status = match channel_store.status() {
            Ok(status) => status,
            Err(error) => {
                return json_error(
                    StatusCode(500),
                    "observation_channel_status_unavailable",
                    &error.to_string(),
                );
            }
        };
        let channel_ids = channel_status
            .working
            .graph
            .channels
            .iter()
            .filter(|channel| channel.broadcast_default)
            .map(|channel| channel.id.clone())
            .collect();
        let source = ObservationSource::from_bytes(
            format!("rey-ui://{}/feed/observations", self.descriptor.address),
            &bytes,
        );
        let store = LocalObservationStore::new(self.config.channel_directory.clone());
        match store.admit_and_broadcast(
            proposal,
            source,
            channel_ids,
            channel_status.head_commit.map(|commit| commit.commit_id),
            &channel_status.working,
            chrono::Utc::now().timestamp(),
        ) {
            Ok(result) => json_response(
                if result.observation_admitted {
                    StatusCode(201)
                } else {
                    StatusCode(200)
                },
                &result,
            ),
            Err(error) => json_error(
                StatusCode(422),
                "observation_admission_rejected",
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
                    occurred_at_unix: Some(commit.committed_at_unix),
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
            "Git and Rey commit wall times provide display order but do not prove causal order"
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
                    label: "Portfolio change scan".to_owned(),
                    source: "/api/v1/revalidation".to_owned(),
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

#[derive(Default)]
struct RevalidationScan {
    entries: usize,
    bytes: u64,
}

fn hash_revalidation_source(
    hasher: &mut SemanticHasher,
    label: &str,
    root: &Path,
    scan: &mut RevalidationScan,
) -> Result<(), String> {
    hasher.add_str(label);
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            hasher.add_str("missing");
            return Ok(());
        }
        Err(error) => {
            return Err(format!(
                "could not inspect revalidation source {}: {error}",
                root.display()
            ));
        }
    };
    hash_revalidation_path(hasher, root, Path::new(""), &metadata, scan)
}

fn hash_revalidation_path(
    hasher: &mut SemanticHasher,
    path: &Path,
    relative: &Path,
    metadata: &fs::Metadata,
    scan: &mut RevalidationScan,
) -> Result<(), String> {
    scan.entries = scan.entries.saturating_add(1);
    if scan.entries > MAX_REVALIDATION_SOURCE_ENTRIES {
        return Err(format!(
            "revalidation sources exceed the {MAX_REVALIDATION_SOURCE_ENTRIES}-entry limit"
        ));
    }
    hasher.add_bytes(relative.as_os_str().as_encoded_bytes());
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        hasher.add_str("directory");
        let mut entries = fs::read_dir(path)
            .map_err(|error| {
                format!(
                    "could not enumerate revalidation source {}: {error}",
                    path.display()
                )
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                format!(
                    "could not enumerate revalidation source {}: {error}",
                    path.display()
                )
            })?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let child_path = entry.path();
            let child_relative = relative.join(entry.file_name());
            let child_metadata = fs::symlink_metadata(&child_path).map_err(|error| {
                format!(
                    "could not inspect revalidation source {}: {error}",
                    child_path.display()
                )
            })?;
            hash_revalidation_path(hasher, &child_path, &child_relative, &child_metadata, scan)?;
        }
        return Ok(());
    }
    if file_type.is_symlink() {
        hasher.add_str("symlink");
        let target = fs::read_link(path).map_err(|error| {
            format!(
                "could not inspect revalidation symlink {}: {error}",
                path.display()
            )
        })?;
        hasher.add_bytes(target.as_os_str().as_encoded_bytes());
        return Ok(());
    }
    if !file_type.is_file() {
        hasher.add_str("non_regular");
        return Ok(());
    }

    hasher.add_str("file");
    let remaining = MAX_REVALIDATION_SOURCE_BYTES.saturating_sub(scan.bytes);
    if metadata.len() > remaining {
        return Err(format!(
            "revalidation sources exceed the {MAX_REVALIDATION_SOURCE_BYTES}-byte limit"
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .map_err(|error| {
            format!(
                "could not read revalidation source {}: {error}",
                path.display()
            )
        })?
        .take(remaining.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "could not read revalidation source {}: {error}",
                path.display()
            )
        })?;
    if bytes.len() as u64 > remaining {
        return Err(format!(
            "revalidation sources exceed the {MAX_REVALIDATION_SOURCE_BYTES}-byte limit"
        ));
    }
    scan.bytes = scan.bytes.saturating_add(bytes.len() as u64);
    hasher.add_bytes(&bytes);
    Ok(())
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
        Ok(bytes) => json_bytes_response(value_status, bytes),
        Err(error) => json_error(StatusCode(500), "json_encoding_failed", &error.to_string()),
    }
}

fn json_bytes_response(value_status: StatusCode, bytes: Vec<u8>) -> Response<Cursor<Vec<u8>>> {
    let response = Response::from_data(bytes).with_status_code(value_status);
    let response = with_header(response, "Content-Type", "application/json; charset=utf-8");
    with_common_headers(response, "no-store")
}

fn cached_json_response(
    status: StatusCode,
    cached: &CachedJsonProjection,
    accepts_gzip: bool,
) -> Response<Cursor<Vec<u8>>> {
    encoded_json_response(
        status,
        cached.bytes.clone(),
        cached.gzip_bytes.clone(),
        accepts_gzip,
    )
}

fn compact_workload_transport(list: &WorkloadList) -> Result<Value, String> {
    let mut value = serde_json::to_value(list).map_err(|error| error.to_string())?;
    compact_regional_projection_packets(&mut value)?;
    let root = value
        .as_object_mut()
        .ok_or_else(|| "workload transport root is not an object".to_owned())?;
    let workloads = root
        .get_mut("workloads")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "workload transport has no workload array".to_owned())?;
    for workload in workloads {
        let Some(workload) = workload.as_object_mut() else {
            return Err("workload transport contains a non-object summary".to_owned());
        };
        if workload
            .get("scene_admissions")
            .and_then(Value::as_array)
            .is_some_and(|admissions| !admissions.is_empty())
        {
            workload.remove("latest_scene_admission");
        }
    }
    root.insert(
        "transport".to_owned(),
        json!({
            "schema": "rey.ui-workload-transport.v1",
            "terrain_grid_encodings": [
                "rey.regional-terrain-grid.transport.v2",
                "rey.regional-terrain-grid.transport.v3"
            ],
            "latest_scene_policy": "scene_admissions is canonical; duplicated latest_scene_admission is omitted when active scene admissions are present",
            "authority": "lossless renderer transport over exact retained scene identities; omitted repeated rows remain available through CLI and exact evidence routes",
        }),
    );
    Ok(value)
}

fn compact_regional_projection_packets(value: &mut Value) -> Result<(), String> {
    if value
        .get("schema")
        .and_then(Value::as_str)
        .is_some_and(|schema| schema == "rey.regional-projection-packet.v1")
    {
        compact_regional_projection_packet(value)?;
        return Ok(());
    }
    match value {
        Value::Array(values) => {
            for value in values {
                compact_regional_projection_packets(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                compact_regional_projection_packets(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn compact_regional_projection_packet(packet: &mut Value) -> Result<(), String> {
    let packet = packet
        .as_object_mut()
        .ok_or_else(|| "regional projection transport is not an object".to_owned())?;
    let Some(grid) = packet
        .get("terrain")
        .and_then(|terrain| terrain.get("grid"))
        .cloned()
    else {
        return Ok(());
    };
    if matches!(
        grid.get("schema").and_then(Value::as_str),
        Some("rey.regional-terrain-grid.v2" | "rey.regional-terrain-grid.v3")
    ) {
        return compact_retained_regional_projection_packet(packet, &grid);
    }
    if grid.get("schema").and_then(Value::as_str) != Some("rey.regional-terrain-grid.v1") {
        return Err("regional terrain transport encountered an unsupported grid".to_owned());
    }
    let cells = grid
        .get("cells")
        .and_then(Value::as_array)
        .ok_or_else(|| "regional terrain grid has no cells".to_owned())?;
    if cells.is_empty() {
        return Err("regional terrain grid is empty".to_owned());
    }
    let objects = packet
        .get("objects")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "regional projection has no objects".to_owned())?;
    let original_object_count = objects.len();
    let objects_by_id = objects
        .iter()
        .filter_map(|object| Some((object.get("object_id")?.as_str()?.to_owned(), object)))
        .collect::<BTreeMap<_, _>>();
    let mut terrain_object_ids = BTreeSet::new();
    let mut cell_ids = Vec::with_capacity(cells.len());
    let mut source_object_ids = Vec::with_capacity(cells.len());
    let mut source_object_revisions = Vec::with_capacity(cells.len());
    let mut elevation_micrometers = Vec::with_capacity(cells.len());
    let mut material_names = BTreeSet::new();
    let mut validity = Vec::with_capacity(cells.len());
    let mut source_id = None;
    let mut source_path = None;
    let mut source_artifact_id = None;
    for cell in cells {
        let cell_id = required_string(cell, "cell_id")?;
        let object_id = required_string(cell, "source_object_id")?;
        let object_revision = required_string(cell, "source_object_revision")?;
        let artifact_id = required_string(cell, "source_artifact_id")?;
        let object = objects_by_id
            .get(&object_id)
            .ok_or_else(|| format!("terrain cell {object_id} lost its exact source object"))?;
        if object.get("layer").and_then(Value::as_str) != Some("terrain")
            || required_string(object, "object_revision")? != object_revision
            || required_string(object, "source_artifact_id")? != artifact_id
        {
            return Err(format!(
                "terrain cell {object_id} does not match its exact source object"
            ));
        }
        let object_source_id = required_string(object, "source_id")?;
        let object_source_path = required_string(object, "source_path")?;
        bind_common(&mut source_id, object_source_id, "terrain source id")?;
        bind_common(&mut source_path, object_source_path, "terrain source path")?;
        bind_common(
            &mut source_artifact_id,
            artifact_id,
            "terrain source artifact",
        )?;
        let cell_valid = required_string(cell, "validity")? == "valid";
        if !cell_valid && cell.get("validity").and_then(Value::as_str) != Some("no_data") {
            return Err(format!("terrain cell {object_id} has unsupported validity"));
        }
        let elevation = if cell_valid {
            cell.get("elevation_micrometers")
                .and_then(Value::as_i64)
                .ok_or_else(|| format!("terrain cell {object_id} has no exact elevation"))?
        } else {
            0
        };
        if let Some(material) = cell.get("material").and_then(Value::as_str) {
            material_names.insert(material.to_owned());
        } else if cell_valid {
            return Err(format!("terrain cell {object_id} has no exact material"));
        }
        terrain_object_ids.insert(object_id.clone());
        cell_ids.push(cell_id);
        source_object_ids.push(object_id);
        source_object_revisions.push(object_revision);
        elevation_micrometers.push(elevation);
        validity.push(u8::from(cell_valid));
    }
    let material_palette = material_names.into_iter().collect::<Vec<_>>();
    if material_palette.len() > 255 {
        return Err("terrain material palette exceeds compact transport".to_owned());
    }
    let material_lookup = material_palette
        .iter()
        .enumerate()
        .map(|(index, material)| (material.as_str(), index as u8))
        .collect::<BTreeMap<_, _>>();
    let material_indices = cells
        .iter()
        .map(|cell| {
            cell.get("material")
                .and_then(Value::as_str)
                .map_or(Ok(255), |material| {
                    material_lookup
                        .get(material)
                        .copied()
                        .ok_or_else(|| "terrain material palette lost a member".to_owned())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut compact = grid
        .as_object()
        .cloned()
        .ok_or_else(|| "regional terrain grid is not an object".to_owned())?;
    compact.remove("cells");
    compact.insert(
        "schema".to_owned(),
        Value::String("rey.regional-terrain-grid.transport.v2".to_owned()),
    );
    compact.insert(
        "source_schema".to_owned(),
        Value::String("rey.regional-terrain-grid.v1".to_owned()),
    );
    compact.insert(
        "cell_source_encoding".to_owned(),
        Value::String("geojson_point_features_v1".to_owned()),
    );
    compact.insert("source_id".to_owned(), Value::String(source_id.unwrap()));
    compact.insert(
        "source_path".to_owned(),
        Value::String(source_path.unwrap()),
    );
    compact.insert(
        "source_artifact_id".to_owned(),
        Value::String(source_artifact_id.unwrap()),
    );
    insert_packed_terrain_identities(
        &mut compact,
        &cell_ids,
        &source_object_ids,
        &source_object_revisions,
    )?;
    compact.insert(
        "validity_hex".to_owned(),
        Value::String(hex_bytes(&validity)),
    );
    compact.insert(
        "elevation_micrometers".to_owned(),
        json!(elevation_micrometers),
    );
    compact.insert("material_palette".to_owned(), json!(material_palette));
    compact.insert(
        "material_indices_hex".to_owned(),
        Value::String(hex_bytes(&material_indices)),
    );
    compact.insert(
        "transport_authority".to_owned(),
        Value::String("lossless row-major transport of the exact admitted grid; coordinates and grid positions are reconstructed only from admitted bounds and dimensions".to_owned()),
    );
    let mut hasher = SemanticHasher::new("rey.regional-terrain-grid.transport.v2");
    hasher.add_bytes(&serde_json::to_vec(&compact).map_err(|error| error.to_string())?);
    compact.insert(
        "transport_id".to_owned(),
        Value::String(hasher.finish().to_string()),
    );

    packet
        .get_mut("terrain")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "regional terrain program is not an object".to_owned())?
        .insert("grid".to_owned(), Value::Object(compact));
    packet
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "regional projection objects changed during transport".to_owned())?
        .retain(|object| {
            object
                .get("object_id")
                .and_then(Value::as_str)
                .is_none_or(|id| !terrain_object_ids.contains(id))
        });
    for layer in packet
        .get_mut("layers")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "regional projection layers changed during transport".to_owned())?
    {
        if layer.get("kind").and_then(Value::as_str) == Some("terrain") {
            layer
                .as_object_mut()
                .ok_or_else(|| "regional terrain layer is not an object".to_owned())?
                .insert("object_ids".to_owned(), Value::Array(Vec::new()));
        }
    }
    packet
        .get_mut("validity")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "regional projection validity changed during transport".to_owned())?
        .retain(|record| {
            record
                .get("scope")
                .and_then(Value::as_str)
                .and_then(|scope| scope.strip_prefix("native_geometry:"))
                .is_none_or(|id| !terrain_object_ids.contains(id))
        });
    packet.insert(
        "transport".to_owned(),
        json!({
            "schema": "rey.regional-projection-packet.transport.v1",
            "source_packet_id": packet.get("packet_id").cloned().unwrap_or(Value::Null),
            "omitted_terrain_objects": original_object_count.saturating_sub(packet.get("objects").and_then(Value::as_array).map_or(0, Vec::len)),
            "authority": "terrain object, layer-membership, and per-object validity repetition is encoded once in the exact compact grid; semantic content is unchanged",
        }),
    );
    Ok(())
}

fn compact_retained_regional_projection_packet(
    packet: &mut serde_json::Map<String, Value>,
    grid: &Value,
) -> Result<(), String> {
    let grid = grid
        .as_object()
        .ok_or_else(|| "retained regional terrain grid is not an object".to_owned())?;
    let source_schema = grid
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| "retained regional terrain grid has no schema".to_owned())?;
    let compact_field = match source_schema {
        "rey.regional-terrain-grid.v2" => "compact",
        "rey.regional-terrain-grid.v3" => "packed_compact",
        _ => return Err("retained regional terrain grid has unsupported compact cells".to_owned()),
    };
    let compact = grid
        .get(compact_field)
        .and_then(Value::as_object)
        .ok_or_else(|| "retained regional terrain grid has no compact cells".to_owned())?;
    let columns = grid
        .get("columns")
        .and_then(Value::as_u64)
        .ok_or_else(|| "retained regional terrain grid has no columns".to_owned())?;
    let rows = grid
        .get("rows")
        .and_then(Value::as_u64)
        .ok_or_else(|| "retained regional terrain grid has no rows".to_owned())?;
    let cell_count = columns
        .checked_mul(rows)
        .ok_or_else(|| "retained regional terrain grid size overflowed".to_owned())?;
    let cell_source_encoding = match grid.get("authority").and_then(Value::as_str) {
        Some(
            "qualified rectilinear height/material grid; validity ends at supported source triangles",
        ) => "geojson_point_features_v1",
        Some(
            "qualified packed rectilinear height/material grid; validity ends at supported source triangles",
        ) => "geojson_packed_grid_v1",
        _ => return Err("retained regional terrain grid has invalid authority".to_owned()),
    };
    for field in [
        "source_id",
        "source_path",
        "source_artifact_id",
        "validity_hex",
        "elevation_micrometers",
        "material_palette",
        "material_indices_hex",
    ] {
        if !compact.contains_key(field) {
            return Err(format!(
                "retained regional terrain grid has no compact {field}"
            ));
        }
    }
    if packet
        .get("objects")
        .and_then(Value::as_array)
        .is_none_or(|objects| {
            objects
                .iter()
                .any(|object| object.get("layer").and_then(Value::as_str) == Some("terrain"))
        })
    {
        return Err("retained compact terrain repeats native terrain objects".to_owned());
    }

    let mut transport = grid.clone();
    transport.remove("compact");
    transport.remove("packed_compact");
    transport.remove("cells");
    transport.insert(
        "source_schema".to_owned(),
        Value::String(source_schema.to_owned()),
    );
    for field in [
        "source_id",
        "source_path",
        "source_artifact_id",
        "validity_hex",
        "elevation_micrometers",
        "material_palette",
        "material_indices_hex",
    ] {
        transport.insert(
            field.to_owned(),
            compact
                .get(field)
                .cloned()
                .ok_or_else(|| format!("retained regional terrain grid lost {field}"))?,
        );
    }
    transport.insert(
        "cell_source_encoding".to_owned(),
        Value::String(cell_source_encoding.to_owned()),
    );
    if source_schema == "rey.regional-terrain-grid.v2" {
        let cell_ids = required_string_array(compact, "cell_ids")?;
        let source_object_ids = required_string_array(compact, "source_object_ids")?;
        let source_object_revisions = required_string_array(compact, "source_object_revisions")?;
        if cell_ids.len() as u64 != cell_count
            || source_object_ids.len() as u64 != cell_count
            || source_object_revisions.len() as u64 != cell_count
        {
            return Err("retained regional terrain identity count changed".to_owned());
        }
        transport.insert(
            "schema".to_owned(),
            Value::String("rey.regional-terrain-grid.transport.v2".to_owned()),
        );
        insert_packed_terrain_identities(
            &mut transport,
            &cell_ids,
            &source_object_ids,
            &source_object_revisions,
        )?;
    } else {
        transport.insert(
            "schema".to_owned(),
            Value::String("rey.regional-terrain-grid.transport.v3".to_owned()),
        );
        transport.insert(
            "identity_encoding".to_owned(),
            Value::String("rey.packed-terrain-grid-cell-identities.v1".to_owned()),
        );
        for field in ["source_feature_id", "source_feature_revision"] {
            transport.insert(
                field.to_owned(),
                compact
                    .get(field)
                    .cloned()
                    .ok_or_else(|| format!("retained packed terrain grid lost {field}"))?,
            );
        }
    }
    transport.insert(
        "transport_authority".to_owned(),
        Value::String("lossless row-major transport of the exact admitted grid; coordinates and grid positions are reconstructed only from admitted bounds and dimensions".to_owned()),
    );
    let transport_schema = transport
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| "regional terrain transport lost its schema".to_owned())?;
    let mut hasher = SemanticHasher::new(transport_schema);
    hasher.add_bytes(&serde_json::to_vec(&transport).map_err(|error| error.to_string())?);
    transport.insert(
        "transport_id".to_owned(),
        Value::String(hasher.finish().to_string()),
    );

    packet
        .get_mut("terrain")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "retained regional terrain program is not an object".to_owned())?
        .insert("grid".to_owned(), Value::Object(transport));
    for layer in packet
        .get_mut("layers")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "retained regional projection layers changed during transport".to_owned())?
    {
        if layer.get("kind").and_then(Value::as_str) == Some("terrain") {
            layer
                .as_object_mut()
                .ok_or_else(|| "retained regional terrain layer is not an object".to_owned())?
                .insert("object_ids".to_owned(), Value::Array(Vec::new()));
        }
    }
    packet.insert(
        "transport".to_owned(),
        json!({
            "schema": "rey.regional-projection-packet.transport.v1",
            "source_packet_id": packet.get("packet_id").cloned().unwrap_or(Value::Null),
            "omitted_terrain_objects": cell_count,
            "authority": "terrain object, layer-membership, and per-object validity repetition is encoded once in the exact compact grid; semantic content is unchanged",
        }),
    );
    Ok(())
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("regional terrain transport has no {field}"))
}

fn required_string_array(
    value: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Vec<String>, String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("regional terrain transport has no {field}"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("regional terrain transport has a non-string {field}"))
        })
        .collect()
}

fn insert_packed_terrain_identities(
    transport: &mut serde_json::Map<String, Value>,
    cell_ids: &[String],
    source_object_ids: &[String],
    source_object_revisions: &[String],
) -> Result<(), String> {
    if cell_ids.len() != source_object_ids.len() || cell_ids.len() != source_object_revisions.len()
    {
        return Err("regional terrain transport identity columns changed length".to_owned());
    }
    let source_object_id_prefix = common_string_prefix(source_object_ids);
    let source_object_id_suffixes = source_object_ids
        .iter()
        .map(|identity| identity[source_object_id_prefix.len()..].to_owned())
        .collect::<Vec<_>>();
    transport.insert(
        "digest_encoding".to_owned(),
        Value::String("base64-concatenated-blake3-256".to_owned()),
    );
    transport.insert(
        "cell_digests_base64".to_owned(),
        Value::String(pack_blake3_digests(cell_ids)?),
    );
    transport.insert(
        "source_object_id_prefix".to_owned(),
        Value::String(source_object_id_prefix),
    );
    transport.insert(
        "source_object_id_suffixes".to_owned(),
        json!(source_object_id_suffixes),
    );
    transport.insert(
        "source_object_revision_digests_base64".to_owned(),
        Value::String(pack_blake3_digests(source_object_revisions)?),
    );
    Ok(())
}

fn pack_blake3_digests(values: &[String]) -> Result<String, String> {
    let mut packed = Vec::with_capacity(values.len().saturating_mul(32));
    for value in values {
        let digest = value
            .strip_prefix("blake3:")
            .filter(|digest| digest.len() == 64)
            .ok_or_else(|| {
                "regional terrain transport encountered a non-BLAKE3 identity".to_owned()
            })?;
        if !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(
                "regional terrain transport encountered a non-canonical BLAKE3 identity".to_owned(),
            );
        }
        for pair in digest.as_bytes().chunks_exact(2) {
            let high = hex_digit(pair[0]).ok_or_else(|| {
                "regional terrain transport encountered an invalid BLAKE3 identity".to_owned()
            })?;
            let low = hex_digit(pair[1]).ok_or_else(|| {
                "regional terrain transport encountered an invalid BLAKE3 identity".to_owned()
            })?;
            packed.push((high << 4) | low);
        }
    }
    Ok(BASE64_STANDARD.encode(packed))
}

fn common_string_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };
    let mut length = first.len();
    for value in values.iter().skip(1) {
        length = first
            .as_bytes()
            .iter()
            .zip(value.as_bytes())
            .take(length)
            .take_while(|(left, right)| left == right)
            .count();
    }
    while !first.is_char_boundary(length) {
        length -= 1;
    }
    first[..length].to_owned()
}

fn bind_common(target: &mut Option<String>, value: String, label: &str) -> Result<(), String> {
    if target.as_ref().is_some_and(|current| current != &value) {
        return Err(format!("regional terrain transport mixes {label}"));
    }
    *target = Some(value);
    Ok(())
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn encoded_json_response(
    status: StatusCode,
    bytes: Vec<u8>,
    gzip_bytes: Vec<u8>,
    accepts_gzip: bool,
) -> Response<Cursor<Vec<u8>>> {
    let response = if accepts_gzip {
        let response = Response::from_data(gzip_bytes).with_status_code(status);
        with_header(response, "Content-Encoding", "gzip")
    } else {
        Response::from_data(bytes).with_status_code(status)
    };
    let response = with_header(response, "Content-Type", "application/json; charset=utf-8");
    let response = with_header(response, "Vary", "Accept-Encoding");
    with_common_headers(response, "no-store")
}

fn gzip_bytes(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(bytes)?;
    encoder.finish()
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
        .headers
        .get(name)
        .and_then(|header| header.to_str().ok())
}

fn accepts_content_encoding(request: &Request, expected: &str) -> bool {
    request_header(request, "Accept-Encoding").is_some_and(|header| {
        header.split(',').any(|entry| {
            let mut parts = entry.trim().split(';');
            let encoding = parts.next().unwrap_or_default().trim();
            let quality = parts
                .find_map(|parameter| {
                    parameter
                        .trim()
                        .strip_prefix("q=")
                        .and_then(|value| value.parse::<f32>().ok())
                })
                .unwrap_or(1.0);
            (encoding.eq_ignore_ascii_case(expected) || encoding == "*") && quality > 0.0
        })
    })
}

fn json_error(status: StatusCode, category: &str, detail: &str) -> Response<Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(&json!({
        "schema": UI_ERROR_SCHEMA,
        "category": category,
        "detail": detail,
    }))
    .unwrap_or_else(|_| {
        b"{\"schema\":\"rey.api-error.v1\",\"category\":\"encoding_failed\"}".to_vec()
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
    #[error("operator listener failed: {0}")]
    Listener(std::io::Error),
    #[error("operator Axum runtime failed: {0}")]
    Runtime(std::io::Error),
    #[error("operator Axum server failed: {0}")]
    Serve(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpStream,
        thread,
    };

    use flate2::read::GzDecoder;
    use tempfile::TempDir;

    use super::{
        STATIC_UI_ASSETS, UiServer, UiServerConfig, compact_retained_regional_projection_packet,
        gzip_bytes,
    };
    use rey::{
        channels::LocalChannelStore,
        conversations::{
            ConversationSessionProposal, ConversationSource as TranscriptSource,
            LocalConversationStore,
        },
        env::{EnvironmentCommit, LocalEnvironmentHistory, LocalEnvironmentStore},
        observations::{LocalObservationStore, ObservationProposal, ObservationSource},
        workloads::LocalWorkloadStore,
    };
    use rey_environment::{
        Availability, CapabilityRecord, CapabilitySnapshot, DISCOVERY_APPLICATION_SCHEMA,
        DiscoveryApplicationProvenance, DiscoveryLimits, TrustClass,
    };

    #[test]
    fn gzip_projection_encoding_round_trips_exact_json_bytes() {
        let source = br#"{"schema":"rey.workload-list.v1","terrain":"repeated"}"#.repeat(256);
        let compressed = gzip_bytes(&source).unwrap();
        assert!(compressed.len() < source.len());
        let mut decoded = Vec::new();
        GzDecoder::new(compressed.as_slice())
            .read_to_end(&mut decoded)
            .unwrap();
        assert_eq!(decoded, source);
    }

    #[test]
    fn packed_terrain_transport_retains_derivation_inputs_without_cell_identity_columns() {
        let mut packet = serde_json::json!({
            "packet_id": "blake3:packet",
            "objects": [],
            "layers": [{"kind": "terrain", "object_ids": []}],
            "terrain": {
                "grid": {
                    "schema": "rey.regional-terrain-grid.v3",
                    "dataset_id": "blake3:dataset",
                    "source_dataset_id": "relief",
                    "columns": 2,
                    "rows": 2,
                    "native_bounds": {
                        "west_microdegrees": -2,
                        "south_microdegrees": 4,
                        "east_microdegrees": 0,
                        "north_microdegrees": 6,
                        "crosses_antimeridian": false
                    },
                    "validity_semantics": "row-major source vertices are explicitly valid or no_data; no_data cuts triangle support",
                    "interpolation": "piecewise linear only within triangles whose three admitted source vertices are valid",
                    "authority": "qualified packed rectilinear height/material grid; validity ends at supported source triangles",
                    "packed_compact": {
                        "encoding": "canonical row-major packed-source cells; positions and identities derive from bounds, dimensions, and the exact source feature; validity and material indices are hexadecimal bytes",
                        "source_id": "terrain",
                        "source_path": "terrain.geojson",
                        "source_artifact_id": "blake3:artifact",
                        "source_feature_id": "terrain/relief",
                        "source_feature_revision": "blake3:feature",
                        "validity_hex": "01010101",
                        "elevation_micrometers": [1, 2, 3, 4],
                        "material_palette": ["granite"],
                        "material_indices_hex": "00000000",
                        "authority": "lossless retained encoding of exact packed-source terrain values and derivation inputs; it grants no interpolation, synthesis, or coverage authority"
                    }
                }
            }
        });
        let grid = packet["terrain"]["grid"].clone();
        compact_retained_regional_projection_packet(packet.as_object_mut().unwrap(), &grid)
            .unwrap();
        let transport = &packet["terrain"]["grid"];
        assert_eq!(
            transport["schema"],
            "rey.regional-terrain-grid.transport.v3"
        );
        assert_eq!(
            transport["identity_encoding"],
            "rey.packed-terrain-grid-cell-identities.v1"
        );
        assert_eq!(transport["source_feature_id"], "terrain/relief");
        assert_eq!(transport["source_feature_revision"], "blake3:feature");
        assert!(transport.get("cell_digests_base64").is_none());
        assert!(transport.get("source_object_id_suffixes").is_none());
        assert!(
            transport
                .get("source_object_revision_digests_base64")
                .is_none()
        );
    }

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
        let environment_store = LocalEnvironmentStore::default_for_workspace(workspace.path());
        let gh_provenance = DiscoveryApplicationProvenance {
            schema: DISCOVERY_APPLICATION_SCHEMA.to_owned(),
            name: "gh".to_owned(),
            groups: vec!["code".to_owned(), "communications".to_owned()],
            purpose: "Potential GitHub communications client; discovery grants no relay authority"
                .to_owned(),
            required: false,
            potential_capabilities: vec!["comms.application.github.identity".to_owned()],
            search_path_count: 1,
        };
        let environment_snapshot = CapabilitySnapshot::new(
            "standalone",
            DiscoveryLimits::default(),
            vec![CapabilityRecord {
                provider_id: "rey.tool.gh".to_owned(),
                provider_revision: 2,
                provider_kind: "known_tool".to_owned(),
                capability_id: "comms.application.github.identity".to_owned(),
                capability_kind: "identity_probe".to_owned(),
                resolved_location: Some("/usr/bin/gh".to_owned()),
                version: Some("gh version fixture".to_owned()),
                content_digest: Some("blake3:gh-fixture".to_owned()),
                provenance: Some(serde_json::to_string(&gh_provenance).unwrap()),
                availability: Availability::Available,
                trust_class: TrustClass::DiscoveredLocal,
                operations: vec!["inspect_identity".to_owned()],
                enforced_limits: Vec::new(),
                unsupported_limits: Vec::new(),
                observed_at: None,
                error_code: None,
                error_detail: None,
            }],
        )
        .unwrap();
        let mut environment_history = LocalEnvironmentHistory::default();
        environment_history.commits.push(
            EnvironmentCommit::new_at(1, None, 100, "Admit gh application", environment_snapshot)
                .unwrap(),
        );
        environment_store.save(&environment_history).unwrap();
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
        assert_eq!(descriptor.schema, "rey.ui-server.v2");
        assert_eq!(descriptor.http_framework, "axum");
        assert_eq!(descriptor.api_root, "/api");
        assert_eq!(descriptor.openapi_document, "/api/openapi.json");
        assert_eq!(descriptor.swagger_ui, "/api/docs/");
        assert!(descriptor.loopback_only);
        assert!(!descriptor.read_only);
        assert!(descriptor.journal_write_enabled);
        assert!(descriptor.observation_write_enabled);
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
                .serve_bounded(Some(55 + STATIC_UI_ASSETS.len()))
                .unwrap()
        });

        let health = request(&address, "GET /api/v1/health HTTP/1.1");
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains("\"schema\":\"rey.agent-health.v2\""));
        assert!(health.contains("\"schema\":\"rey.agent-process.v2\""));
        assert!(health.contains("\"loopback_only\":true"));

        let revalidation = request(&address, "GET /api/v1/revalidation HTTP/1.1");
        assert!(revalidation.starts_with("HTTP/1.1 200"));
        let revalidation: serde_json::Value =
            serde_json::from_str(response_body(&revalidation)).unwrap();
        assert_eq!(revalidation["schema"], "rey.ui-revalidation.v1");
        assert_eq!(revalidation["poll_after_ms"], 5_000);
        assert_eq!(
            revalidation["basis"],
            "exact bounded source bytes; missing roots and non-regular entries are framed explicitly"
        );
        let initial_revalidation_revision = revalidation["revision"].clone();

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
        let cached_workloads = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert_eq!(response_body(&cached_workloads), response_body(&workloads));
        let compressed_workloads = request_headers_only(
            &address,
            "GET /api/v1/workloads HTTP/1.1",
            &[("Accept-Encoding", "br, gzip, deflate")],
        );
        assert!(compressed_workloads.starts_with("HTTP/1.1 200"));
        assert!(compressed_workloads.contains("content-encoding: gzip"));
        assert!(compressed_workloads.contains("vary: Accept-Encoding"));

        let global_before = request(&address, "GET /api/v1/revalidation HTTP/1.1");
        let global_before: serde_json::Value =
            serde_json::from_str(response_body(&global_before)).unwrap();
        let unrelated_conversation_directory = workspace.path().join(".rey/conversations");
        fs::create_dir_all(&unrelated_conversation_directory).unwrap();
        fs::write(
            unrelated_conversation_directory.join("unrelated-ui-tick"),
            "conversation state does not feed the workload projection",
        )
        .unwrap();
        let global_after = request(&address, "GET /api/v1/revalidation HTTP/1.1");
        let global_after: serde_json::Value =
            serde_json::from_str(response_body(&global_after)).unwrap();
        assert_ne!(global_after["revision"], global_before["revision"]);
        let workloads_after_unrelated_tick = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert_eq!(
            response_body(&workloads_after_unrelated_tick),
            response_body(&workloads)
        );

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

        let revalidation = request(&address, "GET /api/v1/revalidation HTTP/1.1");
        assert!(revalidation.starts_with("HTTP/1.1 200"));
        let revalidation: serde_json::Value =
            serde_json::from_str(response_body(&revalidation)).unwrap();
        assert_ne!(revalidation["revision"], initial_revalidation_revision);

        let admitted = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(admitted.starts_with("HTTP/1.1 200"));
        assert!(admitted.contains("\"sequence\":1"));
        assert!(admitted.contains("\"index\":null"));
        assert!(admitted.contains("\"state\":\"clean\""));

        let admissions = request(&address, "GET /api/v1/workloads/admissions HTTP/1.1");
        assert!(admissions.starts_with("HTTP/1.1 200"));
        assert!(admissions.contains("\"schema\":\"rey.workload-log.v1\""));
        assert!(admissions.contains("\"total_commits\":1"));
        assert!(admissions.contains("Approve exact context survey"));

        let feed_admissions = request(&address, "GET /api/v1/feed/admissions HTTP/1.1");
        assert!(feed_admissions.starts_with("HTTP/1.1 200"));
        let feed_admissions_json: serde_json::Value =
            serde_json::from_str(response_body(&feed_admissions)).unwrap();
        assert_eq!(feed_admissions_json["schema"], "rey.ui-feed-admissions.v1");
        assert_eq!(feed_admissions_json["total_admissions"], 2);
        assert_eq!(feed_admissions_json["selected_admissions"], 2);
        assert_eq!(feed_admissions_json["complete"], true);
        assert_eq!(feed_admissions_json["admissions"][0]["kind"], "workload");
        assert_eq!(feed_admissions_json["admissions"][1]["kind"], "environment");
        assert!(
            feed_admissions_json["admissions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|admission| {
                    admission["kind"] == "environment"
                        && admission["commit"]["message"] == "Admit gh application"
                        && admission["changes"]["applications"][0]["name"] == "gh"
                        && admission["changes"]["applications"][0]["availability"] == "available"
                })
        );

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

        let oversized_observation_write = serde_json::json!({
            "schema": "rey.ui-observation-write.v1",
            "kind": "finding",
            "body": "x".repeat(501)
        })
        .to_string();
        let oversized_observation = request_with_body(
            &address,
            "POST /api/v1/observations HTTP/1.1",
            &[("Content-Type", "application/json")],
            &oversized_observation_write,
        );
        assert!(oversized_observation.starts_with("HTTP/1.1 422"));
        assert!(oversized_observation.contains("observation_body_character_limit"));

        let observation_write = serde_json::json!({
            "schema": "rey.ui-observation-write.v1",
            "kind": "question",
            "body": "Should the next bounded survey retain this exact bearing?"
        })
        .to_string();
        let browser_observation = request_with_body(
            &address,
            "POST /api/v1/observations HTTP/1.1",
            &[("Content-Type", "application/json")],
            &observation_write,
        );
        assert!(browser_observation.starts_with("HTTP/1.1 201"));
        assert!(browser_observation.contains("\"schema\":\"rey.observation-admission-result.v1\""));
        assert!(browser_observation.contains("\"kind\":\"human\",\"id\":\"operator\""));
        assert!(browser_observation.contains("\"completeness\":\"partial\""));
        assert!(
            browser_observation.contains("browser composer provides no exact evidence bindings")
        );
        assert!(browser_observation.contains("\"channel_id\":\"workspace\""));

        let observations = request(&address, "GET /api/v1/observations HTTP/1.1");
        assert!(observations.starts_with("HTTP/1.1 200"));
        assert!(observations.contains("\"schema\":\"rey.observation-frontier.v1\""));
        assert!(observations.contains("\"ordering\":\"observation_sequence_ascending\""));
        assert!(observations.contains("\"limit\":64"));
        assert!(observations.contains(observation.observation_id.as_str()));
        assert!(observations.contains("Should the next bounded survey retain this exact bearing?"));

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
        assert!(opportunities.contains("content-length:"));
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
            "/assets/react-three-fiber.js",
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
        assert!(!application.contains("Inputs and topology"));
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
        assert!(!application.contains("DESIRED INVENTORY"));
        assert!(!application.contains("SEARCH RECORD"));
        assert!(!application.contains("PROCESS SEEDS"));
        assert!(application.contains("SUPPORTED"));
        assert!(application.contains("NOT FOUND"));
        assert!(application.contains("react-three-fiber@9.7.0+three@0.185.1:webgpu+tsl"));
        assert!(application.contains("context-globe-samples:"));
        assert!(application.contains("rey-continuous-relief"));
        assert!(application.contains("HISTORY / CHANNELS + RUNTIME"));
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
        assert!(application.contains("POST OBSERVATION"));
        assert!(application.contains("rey.ui-observation-write.v1"));
        assert!(!application.contains("REY / CURRENT PROJECTION"));
        assert!(!application.contains("ADMISSION CONTROL"));
        assert!(application.contains("EXACT SNAPSHOT APPROVAL"));
        assert!(application.contains("REY / WORKLOAD COMMIT"));
        assert!(application.contains("ADMIT EXACT FILE SNAPSHOT"));
        assert!(!application.contains("Display order is not causal order"));
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
        assert!(root.contains("location: /api"));

        let api_root = request(&address, "GET /api HTTP/1.1");
        assert!(api_root.starts_with("HTTP/1.1 307"));
        assert!(api_root.contains("location: /api/docs/"));

        let swagger = request(&address, "GET /api/docs/ HTTP/1.1");
        assert!(swagger.starts_with("HTTP/1.1 200"));
        assert!(swagger.contains("<title>Swagger UI</title>"));

        let swagger_stylesheet = request(&address, "GET /api/docs/swagger-ui.css HTTP/1.1");
        assert!(swagger_stylesheet.starts_with("HTTP/1.1 200"));
        assert!(swagger_stylesheet.contains("text/css"));

        let openapi = request(&address, "GET /api/openapi.json HTTP/1.1");
        assert!(openapi.starts_with("HTTP/1.1 200"));
        assert!(openapi.contains("\"openapi\":\"3.1.0\""));
        assert!(openapi.contains("\"title\":\"Rey Agent API\""));

        let explore = request(&address, "GET /explore HTTP/1.1");
        assert!(explore.starts_with("HTTP/1.1 200"));
        assert!(explore.contains("content-security-policy"));
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
        assert!(journal_new.contains("src=\"/assets/app.js\""));
        assert!(journal_new.contains("href=\"/assets/app.css\""));
        assert!(!journal_new.contains("./assets/"));

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

        let unknown_api = request(&address, "GET /api/v1/not-real HTTP/1.1");
        assert!(unknown_api.starts_with("HTTP/1.1 404"));
        assert!(unknown_api.contains("\"category\":\"api_route_not_found\""));
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
        assert_eq!(initial["mailbox"]["schema"], "rey.channel-mailbox.v1");
        assert_eq!(initial["mailbox"]["messages"], serde_json::json!([]));
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

    fn request_headers_only(address: &str, request_line: &str, headers: &[(&str, &str)]) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        write!(
            stream,
            "{request_line}\r\nHost: {address}\r\nConnection: close\r\nContent-Length: 0\r\n",
        )
        .unwrap();
        for (name, value) in headers {
            write!(stream, "{name}: {value}\r\n").unwrap();
        }
        write!(stream, "\r\n").unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        String::from_utf8(response[..header_end].to_vec()).unwrap()
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
