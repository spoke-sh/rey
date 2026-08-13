import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ConversationTranscript } from "./conversations";
import {
  activateCommunicationAxis,
  browserRouterBasepath,
  CommunicationBackdrop,
  ConversationSurface,
  isViewportLockedPath,
  journalSeedObservationIds,
  normalizeExplorerSearch,
  normalizeFeedSearch,
  normalizeJournalNewSearch,
  PRIMARY_NAV_ITEMS,
  router,
} from "./router";

describe("operator routes", () => {
  it("derives a qualification-only base path for local-file browser voyages", () => {
    expect(
      browserRouterBasepath({
        pathname: "/tmp/rey-qualification/explore.html",
        protocol: "file:",
      }),
    ).toBe("/tmp/rey-qualification");
    expect(
      browserRouterBasepath({ pathname: "/explore", protocol: "http:" }),
    ).toBe("/");
  });

  it("keeps Channel topology behind the operator-facing navigation", () => {
    expect(PRIMARY_NAV_ITEMS.map((item) => item.label)).toEqual([
      "Feed",
      "Explore",
      "Agents",
      "Cadence",
      "Workloads",
      "Environment",
    ]);
    expect(PRIMARY_NAV_ITEMS.map((item) => String(item.to))).not.toContain(
      "/channels",
    );
  });

  it("gives Feed and Explorer the remaining viewport without document scroll", () => {
    expect(isViewportLockedPath("/feed")).toBe(true);
    expect(isViewportLockedPath("/explore")).toBe(true);
    expect(isViewportLockedPath("/cadence")).toBe(false);
  });

  it("retains bounded Feed stream composition in typed route search", () => {
    expect(
      normalizeFeedSearch({
        streams: "signals.journal~Review,admission.all",
      }),
    ).toEqual({ streams: "signals.journal~Review,admission.all" });
    expect(normalizeFeedSearch({ streams: 3 })).toEqual({});
    expect(normalizeFeedSearch({ streams: "x".repeat(4_097) })).toEqual({});
  });

  it("bounds exact observation identities in Journal seed route state", () => {
    const first = `blake3:${"a".repeat(64)}`;
    const second = `blake3:${"b".repeat(64)}`;
    expect(journalSeedObservationIds(`${first},${second}`)).toEqual([
      first,
      second,
    ]);
    expect(normalizeJournalNewSearch({ observations: first })).toEqual({
      observations: first,
    });
    expect(
      normalizeJournalNewSearch({ observations: `${first},${first}` }),
    ).toEqual({});
    expect(
      normalizeJournalNewSearch({ observations: "blake3:not-a-digest" }),
    ).toEqual({});
  });

  it("opens, closes, and switches the two communication axes", () => {
    expect(activateCommunicationAxis(null, "mailbox")).toBe("mailbox");
    expect(activateCommunicationAxis("mailbox", "mailbox")).toBeNull();
    expect(activateCommunicationAxis("mailbox", "conversation")).toBe(
      "conversation",
    );
    expect(activateCommunicationAxis("conversation", "mailbox")).toBe(
      "mailbox",
    );
  });

  it("keeps the traditional conversation composer disabled without transport", () => {
    const markup = renderToStaticMarkup(
      createElement(ConversationSurface, {
        transcript: conversationTranscript(false),
      }),
    );

    expect(markup).toContain("REY / AGENT / OPERATOR");
    expect(markup).toContain("TRANSPORT / UNAVAILABLE");
    expect(markup).toMatch(/<textarea[^>]*disabled=""/);
    expect(markup).toMatch(/<button[^>]*disabled=""/);
    expect(markup).toContain("NO AVAILABLE BROWSER WRITER · SEND DISABLED");
    expect(markup).toContain("NO ADMITTED CONVERSATION");
  });

  it("projects an exact retained transcript and enables only its declared browser writer", () => {
    const markup = renderToStaticMarkup(
      createElement(ConversationSurface, {
        transcript: conversationTranscript(true),
      }),
    );

    expect(markup).toContain("TRANSPORT / AVAILABLE");
    expect(markup).toContain("C@1 · AGENT/ codex · SELF-ASSERTED");
    expect(markup).toContain("Retained agent context.");
    expect(markup).toContain("DELIVERY / NOT ATTEMPTED");
    expect(markup).toContain("Append as Operator · self-asserted");
    expect(markup).toMatch(/<textarea(?![^>]*disabled)[^>]*>/);
    expect(markup).toContain("NO DELIVERY OR EXECUTION");
  });

  it("closes the communication plane when its backdrop is clicked", () => {
    const onClose = vi.fn();
    const backdrop = CommunicationBackdrop({ onClose, open: true });

    expect(backdrop.props["data-communication-backdrop"]).toBe("");
    backdrop.props.onClick();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("matches operator routes without exposing a Channels page", () => {
    expect(router.matchRoutes("/feed").at(-1)?.routeId).toBe("/feed");
    expect(router.matchRoutes("/cadence").at(-1)?.routeId).toBe("/cadence");
    expect(
      router.matchRoutes("/channels").map((match) => String(match.routeId)),
    ).not.toContain("/channels");
    expect(router.matchRoutes("/agents").at(-1)?.routeId).toBe("/agents");
    expect(router.matchRoutes("/journal/new").at(-1)?.routeId).toBe(
      "/journal/new",
    );
    const journal = router
      .matchRoutes("/journal/j1-context--blake3-entry")
      .at(-1);
    expect(journal?.routeId).toBe("/journal/$slug");
    expect(journal?.params).toMatchObject({
      slug: "j1-context--blake3-entry",
    });
    const scenario = router
      .matchRoutes("/workloads/rey.example/scenarios/blake3:scenario-execution")
      .at(-1);
    expect(scenario?.routeId).toBe(
      "/workloads/$workloadId/scenarios/$executionId",
    );
    expect(scenario?.params).toMatchObject({
      workloadId: "rey.example",
      executionId: "blake3:scenario-execution",
    });
    const delta = router
      .matchRoutes("/workloads/rey.example/deltas/blake3:directed-delta")
      .at(-1);
    expect(delta?.routeId).toBe("/workloads/$workloadId/deltas/$deltaId");
    expect(delta?.params).toMatchObject({
      workloadId: "rey.example",
      deltaId: "blake3:directed-delta",
    });
    const coordinate = router.matchRoutes("/explore").at(-1);
    expect(coordinate?.routeId).toBe("/explore");
    expect(
      normalizeExplorerSearch({
        coordinate:
          "rey+local://agent/codex?revision=gpt-5&role=coding_harness",
        renderer: "webgl2",
        scale: "1.46",
      }),
    ).toEqual({
      coordinate: "rey+local://agent/codex?revision=gpt-5&role=coding_harness",
      renderer: "webgl2",
      scale: "1.46",
    });
    expect(normalizeExplorerSearch({ renderer: "unknown" })).toEqual({});
  });
});

function conversationTranscript(available: boolean): ConversationTranscript {
  const limits = {
    max_sessions: 32,
    max_messages: 2_048,
    max_participants_per_session: 16,
    max_writers_per_session: 16,
    max_message_bytes: 16_384,
    max_transcript_rows: 256,
    max_state_bytes: 4_194_304,
  };
  const digest = (character: string) => `blake3:${character.repeat(64)}`;
  if (!available) {
    return {
      schema: "rey.conversation-transcript.v1",
      transcript_id: digest("a"),
      log_id: digest("b"),
      session: null,
      availability: "unavailable",
      availability_detail: "no conversation session is admitted",
      ordering: "none; no session sequence exists",
      retention: "none; no transcript exists",
      read_authority: "local projection",
      cli_write_authority: "none",
      browser_write_authority: "none",
      browser_write_enabled: false,
      effect_authority: "none",
      failure_contract: "message admission fails closed",
      completeness: "complete",
      total_messages: 0,
      omitted_messages: 0,
      messages: [],
      limits,
    };
  }
  const sessionId = digest("c");
  return {
    schema: "rey.conversation-transcript.v1",
    transcript_id: digest("d"),
    log_id: digest("e"),
    session: {
      schema: "rey.conversation-session.v1",
      session_id: sessionId,
      sequence: 1,
      admitted_at_unix: 1,
      source: {
        locator: "worktree:///session.yaml",
        content_digest: digest("f"),
      },
      limits,
      proposal: {
        schema: "rey.conversation-session-proposal.v1",
        title: "Operator and agent conversation",
        transport: {
          kind: "local_transcript",
          provider: "rey.local-transcript",
          provider_revision: "v1",
        },
        participants: [
          { participant_id: "operator", kind: "human", label: "Operator" },
          { participant_id: "codex", kind: "agent", label: "Codex" },
        ],
        writer_ids: ["operator", "codex"],
        browser_writer_id: "operator",
      },
    },
    availability: "available",
    availability_detail: "the admitted local transcript is available",
    ordering: "local_per_session_sequence",
    retention: "workspace_local_append_only",
    read_authority: "listener clients",
    cli_write_authority: "declared writers",
    browser_write_authority: "self-asserted operator; admission only",
    browser_write_enabled: true,
    effect_authority: "none; no delivery or execution",
    failure_contract: "reject before publication",
    completeness: "complete",
    total_messages: 1,
    omitted_messages: 0,
    messages: [
      {
        schema: "rey.conversation-message.v1",
        message_id: digest("1"),
        sequence: 1,
        admitted_at_unix: 2,
        source: {
          locator: "worktree:///message.yaml",
          content_digest: digest("2"),
        },
        delivery: "not_attempted",
        proposal: {
          schema: "rey.conversation-message-proposal.v1",
          session_id: sessionId,
          author_id: "codex",
          body: "Retained agent context.",
          reply_to: null,
        },
      },
    ],
    limits,
  };
}
