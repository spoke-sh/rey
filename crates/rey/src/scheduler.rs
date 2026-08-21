use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command as ProcessCommand, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rey::channels::LocalChannelStore;
use rey_core::SemanticHasher;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;

use crate::ui::UiServerDescriptor;

pub const SCHEDULER_SNAPSHOT_SCHEMA: &str = "rey.scheduler-snapshot.v1";
pub const SCHEDULER_EVENT_SCHEMA: &str = "rey.scheduler-event.v1";
pub const SCHEDULER_CONTROL_SCHEMA: &str = "rey.scheduler-control.v1";
const OUTPUT_SCHEMA: &str = "rey.scheduler-output.v1";
const STATE_SCHEMA: &str = "rey.scheduler-state.v1";
const STATIC_INTERVAL_MS: u64 = 5_000;
const TICK_MS: u64 = 100;
const DISCOVERY_MS: u64 = 1_000;
const HTTP_TIMEOUT_MS: u64 = 3_000;
const MAX_HTTP_BYTES: usize = 16 * 1_024 * 1_024;
const MAX_RECEIPTS: usize = 128;
const GITHUB_PREFIX: &str = "provider.github-inbox/";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulerScheduleProjection {
    pub id: String,
    pub revision: u64,
    pub label: String,
    pub kind: String,
    pub source: String,
    pub topic: String,
    pub interval_ms: u64,
    pub enabled: bool,
    pub state: String,
    pub activation: String,
    pub authority: String,
    pub retention: String,
    pub next_due_unix: Option<i64>,
    pub last_attempt_unix: Option<i64>,
    pub last_success_unix: Option<i64>,
    pub last_revision: Option<String>,
    pub last_error: Option<String>,
    pub run_count: u64,
    pub published_events: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulerRunReceipt {
    pub sequence: u64,
    pub schedule_id: String,
    pub started_at_unix: i64,
    pub finished_at_unix: i64,
    pub outcome: String,
    pub source_revision: Option<String>,
    pub published_event: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulerSnapshot {
    pub schema: String,
    pub process_id: Option<u32>,
    pub sequence: u64,
    pub state: String,
    pub schedules: Vec<SchedulerScheduleProjection>,
    pub receipts: Vec<SchedulerRunReceipt>,
    pub complete: bool,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchedulerEvent {
    pub schema: String,
    pub sequence: u64,
    pub schedule_id: String,
    pub topic: String,
    pub source_revision: String,
    pub occurred_at_unix: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Output {
    Snapshot { schema: String, snapshot: SchedulerSnapshot },
    Event { schema: String, event: SchedulerEvent },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Control {
    SetEnabled {
        schema: String,
        schedule_id: String,
        expected_revision: u64,
        enabled: bool,
    },
    RunNow { schema: String, schedule_id: String },
    Shutdown { schema: String },
}

#[derive(Clone)]
pub struct SchedulerRuntime {
    inner: Arc<Mutex<RuntimeState>>,
    events: broadcast::Sender<SchedulerEvent>,
}

struct RuntimeState {
    snapshot: SchedulerSnapshot,
    control: Option<ChildStdin>,
}

impl SchedulerRuntime {
    #[must_use]
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(Mutex::new(RuntimeState {
                snapshot: starting_snapshot(),
                control: None,
            })),
            events,
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> SchedulerSnapshot {
        self.inner.lock().expect("scheduler runtime lock").snapshot.clone()
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<SchedulerEvent> {
        self.events.subscribe()
    }

    fn attach(&self, control: ChildStdin) {
        self.inner.lock().expect("scheduler runtime lock").control = Some(control);
    }

    fn ingest(&self, line: &str) -> Result<(), SchedulerError> {
        match serde_json::from_str::<Output>(line)? {
            Output::Snapshot { snapshot, .. } => {
                self.inner.lock().expect("scheduler runtime lock").snapshot = snapshot;
            }
            Output::Event { event, .. } => {
                let _ = self.events.send(event);
            }
        }
        Ok(())
    }

    pub fn set_enabled(&self, id: &str, expected: u64, enabled: bool) -> Result<(), SchedulerError> {
        let snapshot = self.snapshot();
        let schedule = snapshot.schedules.iter().find(|item| item.id == id)
            .ok_or_else(|| SchedulerError::UnknownSchedule(id.to_owned()))?;
        if schedule.revision != expected {
            return Err(SchedulerError::StaleSchedule {
                schedule_id: id.to_owned(), expected, actual: schedule.revision,
            });
        }
        self.send(&Control::SetEnabled {
            schema: SCHEDULER_CONTROL_SCHEMA.to_owned(),
            schedule_id: id.to_owned(),
            expected_revision: expected,
            enabled,
        })
    }

    pub fn run_now(&self, id: &str) -> Result<(), SchedulerError> {
        if !self.snapshot().schedules.iter().any(|item| item.id == id) {
            return Err(SchedulerError::UnknownSchedule(id.to_owned()));
        }
        self.send(&Control::RunNow {
            schema: SCHEDULER_CONTROL_SCHEMA.to_owned(), schedule_id: id.to_owned(),
        })
    }

    fn shutdown(&self) -> Result<(), SchedulerError> {
        self.send(&Control::Shutdown { schema: SCHEDULER_CONTROL_SCHEMA.to_owned() })
    }

    fn send(&self, command: &Control) -> Result<(), SchedulerError> {
        let mut state = self.inner.lock().expect("scheduler runtime lock");
        let control = state.control.as_mut().ok_or(SchedulerError::ControlUnavailable)?;
        serde_json::to_writer(&mut *control, command)?;
        control.write_all(b"\n").map_err(SchedulerError::Control)?;
        control.flush().map_err(SchedulerError::Control)
    }
}

pub struct ManagedScheduler {
    child: Child,
    bridge: thread::JoinHandle<Result<(), SchedulerError>>,
    runtime: SchedulerRuntime,
}

impl ManagedScheduler {
    pub fn spawn(operator: &UiServerDescriptor, runtime: SchedulerRuntime) -> Result<Self, SchedulerError> {
        let executable = std::env::current_exe().map_err(SchedulerError::Executable)?;
        let mut child = ProcessCommand::new(executable)
            .arg("scheduler")
            .arg("--workspace").arg(&operator.workspace)
            .arg("--channel-state-dir").arg(&operator.channel_root)
            .arg("--state-dir").arg(PathBuf::from(&operator.workspace).join(".rey/scheduler"))
            .arg("--operator-address").arg(&operator.address)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
            .spawn().map_err(SchedulerError::Spawn)?;
        runtime.attach(child.stdin.take().ok_or(SchedulerError::ControlUnavailable)?);
        let stdout = child.stdout.take().ok_or(SchedulerError::OutputUnavailable)?;
        let bridge_runtime = runtime.clone();
        let bridge = thread::Builder::new()
            .name("rey.scheduler-event-bridge".to_owned())
            .spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let line = line.map_err(SchedulerError::Output)?;
                    if !line.trim().is_empty() { bridge_runtime.ingest(&line)?; }
                }
                Ok(())
            }).map_err(SchedulerError::BridgeSpawn)?;
        Ok(Self { child, bridge, runtime })
    }

    pub fn is_finished(&mut self) -> Result<bool, SchedulerError> {
        self.child.try_wait().map(|status| status.is_some()).map_err(SchedulerError::Wait)
    }

    pub fn finish(mut self, cancelled: bool) -> Result<(), SchedulerError> {
        if cancelled { let _ = self.runtime.shutdown(); }
        let deadline = Instant::now() + Duration::from_secs(3);
        let status = loop {
            if let Some(status) = self.child.try_wait().map_err(SchedulerError::Wait)? { break status; }
            if Instant::now() >= deadline {
                self.child.kill().map_err(SchedulerError::Kill)?;
                break self.child.wait().map_err(SchedulerError::Wait)?;
            }
            thread::sleep(Duration::from_millis(25));
        };
        match self.bridge.join() {
            Ok(result) => result?,
            Err(_) => return Err(SchedulerError::BridgePanicked),
        }
        if cancelled { Ok(()) } else { Err(SchedulerError::UnexpectedExit(status.to_string())) }
    }
}

#[derive(Clone, Debug)]
pub struct SchedulerConfig {
    pub workspace: PathBuf,
    pub channel_directory: PathBuf,
    pub state_directory: PathBuf,
    pub operator_address: String,
    pub rey_executable: PathBuf,
}

#[derive(Clone)]
enum Kind { Http, GitHub(String) }

struct Schedule { projection: SchedulerScheduleProjection, kind: Kind, next_due: Instant }

#[derive(Clone, Deserialize, Serialize)]
struct RetainedControl { enabled: bool, revision: u64 }

#[derive(Deserialize, Serialize)]
struct RetainedState {
    schema: String,
    controls: BTreeMap<String, RetainedControl>,
    receipts: Vec<SchedulerRunReceipt>,
    next_receipt_sequence: u64,
}

impl Default for RetainedState {
    fn default() -> Self {
        Self { schema: STATE_SCHEMA.to_owned(), controls: BTreeMap::new(), receipts: Vec::new(), next_receipt_sequence: 1 }
    }
}

struct Engine {
    config: SchedulerConfig,
    schedules: BTreeMap<String, Schedule>,
    retained: RetainedState,
    sequence: u64,
    last_discovery: Instant,
}

pub fn run_scheduler(config: SchedulerConfig) -> Result<(), SchedulerError> {
    let (send, receive) = mpsc::channel();
    thread::Builder::new().name("rey.scheduler-control".to_owned()).spawn(move || {
        for line in io::stdin().lock().lines() {
            let command = line.map_err(|error| error.to_string())
                .and_then(|line| serde_json::from_str::<Control>(&line).map_err(|error| error.to_string()));
            let _ = send.send(command);
        }
        let _ = send.send(Ok(Control::Shutdown { schema: SCHEDULER_CONTROL_SCHEMA.to_owned() }));
    }).map_err(SchedulerError::BridgeSpawn)?;

    let mut engine = Engine::new(config)?;
    let stdout = io::stdout();
    let mut output = stdout.lock();
    engine.refresh_github()?;
    engine.snapshot(&mut output)?;
    loop {
        while let Ok(command) = receive.try_recv() {
            if engine.control(command.map_err(SchedulerError::ControlProtocol)?, &mut output)? { return Ok(()); }
        }
        if engine.last_discovery.elapsed() >= Duration::from_millis(DISCOVERY_MS) {
            engine.refresh_github()?;
            engine.last_discovery = Instant::now();
        }
        let due = engine.schedules.iter()
            .filter(|(_, schedule)| schedule.projection.enabled && schedule.next_due <= Instant::now())
            .map(|(id, _)| id.clone()).collect::<Vec<_>>();
        for id in due { engine.run(&id, &mut output)?; }
        thread::sleep(Duration::from_millis(TICK_MS));
    }
}

impl Engine {
    fn new(config: SchedulerConfig) -> Result<Self, SchedulerError> {
        let retained = load_state(&config.state_directory)?;
        let mut schedules = BTreeMap::new();
        for (id, label, source, topic) in [
            ("runtime.portfolio", "Portfolio change scan", "/api/v1/revalidation", "portfolio"),
            ("runtime.environment", "Environment scan", "/api/v1/environment", "environment"),
            ("runtime.channels", "Channel state scan", "/api/v1/channels", "channels"),
            ("runtime.observations", "Observation frontier scan", "/api/v1/observations", "observations"),
            ("runtime.cadence", "Git and cadence scan", "/api/v1/cadence", "cadence"),
        ] {
            schedules.insert(id.to_owned(), schedule(id, label, "projection_scanner", source, topic, STATIC_INTERVAL_MS, Kind::Http, retained.controls.get(id)));
        }
        Ok(Self {
            config, schedules, retained, sequence: 0,
            last_discovery: Instant::now().checked_sub(Duration::from_millis(DISCOVERY_MS)).unwrap_or_else(Instant::now),
        })
    }

    fn control(&mut self, command: Control, output: &mut impl Write) -> Result<bool, SchedulerError> {
        match command {
            Control::Shutdown { schema } => { validate_schema(&schema)?; Ok(true) }
            Control::RunNow { schema, schedule_id } => {
                validate_schema(&schema)?;
                let item = self.schedules.get_mut(&schedule_id).ok_or_else(|| SchedulerError::UnknownSchedule(schedule_id.clone()))?;
                if !item.projection.enabled { return Err(SchedulerError::DisabledSchedule(schedule_id)); }
                item.next_due = Instant::now();
                item.projection.next_due_unix = Some(now());
                self.snapshot(output)?;
                Ok(false)
            }
            Control::SetEnabled { schema, schedule_id, expected_revision, enabled } => {
                validate_schema(&schema)?;
                let (revision, topic) = {
                    let item = self.schedules.get_mut(&schedule_id).ok_or_else(|| SchedulerError::UnknownSchedule(schedule_id.clone()))?;
                    if item.projection.revision != expected_revision {
                        return Err(SchedulerError::StaleSchedule { schedule_id, expected: expected_revision, actual: item.projection.revision });
                    }
                    item.projection.revision += 1;
                    item.projection.enabled = enabled;
                    item.projection.state = if enabled { "scheduled" } else { "disabled" }.to_owned();
                    item.projection.next_due_unix = enabled.then_some(now());
                    if enabled { item.next_due = Instant::now(); }
                    (item.projection.revision, item.projection.topic.clone())
                };
                self.retained.controls.insert(schedule_id.clone(), RetainedControl { enabled, revision });
                save_state(&self.config.state_directory, &self.retained)?;
                self.event(&schedule_id, &topic, &format!("control:{revision}"), output)?;
                self.snapshot(output)?;
                Ok(false)
            }
        }
    }

    fn refresh_github(&mut self) -> Result<(), SchedulerError> {
        let store = LocalChannelStore::new(self.config.channel_directory.clone());
        let status = store.status()?;
        let mut admitted = BTreeSet::new();
        if let Some(head) = status.head_commit {
            for application in head.snapshot.graph.applications {
                let Some(inbox) = application.github_inbox else { continue };
                let id = format!("{GITHUB_PREFIX}{}", application.id);
                admitted.insert(id.clone());
                if let Some(item) = self.schedules.get_mut(&id) {
                    item.projection.interval_ms = inbox.poll_interval_seconds * 1_000;
                    continue;
                }
                self.schedules.insert(id.clone(), schedule(
                    &id, &format!("GitHub inbox · {}", application.id), "provider_scanner",
                    &format!("rey channels poll {}", application.id), "channels",
                    inbox.poll_interval_seconds * 1_000, Kind::GitHub(application.id), self.retained.controls.get(&id),
                ));
            }
        }
        self.schedules.retain(|id, _| !id.starts_with(GITHUB_PREFIX) || admitted.contains(id));
        Ok(())
    }

    fn run(&mut self, id: &str, output: &mut impl Write) -> Result<(), SchedulerError> {
        let started = now();
        let (kind, source, previous, interval, topic) = {
            let item = self.schedules.get(id).ok_or_else(|| SchedulerError::UnknownSchedule(id.to_owned()))?;
            (item.kind.clone(), item.projection.source.clone(), item.projection.last_revision.clone(), item.projection.interval_ms, item.projection.topic.clone())
        };
        let result = match kind { Kind::Http => self.http(&source), Kind::GitHub(app) => self.github(&app) };
        let finished = now();
        let mut changed_revision = None;
        let (outcome, source_revision) = match result {
            Ok(revision) => {
                let changed = previous.as_ref().is_some_and(|value| value != &revision);
                let item = self.schedules.get_mut(id).expect("schedule remains registered");
                item.projection.state = "scheduled".to_owned();
                item.projection.last_attempt_unix = Some(started);
                item.projection.last_success_unix = Some(finished);
                item.projection.last_revision = Some(revision.clone());
                item.projection.last_error = None;
                item.projection.run_count += 1;
                if changed { item.projection.published_events += 1; changed_revision = Some(revision.clone()); }
                ("succeeded".to_owned(), Some(revision))
            }
            Err(error) => {
                let item = self.schedules.get_mut(id).expect("schedule remains registered");
                item.projection.state = "degraded".to_owned();
                item.projection.last_attempt_unix = Some(started);
                item.projection.last_error = Some(error.to_string().chars().take(512).collect());
                item.projection.run_count += 1;
                ("failed".to_owned(), None)
            }
        };
        let item = self.schedules.get_mut(id).expect("schedule remains registered");
        item.next_due = Instant::now() + Duration::from_millis(interval);
        item.projection.next_due_unix = Some(finished.saturating_add((interval.saturating_add(999) / 1_000) as i64));
        let published_event = changed_revision.is_some();
        self.retain(SchedulerRunReceipt {
            sequence: self.retained.next_receipt_sequence,
            schedule_id: id.to_owned(), started_at_unix: started, finished_at_unix: finished,
            outcome, source_revision, published_event,
        })?;
        if let Some(revision) = changed_revision { self.event(id, &topic, &revision, output)?; }
        self.snapshot(output)
    }

    fn http(&self, source: &str) -> Result<String, SchedulerError> {
        let mut stream = TcpStream::connect(&self.config.operator_address).map_err(SchedulerError::HttpConnect)?;
        let timeout = Some(Duration::from_millis(HTTP_TIMEOUT_MS));
        stream.set_read_timeout(timeout).map_err(SchedulerError::HttpConnect)?;
        stream.set_write_timeout(timeout).map_err(SchedulerError::HttpConnect)?;
        write!(stream, "GET {source} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n", self.config.operator_address).map_err(SchedulerError::HttpWrite)?;
        let mut response = Vec::new();
        stream.take((MAX_HTTP_BYTES + 1) as u64).read_to_end(&mut response).map_err(SchedulerError::HttpRead)?;
        if response.len() > MAX_HTTP_BYTES { return Err(SchedulerError::HttpLimit(MAX_HTTP_BYTES)); }
        let header = response.windows(4).position(|window| window == b"\r\n\r\n").ok_or(SchedulerError::HttpMalformed)?;
        if !response.starts_with(b"HTTP/1.1 200") && !response.starts_with(b"HTTP/1.0 200") {
            return Err(SchedulerError::HttpStatus(String::from_utf8_lossy(&response[..header]).lines().next().unwrap_or("unknown").to_owned()));
        }
        let mut value: serde_json::Value = serde_json::from_slice(&response[header + 4..])?;
        if source == "/api/v1/cadence" { if let Some(object) = value.as_object_mut() { object.remove("schedules"); } }
        digest(source, &serde_json::to_vec(&value)?)
    }

    fn github(&self, application: &str) -> Result<String, SchedulerError> {
        let result = ProcessCommand::new(&self.config.rey_executable)
            .arg("channels").arg("--workspace").arg(&self.config.workspace)
            .arg("--state-dir").arg(&self.config.channel_directory)
            .arg("poll").arg(application).arg("--format").arg("json")
            .stdout(Stdio::null()).stderr(Stdio::piped()).output().map_err(SchedulerError::ProviderSpawn)?;
        if !result.status.success() && result.status.code() != Some(3) {
            return Err(SchedulerError::ProviderFailed {
                application_id: application.to_owned(), status: result.status.to_string(),
                detail: String::from_utf8_lossy(&result.stderr).trim().chars().take(512).collect(),
            });
        }
        let store = LocalChannelStore::new(self.config.channel_directory.clone());
        let mailbox = store.mailbox(&store.status()?)?;
        digest("provider.github-inbox", &serde_json::to_vec(&mailbox.messages)?)
    }

    fn retain(&mut self, receipt: SchedulerRunReceipt) -> Result<(), SchedulerError> {
        self.retained.next_receipt_sequence += 1;
        self.retained.receipts.push(receipt);
        if self.retained.receipts.len() > MAX_RECEIPTS {
            let remove = self.retained.receipts.len() - MAX_RECEIPTS;
            self.retained.receipts.drain(..remove);
        }
        save_state(&self.config.state_directory, &self.retained)
    }

    fn event(&mut self, id: &str, topic: &str, revision: &str, output: &mut impl Write) -> Result<(), SchedulerError> {
        self.sequence += 1;
        write_line(output, &Output::Event {
            schema: OUTPUT_SCHEMA.to_owned(),
            event: SchedulerEvent {
                schema: SCHEDULER_EVENT_SCHEMA.to_owned(), sequence: self.sequence,
                schedule_id: id.to_owned(), topic: topic.to_owned(), source_revision: revision.to_owned(), occurred_at_unix: now(),
            },
        })
    }

    fn snapshot(&self, output: &mut impl Write) -> Result<(), SchedulerError> {
        write_line(output, &Output::Snapshot {
            schema: OUTPUT_SCHEMA.to_owned(),
            snapshot: SchedulerSnapshot {
                schema: SCHEDULER_SNAPSHOT_SCHEMA.to_owned(), process_id: Some(std::process::id()), sequence: self.sequence,
                state: "running".to_owned(), schedules: self.schedules.values().map(|item| item.projection.clone()).collect(),
                receipts: self.retained.receipts.clone(), complete: true,
                omissions: vec![
                    "event delivery is process-local and reconnect requires a full projection resync".to_owned(),
                    "run receipts retain only the newest 128 scheduler attempts".to_owned(),
                ],
            },
        })
    }
}

fn schedule(id: &str, label: &str, kind: &str, source: &str, topic: &str, interval: u64, runtime_kind: Kind, retained: Option<&RetainedControl>) -> Schedule {
    let enabled = retained.map_or(true, |control| control.enabled);
    Schedule {
        projection: SchedulerScheduleProjection {
            id: id.to_owned(), revision: retained.map_or(1, |control| control.revision.max(1)), label: label.to_owned(), kind: kind.to_owned(),
            source: source.to_owned(), topic: topic.to_owned(), interval_ms: interval, enabled,
            state: if enabled { "scheduled" } else { "disabled" }.to_owned(), activation: "rey_agent_started".to_owned(),
            authority: "scheduler_process_bounded_read_and_event_publication".to_owned(), retention: "newest_128_run_receipts_local".to_owned(),
            next_due_unix: enabled.then_some(now()), last_attempt_unix: None, last_success_unix: None, last_revision: None, last_error: None,
            run_count: 0, published_events: 0,
        },
        kind: runtime_kind, next_due: Instant::now(),
    }
}

fn starting_snapshot() -> SchedulerSnapshot {
    SchedulerSnapshot {
        schema: SCHEDULER_SNAPSHOT_SCHEMA.to_owned(), process_id: None, sequence: 0, state: "starting".to_owned(),
        schedules: Vec::new(), receipts: Vec::new(), complete: false,
        omissions: vec!["scheduler child process has not published its first snapshot".to_owned()],
    }
}

fn validate_schema(schema: &str) -> Result<(), SchedulerError> {
    if schema == SCHEDULER_CONTROL_SCHEMA { Ok(()) } else { Err(SchedulerError::ControlSchema(schema.to_owned())) }
}

fn digest(domain: &str, bytes: &[u8]) -> Result<String, SchedulerError> {
    let mut hasher = SemanticHasher::new(&format!("rey.scheduler.scan.{domain}.v1"));
    hasher.add_bytes(bytes);
    Ok(hasher.finish().to_string())
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn load_state(directory: &Path) -> Result<RetainedState, SchedulerError> {
    reject_symlink(directory)?;
    fs::create_dir_all(directory).map_err(SchedulerError::StateIo)?;
    let path = directory.join("state.json");
    reject_symlink(&path)?;
    if !path.exists() { return Ok(RetainedState::default()); }
    let state: RetainedState = serde_json::from_slice(&fs::read(path).map_err(SchedulerError::StateIo)?)?;
    if state.schema != STATE_SCHEMA { return Err(SchedulerError::StateSchema(state.schema)); }
    Ok(state)
}

fn save_state(directory: &Path, state: &RetainedState) -> Result<(), SchedulerError> {
    reject_symlink(directory)?;
    fs::create_dir_all(directory).map_err(SchedulerError::StateIo)?;
    let path = directory.join("state.json");
    reject_symlink(&path)?;
    let temporary = directory.join(format!(".state.{}.tmp", std::process::id()));
    fs::write(&temporary, serde_json::to_vec_pretty(state)?).map_err(SchedulerError::StateIo)?;
    fs::rename(temporary, path).map_err(SchedulerError::StateIo)
}

fn reject_symlink(path: &Path) -> Result<(), SchedulerError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(SchedulerError::StateSymlink(path.to_owned())),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(SchedulerError::StateIo(error)),
    }
}

fn write_line(output: &mut impl Write, message: &Output) -> Result<(), SchedulerError> {
    serde_json::to_writer(&mut *output, message)?;
    output.write_all(b"\n").map_err(SchedulerError::Output)?;
    output.flush().map_err(SchedulerError::Output)
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("scheduler executable could not be resolved: {0}")] Executable(io::Error),
    #[error("scheduler child process could not be spawned: {0}")] Spawn(io::Error),
    #[error("scheduler event bridge could not be spawned: {0}")] BridgeSpawn(io::Error),
    #[error("scheduler event bridge panicked")] BridgePanicked,
    #[error("scheduler control channel is unavailable")] ControlUnavailable,
    #[error("scheduler child stdout is unavailable")] OutputUnavailable,
    #[error("scheduler control write failed: {0}")] Control(io::Error),
    #[error("scheduler control protocol is invalid: {0}")] ControlProtocol(String),
    #[error("scheduler output read failed: {0}")] Output(io::Error),
    #[error("scheduler child wait failed: {0}")] Wait(io::Error),
    #[error("scheduler child termination failed: {0}")] Kill(io::Error),
    #[error("scheduler child exited unexpectedly with {0}")] UnexpectedExit(String),
    #[error("scheduler protocol JSON is invalid: {0}")] Json(#[from] serde_json::Error),
    #[error("scheduler control schema is unsupported: {0}")] ControlSchema(String),
    #[error("unknown scheduler entry {0}")] UnknownSchedule(String),
    #[error("scheduler entry {0} is disabled")] DisabledSchedule(String),
    #[error("scheduler entry {schedule_id} revision changed: expected {expected}, actual {actual}")]
    StaleSchedule { schedule_id: String, expected: u64, actual: u64 },
    #[error("scheduler state path is a symlink: {0}")] StateSymlink(PathBuf),
    #[error("scheduler state I/O failed: {0}")] StateIo(io::Error),
    #[error("scheduler state schema is unsupported: {0}")] StateSchema(String),
    #[error("scheduler HTTP connection failed: {0}")] HttpConnect(io::Error),
    #[error("scheduler HTTP request failed: {0}")] HttpWrite(io::Error),
    #[error("scheduler HTTP response failed: {0}")] HttpRead(io::Error),
    #[error("scheduler HTTP response exceeds {0} bytes")] HttpLimit(usize),
    #[error("scheduler HTTP response is malformed")] HttpMalformed,
    #[error("scheduler HTTP source returned {0}")] HttpStatus(String),
    #[error("scheduler provider command could not start: {0}")] ProviderSpawn(io::Error),
    #[error("GitHub schedule for {application_id} failed with {status}: {detail}")]
    ProviderFailed { application_id: String, status: String, detail: String },
    #[error(transparent)] Channel(#[from] rey::channels::ChannelGraphError),
}
