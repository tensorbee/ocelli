// What the pinned cornerstone3D cannot do, written down.
//
// The plan's step 6: every row is attempted, and a row the reference cannot
// decode produces a NAMED entry in the committed `unsupported.json` rather
// than a skip. "This is the difference between an oracle that covers 30 rows
// and knows it, and one that covers 30 rows and reports 91."
//
// The match below is strict in both directions on purpose.
//
//  * A row that fails with a reason no entry describes fails the run. A file
//    of excuses that grows to fit each new failure is not a record of the
//    reference's limits, it is a way of never seeing one.
//  * A row an entry claims cannot work, that then works, ALSO fails the run.
//    A stale entry reads as a known limit and hides a coverage gain.

import { readFileSync } from "node:fs";

import { oraclePath } from "./paths.mjs";

export const UNSUPPORTED_PATH = oraclePath("unsupported.json");

const REQUIRED = [
  "transferSyntax",
  "feature",
  "boundary",
  "errorContains",
  "rows",
  "why",
];

/**
 * Refuse a record that cannot say what it claims.
 *
 * Separate from the read so that `tests/unsupported_test.mjs` exercises THIS
 * function rather than a copy of its rules. A test that reimplemented them
 * would pass whatever this file said.
 */
export function validateRecord(parsed) {
  const entries = parsed.entries ?? [];
  entries.forEach((entry, index) => {
    for (const key of REQUIRED) {
      if (entry[key] === undefined) {
        throw new Error(
          `unsupported.json entry ${index} has no ${key}. Every field is ` +
            `load-bearing: without them the file cannot say what failed, ` +
            `where, or which rows it accounts for.`,
        );
      }
    }
    if (typeof entry.errorContains !== "string" || entry.errorContains === "") {
      throw new Error(
        `unsupported.json entry ${index} has an empty errorContains. Every ` +
          `string contains the empty string, so that would retire the fourth ` +
          `conjunct while looking like a complete entry.`,
      );
    }
    if (
      !Array.isArray(entry.rows) ||
      entry.rows.length === 0 ||
      entry.rows.some((row) => typeof row !== "string" || row === "")
    ) {
      throw new Error(
        `unsupported.json entry ${index} lists no rows, or lists an empty ` +
          `one. An entry that accounts for nothing is an excuse, not a record.`,
      );
    }
  });
  return parsed;
}

/** Read and validate the committed record. */
export function readUnsupported() {
  return validateRecord(JSON.parse(readFileSync(UNSUPPORTED_PATH, "utf8")));
}

/**
 * The entry accounting for one row's failure, or null.
 *
 * @param {object} record from `readUnsupported`
 * @param {{path: string, transferSyntax: string}} row
 * @param {{boundary: string, error: string}} failure
 */
export function entryFor(record, row, failure) {
  return (
    (record.entries ?? []).find(
      (entry) =>
        entry.transferSyntax === row.transferSyntax &&
        entry.boundary === failure.boundary &&
        entry.rows.includes(row.path) &&
        String(failure.error ?? "").includes(entry.errorContains),
    ) ?? null
  );
}

/** Every row path any entry claims. */
export function claimedRows(record) {
  const claimed = new Set();
  for (const entry of record.entries ?? []) {
    for (const path of entry.rows) {
      claimed.add(path);
    }
  }
  return claimed;
}
