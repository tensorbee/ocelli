import { describe as group, expect, it } from "vitest";

import { ERROR_CODE } from "./errors.js";
import {
  CORE_OK,
  fatalFromPanic,
  isUsable,
  nextStatus,
  type CoreStatus,
} from "./fatal.js";
import type { PanicRecord } from "./panic.js";

const RECORD: PanicRecord = {
  version: 1,
  code: ERROR_CODE.Panicked,
  message: "ocelli panic probe at crates/ocelli-wasm/src/lib.rs:110:5",
  location: "crates/ocelli-wasm/src/lib.rs:110:5",
  truncated: false,
  unknownVersion: false,
};

const FATAL_A: CoreStatus = {
  kind: "fatal",
  code: ERROR_CODE.Panicked,
  message: "first",
  location: null,
};

const FATAL_B: CoreStatus = {
  kind: "fatal",
  code: ERROR_CODE.Unavailable,
  message: "second",
  location: "elsewhere:1:1",
};

/**
 * Every input the transition function has, so "never returns to ok" is
 * asserted over the whole domain rather than over the one case somebody
 * thought of. Two `ok` values and two distinct `fatal` values, which is what
 * makes the fatal-absorbs-fatal case observable: with one fatal value the
 * latch and a plain assignment would be indistinguishable.
 */
const EVERY: readonly CoreStatus[] = [CORE_OK, { kind: "ok" }, FATAL_A, FATAL_B];

group("CoreStatus", () => {
  it("starts usable", () => {
    expect(isUsable(CORE_OK)).toBe(true);
    expect(CORE_OK.kind).toBe("ok");
  });

  it("is not usable once fatal", () => {
    expect(isUsable(FATAL_A)).toBe(false);
  });

  /**
   * HLD section 23: the instance "must not be reused". A status that could go
   * back to `ok` is a status that lets the shell call into a poisoned
   * instance, and the call would be the one that reads inconsistent memory.
   */
  it("never leaves fatal, for any observation", () => {
    for (const observed of EVERY) {
      expect(nextStatus(FATAL_A, observed)).toBe(FATAL_A);
      expect(nextStatus(FATAL_B, observed)).toBe(FATAL_B);
    }
  });

  it("keeps the FIRST fatal, not the latest", () => {
    // The first failure is the one that explains what happened. A later report
    // is a consequence of it.
    expect(nextStatus(FATAL_A, FATAL_B)).toBe(FATAL_A);
  });

  it("takes any observation while it is still ok", () => {
    for (const observed of EVERY) {
      expect(nextStatus(CORE_OK, observed)).toBe(observed);
    }
  });

  /**
   * The whole domain, exhaustively: from every state, to every state, the
   * result is never `ok` unless the current state was `ok`.
   */
  it("reaches ok only from ok", () => {
    for (const current of EVERY) {
      for (const observed of EVERY) {
        const result = nextStatus(current, observed);
        if (result.kind === "ok") {
          expect(current.kind).toBe("ok");
        }
      }
    }
  });
});

group("fatalFromPanic", () => {
  it("carries the record's code and location, and the shell's message", () => {
    const status = fatalFromPanic(RECORD);
    expect(status.kind).toBe("fatal");
    if (status.kind !== "fatal") {
      return;
    }
    expect(status.code).toBe(ERROR_CODE.Panicked);
    expect(status.location).toBe("crates/ocelli-wasm/src/lib.rs:110:5");
    // The user-facing sentence is the shell's table, not the core's panic
    // text. Section 23 puts the message on this side.
    expect(status.message).not.toBe(RECORD.message);
    expect(status.message.length).toBeGreaterThan(0);
  });

  /**
   * A trapped instance whose record could not be read is still a dead
   * instance. Section 23's last bullet forbids surfacing that as a blank
   * canvas, so there is no `ok` answer here.
   */
  it("is still fatal when there is no record to read", () => {
    const status = fatalFromPanic(null);
    expect(status.kind).toBe("fatal");
    if (status.kind !== "fatal") {
      return;
    }
    expect(status.code).toBe(ERROR_CODE.Panicked);
    expect(status.location).toBeNull();
    expect(status.message.length).toBeGreaterThan(0);
  });

  it("never returns ok, whatever it is handed", () => {
    expect(isUsable(fatalFromPanic(RECORD))).toBe(false);
    expect(isUsable(fatalFromPanic(null))).toBe(false);
  });
});
