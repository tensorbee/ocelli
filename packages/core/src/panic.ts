/**
 * Reading the panic record out of linear memory. HLD sections 23 and 17.2.
 *
 * THIS IS THE SECOND FILE PERMITTED TO BUILD A VIEW OVER WASM LINEAR MEMORY.
 * `packages/core/src/bulk.ts` is the first. `eslint.config.js` turns
 * `no-restricted-syntax` off here and in that one file and nowhere else.
 *
 * Section 17.2's own wording is "outside the two functions that are allowed to
 * do it", so the specification expected two. The repository had one only
 * because nothing else needed linear memory yet. Widening the allowance is a
 * design-plan decision and `.claude/plans/F-005-design.md` item F is that
 * decision. **A third is not granted here**, and in particular
 * `packages/core/src/ring.ts` will have to argue for its own in F-101.
 *
 * ## Why the hazard the rule guards cannot occur here
 *
 * The rule exists because wasm memory growth relocates the `ArrayBuffer` and
 * detaches every outstanding view, and the next use then targets a detached
 * buffer or throws far from the cause. This read happens AFTER the instance
 * has trapped, when no wasm code can run, so a growth between the view's
 * construction and its last use is not merely unlikely, it is impossible.
 *
 * The discipline is kept anyway: the view is built inside the function, used
 * immediately, and neither stored nor returned. The `PanicRecord` handed back
 * carries copied bytes and a decoded string.
 *
 * ## Why this reads memory instead of calling an export
 *
 * Section 23: "that module instance's memory may be inconsistent and it must
 * not be reused". An export that returned the panic message after the trap
 * would be reusing a poisoned instance to ask why it was poisoned. So the
 * shell calls `panic_record_ptr()` and `panic_record_len()` once, immediately
 * after instantiation, caches the two integers, and never calls into the
 * instance again. **A cached pointer stays correct across a memory growth and
 * a cached view does not**, which is the whole distinction section 17.2 is
 * about.
 */

import { ERROR_CODE } from "./errors.js";

/**
 * The record's layout, written out rather than derived.
 *
 * `crates/ocelli-wasm/src/panic.rs` states the same numbers and
 * `scripts/panic_probe.mjs` states them a third time. Three statements of one
 * wire contract, none derived from another, is what makes a disagreement
 * visible instead of unanimous.
 *
 * ```text
 * offset  size  field
 * 0       4     magic     u32, 0 until a panic, then PANIC_MAGIC
 * 4       4     version   u32
 * 8       4     code      u32, an ErrorCode number
 * 12      4     msgLen    u32, bytes written, at most MESSAGE_CAPACITY
 * 16      512   msg       UTF-8, not NUL terminated
 *                         total 528
 * ```
 */

/** "OCP1" little-endian: O=0x4F, C=0x43, P=0x50, 1=0x31. */
export const PANIC_MAGIC = 0x3150434f;

/** The layout this build understands. */
export const PANIC_RECORD_VERSION = 1;

/** Message bytes the record holds. Longer messages are truncated by the core. */
export const PANIC_MESSAGE_CAPACITY = 512;

/** Header bytes before the message: magic, version, code, length. */
export const PANIC_HEADER_BYTES = 16;

/** The whole record. `panic_record_len()` returns this. */
export const PANIC_RECORD_BYTES =
  PANIC_HEADER_BYTES + PANIC_MESSAGE_CAPACITY;

/** What a trapped instance left behind. */
export interface PanicRecord {
  /** The layout version the core wrote. */
  readonly version: number;
  /** An `ERROR_CODE` number. `Panicked` today, and always in practice. */
  readonly code: number;
  /** The panic's payload and its file, line and column, decoded leniently. */
  readonly message: string;
  /**
   * `file:line:column`, or `null` when the message carries no location.
   *
   * Parsed from the message rather than stored separately, because the core
   * writes one string and a second field would be a second thing to keep in
   * step for no gain.
   */
  readonly location: string | null;
  /**
   * Whether the core ran out of room. The message is still the message, it is
   * just not all of it.
   */
  readonly truncated: boolean;
  /**
   * Whether the core wrote a layout version this build does not know.
   *
   * Reported rather than refused. A newer core is a real thing and a shell
   * that returned `null` for it would turn "I do not understand this record"
   * into "there was no panic", which is section 23's silent blank canvas.
   */
  readonly unknownVersion: boolean;
}

/** The subset of a module's exports this file needs. Nothing is called. */
export interface PanicMemory {
  readonly memory: WebAssembly.Memory;
}

/** Trailing ` at file:line:column`, which is what the core appends. */
const LOCATION = / at ([^ ]+:\d+:\d+)$/;

/**
 * Read the panic record, or `null` when there is not one.
 *
 * `null` is the ordinary answer for an instance that has not panicked, and it
 * is also the correct answer for a torn record: the core writes the magic
 * LAST, so a hook that panicked part way through leaves the magic at zero and
 * the record reads as absent rather than as valid with garbage in it.
 *
 * `ptr` and `len` are the two integers cached at instantiation. **This
 * function calls no export**, which is the entire reason the design satisfies
 * section 23.
 */
export function readPanicRecord(
  wasm: PanicMemory,
  ptr: number,
  len: number,
): PanicRecord | null {
  if (ptr <= 0 || len < PANIC_RECORD_BYTES) {
    return null;
  }
  const { memory } = wasm;
  if (ptr + len > memory.buffer.byteLength) {
    return null;
  }

  // Built here, used immediately, never stored and never returned.
  const view = new DataView(memory.buffer, ptr, len);
  if (view.getUint32(0, true) !== PANIC_MAGIC) {
    return null;
  }

  const version = view.getUint32(4, true);
  const code = view.getUint32(8, true);
  const rawLength = view.getUint32(12, true);
  const messageLength = Math.min(rawLength, PANIC_MESSAGE_CAPACITY);

  // Copied out. Handing back a view would reintroduce exactly the detach
  // hazard this file's allowance exists to contain.
  const bytes = new Uint8Array(
    memory.buffer.slice(
      ptr + PANIC_HEADER_BYTES,
      ptr + PANIC_HEADER_BYTES + messageLength,
    ),
  );

  // Lenient, not fatal. A truncation can land inside a UTF-8 sequence, and one
  // replacement character is a better answer than a thrown exception while the
  // shell is already handling a dead instance.
  const message = new TextDecoder("utf-8", { fatal: false }).decode(bytes);
  const found = LOCATION.exec(message);

  return {
    version,
    code,
    message,
    location: found === null ? null : (found[1] ?? null),
    truncated: messageLength >= PANIC_MESSAGE_CAPACITY,
    unknownVersion: version !== PANIC_RECORD_VERSION,
  };
}

/**
 * The code a record that could not be read still means.
 *
 * A trapped instance with no readable record is still a dead instance, and
 * section 23's last bullet forbids surfacing that as a blank canvas. So the
 * caller has a code to use either way.
 */
export const PANIC_FALLBACK_CODE: number = ERROR_CODE.Panicked;
