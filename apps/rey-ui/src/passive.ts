export interface PassiveRevalidationOptions<T> {
  intervalMs: number;
  load: () => Promise<T>;
  publish: (document: T) => void;
  reportError: (error: Error | null) => void;
}

export function startPassiveRevalidation<T>({
  intervalMs,
  load,
  publish,
  reportError,
}: PassiveRevalidationOptions<T>): () => void {
  let active = true;
  let inFlight = false;

  const refresh = async () => {
    if (inFlight) return;
    inFlight = true;
    try {
      const document = await load();
      if (active) {
        publish(document);
        reportError(null);
      }
    } catch (error) {
      if (active) reportError(normalizeError(error));
    } finally {
      inFlight = false;
    }
  };

  const interval = globalThis.setInterval(() => void refresh(), intervalMs);
  return () => {
    active = false;
    globalThis.clearInterval(interval);
  };
}

function normalizeError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
