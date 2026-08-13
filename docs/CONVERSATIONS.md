# Conversation Transcripts

Rey conversations are bounded workspace-local transcripts for traditional
operator ↔ Rey ↔ agent dialogue. They are retained separately from mailbox
history, Channel topology and relay messages, observations, Journal documents,
runtime actions, and proof evidence.

The implemented first transport is deliberately narrow:

```text
session proposal → validate participants + writers → immutable session
message proposal → validate exact session + writer + prior reply → append
                                                               │
                                                               └─ no delivery or execution
```

It gives a local CLI caller a shared, addressable transcript. It does not
invoke an agent, contact a remote service, use Channel relay, create an
observation or Journal entry, schedule work, mutate runtime state, or prove a
claim.

## Session Contract

`rey.conversation-session-proposal.v1` binds:

- one bounded human-readable title;
- the exact `local_transcript` transport provider
  `rey.local-transcript/v1`;
- 1–16 declared human, Rey, or agent participants with unique stable ids and
  self-asserted labels;
- 1–16 participant ids allowed to append messages; and
- an optional declared human writer used by the browser projection.

The optional browser writer must be both a human participant and a declared
writer. Its label is not authenticated. Absence leaves browser sending
disabled even though local CLI writers may append.

Admission produces immutable `rey.conversation-session.v1`, binding content
identity, local session sequence, admission time, workspace source locator and
digest, and the complete effective limits. Identical content is idempotent.
Session admission grants transcript append authority only to its declared
writers; it grants no agent-discovery, assignment, invocation, relay, action,
or proof authority.

## Message And Ordering Contract

`rey.conversation-message-proposal.v1` names one exact session, one declared
writer, a 1–16 KiB canonical body, and an optional exact prior message in the
same session. Admission produces immutable `rey.conversation-message.v1`.

Messages receive a contiguous one-based sequence within their exact session.
That sequence is the only conversation ordering claim. Session sequences order
the local session log; no causal or wall-time order is inferred across
sessions, mailbox sources, Channel logs, providers, or other Rey state.

Every retained message reports `delivery: not_attempted`. Append means the
message is present in the local transcript; it does not mean another
participant saw it, an agent ran, a remote provider accepted it, or a reply is
forthcoming. Reply edges may point only to an earlier retained message in the
same session.

## Retention, Availability, And Failure

The tamper-detecting `rey.conversation-log.v1` is retained at
`.rey/conversations/conversations.json` by default. It is bounded to 32
sessions, 2,048 total messages, 16 KiB per message, 256 rows per transcript
projection, and 4 MiB total state. Publication uses a separate lock and atomic
replacement, rejects symlinked state paths, and verifies every sequence,
identity, participant, writer, reply, source, and limit envelope after restart.

`rey.conversation-transcript.v1` reports exact availability, session and log
identity, provider revision, participants, writers, read/write authority,
ordering, retention, bounds, completeness, omissions, effect authority, and
failure behavior. With no admitted session, it returns an explicit unavailable
boundary and creates no state. A bounded view keeps the newest selected rows in
ascending session order and reports omitted older rows as truncation.

Validation, stale/unknown session, unknown or read-only author, invalid reply,
bounds, lock, persistence, and tamper failures reject the append before
publication. The previously retained log remains authoritative. This local
profile has no authentication, remote durability, delivery receipt, inbound
cursor, presence, unread state, typing state, edit, deletion, or multi-writer
distributed consistency guarantee.

## CLI Surface

```text
rey conversations [--workspace PATH] [--state-dir PATH] status
  [--session SESSION_ID] [-n COUNT] [--format table|json]
rey conversations [--workspace PATH] [--state-dir PATH] session add SESSION.yaml
rey conversations [--workspace PATH] [--state-dir PATH] session list
rey conversations [--workspace PATH] [--state-dir PATH] message add MESSAGE.yaml
```

`status` and `session list` are read-only. `session add` and `message add`
accept only bounded regular non-symlinked files contained by the workspace.
Human rendering exposes the complete authority and availability boundary in
addition to the transcript; JSON returns the same typed documents.

`GET|HEAD /api/v1/conversations` returns the same default bounded transcript.
The footer conversation axis passively revalidates it independently from the
mailbox source relations and renders exact participants, writers, message
sequence, source, identity, coverage, omissions, retention, authority, limits,
and failure posture.

`POST /api/v1/conversations/messages` accepts only
`rey.ui-conversation-message-write.v1`. It conditionally appends against the
exact expected log and session identities, derives the author only from that
session's declared human browser writer, calls the same store and validator,
and returns the retained message/transcript. A stale log, missing session,
missing browser writer, invalid body/reply, tamper, or persistence failure
rejects the write. The composer remains visibly disabled when the transport or
browser writer is unavailable and after a failed stale append until current
state revalidates. It never falls back to UI-only messages, aliases mailbox
history, or implies delivery beyond local admission.

The UI listener has no authentication. Any client that can reach an explicitly
configured listener can append as the session's self-asserted human browser
writer, so `rey ui` reports that exposure and warns on non-loopback binds.
