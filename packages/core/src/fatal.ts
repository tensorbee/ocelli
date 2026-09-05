/**
 * The core's status, shell side. HLD section 23.
 *
 *   "on panic the worker instance is torn down and rebuilt, with the
 *    JavaScript shell reconstructing viewport state from its own copy. That
 *    last clause is a design constraint on the shell, not an afterthought -
 *    the shell must always hold enough state to rebuild a viewport from
 *    nothing."
 *
 * and
 *
 *   "A poisoned instance surfaces to the user as a viewport-level error state,
 *    never as a silent blank canvas."
 *
 * **This file is deliberately small, and what it does not contain is the
 * point.** It does not terminate a worker, spawn a replacement, or replay
 * viewport state. Those need a worker and a viewport, and this repository has
 * neither: F-101 (E16.2) builds the boundary and the workers, and F-100
 * (E16.1) designs the public API and with it the shell's replayable copy.
 *
 * What it encodes is the rule that can be true today and has to be true before
 * either exists: **once the status is `fatal` it never returns to `ok`, and no
 * further call is made into that instance.** A new instance gets a new status,
 * which is what makes the latch safe rather than terminal.
 *
 * No class and no interface for the status. A discriminated union reduces the
 * cases a reader must consider, and an interface with one implementer
 * increases the places they must look, which is AGENTS.md's structural test.
 */

import { describeError } from "./errors.js";
import { PANIC_FALLBACK_CODE, type PanicRecord } from "./panic.js";

/**
 * Whether the core behind one instance can still be called.
 *
 * `location` is `file:line:column` from the panic record, or `null`. It is
 * carried for a log or a bug report and is not shown to a user.
 */
export type CoreStatus =
  | { readonly kind: "ok" }
  | {
      readonly kind: "fatal";
      readonly code: number;
      readonly message: string;
      readonly location: string | null;
    };

/** The status a freshly instantiated core starts in. */
export const CORE_OK: CoreStatus = { kind: "ok" };

/**
 * Whether a call into this instance is allowed.
 *
 * There is exactly one place this is false and it is permanent for that
 * instance. Section 23: the instance "must not be reused".
 */
export function isUsable(status: CoreStatus): boolean {
  return status.kind === "ok";
}

/**
 * The fatal status a panic record describes.
 *
 * The message comes from `describeError`, which is the shell's table, because
 * section 23 puts the human text on this side. The core's own panic text is
 * developer detail and is not what a user is shown.
 */
export function fatalFromPanic(record: PanicRecord | null): CoreStatus {
  if (record === null) {
    // A trapped instance whose record could not be read is still a dead
    // instance. Reporting "ok" here would be the silent blank canvas section
    // 23's last bullet forbids.
    return {
      kind: "fatal",
      code: PANIC_FALLBACK_CODE,
      message: describeError(PANIC_FALLBACK_CODE),
      location: null,
    };
  }
  return {
    kind: "fatal",
    code: record.code,
    message: describeError(record.code),
    location: record.location,
  };
}

/**
 * The next status, given what the shell just learned.
 *
 * **This is the latch, and it is the whole reason this function exists rather
 * than an assignment.** A fatal status absorbs everything, including another
 * fatal one: the first failure is the one that explains what happened, and a
 * later report is a consequence of it. A caller that could assign the status
 * directly would eventually assign `ok` after a retry, and then a poisoned
 * instance would be called again.
 */
export function nextStatus(
  current: CoreStatus,
  observed: CoreStatus,
): CoreStatus {
  if (current.kind === "fatal") {
    return current;
  }
  return observed;
}
