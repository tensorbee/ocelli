/**
 * Migration order. F-116, HLD section 12.
 *
 * The next kind is returned only when the done list is a prefix of
 * stack, then mpr, then volume. The function does not rewrite the list
 * and it does not flip a flag.
 */

import type { ViewportKind } from "./flag.js";

const ORDER: readonly ViewportKind[] = ["stack", "mpr", "volume"];

export function nextMigrationKind(
  done: readonly string[],
): ViewportKind | null {
  if (done.length > ORDER.length) {
    return null;
  }
  for (let index = 0; index < done.length; index += 1) {
    const expected = ORDER[index];
    if (done[index] !== expected) {
      return null;
    }
  }
  const next = ORDER[done.length];
  if (next === undefined) {
    return null;
  }
  return next;
}
