import type { WorkloadCommit, WorkloadList, WorkloadLog } from "./domain";
import type { CadenceProjection } from "./cadence";
import type {
  ConversationMessageAdmission,
  ConversationMessageWrite,
  ConversationTranscript,
} from "./conversations";
import type {
  ChannelApplyResult,
  ChannelProjection,
  ChannelWorkingWriteRequest,
} from "./channels";
import type {
  EnvironmentAvailability,
  EnvironmentObjectChange,
  EnvironmentStatus,
} from "./environment";
import type {
  JournalAdmission,
  JournalEntryProposal,
  JournalOpportunitySurface,
  JournalProjection,
  JournalSeed,
} from "./journal";
import type {
  ObservationBroadcast,
  ObservationFrontier,
  ObservationWrite,
} from "./observations";
import type {
  WorkloadDeltaEvidence,
  WorkloadEvidenceCatalog,
  WorkloadScenarioEvidence,
} from "./workload-evidence";

export interface UiServerIdentity {
  schema: "rey.ui-server.v2";
  http_framework: "axum";
  api_root: "/api";
  openapi_document: "/api/openapi.json";
  swagger_ui: "/api/docs/";
  source_repository: string | null;
  implementation_revision: string;
  journal_write_enabled: boolean;
  observation_write_enabled: boolean;
  workload_admission_enabled: boolean;
  channel_write_enabled: boolean;
  conversation_write_enabled: boolean;
  read_only: boolean;
}

export interface UiRevalidationCursor {
  schema: "rey.ui-revalidation.v1";
  revision: string;
  poll_after_ms: number;
  basis: string;
  source_entries: number;
  source_bytes: number;
  scope: string[];
  authority: string;
  omissions: string[];
}

export interface ReyProcessDescriptor {
  schema: "rey.process.v1";
  process_id: string;
  os_pid: number;
  role: string;
  topology_node_id: string;
  invocation: string;
  lifecycle: string;
  shutdown: string;
  implementation_revision: string;
}

export interface AgentTopologyNode {
  node_id: string;
  kind: string;
  parent_node_id: string | null;
  execution: string;
  lifecycle: string;
  state: string;
  restart_policy: string;
  authority: string;
  endpoint: string | null;
}

export interface AgentTopologyEdge {
  source_node_id: string;
  target_node_id: string;
  relationship: string;
}

export interface AgentTopologyDescriptor {
  schema: "rey.agent-topology.v1";
  root_node_id: string;
  nodes: AgentTopologyNode[];
  edges: AgentTopologyEdge[];
  max_background_workers: number;
  supervision_poll_interval_ms: number;
  agent_runtime_invocation: string;
}

export interface AgentProcessDescriptor {
  schema: "rey.agent-process.v2";
  state: string;
  process: ReyProcessDescriptor;
  topology: AgentTopologyDescriptor;
  operator: UiServerIdentity;
  authority: string;
  omissions: string[];
}

export interface WorkloadApprovalRequest {
  message: string;
  expected_head: string;
  expected_working: string;
}

export type OperatorContext = WorkloadList & {
  agent_process: AgentProcessDescriptor;
  channels: ChannelProjection;
  observations: ObservationFrontier;
  conversation: ConversationTranscript;
  revalidation: UiRevalidationCursor;
  ui_server: UiServerIdentity;
};

export interface FeedSources {
  cadence: CadenceProjection;
  channels: ChannelProjection;
  journal: JournalProjection;
  observations: ObservationFrontier;
  admissions: FeedAdmissions;
}

export interface EnvironmentAdmissionCommit {
  schema: "rey.environment-commit.v1";
  commit_id: string;
  sequence: number;
  parent_commit_id: string | null;
  committed_at_unix: number;
  message: string;
  snapshot: {
    semantic_digest: string;
    complete: boolean;
    capabilities: unknown[];
  };
}

