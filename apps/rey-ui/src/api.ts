import type { WorkloadList } from "./domain";

export async function loadPortfolio(): Promise<WorkloadList> {
  const response = await fetch("/api/v1/workloads", {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`Portfolio request failed (${response.status}): ${detail}`);
  }
  return (await response.json()) as WorkloadList;
}
