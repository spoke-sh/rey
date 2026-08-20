use serde_json::{Map, Value, json};

pub(crate) const OPENAPI_PATH: &str = "/api/openapi.json";
pub(crate) const API_ROOT_PATH: &str = "/api";
pub(crate) const SWAGGER_PATH: &str = "/api/docs";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ApiMethod {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ApiRoute {
    pub path: &'static str,
    pub method: ApiMethod,
    pub operation_id: &'static str,
    pub tag: &'static str,
    pub summary: &'static str,
    pub description: &'static str,
    pub authority: &'static str,
    pub response_schema: &'static str,
    pub request_schema: Option<&'static str>,
    pub path_parameters: &'static [&'static str],
    pub query_parameters: &'static [&'static str],
}

const NO_PARAMETERS: &[&str] = &[];
const WORKLOAD_SCENARIO_PARAMETERS: &[&str] = &["workload_id", "execution_id"];
const WORKLOAD_DELTA_PARAMETERS: &[&str] = &["workload_id", "delta_id"];
const JOURNAL_SEED_QUERY: &[&str] = &["observations"];

pub(crate) const API_ROUTES: &[ApiRoute] = &[
    read(
        "/api/v1/health",
        "getAgentHealth",
        "Runtime",
        "Inspect agent health",
        "Returns readiness plus the exact Rey process, supervised topology, and operator descriptor.",
        "read-only process projection",
        "AgentHealth",
    ),
    read(
        "/api/v1/agent",
        "getAgentProcess",
        "Runtime",
        "Inspect the foreground Rey process",
        "Returns the exact process, worker topology, lifecycle, authority, limits, and omissions.",
        "read-only process projection",
        "AgentProcess",
    ),
    read(
        "/api/v1/revalidation",
        "getRevalidationCursor",
        "Runtime",
        "Inspect the passive-revalidation cursor",
        "Detects exact bounded source changes without assessing, scheduling, or executing work.",
        "read-only change detection",
        "RevalidationCursor",
    ),
    read(
        "/api/v1/cadence",
        "getCadence",
        "Runtime",
        "Inspect partially ordered runtime clocks",
        "Projects retained Git, environment, and mounted-browser schedules without inventing a total event order.",
        "read-only evidence projection",
        "CadenceProjection",
    ),
    read(
        "/api/v1/channels",
        "getChannels",
        "Collaboration",
        "Inspect Channel topology and mailbox",
        "Returns verified Channel state and the current retained provider mailbox frontier.",
        "read-only collaboration projection",
        "ChannelsProjection",
    ),
    write(
        "/api/v1/channels/poll",
        "pollGitHubMailbox",
        "Collaboration",
        "Poll the exact GitHub mailbox application after following evidence",
        "Runs the same bounded admitted GitHub poll as the CLI for a clicked current mailbox message, then returns the refreshed Channel projection.",
        "bounded unauthenticated provider read probe and local message retention; no provider mutation, INDEX, HEAD, relay, or proof authority",
        ("ChannelsProjection", "GitHubPollWrite"),
    ),
    write(
        "/api/v1/channels/working",
        "replaceChannelWorking",
        "Collaboration",
        "Conditionally replace Channel WORKING",
        "Validates a complete Channel graph and replaces only WORKING when both expected snapshots still match.",
        "bounded unauthenticated local write; no INDEX, HEAD, relay, or execution authority",
        ("ChannelWorkingWriteResult", "ChannelWorkingWrite"),
    ),
    read(
        "/api/v1/conversations",
        "getConversationTranscript",
        "Collaboration",
        "Inspect the local conversation transcript",
        "Returns one bounded workspace-local transcript with exact writer and delivery authority.",
        "read-only collaboration projection",
        "ConversationTranscript",
    ),
    write(
        "/api/v1/conversations/messages",
        "appendConversationMessage",
        "Collaboration",
        "Append a browser-authorized conversation message",
        "Appends through the declared human browser writer under exact log and session preconditions; delivery is not attempted.",
        "bounded unauthenticated local transcript write; no delivery or agent invocation",
        ("ConversationMessageWriteResult", "ConversationMessageWrite"),
    ),
    read(
        "/api/v1/environment",
        "getEnvironment",
        "Environment",
        "Inspect current environment evidence",
        "Runs the same bounded environment-status derivation used by the CLI and returns its typed projection.",
        "bounded read-only discovery projection",
        "EnvironmentStatus",
    ),
    read(
        "/api/v1/feed/admissions",
        "getFeedAdmissions",
        "Collaboration",
        "Inspect retained Feed admissions",
        "Merges bounded verified environment and workload commit histories without synthesizing mutable activity.",
        "read-only retained-history projection",
        "FeedAdmissions",
    ),
    read(
        "/api/v1/journal",
        "getJournal",
        "Journal",
        "Inspect the Journal",
        "Returns the verified bounded Journal log and its browser admission boundary.",
        "read-only document projection",
        "JournalProjection",
    ),
    write(
        "/api/v1/journal",
        "admitJournalEntry",
        "Journal",
        "Admit a Journal entry",
        "Validates and idempotently retains one bounded human-authored Journal proposal; no block is executed.",
        "bounded unauthenticated local Journal admission",
        ("JournalAdmission", "JournalEntryProposal"),
    ),
    read(
        "/api/v1/journal/opportunities",
        "getJournalOpportunities",
        "Journal",
        "Inspect authored Journal opportunities",
        "Projects action cells from current Journal leaves without readiness, assignment, execution, or proof authority.",
        "read-only derived document projection",
        "JournalOpportunitySurface",
    ),
    read(
        "/api/v1/journal/queries",
        "getJournalQueries",
        "Journal",
        "Inspect retained Journal query evidence",
        "Returns retained read-only query admissions and executions; the browser exposes no query mutation endpoint.",
        "read-only retained-evidence projection",
        "JournalQueryState",
    ),
    ApiRoute {
        path: "/api/v1/journal/seed",
        method: ApiMethod::Read,
        operation_id: "getJournalSeed",
        tag: "Journal",
        summary: "Project an unretained Journal seed",
        description: "Projects one bounded Journal proposal from exact unresolved observations; it does not retain an entry.",
        authority: "read-only deterministic projection",
        response_schema: "JournalSeed",
        request_schema: None,
        path_parameters: NO_PARAMETERS,
        query_parameters: JOURNAL_SEED_QUERY,
    },
    read(
        "/api/v1/observations",
        "getObservations",
        "Collaboration",
        "Inspect the observation frontier",
        "Returns the bounded unresolved collaboration frontier and exact local Channel admissions.",
        "read-only collaboration projection",
        "ObservationFrontier",
    ),
    write(
        "/api/v1/observations",
        "admitObservation",
        "Collaboration",
        "Admit a compact human observation",
        "Admits one partial self-asserted observation and broadcasts it to the effective graph's default local Channels.",
        "bounded unauthenticated local observation admission; no action or proof authority",
        ("ObservationWriteResult", "ObservationWrite"),
    ),
    read(
        "/api/v1/workloads",
        "getWorkloads",
        "Workloads",
        "Inspect the workload portfolio",
        "Returns the bounded workload list, retained results, attention, atlas, and compact regional terrain transport.",
        "read-only evidence projection; may be expensive on a cold source revision",
        "WorkloadList",
    ),
    read(
        "/api/v1/workloads/admissions",
        "getWorkloadAdmissions",
        "Workloads",
        "Inspect workload admission history",
        "Returns the newest verified workload commits under the retained history bound.",
        "read-only retained-history projection",
        "WorkloadLog",
    ),
    write(
        "/api/v1/workloads/admit",
        "admitWorkloadSnapshot",
        "Workloads",
        "Qualify and admit an exact workload snapshot",
        "Checks expected HEAD and WORKING, freezes the complete reviewed files, runs the full suite, and commits only that exact qualified INDEX.",
        "bounded unauthenticated qualification and workload-HEAD write",
        ("WorkloadAdmissionResult", "WorkloadApproval"),
    ),
    read(
        "/api/v1/workloads/evidence",
        "getWorkloadEvidenceCatalog",
        "Workloads",
        "Inspect the exact workload-evidence index",
        "Returns retained scenario and directed-delta references without reevaluating a result.",
        "read-only retained-evidence projection",
        "WorkloadEvidenceCatalog",
    ),
    ApiRoute {
        path: "/api/v1/workloads/{workload_id}/scenarios/{execution_id}",
        method: ApiMethod::Read,
        operation_id: "getWorkloadScenarioEvidence",
        tag: "Workloads",
        summary: "Inspect exact scenario evidence",
        description: "Resolves one content-addressed retained scenario execution; unknown or stale identities never fall back to latest.",
        authority: "read-only exact retained-evidence projection",
        response_schema: "WorkloadScenarioEvidence",
        request_schema: None,
        path_parameters: WORKLOAD_SCENARIO_PARAMETERS,
        query_parameters: NO_PARAMETERS,
    },
    ApiRoute {
        path: "/api/v1/workloads/{workload_id}/deltas/{delta_id}",
        method: ApiMethod::Read,
        operation_id: "getWorkloadDeltaEvidence",
        tag: "Workloads",
        summary: "Inspect an exact directed delta",
        description: "Resolves one content-addressed retained output-text, source-match, or topography delta in its original direction.",
        authority: "read-only exact retained-evidence projection",
        response_schema: "WorkloadDeltaEvidence",
        request_schema: None,
        path_parameters: WORKLOAD_DELTA_PARAMETERS,
        query_parameters: NO_PARAMETERS,
    },
];

