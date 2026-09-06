import { describe as group, expect, it } from "vitest";

import { EVENT_STRIDE, HEADER_BYTES, readEvent } from "./ring.js";

/**
 * HLD section 17.3, the event ring's consumer side.
 *
 * The specification gives the layout as two `#[repr(C)]` structs:
 *
 *   RingHeader { head: u32, tail: u32, cap: u32, dropped: u32 }
 *   Event      { kind: u32, viewport: u32, seq: u64, payload: [u8; 32] }
 *   "48-byte stride. JS reads with a DataView at header_size + i * 48."
 *
 * So the header is 16 bytes and an event is 4 + 4 + 8 + 32 = 48, with the
 * fields at 0, 4, 8 and 16.
 *
 * **Every offset and every size below is typed in from that paragraph rather
 * than imported from `ring.ts`.** A test that laid its fixture out using
 * `EVENT_STRIDE` would move the writer and the reader together and assert
 * nothing: mutating the constant from 48 to 64 would keep it green. Measured
 * in the S03 sprint review's ninth pass, along with two surviving offset
 * mutations, on a `readEvent` that is exported from `@ocelli/core`'s index and
 * had no test at all.
 *
 * **The wire contract itself is not yet checkable and the file says so.**
 * `crates/ocelli-wasm/src/ring.rs` does not exist, F-101 writes it, so
 * nothing here can compare these numbers against the producer. What this file
 * covers is the other half: that the arithmetic `ring.ts` performs matches the
 * layout the HLD specifies. When F-101 lands the Rust side, a fixture
 * generated from it replaces the hand-written bytes below and closes the gap.
 */

/** `RingHeader`, four `u32` fields. HLD 17.3. */
const SPEC_HEADER_BYTES = 16;

/** `Event`, `u32 + u32 + u64 + [u8; 32]`. HLD 17.3. */
const SPEC_EVENT_STRIDE = 48;

/** Field offsets within one event, from the struct's field order. */
const SPEC_KIND_AT = 0;
const SPEC_VIEWPORT_AT = 4;
const SPEC_SEQ_AT = 8;
const SPEC_PAYLOAD_AT = 16;
const SPEC_PAYLOAD_BYTES = 32;

interface EventFields {
  readonly kind: number;
  readonly viewport: number;
  readonly seq: bigint;
  readonly payload: readonly number[];
}

/**
 * Lay events out by hand at the specified stride, into a buffer with `pad`
 * bytes in front of the ring.
 *
 * The pad is not decoration. `readEvent` reads through `view.byteOffset`, and
 * a `DataView` starting at byte 0 makes that term zero, so an implementation
 * that dropped it would stay green. The real ring lives at some offset inside
 * linear memory and never at its start.
 *
 * Every byte outside a written field is `0xcd`, so a read that strays into a
 * neighbouring field or past the end of the payload produces a recognisable
 * value rather than a plausible zero.
 */
function ringOf(events: readonly EventFields[], pad: number): DataView {
  const ringBytes = SPEC_HEADER_BYTES + events.length * SPEC_EVENT_STRIDE;
  const buffer = new ArrayBuffer(pad + ringBytes + SPEC_EVENT_STRIDE);
  const bytes = new Uint8Array(buffer);
  bytes.fill(0xcd);
  const view = new DataView(buffer, pad, ringBytes);

  events.forEach((event, index) => {
    const at = SPEC_HEADER_BYTES + index * SPEC_EVENT_STRIDE;
    view.setUint32(at + SPEC_KIND_AT, event.kind, true);
    view.setUint32(at + SPEC_VIEWPORT_AT, event.viewport, true);
    view.setBigUint64(at + SPEC_SEQ_AT, event.seq, true);
    for (let i = 0; i < SPEC_PAYLOAD_BYTES; i += 1) {
      view.setUint8(at + SPEC_PAYLOAD_AT + i, event.payload[i] ?? 0);
    }
  });

  return view;
}

/** Distinct in every byte, so a one-byte slice slip is visible. */
function payload(seed: number): number[] {
  return Array.from({ length: SPEC_PAYLOAD_BYTES }, (_, i) => (seed + i) & 0xff);
}

const FIRST: EventFields = {
  kind: 0x01020304,
  viewport: 0x11223344,
  seq: 0x0102030405060708n,
  payload: payload(0x40),
};

const SECOND: EventFields = {
  kind: 0xdeadbeef,
  viewport: 0xfeedface,
  seq: 0xfffffffffffffffen,
  payload: payload(0x80),
};

group("the ring layout, HLD 17.3", () => {
  /**
   * The two exported constants against the struct definitions. `RingHeader`
   * is four `u32`, `Event` is `u32 + u32 + u64 + [u8; 32]`, and the HLD states
   * the stride in prose as well: "48-byte stride".
   */
  it("matches the struct sizes the HLD specifies", () => {
    expect(HEADER_BYTES).toBe(SPEC_HEADER_BYTES);
    expect(EVENT_STRIDE).toBe(SPEC_EVENT_STRIDE);
    expect(EVENT_STRIDE).toBe(4 + 4 + 8 + SPEC_PAYLOAD_BYTES);
  });
});

