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

/** `RingHeader` in `ocelli-viewport/src/ring.rs`, four u32 fields. */
export const HEADER_BYTES = 16;

/** `Event` in `ocelli-viewport/src/ring.rs`: u32 + u32 + u64 + [u8; 32]. */
export const EVENT_STRIDE = 48;

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
    kind: view.getUint32(offset, true),
    viewport: view.getUint32(offset + 4, true),
    seq: view.getBigUint64(offset + 8, true),
    payload: new Uint8Array(
      view.buffer.slice(
        view.byteOffset + offset + 16,
        view.byteOffset + offset + 48,
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
  if (copied.byteLength < HEADER_BYTES) {
    return { events: [], dropped: 0 };
  }
  const view = new DataView(
    copied.buffer,
    copied.byteOffset,
    copied.byteLength,
  );
  const pending = view.getUint32(0, true);
  const dropped = view.getUint32(12, true);
  const bytes = HEADER_BYTES + pending * EVENT_STRIDE;
  if (copied.byteLength < bytes) {
    return { events: [], dropped };
  }
  const events: OcelliEvent[] = [];
  for (let index = 0; index < pending; index += 1) {
    events.push(readEvent(view, HEADER_BYTES + index * EVENT_STRIDE));
  }
  return { events, dropped };
}