const fn read(
    path: &'static str,
    operation_id: &'static str,
    tag: &'static str,
    summary: &'static str,
    description: &'static str,
    authority: &'static str,
    response_schema: &'static str,
) -> ApiRoute {
    ApiRoute {
        path,
        method: ApiMethod::Read,
        operation_id,
        tag,
        summary,
        description,
        authority,
        response_schema,
        request_schema: None,
        path_parameters: NO_PARAMETERS,
        query_parameters: NO_PARAMETERS,
    }
}

const fn write(
    path: &'static str,
    operation_id: &'static str,
    tag: &'static str,
    summary: &'static str,
    description: &'static str,
    authority: &'static str,
    schemas: (&'static str, &'static str),
) -> ApiRoute {
    ApiRoute {
        path,
        method: ApiMethod::Write,
        operation_id,
        tag,
        summary,
        description,
        authority,
        response_schema: schemas.0,
        request_schema: Some(schemas.1),
        path_parameters: NO_PARAMETERS,
        query_parameters: NO_PARAMETERS,
    }
}

pub(crate) fn openapi() -> Value {
    let mut paths = Map::new();
    for route in API_ROUTES {
        let path = paths
            .entry(route.path.to_owned())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("OpenAPI path entries are objects");
        match route.method {
            ApiMethod::Read => {
                path.insert("get".to_owned(), operation(route, false));
                path.insert("head".to_owned(), operation(route, true));
            }
            ApiMethod::Write => {
                path.insert("post".to_owned(), operation(route, false));
            }
        }
    }

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Rey Agent API",
            "version": env!("CARGO_PKG_VERSION"),
            "summary": "The local typed HTTP projection hosted by `rey agent`.",
            "description": "Rey's API projects the same bounded evidence as the CLI and admits only the explicitly documented local writes. It is unauthenticated, loopback-only by default, and is not a public multi-user service. GET and HEAD are read surfaces. POST authority is endpoint-specific and never widens merely because Swagger can issue the request."
        },
        "servers": [{ "url": "/", "description": "The current rey agent origin" }],
        "tags": [
            { "name": "Runtime", "description": "Foreground process, topology, health, revalidation, and cadence." },
            { "name": "Environment", "description": "Bounded current environment evidence." },
            { "name": "Workloads", "description": "Portfolio, admission history, exact results, and qualification-bound admission." },
            { "name": "Collaboration", "description": "Channels, observations, Feed admissions, and local conversation transcripts." },
            { "name": "Journal", "description": "Retained collaboration documents and read-only derived projections." }
        ],
        "paths": paths,
        "components": { "schemas": component_schemas() }
    })
}

