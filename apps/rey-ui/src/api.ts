import type { WorkloadList } from "./domain";
import type { CadenceProjection } from "./cadence";
import type { EnvironmentStatus } from "./environment";
import type {
  JournalAdmission,
  JournalEntryProposal,
  JournalProjection,
} from "./journal";

export interface UiServerIdentity {
  source_repository: string;
  implementation_revision: string;
  journal_write_enabled: boolean;
  workload_admission_enabled: boolean;
  read_only: boolean;
}

export interface WorkloadApprovalRequest {
  message: string;
  expected_head: string;
  expected_index: string;
}

export type OperatorContext = WorkloadList & {
  ui_server: UiServerIdentity;
};

export interface FeedSources {
  cadence: CadenceProjection;
  journal: JournalProjection;
}

export async function loadPortfolio(): Promise<OperatorContext> {
  const [portfolioResponse, healthResponse] = await Promise.all([
    fetch("/api/v1/workloads", { headers: { Accept: "application/json" } }),
    fetch("/api/v1/health", { headers: { Accept: "application/json" } }),
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
  return Object.assign(portfolio, { ui_server: health.server });
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
  const [cadence, journal] = await Promise.all([loadCadence(), loadJournal()]);
  return { cadence, journal };
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

export async function approveWorkloadIndex(
  approval: WorkloadApprovalRequest,
): Promise<void> {
  const response = await fetch("/api/v1/workloads/commit", {
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
