/**
 * Decode worker pool. F-112, HLD section 9.
 *
 * Each index gets its own starter call. The render worker is not started
 * here. The escalation decision stays on stable Rust. This file does not
 * construct a shared-memory buffer.
 */

export const DEFAULT_DECODE_WORKERS = 1;
export const MAX_DECODE_WORKERS = 8;

export interface WorkerHandle {
  terminate: () => void;
}

export interface DecodePool {
  readonly count: number;
  readonly sharedMemory: false;
  readonly threading: "stable";
  close: () => boolean;
}

const openPools = new Map<object, WorkerHandle[]>();

export function threadingEscalation(): "stable" {
  return "stable";
}

function terminateAll(handles: readonly WorkerHandle[]): void {
  for (const handle of handles) {
    try {
      handle.terminate();
    } catch {
      // One failure does not restore the slot and does not skip the rest.
    }
  }
}

export function openDecodePool(
  owner: object,
  count: number,
  start: (index: number) => WorkerHandle,
): DecodePool | null {
  if (owner === null || typeof owner !== "object") {
    return null;
  }
  if (!Number.isInteger(count) || count < 1 || count > MAX_DECODE_WORKERS) {
    return null;
  }
  if (openPools.has(owner)) {
    return null;
  }
  const handles: WorkerHandle[] = [];
  openPools.set(owner, handles);
  try {
    for (let index = 0; index < count; index += 1) {
      handles.push(start(index));
    }
  } catch (error) {
    openPools.delete(owner);
    terminateAll(handles);
    throw error;
  }
  let closed = false;
  return {
    count,
    sharedMemory: false,
    threading: threadingEscalation(),
    close() {
      if (closed) {
        return false;
      }
      closed = true;
      openPools.delete(owner);
      terminateAll(handles);
      return true;
    },
  };
}
