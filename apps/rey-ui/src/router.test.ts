import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import {
  activateCommunicationAxis,
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
  it("keeps Channel collaboration visible while preserving the primary order", () => {
    expect(PRIMARY_NAV_ITEMS.map((item) => item.label)).toEqual([
      "Feed",
      "Explore",
      "Agents",
      "Cadence",
      "Channels",
      "Workloads",
      "Environment",
    ]);
  });

  it("gives Feed and Explorer the remaining viewport without document scroll", () => {
    expect(isViewportLockedPath("/feed")).toBe(true);
    expect(isViewportLockedPath("/explore")).toBe(true);
    expect(isViewportLockedPath("/cadence")).toBe(false);
  });

  it("retains bounded Feed stream composition in typed route search", () => {
    expect(
      normalizeFeedSearch({
        streams: "signals.journal~Review,admission.now",
      }),
    ).toEqual({ streams: "signals.journal~Review,admission.now" });
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
    const markup = renderToStaticMarkup(createElement(ConversationSurface));

    expect(markup).toContain("REY / AGENT / OPERATOR");
    expect(markup).toContain("TRANSPORT / UNAVAILABLE");
    expect(markup).toMatch(/<textarea[^>]*disabled=""/);
    expect(markup).toMatch(/<button[^>]*disabled=""/);
    expect(markup).toContain(
      "NO TRANSPORT · NO RETENTION · NO WRITE AUTHORITY",
    );
  });

  it("closes the communication plane when its backdrop is clicked", () => {
    const onClose = vi.fn();
    const backdrop = CommunicationBackdrop({ onClose, open: true });

    expect(backdrop.props["data-communication-backdrop"]).toBe("");
    backdrop.props.onClick();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("matches feed, cadence, channels, agents, Journal documents, and coordinate views", () => {
    expect(router.matchRoutes("/feed").at(-1)?.routeId).toBe("/feed");
    expect(router.matchRoutes("/cadence").at(-1)?.routeId).toBe("/cadence");
    expect(router.matchRoutes("/channels").at(-1)?.routeId).toBe("/channels");
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
    const coordinate = router.matchRoutes("/explore").at(-1);
    expect(coordinate?.routeId).toBe("/explore");
    expect(
      normalizeExplorerSearch({
        coordinate:
          "rey+local://agent/codex?revision=gpt-5&role=coding_harness",
        scale: "1.46",
      }),
    ).toEqual({
      coordinate: "rey+local://agent/codex?revision=gpt-5&role=coding_harness",
      scale: "1.46",
    });
  });
});
