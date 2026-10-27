/**
 * The event ring, consumer side. HLD section 17.3.
 *
 * Single producer (Rust), single consumer (JavaScript). Drained once per
 * animation frame rather than invoking a callback per event, which removes a
 * boundary crossing from the hot path and gives coalescing for free: a burst
 * of camera changes during a drag collapses to one delivered event.
 *
 * Fixed 48-byte stride, so this side is arithmetic and not deserialisation.
 *
 * F-101 (E16.2) built the producer in `crates/ocelli-viewport/src/ring.rs`.
 * `drainRing` reads a buffer that `drain_events` already copied out, never a
 * view over wasm linear memory.
 */

import {
  EVENT_HEADER_BYTES,
  EVENT_KIND_OFFSET,
  EVENT_PAYLOAD_BYTES,
  EVENT_PAYLOAD_OFFSET,
  EVENT_RING_DROPPED_OFFSET,
  EVENT_RING_PENDING_OFFSET,
  EVENT_SEQ_OFFSET,
  EVENT_STRIDE,
  EVENT_VIEWPORT_OFFSET,
} from "./generated.js";

export { EVENT_STRIDE };

/** `RingHeader` in `ocelli-viewport/src/boundary.rs`, four u32 fields. */
export const HEADER_BYTES = EVENT_HEADER_BYTES;

export interface OcelliEvent {
  readonly kind: number;
  readonly viewport: number;
  readonly seq: bigint;
  /** The 32 payload bytes, copied. Never a view over linear memory. */
  readonly payload: Uint8Array;
}

export interface DrainResult {
  readonly events: readonly OcelliEvent[];
  /**
   * The producer's overflow count. Nonzero means JavaScript is not draining
   * fast enough. Surface it in telemetry rather than swallowing it.
   */
  readonly dropped: number;
}

/**
 * Read one event at `offset` from a DataView over the ring.
 *
 * The payload is copied out. Handing back a view would reintroduce exactly
 * the detach hazard `bulk.ts` exists to avoid.
 */
export function readEvent(view: DataView, offset: number): OcelliEvent {
  return {
    kind: view.getUint32(offset + EVENT_KIND_OFFSET, true),
    viewport: view.getUint32(offset + EVENT_VIEWPORT_OFFSET, true),
    seq: view.getBigUint64(offset + EVENT_SEQ_OFFSET, true),
    payload: new Uint8Array(
      view.buffer.slice(
        view.byteOffset + offset + EVENT_PAYLOAD_OFFSET,
        view.byteOffset + offset + EVENT_PAYLOAD_OFFSET + EVENT_PAYLOAD_BYTES,
      ),
    ),
  };
}

/**
 * Drain one copied ring buffer.
 *
 * `copied` is the bytes `drain_events` already copied out. This reads that
 * copy. It does not take a view over wasm linear memory. The copy's first
 * word is the pending count and its tail word is 0, so the events are a
 * linear prefix. A buffer shorter than the declared events yields none.
 */
export function drainRing(copied: Uint8Array): DrainResult {
  if (copied.byteLength < EVENT_HEADER_BYTES) {
    return { events: [], dropped: 0 };
  }
  const view = new DataView(
    copied.buffer,
    copied.byteOffset,
    copied.byteLength,
  );
  const pending = view.getUint32(EVENT_RING_PENDING_OFFSET, true);
  const dropped = view.getUint32(EVENT_RING_DROPPED_OFFSET, true);
  const bytes = EVENT_HEADER_BYTES + pending * EVENT_STRIDE;
  if (copied.byteLength < bytes) {
    return { events: [], dropped };
  }
  const events: OcelliEvent[] = [];
  for (let index = 0; index < pending; index += 1) {
    events.push(readEvent(view, EVENT_HEADER_BYTES + index * EVENT_STRIDE));
  }
  return { events, dropped };
}
