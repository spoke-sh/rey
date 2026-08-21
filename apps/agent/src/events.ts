export interface SchedulerEvent {
  schema: "rey.scheduler-event.v1";
  sequence: number;
  schedule_id: string;
  topic: string;
  source_revision: string;
  occurred_at_unix: number;
}

interface RuntimeChangeListener {
  publish: (event: SchedulerEvent) => void;
  reportError: (error: Error | null) => void;
}

const listeners = new Set<RuntimeChangeListener>();
let source: EventSource | null = null;

function reportError(error: Error | null): void {
  for (const listener of listeners) listener.reportError(error);
}

function ensureSource(): void {
  if (source !== null || typeof EventSource === "undefined") return;

  source = new EventSource("/api/v1/events");
  source.onopen = () => reportError(null);
  source.onerror = () =>
    reportError(new Error("Scheduler event stream is reconnecting"));
  source.onmessage = (message) => {
    try {
      const event = JSON.parse(message.data) as SchedulerEvent;
      if (event.schema !== "rey.scheduler-event.v1") {
        throw new Error("Scheduler event used an unsupported schema");
      }
      for (const listener of listeners) listener.publish(event);
    } catch (cause) {
      reportError(
        cause instanceof Error
          ? cause
          : new Error("Scheduler event could not be decoded"),
      );
    }
  };
}

export function subscribeRuntimeChanges(
  listener: RuntimeChangeListener,
): () => void {
  listeners.add(listener);
  ensureSource();
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      source?.close();
      source = null;
    }
  };
}
