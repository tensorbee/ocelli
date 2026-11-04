/**
 * The page's handle on the wasm worker.
 *
 * Status carries the tier and the two flags. It does not carry pixels.
 */

export interface CoreStatus {
  readonly available: boolean;
  readonly tier: string;
  readonly published: boolean;
}

export interface CoreWorkerHandle {
  subscribe(listener: (status: CoreStatus) => void): () => void;
  postInstance(bytes: Uint8Array, boundary: string): Promise<number>;
  close(): void;
}

function readStatus(value: unknown): CoreStatus | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (record.type !== "status") {
    return null;
  }
  if (
    typeof record.available !== "boolean" ||
    typeof record.tier !== "string" ||
    typeof record.published !== "boolean"
  ) {
    return null;
  }
  return {
    available: record.available,
    tier: record.tier,
    published: record.published,
  };
}

export function openCoreWorker(canvas: HTMLCanvasElement): CoreWorkerHandle | null {
  if (
    typeof Worker === "undefined" ||
    typeof canvas.transferControlToOffscreen !== "function"
  ) {
    return null;
  }
  let worker: Worker;
  try {
    const offscreen = canvas.transferControlToOffscreen();
    worker = new Worker(new URL("./core-worker.ts", import.meta.url), {
      type: "module",
    });
    worker.postMessage({ type: "start", canvas: offscreen }, [offscreen]);
  } catch {
    return null;
  }
  const listeners = new Set<(status: CoreStatus) => void>();
  worker.onmessage = (event: MessageEvent<unknown>) => {
    const status = readStatus(event.data);
    if (status === null) {
      return;
    }
    for (const listener of listeners) {
      listener(status);
    }
  };
  return {
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    postInstance(bytes, boundary) {
      const copy = bytes.slice();
      const buffer = copy.buffer;
      return new Promise((resolve) => {
        const onMessage = (event: MessageEvent<unknown>) => {
          if (typeof event.data !== "object" || event.data === null) {
            return;
          }
          const record = event.data as Record<string, unknown>;
          if (record.type !== "accepted") {
            return;
          }
          worker.removeEventListener("message", onMessage);
          resolve(typeof record.bytes === "number" ? record.bytes : 0);
        };
        worker.addEventListener("message", onMessage);
        worker.postMessage({ type: "instance", bytes: buffer, boundary }, [buffer]);
      });
    },
    close() {
      worker.terminate();
    },
  };
}