fn operation(route: &ApiRoute, head: bool) -> Value {
    let operation_id = if head {
        format!("{}Head", route.operation_id)
    } else {
        route.operation_id.to_owned()
    };
    let mut parameters = Vec::new();
    for name in route.path_parameters {
        parameters.push(json!({
            "name": name,
            "in": "path",
            "required": true,
            "description": "Exact percent-encoded retained resource identity.",
            "schema": { "type": "string", "minLength": 1 }
        }));
    }
    for name in route.query_parameters {
        parameters.push(json!({
            "name": name,
            "in": "query",
            "required": true,
            "description": "Comma-separated exact unresolved observation identities.",
            "schema": { "type": "string", "minLength": 1 }
        }));
    }
    let mut value = json!({
        "operationId": operation_id,
        "tags": [route.tag],
        "summary": if head { format!("{} without a response body", route.summary) } else { route.summary.to_owned() },
        "description": route.description,
        "x-rey-authority": route.authority,
        "responses": success_responses(route, head)
    });
    let object = value
        .as_object_mut()
        .expect("OpenAPI operation is an object");
    if !parameters.is_empty() {
        object.insert("parameters".to_owned(), Value::Array(parameters));
    }
    if let Some(request_schema) = route.request_schema {
        object.insert(
            "requestBody".to_owned(),
            json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": { "$ref": format!("#/components/schemas/{request_schema}") },
                        "example": request_example(request_schema)
                    }
                }
            }),
        );
    }
    value
}

