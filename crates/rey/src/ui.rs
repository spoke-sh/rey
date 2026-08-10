use std::{
    env,
    io::Cursor,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use rey::{
    current_environment_status,
    env::LocalEnvironmentStore,
    workloads::{LocalWorkloadStore, WorkloadCatalog},
};
use rey_environment::{DiscoveryLimits, resolve_executable};
use rey_git::{GitInspector, GitLimits};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const UI_SERVER_SCHEMA: &str = "rey.ui-server.v1";
const UI_HEALTH_SCHEMA: &str = "rey.ui-health.v1";
const UI_ERROR_SCHEMA: &str = "rey.ui-error.v1";
const UI_CADENCE_SCHEMA: &str = "rey.ui-cadence.v1";
const MAX_REQUEST_TARGET_BYTES: usize = 4_096;
const LIVE_REFRESH_INTERVAL_MS: u64 = 5_000;
const CADENCE_GIT_COMMIT_LIMIT: usize = 24;
const CADENCE_ENVIRONMENT_COMMIT_LIMIT: usize = 24;
const HIFI_GRAMMAR_REVISION: &str = "git:0440cfe774405070facdb1106f3e247fa980060f";
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
    lanes: Vec<UiCadenceLane>,
    schedules: Vec<UiCadenceSchedule>,
    omissions: Vec<String>,
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
            read_only: true,
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
            let request = self.server.recv().map_err(UiError::Receive)?;
            let response = self.route(&request);
            request.respond(response).map_err(UiError::Respond)?;
            served = served.saturating_add(1);
        }
    }

    fn route(&self, request: &Request) -> Response<Cursor<Vec<u8>>> {
        if request.url().len() > MAX_REQUEST_TARGET_BYTES {
            return json_error(
                StatusCode(414),
                "request_target_limit",
                "request target exceeds the 4096-byte limit",
            );
        }
        let path = request.url().split('?').next().unwrap_or("/");
        let head = request.method() == &Method::Head;
        if request.method() != &Method::Get && !head {
            return with_header(
                json_error(
                    StatusCode(405),
                    "method_not_allowed",
                    "the Rey UI data plane is read-only; use GET or HEAD",
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
                match inspector.inspect_recent_commits(CADENCE_GIT_COMMIT_LIMIT) {
                    Ok(Some(sequence)) => {
                        if sequence.head_oid.as_deref() == Some(REY_IMPLEMENTATION_REVISION) {
                            source_repository = Some(REY_SOURCE_REPOSITORY.to_owned());
                        }
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
    fn server_is_loopback_read_only_and_serves_api_assets_and_spa_routes() {
        let workspace = TempDir::new().unwrap();
        let server = UiServer::bind(UiServerConfig {
            workspace: workspace.path().to_owned(),
            state_directory: workspace.path().join(".rey/workloads"),
            catalog_directory: "workloads".into(),
            host: "127.0.0.1".parse().unwrap(),
            port: 0,
        })
        .unwrap();
        let descriptor = server.descriptor();
        assert!(descriptor.loopback_only);
        assert!(descriptor.read_only);
        assert_eq!(descriptor.grammar, "kinetic");
        assert_eq!(descriptor.theme, "precision");
        assert_eq!(
            descriptor.source_repository,
            "https://github.com/spoke-sh/rey"
        );
        assert!(!descriptor.implementation_revision.is_empty());
        assert_eq!(
            descriptor.grammar_revision,
            "git:0440cfe774405070facdb1106f3e247fa980060f"
        );
        let address = descriptor.address.clone();
        let handle = thread::spawn(move || server.serve_bounded(Some(13)).unwrap());

        let health = request(&address, "GET /api/v1/health HTTP/1.1");
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains("\"schema\":\"rey.ui-health.v1\""));
        assert!(health.contains("\"loopback_only\":true"));

        let workloads = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(workloads.starts_with("HTTP/1.1 200"));
        assert!(workloads.contains("\"schema\":\"rey.workload-list.v5\""));

        let environment = request(&address, "GET /api/v1/environment HTTP/1.1");
        assert!(environment.starts_with("HTTP/1.1 200"));
        assert!(environment.contains("\"schema\":\"rey.environment-status.v4\""));
        assert!(
            environment
                .contains("\"operator\":{\"schema\":\"rey.environment-operator-projection.v2\"")
        );

        let cadence = request(&address, "GET /api/v1/cadence HTTP/1.1");
        assert!(cadence.starts_with("HTTP/1.1 200"));
        assert!(cadence.contains("\"schema\":\"rey.ui-cadence.v1\""));
        assert!(cadence.contains("\"ordering\":\"partial\""));
        assert!(cadence.contains("\"source_repository\":null"));
        assert!(cadence.contains("ui.portfolio.passive-revalidation"));
        assert!(cadence.contains("ui.cadence.passive-revalidation"));

        let application = request(&address, "GET /assets/app.js HTTP/1.1");
        assert!(application.starts_with("HTTP/1.1 200"));
        assert!(application.contains("text/javascript"));
        assert!(application.contains("01 / DIRECTED TEXT"));
        assert!(application.contains("02 / BOUNDED SEARCH"));
        assert!(application.contains("REFERENCE PLANE"));
        assert!(application.contains("Inputs and topology"));
        assert!(application.contains("RETAINED SEQUENCE"));
        assert!(application.contains("IDENTIFIED AGENTS"));
        assert!(application.contains("DESIRED INVENTORY"));
        assert!(application.contains("SEARCH RECORD"));
        assert!(application.contains("LOCATE IN EXPLORER"));
        assert!(application.contains("explore/$kind/$coordinate"));
        assert!(application.contains("NO CURRENT OBJECT SATISFIES THIS IDENTITY"));
        assert!(application.contains("--kinetic-control-press-x"));
        assert!(application.contains("--kinetic-light-highlight"));
        assert!(application.contains("--kinetic-shadow-soft-y"));
        assert!(!application.contains("WORKING TREE"));

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

        let environment = request(&address, "GET /environment HTTP/1.1");
        assert!(environment.starts_with("HTTP/1.1 200"));
        assert!(environment.contains("<title>Rey / Explore</title>"));

        let cadence = request(&address, "GET /cadence HTTP/1.1");
        assert!(cadence.starts_with("HTTP/1.1 200"));
        assert!(cadence.contains("<title>Rey / Explore</title>"));

        let agents = request(&address, "GET /agents HTTP/1.1");
        assert!(agents.starts_with("HTTP/1.1 200"));
        assert!(agents.contains("<title>Rey / Explore</title>"));

        let coordinate = request(
            &address,
            "GET /explore/agent/codex;at=gpt-5;lens=objects HTTP/1.1",
        );
        assert!(coordinate.starts_with("HTTP/1.1 200"));
        assert!(coordinate.contains("<title>Rey / Explore</title>"));

        let rejected = request(&address, "POST /api/v1/workloads HTTP/1.1");
        assert!(rejected.starts_with("HTTP/1.1 405"));
        assert!(rejected.contains("\"category\":\"method_not_allowed\""));
        handle.join().unwrap();
    }

    fn request(address: &str, request_line: &str) -> String {
        let mut stream = TcpStream::connect(address).unwrap();
        write!(
            stream,
            "{request_line}\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
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
