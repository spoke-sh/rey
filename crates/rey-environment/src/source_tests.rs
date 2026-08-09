use std::{collections::BTreeMap, fs, path::PathBuf};

use rey_core::{ContractIdentity, SemanticDigest, SemanticHasher};
use rey_mining::{
    MiningCompleteness, MiningLimits, MiningOmissionKind, MiningParameterValue,
    MiningRationaleKind, MiningRequest, MiningRequestContext,
};
use tempfile::TempDir;

use crate::{
    LocalSourceCorpus, SourceBindingLimits, SourceContentClass, SourceMiningError,
    SourceSearchEvidence, builtin_source_search_operation, local_source_provider,
};

fn digest(label: &str) -> SemanticDigest {
    let mut hasher = SemanticHasher::new("rey.source-mining.test");
    hasher.add_str(label);
    hasher.finish()
}

fn contract(id: &str) -> ContractIdentity {
    ContractIdentity::new(id, 1, &format!("{id} fixture contract"))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/source-corpus")
}

fn fixture_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("alpha.txt"), PathBuf::from("nested/beta.rs")]
}

fn bind_fixture() -> LocalSourceCorpus {
    LocalSourceCorpus::bind(
        fixture_root(),
        fixture_paths(),
        SourceBindingLimits::default(),
    )
    .expect("bind fixture corpus")
}

fn request(
    corpus: &LocalSourceCorpus,
    pattern: &str,
    context_before: u64,
    context_after: u64,
    effective_limits: MiningLimits,
) -> MiningRequest {
    let operation = builtin_source_search_operation();
    MiningRequest::new(
        MiningRequestContext {
            workload: contract("workload.source-search"),
            graph: contract("graph.source-search"),
            scenario: Some(contract("scenario.source-search")),
            campaign_id: Some(digest("campaign")),
            space: contract("space.local"),
            active_transition_id: Some(digest("transition")),
            graph_node_id: "search".to_owned(),
            rationale: MiningRationaleKind::Frontier,
            frontier_row_ids: vec![digest("frontier")],
            delta_ids: vec![digest("delta")],
        },
        &operation,
        local_source_provider(),
        digest("capability-snapshot"),
        vec![corpus.binding().artifact_ref()],
        BTreeMap::from([
            (
                "pattern".to_owned(),
                MiningParameterValue::Utf8(pattern.to_owned()),
            ),
            (
                "context_before".to_owned(),
                MiningParameterValue::U64(context_before),
            ),
            (
                "context_after".to_owned(),
                MiningParameterValue::U64(context_after),
            ),
        ]),
        MiningLimits::default(),
        effective_limits,
    )
    .expect("source search request")
}

#[test]
fn checked_in_corpus_binding_is_exact_canonical_and_repeatable() {
    let first = bind_fixture();
    let second = LocalSourceCorpus::bind(
        fixture_root(),
        fixture_paths().into_iter().rev().collect::<Vec<_>>(),
        SourceBindingLimits::default(),
    )
    .expect("repeat binding");

    first.binding().verify().expect("binding verifies");
    first.verify_current().expect("source remains current");
    assert_eq!(first.binding(), second.binding());
    assert_eq!(first.binding().files.len(), 2);
    assert!(
        first
            .binding()
            .files
            .windows(2)
            .all(|window| window[0].path < window[1].path)
    );
    assert!(
        first
            .binding()
            .files
            .iter()
            .all(|file| file.content_class == SourceContentClass::Utf8Text)
    );
    assert_eq!(
        first
            .file_bytes(&first.binding().files[0].artifact_id)
            .expect("native bytes")
            .len() as u64,
        first.binding().files[0].byte_len
    );
}