fn success_responses(route: &ApiRoute, head: bool) -> Value {
    let response = if head {
        json!({ "description": "The same status and headers as GET, with no response body." })
    } else {
        json!({
            "description": "Verified bounded Rey document.",
            "content": {
                "application/json": {
                    "schema": { "$ref": format!("#/components/schemas/{}", route.response_schema) }
                }
            }
        })
    };
    let mut responses = Map::new();
    if route.method == ApiMethod::Write {
        responses.insert("200".to_owned(), response.clone());
        responses.insert("201".to_owned(), response);
    } else {
        responses.insert("200".to_owned(), response);
    }
    responses.insert(
        "default".to_owned(),
        json!({
            "description": "A bounded typed API error.",
            "content": {
                "application/json": {
                    "schema": { "$ref": "#/components/schemas/ApiError" }
                }
            }
        }),
    );
    Value::Object(responses)
}

fn component_schemas() -> Value {
    let mut schemas = Map::new();
    schemas.insert(
        "ReyDocument".to_owned(),
        json!({
            "type": "object",
            "description": "A bounded typed Rey document. Endpoint-specific fields retain their native schema and exact identities.",
            "required": ["schema"],
            "properties": { "schema": { "type": "string", "minLength": 1 } },
            "additionalProperties": true
        }),
    );
    for (name, schema, description) in [
        (
            "AgentHealth",
            "rey.agent-health.v2",
            "Readiness and exact process/operator topology.",
        ),
        (
            "AgentProcess",
            "rey.agent-process.v2",
            "Exact foreground process and supervised worker topology.",
        ),
        (
            "RevalidationCursor",
            "rey.ui-revalidation.v1",
            "Exact bounded change-detection cursor.",
        ),
        (
            "CadenceProjection",
            "rey.ui-cadence.v1",
            "Partially ordered cadence evidence.",
        ),
        (
            "ChannelsProjection",
            "rey.ui-channels.v1",
            "Channel status and current provider mailbox.",
        ),
        (
            "GitHubPollWrite",
            "rey.ui-github-poll-write.v1",
            "Exact current mailbox message and admitted GitHub application poll preconditions.",
        ),
        (
            "ConversationTranscript",
            "rey.conversation-transcript.v1",
            "Bounded local transcript and authority.",
        ),
        (
            "EnvironmentStatus",
            "rey.environment-status.v2",
            "Current bounded environment evidence.",
        ),
        (
            "FeedAdmissions",
            "rey.ui-feed-admissions.v1",
            "Retained environment and workload admissions.",
        ),
        (
            "JournalProjection",
            "rey.ui-journal.v2",
            "Verified Journal log and write boundary.",
        ),
        (
            "JournalAdmission",
            "rey.journal-admission.v2",
            "Journal admission result.",
        ),
        (
            "JournalOpportunitySurface",
            "rey.journal-opportunity-surface.v1",
            "Authored-only opportunity projection.",
        ),
        (
            "JournalSeed",
            "rey.journal-seed.v1",
            "Unretained Journal proposal derived from observations.",
        ),
        (
            "ObservationFrontier",
            "rey.observation-frontier.v1",
            "Bounded unresolved observation frontier.",
        ),
        (
            "WorkloadList",
            "rey.workload-list.v1",
            "Current workload portfolio and retained results.",
        ),
        (
            "WorkloadLog",
            "rey.workload-log.v1",
            "Verified workload admission history.",
        ),
        (
            "WorkloadEvidenceCatalog",
            "rey.ui-workload-evidence-catalog.v1",
            "Exact retained scenario and delta index.",
        ),
        (
            "JournalQueryState",
            "rey.journal-query-state.v1",
            "Retained Journal query admission and execution evidence.",
        ),
        (
            "ChannelWorkingWriteResult",
            "rey.channel-apply-result.v1",
            "Conditional Channel WORKING replacement receipt.",
        ),
        (
            "ConversationMessageWriteResult",
            "rey.conversation-message-admission.v1",
            "Conditional local transcript append receipt.",
        ),
        (
            "ObservationWriteResult",
            "rey.observation-broadcast.v1",
            "Observation admission and bounded broadcast receipt.",
        ),
        (
            "WorkloadAdmissionResult",
            "rey.workload-commit-result.v1",
            "Exact workload qualification and admission receipt.",
        ),
        (
            "WorkloadScenarioEvidence",
            "rey.ui-workload-scenario-evidence.v1",
            "One exact retained scenario execution projection.",
        ),
        (
            "WorkloadDeltaEvidence",
            "rey.ui-workload-delta-evidence.v1",
            "One exact retained directed delta projection.",
        ),
    ] {
        schemas.insert(name.to_owned(), typed_document(schema, description));
    }
    schemas.insert("ApiError".to_owned(), api_error_schema());
    schemas.insert("JournalEntryProposal".to_owned(), journal_request_schema());
    schemas.insert("ObservationWrite".to_owned(), observation_request_schema());
    schemas.insert("WorkloadApproval".to_owned(), workload_approval_schema());
    schemas.insert("ChannelWorkingWrite".to_owned(), channel_working_schema());
    schemas.insert(
        "ConversationMessageWrite".to_owned(),
        conversation_message_schema(),
    );
    Value::Object(schemas)
}

