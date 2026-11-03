/**
 * Mechanical listener renames. F-108, HLD section 12.
 *
 * A string literal that is the first argument of addEventListener or
 * removeEventListener is rewritten when the F-103 map has an Ocelli name.
 * Every other token stays. This function does not write a file.
 */

import { CORNERSTONE_EVENT_MAP } from "./events.js";

const pairs: readonly (readonly [string, string])[] = CORNERSTONE_EVENT_MAP.flatMap(
  (row) => (row.ocelli === null ? [] : [[row.cornerstone, row.ocelli] as const]),
);
const RENAMES: ReadonlyMap<string, string> = new Map(pairs);

const LISTENER =
  /(^|[^A-Za-z0-9_])(addEventListener|removeEventListener)(\s*\(\s*)(['"])([A-Z0-9_]+)\4/g;

export function rewriteListenerNames(source: string): string | null {
  if (typeof source !== "string") {
    return null;
  }
  return source.replace(
    LISTENER,
    (whole, prefix: string, call: string, gap: string, quote: string, name: string) => {
      const next = RENAMES.get(name);
      if (next === undefined) {
        return whole;
      }
      return `${prefix}${call}${gap}${quote}${next}${quote}`;
    },
  );
}