export interface EnvironmentApplicationAdmission {
  name: string;
  change: EnvironmentObjectChange;
  availability: EnvironmentAvailability | null;
  resolved_path: string | null;
  groups: string[];
}

export type FeedAdmission =
  | {
      kind: "environment";
      commit: EnvironmentAdmissionCommit;
      changes: {
        variables: number;
        applications: EnvironmentApplicationAdmission[];
        inputs: number;
        references: number;
      };
    }
  | {
      kind: "workload";
      commit: WorkloadCommit;
    };

export interface FeedAdmissions {
  schema: "rey.ui-feed-admissions.v1";
  ordering: "committed_at_unix_desc_then_stable_identity";
  total_admissions: number;
  selected_admissions: number;
  complete: boolean;
  admissions: FeedAdmission[];
  omissions: string[];
}

export interface AgentJournalDocument {
  journal: JournalProjection;
  opportunities: JournalOpportunitySurface;
}

export async function loadPortfolio(): Promise<OperatorContext> {
  return loadPortfolioDocument();
}

async function loadPortfolioDocument(
  retainedRevalidation?: UiRevalidationCursor,
): Promise<OperatorContext> {
  const [
    portfolioResponse,
    healthResponse,
    observations,
    conversation,
    channels,
    revalidation,
  ] = await Promise.all([
    fetch("/api/v1/workloads", { headers: { Accept: "application/json" } }),
    fetch("/api/v1/health", { headers: { Accept: "application/json" } }),
    loadObservations(),
    loadConversation(),
    loadChannels(),
    retainedRevalidation ?? loadPortfolioRevalidation(),
  ]);
  if (!portfolioResponse.ok) {
    const detail = await portfolioResponse.text();
    throw new Error(
      `Portfolio request failed (${portfolioResponse.status}): ${detail}`,
    );
  }
  if (!healthResponse.ok) {
    const detail = await healthResponse.text();
    throw new Error(
      `Server identity request failed (${healthResponse.status}): ${detail}`,
    );
  }
  const portfolio = (await portfolioResponse.json()) as WorkloadList;
  const health = (await healthResponse.json()) as {
    agent: AgentProcessDescriptor;
    server: UiServerIdentity;
  };
  return Object.assign(portfolio, {
    agent_process: health.agent,
    channels,
    observations,
    conversation,
    revalidation,
    ui_server: health.server,
  });
}

