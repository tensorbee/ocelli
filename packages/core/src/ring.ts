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
 * Scaffold. F-096 (E16.2) implements it against the real ring.
 */

/** `RingHeader` in `ocelli-wasm/src/ring.rs`, four u32 fields. */
export const HEADER_BYTES = 16;

/** `Event` in `ocelli-wasm/src/ring.rs`: u32 + u32 + u64 + [u8; 32]. */
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
