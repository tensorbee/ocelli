import { describe as group, expect, it } from "vitest";

import {
  decodeRecord,
  describeError,
  ERROR_CODE,
  LOG_LEVEL,
  RECORD_BYTES,
  SEVERITY,
} from "./errors.js";

/**
 * **These three byte arrays are copied from the Rust test, character for
 * character**, and the claim is checkable: `ERROR_BYTES`, `LOG_BYTES` and
 * `TWO_OPERAND_BYTES` below are byte-for-byte the constants of the same names
 * in `crates/ocelli-core/src/error.rs`. That file writes them out by hand from
 * the layout table, which is derived from HLD section 17.3's `Event::payload`,
 * and the two sides are the two implementations that have to agree. Neither is
 * produced by running the other, and neither is produced by running its own
 * encoder.
 *
 * The claim was false between the S03 review's fourth pass and its eighth:
 * `LOG_BYTES` held `0x03` at offset 2 here and `0x05` there. See that constant
 * for what the difference cost.
 *
 * `ERROR_BYTES`: code 1, severity Fatal (2), arity 1, reserved 0,
 * a0 = 0x0102030405060708.
 */
const ERROR_BYTES = new Uint8Array([
  0x01, 0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, 0x04,
  0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

/**
 * `LOG_BYTES`: code 700, level Trace (5), arity 3, operands 1, 2, u64::MAX.
 *
 * **`Trace` and not `Info`, and the choice is the test.** Bytes 2 and 3 are
 * adjacent single bytes carrying different meanings, so a fixture whose level
 * and arity are the SAME number is symmetric under a swap of them and cannot
 * detect one. This array carried level 3 and arity 3 until the S03 review's
 * eighth pass, while the Rust fixture it claims to copy carried `05` at offset
 * 2 from the fourth pass onward. Reading `severityOrLevel` from byte 3 and
 * `arity` from byte 2 left this test green, so the shell's half of the wire
 * contract was guarded by `ERROR_BYTES` alone, whose severity is 2 and whose
 * arity is 1.
 */
const LOG_BYTES = new Uint8Array([
  0xbc, 0x02, 0x05, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff,
  0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
]);

/**
 * `TWO_OPERAND_BYTES`: code 701, severity Recoverable (1), arity 2, operands
 * 512 and 256.
 *
 * The arity the other two fixtures do not carry. `ERROR_BYTES` carries one
 * operand and `LOG_BYTES` carries three, so byte 3 was never observed holding
 * `2` on either side of the boundary, and the middle row of the Rust
 * constructor's arity table could return `1` with both suites green. A
 * consumer would then drop the second operand, which is the quiet loss the
 * constructor's own documentation says it refuses to perform.
 *
 * Severity `Recoverable` and not `Fatal`, so byte 2 and byte 3 differ here
 * too, for `LOG_BYTES`' reason.
 */
const TWO_OPERAND_BYTES = new Uint8Array([
  0xbd, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

function withByte(bytes: Uint8Array, offset: number, value: number): Uint8Array {
  const copy = new Uint8Array(bytes);
  copy.set([value], offset);
  return copy;
}

group("decodeRecord", () => {
  it("reads the same fields the Rust test asserts, for an error", () => {
    expect(decodeRecord(ERROR_BYTES)).toEqual({
      code: 1,
      severityOrLevel: SEVERITY.Fatal,
      arity: 1,
      reserved: 0,
      operands: [0x0102030405060708n, 0n, 0n],
    });
  });

  it("reads the same fields the Rust test asserts, for a log line", () => {
    expect(decodeRecord(LOG_BYTES)).toEqual({
      code: 700,
      severityOrLevel: LOG_LEVEL.Trace,
      arity: 3,
      reserved: 0,
      operands: [1n, 2n, 18446744073709551615n],
    });
  });

  /**
   * Byte 2 is the level and byte 3 is the arity, and neither fixture above can
   * be read the other way round: `ERROR_BYTES` holds 2 then 1, `LOG_BYTES`
   * holds 5 then 3, and `TWO_OPERAND_BYTES` holds 1 then 2. Asserted as one
   * test as well, because the property is about the pair of offsets rather
   * than about any single record.
   */
  it("takes the level from byte 2 and the arity from byte 3", () => {
    expect(decodeRecord(ERROR_BYTES)?.severityOrLevel).toBe(SEVERITY.Fatal);
    expect(decodeRecord(ERROR_BYTES)?.arity).toBe(1);
    expect(decodeRecord(LOG_BYTES)?.severityOrLevel).toBe(LOG_LEVEL.Trace);
    expect(decodeRecord(LOG_BYTES)?.arity).toBe(3);
    expect(decodeRecord(TWO_OPERAND_BYTES)?.severityOrLevel).toBe(
      SEVERITY.Recoverable,
    );
    expect(decodeRecord(TWO_OPERAND_BYTES)?.arity).toBe(2);
  });

  /**
   * Two meaningful operands. The arity no other fixture carries, and the one
   * the Rust constructor's table could get wrong without either suite noticing.
   */
  it("reads the same fields the Rust test asserts, for two operands", () => {
    expect(decodeRecord(TWO_OPERAND_BYTES)).toEqual({
      code: 701,
      severityOrLevel: SEVERITY.Recoverable,
      arity: 2,
      reserved: 0,
      operands: [512n, 256n, 0n],
    });
  });

  /**
   * A zeroed payload is never a record. That is what lets a caller treat "no
   * record" and "an all-zero region" as one answer, and it is why `0` is not a
   * code and not a severity and not a level.
   */
  it("refuses an all-zero payload", () => {
    expect(decodeRecord(new Uint8Array(RECORD_BYTES))).toBeNull();
  });

  it("refuses code 0", () => {
    const bytes = withByte(withByte(ERROR_BYTES, 0, 0), 1, 0);
    expect(decodeRecord(bytes)).toBeNull();
  });

  it("refuses severity or level 0", () => {
    expect(decodeRecord(withByte(ERROR_BYTES, 2, 0))).toBeNull();
  });

  it("refuses an arity above three", () => {
    expect(decodeRecord(withByte(ERROR_BYTES, 3, 4))).toBeNull();
  });

  it("refuses a payload shorter than the layout", () => {
    expect(decodeRecord(ERROR_BYTES.subarray(0, RECORD_BYTES - 1))).toBeNull();
  });

  /**
   * Reported, not refused and not dropped. HLD section 17.3's rule for
   * `dropped`, applied to the one field a newer core may use. The Rust test
   * `a_nonzero_reserved_decodes_and_is_reported` asserts the same thing.
   */
  it("decodes a nonzero reserved and reports it", () => {
    const decoded = decodeRecord(withByte(ERROR_BYTES, 4, 0x2a));
    expect(decoded?.reserved).toBe(0x2a);
    expect(decoded?.code).toBe(1);
    expect(decoded?.operands[0]).toBe(0x0102030405060708n);
  });

  /**
   * An unregistered code is not a corrupt record. A newer core may send one.
   */
  it("decodes an unregistered code", () => {
    const bytes = withByte(withByte(ERROR_BYTES, 0, 0xfe), 1, 0xff);
    expect(decodeRecord(bytes)?.code).toBe(0xfffe);
  });

  /**
   * The record may arrive as a window onto a larger buffer, which is what
   * draining the event ring produces: `payload` is thirty-two bytes at an
   * offset inside a forty-eight byte event. Reading it must respect that
   * offset rather than reading from the start of the buffer.
   */
  it("reads from the payload's own offset inside a larger buffer", () => {
    const framed = new Uint8Array(16 + RECORD_BYTES + 16);
    framed.set(ERROR_BYTES, 16);
    const payload = framed.subarray(16, 16 + RECORD_BYTES);
    expect(decodeRecord(payload)?.code).toBe(1);
    expect(decodeRecord(payload)?.operands[0]).toBe(0x0102030405060708n);
  });
});

group("the code mirror", () => {
  /**
   * The numbers are `ci/error-codes.json`'s, asserted as literals.
   * `scripts/error_code_check.py` is what compares the three files. This is
   * what catches the mirror drifting away from what a test was written
   * against.
   */
  it("carries the registered numbers", () => {
    expect(ERROR_CODE.Panicked).toBe(1);
    expect(ERROR_CODE.Unavailable).toBe(700);
    expect(ERROR_CODE.Workgroup).toBe(701);
  });

  it("reserves zero for severity and for level", () => {
    expect(SEVERITY.Recoverable).toBe(1);
    expect(SEVERITY.Fatal).toBe(2);
    expect(LOG_LEVEL.Error).toBe(1);
    expect(LOG_LEVEL.Trace).toBe(5);
    expect(Object.values(SEVERITY)).not.toContain(0);
    expect(Object.values(LOG_LEVEL)).not.toContain(0);
  });
});

group("describeError", () => {
  it("has a sentence for every registered code", () => {
    for (const code of Object.values(ERROR_CODE)) {
      expect(describeError(code)).not.toBe("");
      expect(describeError(code)).not.toContain("does not recognise");
    }
  });

  /**
   * An error rendered as nothing at all is the same failure as section 23's
   * silent blank canvas, one layer up. So an unknown code gets a sentence
   * naming its number.
   */
  it("names the number of a code it does not know", () => {
    expect(describeError(4242)).toContain("4242");
  });
});