fn typed_document(schema: &str, description: &str) -> Value {
    json!({
        "allOf": [
            { "$ref": "#/components/schemas/ReyDocument" },
            {
                "type": "object",
                "description": description,
                "properties": { "schema": { "const": schema } }
            }
        ]
    })
}

fn api_error_schema() -> Value {
    json!({
        "type": "object",
        "required": ["schema", "category", "detail"],
        "properties": {
            "schema": { "const": "rey.api-error.v1" },
            "category": { "type": "string" },
            "detail": { "type": "string" }
        },
        "additionalProperties": false
    })
}

fn journal_request_schema() -> Value {
    json!({
        "type": "object",
        "required": ["schema", "author", "coordinate", "scale", "title", "bands", "blocks"],
        "properties": {
            "schema": { "const": "rey.journal-entry-proposal.v2" },
            "author": { "type": "object" },
            "coordinate": { "type": "string" },
            "scale": { "type": "number" },
            "title": { "type": "string" },
            "bands": { "type": "array", "items": { "type": "object" } },
            "blocks": { "type": "array", "items": { "type": "object" } },
            "supersedes": { "type": ["string", "null"] }
        },
        "additionalProperties": false
    })
}

fn observation_request_schema() -> Value {
    json!({
        "type": "object",
        "required": ["schema", "kind", "body"],
        "properties": {
            "schema": { "const": "rey.ui-observation-write.v1" },
            "kind": { "type": "string", "enum": ["finding"] },
            "body": { "type": "string", "maxLength": 500 }
        },
        "additionalProperties": false
    })
}