export async function loadPortfolioRevalidation(): Promise<UiRevalidationCursor> {
  const response = await fetch("/api/v1/revalidation", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Portfolio revalidation request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as UiRevalidationCursor;
}

export async function loadPortfolioAfterRevision(
  retainedRevision: string,
): Promise<OperatorContext | null> {
  const revalidation = await loadPortfolioRevalidation();
  if (revalidation.revision === retainedRevision) return null;
  return loadPortfolioDocument(revalidation);
}

export async function loadConversation(): Promise<ConversationTranscript> {
  const response = await fetch("/api/v1/conversations", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Conversation request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as ConversationTranscript;
}

export async function writeConversationMessage(
  write: ConversationMessageWrite,
): Promise<ConversationMessageAdmission> {
  const response = await fetch("/api/v1/conversations/messages", {
    body: JSON.stringify(write),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Conversation append failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as ConversationMessageAdmission;
}

export async function loadWorkloadEvidence(): Promise<WorkloadEvidenceCatalog> {
  const response = await fetch("/api/v1/workloads/evidence", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Workload evidence request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as WorkloadEvidenceCatalog;
}

export async function loadWorkloadScenarioEvidence(
  workloadId: string,
  executionId: string,
): Promise<WorkloadScenarioEvidence> {
  const response = await fetch(
    `/api/v1/workloads/${encodeURIComponent(workloadId)}/scenarios/${encodeURIComponent(executionId)}`,
    { headers: { Accept: "application/json" } },
  );
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Scenario evidence request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as WorkloadScenarioEvidence;
}

export async function loadWorkloadDeltaEvidence(
  workloadId: string,
  deltaId: string,
): Promise<WorkloadDeltaEvidence> {
  const response = await fetch(
    `/api/v1/workloads/${encodeURIComponent(workloadId)}/deltas/${encodeURIComponent(deltaId)}`,
    { headers: { Accept: "application/json" } },
  );
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Directed delta request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as WorkloadDeltaEvidence;
}

export async function loadEnvironment(): Promise<EnvironmentStatus> {
  const response = await fetch("/api/v1/environment", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Environment request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as EnvironmentStatus;
}

export async function loadChannels(): Promise<ChannelProjection> {
  const response = await fetch("/api/v1/channels", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`Channel request failed (${response.status}): ${detail}`);
  }
  return (await response.json()) as ChannelProjection;
}

export async function writeChannelWorking(
  write: ChannelWorkingWriteRequest,
): Promise<ChannelApplyResult> {
  const response = await fetch("/api/v1/channels/working", {
    body: JSON.stringify(write),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Channel WORKING write failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as ChannelApplyResult;
}

export async function loadCadence(): Promise<CadenceProjection> {
  const response = await fetch("/api/v1/cadence", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`Cadence request failed (${response.status}): ${detail}`);
  }
  return (await response.json()) as CadenceProjection;
}

export async function loadFeed(): Promise<FeedSources> {
  const [cadence, channels, journal, observations, admissions] =
    await Promise.all([
      loadCadence(),
      loadChannels(),
      loadJournal(),
      loadObservations(),
      loadFeedAdmissions(),
    ]);
  return { cadence, channels, journal, observations, admissions };
}

export async function loadFeedAdmissions(): Promise<FeedAdmissions> {
  const response = await fetch("/api/v1/feed/admissions", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Feed admissions request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as FeedAdmissions;
}

export async function loadWorkloadAdmissions(): Promise<WorkloadLog> {
  const response = await fetch("/api/v1/workloads/admissions", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Workload admissions request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as WorkloadLog;
}

export async function loadObservations(): Promise<ObservationFrontier> {
  const response = await fetch("/api/v1/observations", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Observation frontier request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as ObservationFrontier;
}

export async function writeObservation(
  write: ObservationWrite,
): Promise<ObservationBroadcast> {
  const response = await fetch("/api/v1/observations", {
    body: JSON.stringify(write),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Observation admission failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as ObservationBroadcast;
}

export async function loadJournal(): Promise<JournalProjection> {
  const response = await fetch("/api/v1/journal", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`Journal request failed (${response.status}): ${detail}`);
  }
  return (await response.json()) as JournalProjection;
}

export async function loadJournalOpportunities(): Promise<JournalOpportunitySurface> {
  const response = await fetch("/api/v1/journal/opportunities", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Journal opportunity request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as JournalOpportunitySurface;
}

export async function loadAgentJournal(): Promise<AgentJournalDocument> {
  const [journal, opportunities] = await Promise.all([
    loadJournal(),
    loadJournalOpportunities(),
  ]);
  return { journal, opportunities };
}

export async function loadJournalSeed(
  observationIds: string[],
): Promise<JournalSeed> {
  const query = new URLSearchParams({
    observations: observationIds.join(","),
  });
  const response = await fetch(`/api/v1/journal/seed?${query}`, {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Journal seed request failed (${response.status}): ${detail}`,
    );
  }
  return (await response.json()) as JournalSeed;
}

export async function admitJournalEntry(
  proposal: JournalEntryProposal,
): Promise<JournalAdmission> {
  const response = await fetch("/api/v1/journal", {
    body: JSON.stringify(proposal),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`Journal admission failed (${response.status}): ${detail}`);
  }
  return (await response.json()) as JournalAdmission;
}

export async function admitWorkloadFiles(
  approval: WorkloadApprovalRequest,
): Promise<void> {
  const response = await fetch("/api/v1/workloads/admit", {
    body: JSON.stringify(approval),
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Workload admission failed (${response.status}): ${detail}`,
    );
  }
}
