use std::{
    io::Cursor,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use rey::workloads::{LocalWorkloadStore, WorkloadCatalog};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const UI_SERVER_SCHEMA: &str = "rey.ui-server.v1";
const UI_HEALTH_SCHEMA: &str = "rey.ui-health.v1";
const UI_ERROR_SCHEMA: &str = "rey.ui-error.v1";
const MAX_REQUEST_TARGET_BYTES: usize = 4_096;
const LIVE_REFRESH_INTERVAL_MS: u64 = 5_000;
const HIFI_GRAMMAR_REVISION: &str = "git:5874cdfe0c237ddd35bb121824a166ebb5b5654e";

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
            descriptor.grammar_revision,
            "git:5874cdfe0c237ddd35bb121824a166ebb5b5654e"
        );
        let address = descriptor.address.clone();
        let handle = thread::spawn(move || server.serve_bounded(Some(8)).unwrap());

        let health = request(&address, "GET /api/v1/health HTTP/1.1");
        assert!(health.starts_with("HTTP/1.1 200"));
        assert!(health.contains("\"schema\":\"rey.ui-health.v1\""));
        assert!(health.contains("\"loopback_only\":true"));

        let workloads = request(&address, "GET /api/v1/workloads HTTP/1.1");
        assert!(workloads.starts_with("HTTP/1.1 200"));
        assert!(workloads.contains("\"schema\":\"rey.workload-list.v5\""));

        let application = request(&address, "GET /assets/app.js HTTP/1.1");
        assert!(application.starts_with("HTTP/1.1 200"));
        assert!(application.contains("text/javascript"));

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
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }
}