fn workload_approval_schema() -> Value {
    json!({
        "type": "object",
        "required": ["message", "expected_head", "expected_working"],
        "properties": {
            "message": { "type": "string" },
            "expected_head": { "type": "string" },
            "expected_working": { "type": "string" }
        },
        "additionalProperties": false
    })
}

fn channel_working_schema() -> Value {
    json!({
        "type": "object",
        "required": ["schema", "expected_head_snapshot_id", "expected_working_snapshot_id", "graph"],
        "properties": {
            "schema": { "const": "rey.ui-channel-working-write.v1" },
            "expected_head_snapshot_id": { "type": "string" },
            "expected_working_snapshot_id": { "type": "string" },
            "graph": { "type": "object" }
        },
        "additionalProperties": false
    })
}

fn conversation_message_schema() -> Value {
    json!({
        "type": "object",
        "required": ["schema", "expected_log_id", "session_id", "body", "reply_to"],
        "properties": {
            "schema": { "const": "rey.ui-conversation-message-write.v1" },
            "expected_log_id": { "type": "string" },
            "session_id": { "type": "string" },
            "body": { "type": "string" },
            "reply_to": { "type": ["string", "null"] }
        },
        "additionalProperties": false
    })
}

fn request_example(schema: &str) -> Value {
    match schema {
        "ObservationWrite" => json!({
            "schema": "rey.ui-observation-write.v1",
            "kind": "finding",
            "body": "A bounded observation from the human operator."
        }),
        "WorkloadApproval" => json!({
            "message": "Admit the exact reviewed workload snapshot",
            "expected_head": "EMPTY",
            "expected_working": "blake3:..."
        }),
        "ChannelWorkingWrite" => json!({
            "schema": "rey.ui-channel-working-write.v1",
            "expected_head_snapshot_id": "blake3:...",
            "expected_working_snapshot_id": "blake3:...",
            "graph": { "schema": "rey.channel-graph.v1" }
        }),
        "ConversationMessageWrite" => json!({
            "schema": "rey.ui-conversation-message-write.v1",
            "expected_log_id": "blake3:...",
            "session_id": "blake3:...",
            "body": "A bounded local transcript message.",
            "reply_to": null
        }),
        _ => json!({
            "schema": "rey.journal-entry-proposal.v2",
            "author": { "kind": "human", "id": "operator" },
            "coordinate": "rey+local://workspace/current?revision=...",
            "scale": 1.0,
            "title": "Bounded Journal entry",
            "bands": [],
            "blocks": [],
            "supersedes": null
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{API_ROUTES, ApiMethod, openapi};

    #[test]
    fn route_catalog_has_unique_operations_and_methods() {
        let mut operations = std::collections::BTreeSet::new();
        let mut route_methods = std::collections::BTreeSet::new();
        for route in API_ROUTES {
            assert!(operations.insert(route.operation_id));
            assert!(route_methods.insert((route.path, route.method)));
            assert_eq!(
                route.method == ApiMethod::Write,
                route.request_schema.is_some()
            );
        }
    }

    #[test]
    fn openapi_document_covers_every_registered_operation() {
        let document = openapi();
        assert_eq!(document["openapi"], "3.1.0");
        let schemas = document["components"]["schemas"].as_object().unwrap();
        for route in API_ROUTES {
            let method = match route.method {
                ApiMethod::Read => "get",
                ApiMethod::Write => "post",
            };
            assert_eq!(
                document["paths"][route.path][method]["operationId"],
                route.operation_id
            );
            if route.method == ApiMethod::Read {
                assert!(document["paths"][route.path]["head"].is_object());
            }
            assert!(schemas.contains_key(route.response_schema));
            if let Some(request_schema) = route.request_schema {
                assert!(schemas.contains_key(request_schema));
            }
        }
        assert!(schemas.contains_key("ApiError"));
    }
}
