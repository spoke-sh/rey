import type { WorkloadList } from "./domain";
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
import type { EnvironmentStatus } from "./environment";
import type {
  JournalAdmission,
  JournalEntryProposal,
  JournalOpportunitySurface,
  JournalProjection,
  JournalSeed,
} from "./journal";
import type { ObservationFrontier } from "./observations";
import type {
  WorkloadDeltaEvidence,
  WorkloadEvidenceCatalog,
  WorkloadScenarioEvidence,
} from "./workload-evidence";

export interface UiServerIdentity {
  source_repository: string | null;
  implementation_revision: string;
  journal_write_enabled: boolean;
  workload_admission_enabled: boolean;
  channel_write_enabled: boolean;
  conversation_write_enabled: boolean;
  read_only: boolean;
}

export interface WorkloadApprovalRequest {
  message: string;
  expected_head: string;
  expected_working: string;
}

export type OperatorContext = WorkloadList & {
  observations: ObservationFrontier;
  conversation: ConversationTranscript;
  workload_evidence: WorkloadEvidenceCatalog;
  ui_server: UiServerIdentity;
};

export interface FeedSources {
  cadence: CadenceProjection;
  channels: ChannelProjection;
  journal: JournalProjection;
  observations: ObservationFrontier;
}

export interface AgentJournalDocument {
  journal: JournalProjection;
  opportunities: JournalOpportunitySurface;
}

export async function loadPortfolio(): Promise<OperatorContext> {
  const [
    portfolioResponse,
    healthResponse,
    observations,
    workloadEvidence,
    conversation,
  ] = await Promise.all([
    fetch("/api/v1/workloads", { headers: { Accept: "application/json" } }),
    fetch("/api/v1/health", { headers: { Accept: "application/json" } }),
    loadObservations(),
    loadWorkloadEvidence(),
    loadConversation(),
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
    server: UiServerIdentity;
  };
  return Object.assign(portfolio, {
    observations,
    conversation,
    ui_server: health.server,
    workload_evidence: workloadEvidence,
  });
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
  const [cadence, channels, journal, observations] = await Promise.all([
    loadCadence(),
    loadChannels(),
    loadJournal(),
    loadObservations(),
  ]);
  return { cadence, channels, journal, observations };
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
