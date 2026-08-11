use std::{
    env,
    io::{Cursor, Read},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use rey::{
    current_environment_status,
    env::LocalEnvironmentStore,
    journal::{
        JournalAdmission, JournalAuthorKind, JournalEntryProposal, JournalLog, LocalJournalStore,
        MAX_JOURNAL_PROPOSAL_BYTES,
    },
    workloads::{LocalWorkloadStore, WorkloadCatalog},
};
use rey_environment::{DiscoveryLimits, resolve_executable};
use rey_git::{GitInspector, GitLimits, GitRepositoryStatus};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const UI_SERVER_SCHEMA: &str = "rey.ui-server.v3";
const UI_HEALTH_SCHEMA: &str = "rey.ui-health.v1";
const UI_ERROR_SCHEMA: &str = "rey.ui-error.v1";
const UI_CADENCE_SCHEMA: &str = "rey.ui-cadence.v2";
const UI_JOURNAL_SCHEMA: &str = "rey.ui-journal.v3";
const MAX_REQUEST_TARGET_BYTES: usize = 4_096;
const LIVE_REFRESH_INTERVAL_MS: u64 = 5_000;
const CADENCE_GIT_COMMIT_LIMIT: usize = 24;
const CADENCE_ENVIRONMENT_COMMIT_LIMIT: usize = 24;
const HIFI_GRAMMAR_REVISION: &str = "git:058c6504fc10740360717e97e687fd77bef6a5c5";
const REY_SOURCE_REPOSITORY: &str = "https://github.com/spoke-sh/rey";
const REY_IMPLEMENTATION_REVISION: &str = env!("REY_BUILD_REVISION");

const INDEX_HTML: &[u8] = include_bytes!("../../../apps/rey-ui/dist/index.html");
const APP_JAVASCRIPT: &[u8] = include_bytes!("../../../apps/rey-ui/dist/assets/app.js");
const APP_CSS: &[u8] = include_bytes!("../../../apps/rey-ui/dist/assets/app.css");

#[derive(Clone, Debug)]
pub struct UiServerConfig {
    pub workspace: PathBuf,
    pub state_directory: PathBuf,
    pub catalog_directory: PathBuf,
    pub journal_directory: PathBuf,
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
    pub workspace: String,
    pub catalog_root: String,
    pub application: String,
    pub grammar: String,
    pub theme: String,
    pub grammar_revision: String,
    pub entry_route: String,
    pub live_refresh_interval_ms: u64,
    pub source_repository: String,
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
            workspace: config.workspace.display().to_string(),
            catalog_root: config.catalog_directory.display().to_string(),
            application: "tanstack_router".to_owned(),
            grammar: "kinetic".to_owned(),
            theme: "precision".to_owned(),
            grammar_revision: HIFI_GRAMMAR_REVISION.to_owned(),
            entry_route: "/explore".to_owned(),
            live_refresh_interval_ms: LIVE_REFRESH_INTERVAL_MS,
            source_repository: REY_SOURCE_REPOSITORY.to_owned(),
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

    pub fn serve(self) -> Result<(), UiError> {
        self.serve_bounded(None)
    }

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
        let path = request.url().split('?').next().unwrap_or("/");
        let head = request.method() == &Method::Head;
        if request.method() == &Method::Post && path == "/api/v1/journal" {
            return self.admit_journal(request);
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
            "/api/v1/cadence" => self.cadence(),
            "/api/v1/environment" => self.environment(),
            "/api/v1/journal" => self.journal(),
            "/api/v1/workloads" => self.workloads(),
            path if path.starts_with("/api/") => json_error(
                StatusCode(404),
                "api_route_not_found",
                "no read-only Rey UI API route matches this target",
            ),
            "/assets/app.js" => static_response(APP_JAVASCRIPT, "text/javascript; charset=utf-8"),
            "/assets/app.css" => static_response(APP_CSS, "text/css; charset=utf-8"),
            _ => index_response(),
        };
        if head {
            response.with_data(Cursor::new(Vec::new()), Some(0))
        } else {
            response
        }
    }

    fn health(&self) -> Response<Cursor<Vec<u8>>> {
        json_response(
            StatusCode(200),
            &json!({
                "schema": UI_HEALTH_SCHEMA,
                "status": "ready",
                "server": self.descriptor,
            }),
        )
    }

    fn workloads(&self) -> Response<Cursor<Vec<u8>>> {
        let result = (|| {
            let catalog = WorkloadCatalog::load_workspace(
                &self.config.workspace,
                &self.config.catalog_directory,
            )
            .map_err(|error| error.to_string())?;
            let store = LocalWorkloadStore::new(self.config.state_directory.clone());
            super::current_workload_list(&store, &self.config.workspace, &catalog)
                .map_err(|error| error.to_string())
        })();
        match result {
            Ok(list) => json_response(StatusCode(200), &list),
            Err(detail) => json_error(StatusCode(500), "portfolio_unavailable", &detail),
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
        let mut source_repository = None;
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
                        if sequence.head_oid.as_deref() == Some(REY_IMPLEMENTATION_REVISION) {
                            source_repository = Some(REY_SOURCE_REPOSITORY.to_owned());
                        }
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
    #[error("UI could not bind {address}: {detail}")]
    Bind { address: SocketAddr, detail: String },
    #[error("UI listener did not resolve to an IP socket")]
    NonIpListener,
    #[error("UI request receive failed: {0}")]
    Receive(std::io::Error),
    #[error("UI response failed: {0}")]
    Respond(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        thread,
    };

    use tempfile::TempDir;

    use super::{UiServer, UiServerConfig};

    #[test]
    fn server_admits_unauthenticated_journal_writes_and_serves_deep_links() {
        let workspace = TempDir::new().unwrap();
        let server = UiServer::bind(UiServerConfig {
            workspace: workspace.path().to_owned(),
            state_directory: workspace.path().join(".rey/workloads"),
            catalog_directory: "workloads".into(),
            journal_directory: workspace.path().join(".rey/journal"),
            host: "127.0.0.1".parse().unwrap(),
            port: 0,
        })
        .unwrap();
        let descriptor = server.descriptor();
        assert_eq!(descriptor.schema, "rey.ui-server.v3");
        assert!(descriptor.loopback_only);
        assert!(!descriptor.read_only);
        assert!(descriptor.journal_write_enabled);
        assert_eq!(descriptor.grammar, "kinetic");
        assert_eq!(descriptor.theme, "precision");
        assert_eq!(
            descriptor.source_repository,
            "https://github.com/spoke-sh/rey"
        );
        assert!(!descriptor.implementation_revision.is_empty());
        assert_eq!(
            descriptor.grammar_revision,
            "git:058c6504fc10740360717e97e687fd77bef6a5c5"
        );
        let address = descriptor.address.clone();
        let origin = descriptor.url.clone();
        let handle = thread::spawn(move || server.serve_bounded(Some(20)).unwrap());

        let health = request(&address, "GET /api/v1/health HTTP/1.1");
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains("\"schema\":\"rey.ui-health.v1\""));
        assert!(health.contains("\"loopback_only\":true"));

        let workloads = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(workloads.starts_with("HTTP/1.1 200"));
        assert!(workloads.contains("\"schema\":\"rey.workload-list.v8\""));

        let environment = request(&address, "GET /api/v1/environment HTTP/1.1");
        assert!(environment.starts_with("HTTP/1.1 200"));
        assert!(environment.contains("\"schema\":\"rey.environment-status.v5\""));
        assert!(
            environment
                .contains("\"operator\":{\"schema\":\"rey.environment-operator-projection.v3\"")
        );

        let cadence = request(&address, "GET /api/v1/cadence HTTP/1.1");
        assert!(cadence.starts_with("HTTP/1.1 200"));
        assert!(cadence.contains("\"schema\":\"rey.ui-cadence.v2\""));
        assert!(cadence.contains("\"ordering\":\"partial\""));
        assert!(cadence.contains("\"source_repository\":null"));
        assert!(cadence.contains("\"repository_state\":null"));
        assert!(cadence.contains("ui.portfolio.passive-revalidation"));
        assert!(cadence.contains("ui.cadence.passive-revalidation"));

        let journal = request(&address, "GET /api/v1/journal HTTP/1.1");
        assert!(journal.starts_with("HTTP/1.1 200"));
        assert!(journal.contains("\"schema\":\"rey.ui-journal.v3\""));
        assert!(journal.contains("\"write_enabled\":true"));
        assert!(journal.contains("\"authority\":\"unauthenticated_journal_admission\""));
        assert!(journal.contains("\"entries\":[]"));

        let proposal = serde_json::json!({
            "schema": "rey.journal-entry-proposal.v2",
            "title": "Bind the Journal",
            "author": { "kind": "human", "id": "operator" },
            "binding": {
                "coordinate": "rey+local://portfolio/current?revision=blake3%3Aportfolio",
                "scale": 0.68,
                "source_revision": "blake3:portfolio"
            },
            "blocks": [{
                "kind": "prose",
                "id": "context",
                "document": [{ "kind": "paragraph", "text": "A retained human entry." }]
            }]
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

        let application = request(&address, "GET /assets/app.js HTTP/1.1");
        assert!(application.starts_with("HTTP/1.1 200"));
        assert!(application.contains("text/javascript"));
        assert!(application.contains("01 / DIRECTED TEXT"));
        assert!(application.contains("02 / BOUNDED SEARCH"));
        assert!(application.contains("REFERENCE PLANE"));
        assert!(application.contains("Inputs and topology"));
        assert!(application.contains("RETAINED SEQUENCE"));
        assert!(application.contains("01 / JOURNAL"));
        assert!(application.contains("WRITE A JOURNAL ENTRY"));
        assert!(application.contains("HUMAN + AGENT · EXPLORE-BOUND"));
        assert!(application.contains("UNAUTHENTICATED · VALIDATED DOCUMENT ADMISSION"));
        assert!(application.contains("journal/$slug"));
        assert!(application.contains("JOURNAL / NEW"));
        assert!(application.contains("JOURNAL / ENTRY"));
        assert!(!application.contains("RECOMMENDATION BASIS"));
        assert!(application.contains("WORK LEDGER"));
        assert!(!application.contains("RETAINED RESULTS / NOT LIVE AGENT TELEMETRY"));
        assert!(application.contains("DESIRED INVENTORY"));
        assert!(application.contains("SEARCH RECORD"));
        assert!(application.contains("PROCESS SEEDS"));
        assert!(application.contains("HISTORY / RUNTIME ATTENTION"));
        assert!(application.contains("Mailbox history"));
        assert!(application.contains("REY / AGENT / OPERATOR"));
        assert!(application.contains("data-communication-backdrop"));
        assert!(application.contains("No Rey or agent session is connected"));
        assert!(application.contains("NO TRANSPORT · NO RETENTION · NO WRITE AUTHORITY"));
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
        assert!(application.contains("STREAM COORDINATE"));
        assert!(application.contains("ADD STREAM"));
        assert!(application.contains("APPLY LENS"));
        assert!(application.contains("Rename stream"));
        assert!(!application.contains("ALL LENS"));
        assert!(application.contains("Share an observation"));
        assert!(application.contains("INSPECT-ONLY"));
        assert!(application.contains("Display order is not causal order"));
        assert!(application.contains("data-kinetic-dense-table"));
        assert!(application.contains("Admitted workload revisions"));
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

    fn request(address: &str, request_line: &str) -> String {
        request_with_body(address, request_line, &[], "")
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
