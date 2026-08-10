import type { WorkloadList } from "./domain";

export interface UiServerIdentity {
  source_repository: string;
  implementation_revision: string;
}

export type OperatorContext = WorkloadList & {
  ui_server: UiServerIdentity;
};

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
