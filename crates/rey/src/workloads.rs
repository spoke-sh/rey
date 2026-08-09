use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rey_core::ContractIdentity;
use rey_runtime::{
    QualificationRecord, RunStatus, TestStatus, WorkloadDefinition, WorkloadRunResult,
    WorkloadTestResult,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const LOCAL_WORKLOAD_STATE_SCHEMA: &str = "rey.local-workload-state.v2";
pub const WORKLOAD_LIST_SCHEMA: &str = "rey.workload-list.v2";
pub const WORKLOAD_STATUS_SCHEMA: &str = "rey.workload-status.v2";
pub const WORKLOAD_STATUS_BATCH_SCHEMA: &str = "rey.workload-status-batch.v2";
pub const WORKLOAD_TEST_BATCH_SCHEMA: &str = "rey.workload-test-batch.v2";

const STATE_FILE_NAME: &str = "state.json";
const MAX_STATE_BYTES: u64 = 4 * 1_024 * 1_024;
const MAX_STATE_RECORDS: usize = 64;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalWorkloadRecord {
    pub last_test: Option<WorkloadTestResult>,
    pub last_run: Option<WorkloadRunResult>,
}

impl LocalWorkloadRecord {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            last_test: None,
            last_run: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalWorkloadState {
    pub schema: String,
    pub records: BTreeMap<String, LocalWorkloadRecord>,
}

impl Default for LocalWorkloadState {
    fn default() -> Self {
        Self {
            schema: LOCAL_WORKLOAD_STATE_SCHEMA.to_owned(),
            records: BTreeMap::new(),
        }
    }
}

impl LocalWorkloadState {
    pub fn verify(&self) -> Result<(), LocalWorkloadStateError> {
        if self.schema != LOCAL_WORKLOAD_STATE_SCHEMA {
            return Err(LocalWorkloadStateError::UnsupportedSchema {
                actual: self.schema.clone(),
            });
        }
        if self.records.len() > MAX_STATE_RECORDS {
            return Err(LocalWorkloadStateError::RecordLimit {
                limit: MAX_STATE_RECORDS,
            });
        }
        for (workload_id, record) in &self.records {
            if workload_id.is_empty() {
                return Err(LocalWorkloadStateError::EmptyWorkloadId);
            }
            if record.last_test.is_none() && record.last_run.is_none() {
                return Err(LocalWorkloadStateError::EmptyRecord(workload_id.clone()));
            }
            if let Some(result) = &record.last_test {
                result.verify()?;
                if result.workload.id != *workload_id {
                    return Err(LocalWorkloadStateError::RecordIdentity {
                        key: workload_id.clone(),
                        artifact: result.workload.id.clone(),
                    });
                }
            }
            if let Some(result) = &record.last_run {
                result.verify()?;
                if result.workload.id != *workload_id {
                    return Err(LocalWorkloadStateError::RecordIdentity {
                        key: workload_id.clone(),
                        artifact: result.workload.id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn record(&self, workload_id: &str) -> Option<&LocalWorkloadRecord> {
        self.records.get(workload_id)
    }

    pub fn retain_test(&mut self, result: WorkloadTestResult) {
        let workload_id = result.workload.id.clone();
        self.records
            .entry(workload_id)
            .or_insert_with(LocalWorkloadRecord::empty)
            .last_test = Some(result);
    }

    pub fn retain_run(&mut self, result: WorkloadRunResult) {
        let workload_id = result.workload.id.clone();
        self.records
            .entry(workload_id)
            .or_insert_with(LocalWorkloadRecord::empty)
            .last_run = Some(result);
    }
}

#[derive(Clone, Debug)]
pub struct LocalWorkloadStore {
    directory: PathBuf,
}

impl LocalWorkloadStore {
    #[must_use]
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    #[must_use]
    pub fn default_for_workspace(workspace: &Path) -> Self {
        Self::new(workspace.join(".rey").join("workloads"))
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.directory.join(STATE_FILE_NAME)
    }

    pub fn load(&self) -> Result<LocalWorkloadState, LocalWorkloadStateError> {
        let path = self.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LocalWorkloadState::default());
            }
            Err(source) => return Err(LocalWorkloadStateError::Read { path, source }),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LocalWorkloadStateError::UnsafeStatePath(path));
        }
        if metadata.len() > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path,
                limit: MAX_STATE_BYTES,
            });
        }
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|source| LocalWorkloadStateError::Read {
                path: path.clone(),
                source,
            })?
            .take(MAX_STATE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| LocalWorkloadStateError::Read {
                path: path.clone(),
                source,
            })?;
        if bytes.len() as u64 > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path,
                limit: MAX_STATE_BYTES,
            });
        }
        let state: LocalWorkloadState =
            serde_json::from_slice(&bytes).map_err(|source| LocalWorkloadStateError::Json {
                path: path.clone(),
                source,
            })?;
        state.verify()?;
        Ok(state)
    }

    pub fn save(&self, state: &LocalWorkloadState) -> Result<(), LocalWorkloadStateError> {
        state.verify()?;
        let bytes =
            serde_json::to_vec_pretty(state).map_err(|source| LocalWorkloadStateError::Json {
                path: self.path(),
                source,
            })?;
        if bytes.len().saturating_add(1) as u64 > MAX_STATE_BYTES {
            return Err(LocalWorkloadStateError::ByteLimit {
                path: self.path(),
                limit: MAX_STATE_BYTES,
            });
        }
        self.prepare_directory()?;
        let target = self.path();
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(LocalWorkloadStateError::UnsafeStatePath(target));
        }
        let (temporary, mut file) = self.create_temporary()?;
        let publication = (|| {
            file.write_all(&bytes)
                .and_then(|()| file.write_all(b"\n"))
                .and_then(|()| file.flush())?;
            drop(file);
            fs::rename(&temporary, &target)
        })();
        if let Err(source) = publication {
            let _ = fs::remove_file(&temporary);
            return Err(LocalWorkloadStateError::Write {
                path: target,
                source,
            });
        }
        Ok(())
    }

    fn prepare_directory(&self) -> Result<(), LocalWorkloadStateError> {
        match fs::symlink_metadata(&self.directory) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
                LocalWorkloadStateError::UnsafeStatePath(self.directory.clone()),
            ),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir_all(&self.directory).map_err(|source| {
                    LocalWorkloadStateError::Write {
                        path: self.directory.clone(),
                        source,
                    }
                })
            }
            Err(source) => Err(LocalWorkloadStateError::Write {
                path: self.directory.clone(),
                source,
            }),
        }
    }

    fn create_temporary(&self) -> Result<(PathBuf, File), LocalWorkloadStateError> {
        for attempt in 0..32_u8 {
            let path = self
                .directory
                .join(format!(".state.json.tmp-{}-{attempt}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok((path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(LocalWorkloadStateError::Write { path, source }),
            }
        }
        Err(LocalWorkloadStateError::TemporaryLimit(
            self.directory.clone(),
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadFreshness {
    Untested,
    Fresh,
    Stale,
}

impl WorkloadFreshness {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Fresh => "fresh",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationState {
    Untested,
    Qualified,
    Failing,
    Inconclusive,
    Stale,
}

impl QualificationState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Qualified => "qualified",
            Self::Failing => "failing",
            Self::Inconclusive => "inconclusive",
            Self::Stale => "stale",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadSummary {
    pub workload: ContractIdentity,
    pub title: String,
    pub candidate_graph: ContractIdentity,
    pub scenario_suite: ContractIdentity,
    pub evaluator: ContractIdentity,
    pub freshness: WorkloadFreshness,
    pub qualification: QualificationState,
    pub qualified_graph: Option<ContractIdentity>,
    pub required: u64,
    pub passed: u64,
    pub failed: u64,
    pub inconclusive: u64,
    pub evaluated: u64,
    pub stale: u64,
    pub optional: u64,
    pub last_test_result_id: Option<rey_core::SemanticDigest>,
    pub last_run_status: Option<RunStatus>,
    pub operations: Vec<ContractIdentity>,
    pub mining_operations: u64,
    pub mining_results: u64,
    pub complete_mining_results: u64,
    pub incomplete_mining_results: u64,
    pub relation_deltas: u64,
    pub reasoning_surfaces: u64,
}

impl WorkloadSummary {
    #[must_use]
    pub fn derive(workload: &WorkloadDefinition, record: Option<&LocalWorkloadRecord>) -> Self {
        let required = workload.required_scenario_count();
        let optional = workload.scenario_suite.scenarios.len() as u64 - required;
        let retained_test = record.and_then(|record| record.last_test.as_ref());
        let (freshness, qualification, passed, failed, inconclusive, evaluated, stale) =
            match retained_test {
                None => (
                    WorkloadFreshness::Untested,
                    QualificationState::Untested,
                    0,
                    0,
                    0,
                    0,
                    0,
                ),
                Some(result) if result.verify_for(workload).is_err() => (
                    WorkloadFreshness::Stale,
                    QualificationState::Stale,
                    0,
                    0,
                    0,
                    0,
                    required,
                ),
                Some(result) => (
                    WorkloadFreshness::Fresh,
                    match result.status {
                        TestStatus::Passed => QualificationState::Qualified,
                        TestStatus::Failed => QualificationState::Failing,
                        TestStatus::Inconclusive => QualificationState::Inconclusive,
                    },
                    result.summary.passed,
                    result.summary.failed,
                    result.summary.inconclusive,
                    result.summary.evaluated,
                    0,
                ),
            };
        let qualified_graph = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .and_then(|result| result.qualification.as_ref())
            .map(|qualification| qualification.graph.clone());
        let mut operations = Vec::new();
        for node in &workload.graph.nodes {
            if !operations.contains(&node.operation) {
                operations.push(node.operation.clone());
            }
        }
        let mining_operations = operations
            .iter()
            .filter(|operation| operation.id.starts_with("rey.source-"))
            .count() as u64;
        let mining = retained_test
            .filter(|result| result.verify_for(workload).is_ok())
            .into_iter()
            .flat_map(|result| &result.scenarios)
            .flat_map(|scenario| &scenario.mining)
            .collect::<Vec<_>>();
        let mining_results = mining.len() as u64;
        let complete_mining_results = mining
            .iter()
            .filter(|evidence| {
                evidence.execution.evidence.result.completeness
                    == rey_mining::MiningCompleteness::Complete
            })
            .count() as u64;
        let incomplete_mining_results = mining_results.saturating_sub(complete_mining_results);
        let reasoning_surfaces = mining
            .iter()
            .filter(|evidence| evidence.reasoning.is_some())
            .count() as u64;
        Self {
            workload: workload.workload.clone(),
            title: workload.title.clone(),
            candidate_graph: workload.graph.graph.clone(),
            scenario_suite: workload.scenario_suite.suite.clone(),
            evaluator: workload.evaluator.clone(),
            freshness,
            qualification,
            qualified_graph,
            required,
            passed,
            failed,
            inconclusive,
            evaluated,
            stale,
            optional,
            last_test_result_id: retained_test.map(|result| result.result_id.clone()),
            last_run_status: record
                .and_then(|record| record.last_run.as_ref().map(|result| result.status)),
            operations,
            mining_operations,
            mining_results,
            complete_mining_results,
            incomplete_mining_results,
            relation_deltas: mining_results,
            reasoning_surfaces,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadList {
    pub schema: String,
    pub workloads: Vec<WorkloadSummary>,
}

impl WorkloadList {
    #[must_use]
    pub fn new(workloads: Vec<WorkloadSummary>) -> Self {
        Self {
            schema: WORKLOAD_LIST_SCHEMA.to_owned(),
            workloads,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadStatusView {
    pub schema: String,
    pub summary: WorkloadSummary,
    pub definition: WorkloadDefinition,
    pub last_test: Option<WorkloadTestResult>,
    pub last_run: Option<WorkloadRunResult>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadStatusBatch {
    pub schema: String,
    pub statuses: Vec<WorkloadStatusView>,
}

impl WorkloadStatusBatch {
    #[must_use]
    pub fn new(statuses: Vec<WorkloadStatusView>) -> Self {
        Self {
            schema: WORKLOAD_STATUS_BATCH_SCHEMA.to_owned(),
            statuses,
        }
    }
}

impl WorkloadStatusView {
    #[must_use]
    pub fn new(workload: WorkloadDefinition, record: Option<&LocalWorkloadRecord>) -> Self {
        Self {
            schema: WORKLOAD_STATUS_SCHEMA.to_owned(),
            summary: WorkloadSummary::derive(&workload, record),
            definition: workload,
            last_test: record.and_then(|record| record.last_test.clone()),
            last_run: record.and_then(|record| record.last_run.clone()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkloadTestBatch {
    pub schema: String,
    pub results: Vec<WorkloadTestResult>,
}

impl WorkloadTestBatch {
    #[must_use]
    pub fn new(results: Vec<WorkloadTestResult>) -> Self {
        Self {
            schema: WORKLOAD_TEST_BATCH_SCHEMA.to_owned(),
            results,
        }
    }
}

#[must_use]
pub fn fresh_qualification<'a>(
    workload: &WorkloadDefinition,
    record: Option<&'a LocalWorkloadRecord>,
) -> Option<&'a QualificationRecord> {
    record
        .and_then(|record| record.last_test.as_ref())
        .filter(|result| result.verify_for(workload).is_ok())
        .and_then(|result| result.qualification.as_ref())
        .filter(|qualification| qualification.is_fresh_for(workload))
}

#[derive(Debug, Error)]
pub enum LocalWorkloadStateError {
    #[error("unsupported local workload state schema {actual}")]
    UnsupportedSchema { actual: String },
    #[error("local workload state exceeds the {limit}-record limit")]
    RecordLimit { limit: usize },
    #[error("local workload state contains an empty workload id")]
    EmptyWorkloadId,
    #[error("local workload state record {0} has no retained artifact")]
    EmptyRecord(String),
    #[error("state record key {key} does not match artifact workload {artifact}")]
    RecordIdentity { key: String, artifact: String },
    #[error("unsafe symlink or non-regular local workload state path {0}")]
    UnsafeStatePath(PathBuf),
    #[error("local workload state {path} exceeds the {limit}-byte limit")]
    ByteLimit { path: PathBuf, limit: u64 },
    #[error("local workload state {path} could not be read: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local workload state {path} could not be written: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local workload state {path} is invalid JSON: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not allocate a local workload state temporary file in {0}")]
    TemporaryLimit(PathBuf),
    #[error(transparent)]
    Workload(#[from] rey_runtime::WorkloadError),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rey_runtime::{
        BUILT_IN_NORMALIZE_WORKLOAD_ID, WorkloadRunResult, built_in_workload, test_workload,
    };
    use tempfile::TempDir;

    use super::{
        LocalWorkloadState, LocalWorkloadStore, QualificationState, WorkloadFreshness,
        WorkloadSummary,
    };

    #[test]
    fn state_round_trips_verified_results_and_derives_progress() {
        let directory = TempDir::new().unwrap();
        let store = LocalWorkloadStore::new(directory.path().join("state"));
        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let result = test_workload(&workload).unwrap();
        let mut state = LocalWorkloadState::default();
        state.retain_test(result);

        store.save(&state).unwrap();
        let loaded = store.load().unwrap();
        let summary =
            WorkloadSummary::derive(&workload, loaded.record(BUILT_IN_NORMALIZE_WORKLOAD_ID));

        assert_eq!(summary.freshness, WorkloadFreshness::Fresh);
        assert_eq!(summary.qualification, QualificationState::Qualified);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.evaluated, 2);
        assert!(summary.qualified_graph.is_some());
    }

    #[test]
    fn missing_state_is_empty_and_tampering_fails_closed() {
        let directory = TempDir::new().unwrap();
        let store = LocalWorkloadStore::new(directory.path().join("state"));
        assert!(store.load().unwrap().records.is_empty());

        let workload = built_in_workload(BUILT_IN_NORMALIZE_WORKLOAD_ID).unwrap();
        let mut state = LocalWorkloadState::default();
        state.retain_run(WorkloadRunResult::blocked(&workload, Default::default()));
        store.save(&state).unwrap();
        let path = store.path();
        let mut document: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        document["records"][BUILT_IN_NORMALIZE_WORKLOAD_ID]["last_run"]["stop_reason"] =
            "tampered".into();
        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();

        assert!(store.load().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn state_file_symlinks_are_rejected() {
        use std::os::unix::fs::symlink;

        let directory = TempDir::new().unwrap();
        let state_directory = directory.path().join("state");
        fs::create_dir(&state_directory).unwrap();
        let target = directory.path().join("target");
        fs::write(&target, b"{}").unwrap();
        symlink(&target, state_directory.join("state.json")).unwrap();

        assert!(LocalWorkloadStore::new(state_directory).load().is_err());
    }
}
