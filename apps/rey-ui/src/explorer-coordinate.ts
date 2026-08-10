import {
  agentIdentity,
  deriveAgentIndex,
  type AgentSummary,
  type GeneratorProvenance,
  type WorkloadList,
} from "./domain";
import { MAX_LENS_ZOOM, MIN_LENS_ZOOM, OBJECT_LENS_ZOOM } from "./topology";

export type ExplorerCoordinateKind =
  "agent" | "attention" | "cluster" | "portfolio" | "workload";

export interface ExplorerCoordinate {
  scheme: "rey+local";
  kind: ExplorerCoordinateKind;
  identity: string;
  revision: string;
  role?: GeneratorProvenance["kind"];
}

export interface ExplorerView {
  coordinate: ExplorerCoordinate;
  scale: number;
}

export interface ExplorerViewResolution {
  view: ExplorerView;
  focus_id: string;
  status: "current" | "stale" | "missing";
  actual_revision: string | null;
}

const kinds = new Set<ExplorerCoordinateKind>([
  "agent",
  "attention",
  "cluster",
  "portfolio",
  "workload",
]);
const roles = new Set<GeneratorProvenance["kind"]>([
  "coding_harness",
  "human",
  "rule",
]);
const coordinateKeys = new Set(["revision", "role"]);

export function explorerCoordinateUri(coordinate: ExplorerCoordinate): string {
  const parameters = [`revision=${encodeURIComponent(coordinate.revision)}`];
  if (coordinate.role) {
    parameters.push(`role=${encodeURIComponent(coordinate.role)}`);
  }
  return `rey+local://${coordinate.kind}/${encodeURIComponent(coordinate.identity)}?${parameters.join("&")}`;
}

export function explorerViewPath(view: ExplorerView): string {
  return `/explore?coordinate=${encodeURIComponent(explorerCoordinateUri(view.coordinate))}&scale=${encodeURIComponent(canonicalScale(view.scale))}`;
}

export function agentExplorerView(agent: AgentSummary): ExplorerView {
  return {
    coordinate: {
      scheme: "rey+local",
      kind: "agent",
      identity: agent.producer,
      revision: agent.producer_revision,
      role: agent.kind,
    },
    scale: OBJECT_LENS_ZOOM,
  };
}

export function parseExplorerView(
  coordinateValue: string,
  scaleValue: string,
): ExplorerView | null {
  const coordinate = parseExplorerCoordinate(coordinateValue);
  const scale = Number(scaleValue);
  if (
    !coordinate ||
    !Number.isFinite(scale) ||
    scale < MIN_LENS_ZOOM ||
    scale > MAX_LENS_ZOOM ||
    canonicalScale(scale) !== scaleValue
  ) {
    return null;
  }
  return { coordinate, scale };
}

export function parseExplorerCoordinate(
  value: string,
): ExplorerCoordinate | null {
  const match = /^rey\+local:\/\/([^/]+)\/([^?#]+)\?([^#]+)$/.exec(value);
  if (!match) return null;
  const [, kindValue, encodedIdentity, query] = match;
  if (!kinds.has(kindValue as ExplorerCoordinateKind)) return null;
  const identity = decode(encodedIdentity);
  if (!identity) return null;

  const parameters = new Map<string, string>();
  for (const part of query!.split("&")) {
    const separator = part.indexOf("=");
    if (separator <= 0) return null;
    const name = decode(part.slice(0, separator));
    const parameterValue = decode(part.slice(separator + 1));
    if (
      !name ||
      !parameterValue ||
      !coordinateKeys.has(name) ||
      parameters.has(name)
    ) {
      return null;
    }
    parameters.set(name, parameterValue);
  }

  const revision = parameters.get("revision");
  if (!revision) return null;
  const roleValue = parameters.get("role");
  if (roleValue && !roles.has(roleValue as GeneratorProvenance["kind"])) {
    return null;
  }
  const kind = kindValue as ExplorerCoordinateKind;
  if ((kind === "agent") !== Boolean(roleValue)) return null;
  const coordinate: ExplorerCoordinate = {
    scheme: "rey+local",
    kind,
    identity,
    revision,
    role: roleValue as GeneratorProvenance["kind"] | undefined,
  };
  return explorerCoordinateUri(coordinate) === value ? coordinate : null;
}

export function resolveExplorerView(
  portfolio: WorkloadList,
  view: ExplorerView,
): ExplorerViewResolution {
  const { coordinate } = view;
  let focusId = `cluster:${coordinate.identity}`;
  let actualRevision: string | null = portfolio.attention.source_snapshot_id;
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
    actualRevision =
      workload?.workload.semantic_digest ?? draft?.request.request_id ?? null;
    present = Boolean(workload ?? draft);
  } else if (coordinate.kind === "attention") {
    focusId = `attention:${coordinate.identity}`;
    present = portfolio.attention.rows.some(
      (row) => row.row_id === coordinate.identity,
    );
    actualRevision = present ? portfolio.attention.attention_id : null;
  } else if (coordinate.kind === "agent") {
    const candidates = deriveAgentIndex(portfolio).filter(
      (candidate) =>
        candidate.producer === coordinate.identity &&
        candidate.kind === coordinate.role,
    );
    const agent =
      candidates.find(
        (candidate) => candidate.producer_revision === coordinate.revision,
      ) ?? candidates.at(-1);
    focusId = agent
      ? `agent:${agent.id}`
      : `agent:${agentIdentity(
          coordinate.role ?? "coding_harness",
          coordinate.identity,
          coordinate.revision,
        )}`;
    actualRevision = agent?.producer_revision ?? null;
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
    view,
    focus_id: focusId,
    status: !present
      ? "missing"
      : coordinate.revision !== actualRevision
        ? "stale"
        : "current",
    actual_revision: actualRevision,
  };
}

function canonicalScale(scale: number): string {
  return String(scale);
}

function decode(value: string | undefined): string | null {
  if (!value) return null;
  try {
    return decodeURIComponent(value);
  } catch {
    return null;
  }
}