#[test]
fn literal_search_produces_typed_matches_and_native_context() {
    let corpus = bind_fixture();
    let request = request(&corpus, "delta", 1, 1, MiningLimits::default());
    let evidence = corpus.search(&request).expect("search");
    let repeated = corpus.search(&request).expect("repeat search");

    evidence
        .verify_against(&corpus, &request)
        .expect("evidence verifies");
    assert_eq!(evidence, repeated);
    assert_eq!(evidence.result.completeness, MiningCompleteness::Complete);
    assert_eq!(evidence.matches.len(), 4);
    assert_eq!(evidence.result.consumption.files, 2);
    assert_eq!(evidence.result.consumption.matches, 4);
    assert!(
        evidence
            .matches
            .windows(2)
            .all(|window| window[0].path < window[1].path
                || (window[0].path == window[1].path
                    && window[0].start_byte < window[1].start_byte))
    );
    assert!(
        evidence
            .contexts
            .iter()
            .all(|context| context.text.contains("delta"))
    );
    let frame = evidence.to_frame().expect("match frame");
    assert_eq!(frame.dataframe().height(), 4);
    assert_eq!(frame.metadata().key_columns, ["match_id"]);
    assert_eq!(
        frame.metadata().semantic_digest,
        evidence
            .match_artifact
            .as_ref()
            .expect("match artifact")
            .artifact_id
            .to_string()
    );
    let arrow = frame.to_arrow_stream().expect("encode Arrow");
    let decoded_frame = rey_dataframe::Frame::from_arrow_stream(&arrow).expect("decode Arrow");
    assert!(decoded_frame.dataframe().equals_missing(frame.dataframe()));
    assert_eq!(decoded_frame.metadata(), frame.metadata());

    let encoded = serde_json::to_vec(&evidence).expect("encode evidence");
    let decoded: SourceSearchEvidence = serde_json::from_slice(&encoded).expect("decode evidence");
    decoded
        .verify_against(&corpus, &request)
        .expect("decoded evidence verifies");
}

#[test]
fn empty_match_relation_keeps_its_exact_schema_and_lineage() {
    let corpus = bind_fixture();
    let request = request(&corpus, "absent-literal", 0, 0, MiningLimits::default());
    let evidence = corpus.search(&request).expect("search");

    assert_eq!(evidence.result.completeness, MiningCompleteness::Complete);
    assert!(evidence.matches.is_empty());
    assert!(evidence.contexts.is_empty());
    let frame = evidence.to_frame().expect("typed empty frame");
    assert_eq!(frame.dataframe().height(), 0);
    assert_eq!(frame.dataframe().width(), 19);
    assert_eq!(frame.metadata().row_count, 0);
}

#[test]
fn match_and_file_bounds_become_explicit_truncation() {
    let corpus = bind_fixture();
    let match_limits = MiningLimits {
        max_matches: 1,
        max_rows: 1,
        ..MiningLimits::default()
    };
    let match_request = request(&corpus, "delta", 0, 0, match_limits);
    let match_evidence = corpus.search(&match_request).expect("bounded search");
    assert_eq!(
        match_evidence.result.completeness,
        MiningCompleteness::Truncated
    );
    assert_eq!(match_evidence.matches.len(), 1);
    assert!(
        match_evidence
            .result
            .omissions
            .iter()
            .any(|omission| omission.kind == MiningOmissionKind::MatchLimit)
    );

    let file_limits = MiningLimits {
        max_files: 1,
        ..MiningLimits::default()
    };
    let file_request = request(&corpus, "delta", 0, 0, file_limits);
    let file_evidence = corpus.search(&file_request).expect("file-bounded search");
    assert_eq!(
        file_evidence.result.completeness,
        MiningCompleteness::Truncated
    );
    assert!(
        file_evidence
            .result
            .omissions
            .iter()
            .any(|omission| omission.kind == MiningOmissionKind::FileLimit)
    );
}

#[test]
fn binary_and_invalid_utf8_are_explicit_unsupported_evidence() {
    let directory = TempDir::new().expect("temporary corpus");
    fs::write(directory.path().join("binary.bin"), b"delta\0binary").expect("binary");
    fs::write(directory.path().join("invalid.txt"), [b'd', 0xff, b'a']).expect("invalid");
    let corpus = LocalSourceCorpus::bind(
        directory.path(),
        [PathBuf::from("binary.bin"), PathBuf::from("invalid.txt")],
        SourceBindingLimits::default(),
    )
    .expect("bind unsupported corpus");
    assert_eq!(
        corpus.binding().files[0].content_class,
        SourceContentClass::Binary
    );
    assert_eq!(
        corpus.binding().files[1].content_class,
        SourceContentClass::InvalidUtf8
    );
    let unsupported_request = request(&corpus, "delta", 0, 0, MiningLimits::default());
    let evidence = corpus
        .search(&unsupported_request)
        .expect("unsupported result");
    assert_eq!(
        evidence.result.completeness,
        MiningCompleteness::Unsupported
    );
    assert!(evidence.match_artifact.is_none());
    assert_eq!(evidence.result.omissions.len(), 2);
    assert!(
        evidence
            .result
            .omissions
            .iter()
            .all(|omission| omission.kind == MiningOmissionKind::Unsupported)
    );

    fs::write(directory.path().join("text.txt"), "delta text\n").expect("text");
    let mixed = LocalSourceCorpus::bind(
        directory.path(),
        [PathBuf::from("binary.bin"), PathBuf::from("text.txt")],
        SourceBindingLimits::default(),
    )
    .expect("mixed corpus");
    let mixed_request = request(&mixed, "delta", 0, 0, MiningLimits::default());
    let mixed_evidence = mixed.search(&mixed_request).expect("partial result");
    assert_eq!(
        mixed_evidence.result.completeness,
        MiningCompleteness::Partial
    );
    assert_eq!(mixed_evidence.matches.len(), 1);
    assert!(mixed_evidence.match_artifact.is_some());
}

