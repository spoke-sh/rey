import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import {
  activateCommunicationAxis,
  CommunicationBackdrop,
  ConversationSurface,
  router,
} from "./router";

describe("operator routes", () => {
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

  it("matches cadence, agents, and matrix-style Explorer coordinates", () => {
    expect(router.matchRoutes("/cadence").at(-1)?.routeId).toBe("/cadence");
    expect(router.matchRoutes("/agents").at(-1)?.routeId).toBe("/agents");
    const coordinate = router
      .matchRoutes(
        "/explore/agent/codex;at=gpt-5;lens=objects;role=coding_harness",
      )
      .at(-1);
    expect(coordinate?.routeId).toBe("/explore/$kind/$coordinate");
    expect(coordinate?.params).toMatchObject({
      kind: "agent",
      coordinate: "codex;at=gpt-5;lens=objects;role=coding_harness",
    });
  });
});
