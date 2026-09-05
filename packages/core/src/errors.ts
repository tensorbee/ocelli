/**
 * The error model, shell side. HLD section 23.
 *
 *   "Error codes are stable and versioned. The shell switches on the code,
 *    the message is for humans and may change."
 *
 * Both halves of that sentence live here. The code is a number this file
 * mirrors from `ci/error-codes.json`, and the human text is a table this file
 * owns. `scripts/error_code_check.py` refuses a disagreement between the two
 * and `crates/ocelli-core/src/error.rs`.
 *
 * **The message is on this side deliberately.** The wasm module is measured
 * against `ci/wasm-size-budget.json` and built with `opt-level = "z"` and
 * `strip = true`, so format strings and the `core::fmt` machinery to assemble
 * them are exactly the weight that budget exists to notice. And "may change"
 * means a message that crossed the boundary would become a contract the moment
 * a consumer matched on it.
 *
 * Pure. Nothing here touches wasm, and `decodeRecord` is handed bytes that
 * have already been copied out of linear memory. See `./panic.js` for the one
 * read that has not.
 */

/**
 * The stable numeric codes, mirroring `ErrorCode` in
 * `crates/ocelli-core/src/error.rs` and `ci/error-codes.json`.
 *
 * Only codes with a live producer today. A fourth arrives with the story whose
 * code it is, as one appended line in three files.
 */
export const ERROR_CODE = {
  /** A Rust panic reached the hook. The instance is poisoned. */
  Panicked: 1,
  /** The feature cannot run on the resolved tier and declares no fallback. */
  Unavailable: 700,
  /** The kernel asked for a workgroup this device cannot dispatch. */
  Workgroup: 701,
} as const;

/**
 * The human text, keyed on the code. Section 23 says this may change, so
 * nothing may match on it. Match on the code.
 */
const MESSAGES: Readonly<Record<number, string>> = {
  1: "The imaging core stopped and cannot be reused. The viewport is being rebuilt.",
  700: "This feature cannot run on this machine's graphics capability.",
  701: "This device cannot dispatch the workgroup size this operation needs.",
};

/** Byte 2 of an error record. `0` is reserved, so a zeroed payload is not one. */
export const SEVERITY = {
  /** The call failed and the instance is fine. */
  Recoverable: 1,
  /** The instance is poisoned. No further call is made into it. */
  Fatal: 2,
} as const;

/** Byte 2 of a log record. `0` is reserved, for `SEVERITY`'s reason. */
export const LOG_LEVEL = {
  Error: 1,
  Warn: 2,
  Info: 3,
  Debug: 4,
  Trace: 5,
} as const;

/**
 * The payload width. HLD section 17.3's `Event` carries `payload: [u8; 32]`,
 * and an error and a log line are both exactly that.
 */
export const RECORD_BYTES = 32;

/** The three operand slots, whatever `arity` says is meaningful. */
export type RecordOperands = readonly [bigint, bigint, bigint];

/**
 * One decoded record. The same thirty-two bytes carry an error and a log line,
 * and `Event.kind` in the ring header is what separates them, so the kind is
 * not repeated here.
 */
export interface OcelliRecord {
  /** Never 0. */
  readonly code: number;
  /** A `SEVERITY` for an error, a `LOG_LEVEL` for a log line. Never 0. */
  readonly severityOrLevel: number;
  /** How many of `operands` are meaningful. At most 3. */
  readonly arity: number;
  /**
   * The `u32` at offset 4. A producer writes 0. A nonzero value means a newer
   * core wrote something this build does not understand, and it is reported
   * rather than refused or dropped, which is HLD section 17.3's rule for
   * `dropped` applied to the one reserved field.
   */
  readonly reserved: number;
  readonly operands: RecordOperands;
}

/**
 * Read the thirty-two bytes of a record, little-endian throughout.
 *
 * Returns `null` for anything that is not a record: a payload shorter than
 * thirty-two bytes, code 0, severity or level 0, or an arity above three.
 * A zeroed payload is therefore never a record, which is what lets a caller
 * treat "no record" and "an all-zero region" as the same answer.
 *
 * An unregistered code decodes. A newer core may send one, and the honest
 * answer is that the number is unknown rather than that the bytes are corrupt.
 * `describeError` says so.
 */
export function decodeRecord(payload: Uint8Array): OcelliRecord | null {
  if (payload.byteLength < RECORD_BYTES) {
    return null;
  }
  const view = new DataView(
    payload.buffer,
    payload.byteOffset,
    RECORD_BYTES,
  );
  const code = view.getUint16(0, true);
  const severityOrLevel = view.getUint8(2);
  const arity = view.getUint8(3);
  if (code === 0 || severityOrLevel === 0 || arity > 3) {
    return null;
  }
  return {
    code,
    severityOrLevel,
    arity,
    reserved: view.getUint32(4, true),
    operands: [
      view.getBigUint64(8, true),
      view.getBigUint64(16, true),
      view.getBigUint64(24, true),
    ] as const,
  };
}

/**
 * The human sentence for a code.
 *
 * Named `describeError` rather than `describe`, which is what
 * `.claude/plans/F-005-design.md` item E calls it informally. The package root
 * re-exports every name in this file, and a bare `describe` at the root of a
 * published package collides with the one every test runner already has.
 *
 * An unknown code gets a sentence naming its number rather than an empty
 * string. Section 23's last bullet is that a poisoned instance "surfaces to
 * the user as a viewport-level error state, never as a silent blank canvas",
 * and an error rendered as nothing at all is the same failure one layer up.
 */
export function describeError(code: number): string {
  return (
    MESSAGES[code] ??
    `The imaging core reported error code ${String(code)}, which this build ` +
      `does not recognise.`
  );
}