#[test]
fn source_drift_fails_without_combining_revisions() {
    let directory = TempDir::new().expect("temporary corpus");
    fs::write(directory.path().join("source.txt"), "delta before\n").expect("source");
    let corpus = LocalSourceCorpus::bind(
        directory.path(),
        [PathBuf::from("source.txt")],
        SourceBindingLimits::default(),
    )
    .expect("bind corpus");
    let request = request(&corpus, "delta", 0, 0, MiningLimits::default());
    fs::write(directory.path().join("source.txt"), "delta after\n").expect("change source");

    let evidence = corpus.search(&request).expect("failed result");
    assert_eq!(evidence.result.completeness, MiningCompleteness::Failed);
    assert!(evidence.match_artifact.is_none());
    assert_eq!(
        evidence.result.omissions[0].kind,
        MiningOmissionKind::SourceDrift
    );
}

#[test]
fn malformed_empty_pattern_is_a_failed_result() {
    let corpus = bind_fixture();
    let request = request(&corpus, "", 0, 0, MiningLimits::default());
    let evidence = corpus.search(&request).expect("failed result");

    assert_eq!(evidence.result.completeness, MiningCompleteness::Failed);
    assert_eq!(
        evidence.result.omissions[0].kind,
        MiningOmissionKind::MalformedInput
    );
}

#[test]
fn path_escape_and_binding_limits_fail_before_any_search() {
    let directory = TempDir::new().expect("temporary corpus");
    fs::write(directory.path().join("source.txt"), "delta\n").expect("source");
    assert!(matches!(
        LocalSourceCorpus::bind(
            directory.path(),
            [PathBuf::from("../escape.txt")],
            SourceBindingLimits::default(),
        ),
        Err(SourceMiningError::UnsafePath(_))
    ));
    let limits = SourceBindingLimits {
        max_file_bytes: 1,
        ..SourceBindingLimits::default()
    };
    assert!(matches!(
        LocalSourceCorpus::bind(directory.path(), [PathBuf::from("source.txt")], limits,),
        Err(SourceMiningError::Limit {
            kind: "source file byte",
            ..
        })
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_source_paths_are_rejected() {
    use std::os::unix::fs::symlink;

    let directory = TempDir::new().expect("temporary corpus");
    let outside = TempDir::new().expect("outside corpus");
    fs::write(outside.path().join("source.txt"), "delta\n").expect("source");
    symlink(
        outside.path().join("source.txt"),
        directory.path().join("linked.txt"),
    )
    .expect("symlink");
    assert!(matches!(
        LocalSourceCorpus::bind(
            directory.path(),
            [PathBuf::from("linked.txt")],
            SourceBindingLimits::default(),
        ),
        Err(SourceMiningError::Symlink(_))
    ));
}

#[test]
fn request_must_bind_the_exact_corpus_and_provider() {
    let first = bind_fixture();
    let directory = TempDir::new().expect("other corpus");
    fs::write(directory.path().join("other.txt"), "delta\n").expect("source");
    let other = LocalSourceCorpus::bind(
        directory.path(),
        [PathBuf::from("other.txt")],
        SourceBindingLimits::default(),
    )
    .expect("other corpus");
    let request = request(&first, "delta", 0, 0, MiningLimits::default());

    assert!(matches!(
        other.search(&request),
        Err(SourceMiningError::RequestInput)
    ));
}

#[test]
fn self_consistent_match_tampering_is_rejected_against_native_source() {
    let corpus = bind_fixture();
    let request = request(&corpus, "delta", 0, 0, MiningLimits::default());
    let mut evidence = corpus.search(&request).expect("search");
    evidence.matches[0].matched_text = "other".to_owned();

    assert!(matches!(
        evidence.verify_against(&corpus, &request),
        Err(SourceMiningError::EvidenceBinding | SourceMiningError::EvidenceDigest)
    ));
}
