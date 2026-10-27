/**
 * One animation frame for one viewport. F-102, HLD sections 5.1, 5.3 and 17.3.
 *
 * Commands wait in an array. A frame packs that array once and submits it
 * once. The ring is drained once. State is read once and reported only when
 * a field changed. The wasm view stays in `writeCommands`.
 */

import {
  decodeNamedEvent,
  drainRing,
  packCommands,
  readViewportState,
  sessionCommandSink,
  VIEWPORT_STATE_FIELDS,
  writeCommands,
} from "@ocelli/core";
import type {
  OcelliNamedEvent,
  PackedCommand,
  ViewportId,
  ViewportReadback,
  WasmMemory,
} from "@ocelli/core";

/**
 * The session the pump drives.
 *
 * `memory` is the module `writeCommands` copies into. A view over it does
 * not belong in this package.
 */
export interface OcelliSession {
  readonly memory: WebAssembly.Memory;
  alloc(len: number): number;
  bulk_ticket(): number;
  release_bulk(ticket: number): number;
  apply_commands(ticket: number, len: number): number;
  drain_events(): Uint8Array;
  copy_state(): Uint8Array;
  snapshot(): Uint8Array;
  snapshot_error(): number;
  restore_snapshot(bytes: Uint8Array): number;
}

export interface ViewportCheckpoint {
  readonly snapshot: Uint8Array;
  readonly sliceIndex: number;
}

export interface ViewportPumpOptions {
  readonly viewport: ViewportId;
  readonly onEvents?: (events: readonly OcelliNamedEvent[], dropped: number) => void;
  readonly onState?: (state: ViewportReadback) => void;
  readonly onRefused?: (commands: readonly PackedCommand[]) => void;
  readonly requestFrame?: (callback: FrameRequestCallback) => number;
  readonly cancelFrame?: (handle: number) => void;
}

export interface ViewportPump {
  enqueue(command: PackedCommand): void;
  start(): void;
  stop(): void;
  flushNow(): void;
  checkpoint(): ViewportCheckpoint | null;
  restore(checkpoint: ViewportCheckpoint): boolean;
}

/**
 * Callbacks the frame reads. Replacing them does not restart the loop.
 */
export interface ViewportPumpHandlers {
  onEvents: ViewportPumpOptions["onEvents"];
  onState: ViewportPumpOptions["onState"];
  onRefused: ViewportPumpOptions["onRefused"];
}

const handlers = new WeakMap<ViewportPump, ViewportPumpHandlers>();

/**
 * Point an existing pump at the latest callbacks.
 *
 * The React binding calls this after commit. A new function identity must
 * not create a second animation loop.
 */
export function bindViewportPump(
  pump: ViewportPump,
  next: ViewportPumpHandlers,
): void {
  const slot = handlers.get(pump);
  if (slot === undefined) {
    return;
  }
  slot.onEvents = next.onEvents;
  slot.onState = next.onState;
  slot.onRefused = next.onRefused;
}

function sameCamera(
  left: ViewportReadback["camera"],
  right: ViewportReadback["camera"],
): boolean {
  for (let index = 0; index < left.length; index += 1) {
    if (!Object.is(left[index], right[index])) {
      return false;
    }
  }
  return true;
}

/** A matching readback does not notify. Camera is compared by value. */
function sameReadback(left: ViewportReadback, right: ViewportReadback): boolean {
  for (const field of VIEWPORT_STATE_FIELDS) {
    if (field === "camera") {
      if (!sameCamera(left.camera, right.camera)) {
        return false;
      }
      continue;
    }
    if (!Object.is(left[field], right[field])) {
      return false;
    }
  }
  return true;
}

export function createViewportPump(
  session: OcelliSession,
  options: ViewportPumpOptions,
): ViewportPump {
  const requestFrame = options.requestFrame
    ?? ((callback: FrameRequestCallback) => requestAnimationFrame(callback));
  const cancelFrame = options.cancelFrame
    ?? ((handle: number) => cancelAnimationFrame(handle));
  const memory: WasmMemory = { memory: session.memory };
  const slot: ViewportPumpHandlers = {
    onEvents: options.onEvents,
    onState: options.onState,
    onRefused: options.onRefused,
  };
  let pending: PackedCommand[] = [];
  let lastState: ViewportReadback | null = null;
  let handle: number | null = null;
  let running = false;
  let stepping = false;

  function step(): void {
    // A callback runs inside this frame. Re-entering would drain twice, and
    // stop then start would schedule a second loop.
    if (stepping) {
      return;
    }
    stepping = true;
    try {
      stepBody();
    } finally {
      stepping = false;
    }
  }

  function stepBody(): void {
    if (pending.length > 0) {
      const batch = pending;
      pending = [];
      const applied = writeCommands(
        memory,
        sessionCommandSink(session),
        packCommands(options.viewport, batch),
      );
      if (applied === 0) {
        slot.onRefused?.(batch);
      }
    }
    const drained = drainRing(session.drain_events());
    const named: OcelliNamedEvent[] = [];
    for (const event of drained.events) {
      const decoded = decodeNamedEvent(event);
      if (decoded !== null) {
        named.push(decoded);
      }
    }
    slot.onEvents?.(named, drained.dropped);
    const read = readViewportState(session.copy_state());
    if (read !== null && (lastState === null || !sameReadback(lastState, read))) {
      lastState = read;
      slot.onState?.(read);
    }
  }

  function onFrame(): void {
    handle = null;
    if (!running) {
      return;
    }
    step();
    if (!running || handle !== null) {
      return;
    }
    handle = requestFrame(onFrame);
  }

  const pump: ViewportPump = {
    enqueue(command: PackedCommand): void {
      pending.push(command);
    },
    start(): void {
      if (running) {
        return;
      }
      running = true;
      handle = requestFrame(onFrame);
    },
    stop(): void {
      running = false;
      if (handle === null) {
        return;
      }
      cancelFrame(handle);
      handle = null;
    },
    flushNow(): void {
      step();
    },
    checkpoint(): ViewportCheckpoint | null {
      const snapshot = session.snapshot();
      if (snapshot.byteLength === 0) {
        session.snapshot_error();
        return null;
      }
      return {
        snapshot,
        sliceIndex: lastState?.sliceIndex ?? 0,
      };
    },
    restore(checkpoint: ViewportCheckpoint): boolean {
      if (session.restore_snapshot(checkpoint.snapshot) !== 0) {
        return false;
      }
      pending.push({ kind: "Scroll", step: checkpoint.sliceIndex });
      step();
      return true;
    },
  };
  handlers.set(pump, slot);
  return pump;
}
