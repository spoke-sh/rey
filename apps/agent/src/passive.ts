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
  let timeout: ReturnType<typeof globalThis.setTimeout> | undefined;

  const schedule = () => {
    if (!active) return;
    timeout = globalThis.setTimeout(() => void refresh(), intervalMs);
  };

  const refresh = async () => {
    try {
      const document = await load();
      if (active) {
        publish(document);
        reportError(null);
      }
    } catch (error) {
      if (active) reportError(normalizeError(error));
    } finally {
      schedule();
    }
  };

  schedule();
  return () => {
    active = false;
    if (timeout !== undefined) globalThis.clearTimeout(timeout);
  };
}

function normalizeError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