group("readEvent, HLD 17.3", () => {
  /** Every field of the first event, at the offsets the struct gives. */
  it("decodes each field at its specified offset", () => {
    const view = ringOf([FIRST], 24);

    const event = readEvent(view, SPEC_HEADER_BYTES);

    expect(event.kind).toBe(FIRST.kind);
    expect(event.viewport).toBe(FIRST.viewport);
    expect(event.seq).toBe(FIRST.seq);
    expect(Array.from(event.payload)).toEqual(FIRST.payload);
  });

  /**
   * `kind` and `viewport` are adjacent `u32`s and are the pair an offset slip
   * silently swaps. They are given different values here for that reason, and
   * a `seq` whose low four bytes are neither of them.
   */
  it("does not confuse kind with viewport", () => {
    const view = ringOf([SECOND], 8);

    const event = readEvent(view, SPEC_HEADER_BYTES);

    expect(event.kind).toBe(SECOND.kind);
    expect(event.viewport).toBe(SECOND.viewport);
    expect(event.kind).not.toBe(event.viewport);
  });

  /**
   * Little-endian, which is what wasm linear memory is and what the HLD's
   * `#[repr(C)]` structs are on every target this ships to. Asserted by
   * reading a `seq` whose byte reversal is a different number.
   */
  it("reads the fields little-endian", () => {
    const view = ringOf([FIRST], 0);

    const event = readEvent(view, SPEC_HEADER_BYTES);

    expect(event.seq).toBe(0x0102030405060708n);
    expect(event.seq).not.toBe(0x0807060504030201n);
  });

  /**
   * `seq` is a `u64` and the whole range must survive. A number-typed read
   * would lose the low bits of a value this large, and `2n ** 64n - 2n` is
   * chosen so that both halves are nonzero and the value is not
   * representable exactly as a `Number`.
   */
  it("carries the full u64 sequence", () => {
    const view = ringOf([SECOND], 16);

    const event = readEvent(view, SPEC_HEADER_BYTES);

    expect(event.seq).toBe(2n ** 64n - 2n);
    expect(typeof event.seq).toBe("bigint");
  });

  /**
   * The second event, read at `header + stride`, which is the arithmetic the
   * HLD spells out: "JS reads with a DataView at header_size + i * 48". The
   * caller supplies the offset, so this is what catches a payload slice that
   * ignores the offset it was given.
   */
  it("reads the event at index i from header plus i strides", () => {
    const view = ringOf([FIRST, SECOND], 32);

    const event = readEvent(view, SPEC_HEADER_BYTES + SPEC_EVENT_STRIDE);

    expect(event.kind).toBe(SECOND.kind);
    expect(event.viewport).toBe(SECOND.viewport);
    expect(event.seq).toBe(SECOND.seq);
    expect(Array.from(event.payload)).toEqual(SECOND.payload);
  });

  /**
   * The payload is exactly the 32 bytes of the array, neither short nor
   * running into the next event's `kind`. The filler byte is `0xcd` and the
   * payload never contains it, so an over-long slice is caught by value as
   * well as by length.
   */
  it("copies exactly the 32 payload bytes", () => {
    const view = ringOf([FIRST, SECOND], 8);

    const event = readEvent(view, SPEC_HEADER_BYTES);

    expect(event.payload).toBeInstanceOf(Uint8Array);
    expect(event.payload.byteLength).toBe(SPEC_PAYLOAD_BYTES);
    expect(Array.from(event.payload)).toEqual(FIRST.payload);
  });

  /**
   * **The payload is a copy, not a view.** `ring.ts` says so and says why:
   * "Handing back a view would reintroduce exactly the detach hazard
   * `bulk.ts` exists to avoid." Overwriting the ring after the read must not
   * change what was read, and the copy must not alias the ring's buffer.
   */
  it("copies the payload out rather than viewing the ring", () => {
    const view = ringOf([FIRST], 8);

    const event = readEvent(view, SPEC_HEADER_BYTES);
    new Uint8Array(view.buffer).fill(0x5a);

    expect(Array.from(event.payload)).toEqual(FIRST.payload);
    expect(event.payload.buffer).not.toBe(view.buffer);
    expect(event.payload.byteOffset).toBe(0);
  });

  /**
   * A ring that does not start at byte 0 of its buffer, which is the only
   * arrangement the real one has: the ring lives at an address inside linear
   * memory. `readEvent` must add `view.byteOffset` when it slices the
   * payload, and the two events differ in every byte, so a dropped
   * `byteOffset` reads the wrong event or the filler.
   */
  it("respects the view's own byteOffset", () => {
    const flat = ringOf([FIRST, SECOND], 0);
    const shifted = ringOf([FIRST, SECOND], SPEC_EVENT_STRIDE + 5);

    const fromFlat = readEvent(flat, SPEC_HEADER_BYTES);
    const fromShifted = readEvent(shifted, SPEC_HEADER_BYTES);

    expect(Array.from(fromShifted.payload)).toEqual(FIRST.payload);
    expect(Array.from(fromShifted.payload)).toEqual(
      Array.from(fromFlat.payload),
    );
  });
});
