import {
  agentIdentity,
  deriveAgentIndex,
  type AgentSummary,
  type GeneratorProvenance,
  type WorkloadList,
} from "./domain";
import {
  DEFAULT_LENS_ZOOM,
  NEIGHBORHOOD_LENS_ZOOM,
  OBJECT_LENS_ZOOM,
  type LensRegime,
} from "./topology";

export type ExplorerCoordinateKind =
  "agent" | "attention" | "cluster" | "portfolio" | "workload";

export interface ExplorerCoordinate {
  kind: ExplorerCoordinateKind;
  identity: string;
  lens: LensRegime;
  at?: string;
  role?: GeneratorProvenance["kind"];
}

export interface ExplorerCoordinateResolution {
  coordinate: ExplorerCoordinate;
  focus_id: string;
  status: "current" | "stale" | "missing";
  actual_at: string | null;
}

const kinds = new Set<ExplorerCoordinateKind>([
  "agent",
  "attention",
  "cluster",
  "portfolio",
  "workload",
]);
const lenses = new Set<LensRegime>(["landscape", "neighborhoods", "objects"]);
const roles = new Set<GeneratorProvenance["kind"]>([
  "coding_harness",
  "human",
  "rule",
]);
const matrixKeys = new Set(["at", "lens", "role"]);

export function explorerCoordinateSegment(
  coordinate: ExplorerCoordinate,
): string {
  const parameters = new Map<string, string>();
  if (coordinate.at) parameters.set("at", coordinate.at);
  parameters.set("lens", coordinate.lens);
  if (coordinate.role) parameters.set("role", coordinate.role);
  const matrix = [...parameters]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(
      ([name, value]) =>
        `;${encodeURIComponent(name)}=${encodeURIComponent(value)}`,
    )
    .join("");
  return `${encodeURIComponent(coordinate.identity)}${matrix}`;
}

export function explorerCoordinatePath(coordinate: ExplorerCoordinate): string {
  return `/explore/${coordinate.kind}/${explorerCoordinateSegment(coordinate)}`;
}

export function agentExplorerCoordinate(
  agent: AgentSummary,
): ExplorerCoordinate {
  return {
    kind: "agent",
    identity: agent.producer,
    lens: "objects",
    at: agent.producer_revision,
    role: agent.kind,
  };
}

export function parseExplorerCoordinate(
  kindValue: string,
  segment: string,
): ExplorerCoordinate | null {
  if (!kinds.has(kindValue as ExplorerCoordinateKind)) return null;
  const [encodedIdentity, ...encodedParameters] = segment.split(";");
  const identity = decode(encodedIdentity);
  if (!identity) return null;
  const parameters = new Map<string, string>();
  for (const parameter of encodedParameters) {
    const separator = parameter.indexOf("=");
    if (separator <= 0) return null;
    const name = decode(parameter.slice(0, separator));
    const value = decode(parameter.slice(separator + 1));
    if (!name || !value || !matrixKeys.has(name) || parameters.has(name)) {
      return null;
    }
    parameters.set(name, value);
  }
  const lensValue = parameters.get("lens") ?? "objects";
  if (!lenses.has(lensValue as LensRegime)) return null;
  const roleValue = parameters.get("role");
  if (roleValue && !roles.has(roleValue as GeneratorProvenance["kind"])) {
    return null;
  }
  const kind = kindValue as ExplorerCoordinateKind;
  const at = parameters.get("at");
  if (kind !== "cluster" && !at) return null;
  if ((kind === "agent") !== Boolean(roleValue)) return null;
  return {
    kind,
    identity,
    lens: lensValue as LensRegime,
    at,
    role: roleValue as GeneratorProvenance["kind"] | undefined,
  };
}

export function resolveExplorerCoordinate(
  portfolio: WorkloadList,
  coordinate: ExplorerCoordinate,
): ExplorerCoordinateResolution {
  let focusId = `cluster:${coordinate.identity}`;
  let actualAt: string | null = portfolio.attention.source_snapshot_id;
  let present = true;
  if (coordinate.kind === "portfolio") {
    focusId = "cluster:portfolio";
    present = coordinate.identity === "current";
  } else if (coordinate.kind === "workload") {
    focusId = `workload:${coordinate.identity}`;
    const workload = portfolio.workloads.find(
      (candidate) => candidate.workload.id === coordinate.identity,
    );
    const draft = portfolio.drafts.find(
      (candidate) => candidate.request.workload_id === coordinate.identity,
    );
    actualAt =
      workload?.workload.semantic_digest ?? draft?.request.request_id ?? null;
    present = Boolean(workload ?? draft);
  } else if (coordinate.kind === "attention") {
    focusId = `attention:${coordinate.identity}`;
    present = portfolio.attention.rows.some(
      (row) => row.row_id === coordinate.identity,
    );
    actualAt = present ? portfolio.attention.attention_id : null;
  } else if (coordinate.kind === "agent") {
    const candidates = deriveAgentIndex(portfolio).filter(
      (candidate) =>
        candidate.producer === coordinate.identity &&
        candidate.kind === coordinate.role,
    );
    const agent =
      candidates.find(
        (candidate) => candidate.producer_revision === coordinate.at,
      ) ?? candidates.at(-1);
    focusId = agent
      ? `agent:${agent.id}`
      : `agent:${agentIdentity(
          coordinate.role ?? "coding_harness",
          coordinate.identity,
          coordinate.at ?? "unknown",
        )}`;
    actualAt = agent?.producer_revision ?? null;
    present = Boolean(agent);
  } else if (coordinate.kind === "cluster") {
    present = [
      "agents",
      "attention",
      "context",
      "evidence",
      "portfolio",
      "workloads",
    ].includes(coordinate.identity);
  }
  return {
    coordinate,
    focus_id: focusId,
    status: !present
      ? "missing"
      : coordinate.at && coordinate.at !== actualAt
        ? "stale"
        : "current",
    actual_at: actualAt,
  };
}

export function zoomForExplorerLens(lens: LensRegime): number {
  if (lens === "landscape") return DEFAULT_LENS_ZOOM;
  if (lens === "neighborhoods") return NEIGHBORHOOD_LENS_ZOOM;
  return OBJECT_LENS_ZOOM;
}

function decode(value: string | undefined): string | null {
  if (!value) return null;
  try {
    return decodeURIComponent(value);
  } catch {
    return null;
  }
}
