// Unit tests for the record of what the pinned cornerstone3D cannot render.
//
// `entryFor` is a four-way conjunct: transfer syntax, boundary, row path, and a
// fragment of the error. `docs/lld/oracle.md` makes that strictness a
// load-bearing claim, and it is what stops one entry absorbing another row's
// unrelated failure as the file grows.
//
// **Every one of the four is redundant today**, because the two committed
// entries differ in all four fields, so any one of them alone separates them
// and none would go red if it were deleted. Measured by replaying the two
// observed failures against the record with each conjunct removed in turn.
// That is exactly the shape of a guard nobody has watched fail, and it is why
// each conjunct gets its own test below rather than one test of the whole.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  claimedRows,
  entryFor,
  readUnsupported,
  UNSUPPORTED_PATH,
  validateRecord,
} from "../src/unsupported.mjs";

const ENTRY = {
  transferSyntax: "1.2.840.10008.1.2.1",
  feature: "Photometric Interpretation YBR_FULL_422",
  boundary: "read-back",
  errorContains: "every pixel of the frame is rgba(0,0,0,255)",
  rows: ["synthetic/us_ybr_full_422.dcm"],
  why: "because the standard says so",
};

const RECORD = { cornerstone3DVersion: "5.8.2", entries: [ENTRY] };

const ROW = {
  path: "synthetic/us_ybr_full_422.dcm",
  transferSyntax: "1.2.840.10008.1.2.1",
};

const FAILURE = {
  boundary: "read-back",
  error:
    "read back: every pixel of the frame is rgba(0,0,0,255). A blank canvas " +
    "reads back perfectly.",
};

test("a row that matches on all four counts is accounted for", () => {
  assert.equal(entryFor(RECORD, ROW, FAILURE), ENTRY);
});

// Each of the four conjuncts, alone. A row failing for a reason no entry
// describes fails the run, and the file cannot grow into a list of excuses.
test("a different transfer syntax is not accounted for", () => {
  const row = { ...ROW, transferSyntax: "1.2.840.10008.1.2" };
  assert.equal(entryFor(RECORD, row, FAILURE), null);
});

// The boundary is the conjunct that stops an entry absorbing a DIFFERENT
// failure of the same row. The YBR entry says the load resolves and the frame
// reads back blank. A decode failure of that row is news.
test("the same row failing at another boundary is not accounted for", () => {
  const failure = { boundary: "decoded", error: FAILURE.error };
  assert.equal(entryFor(RECORD, ROW, failure), null);
});

test("a row the entry does not list is not accounted for", () => {
  const row = { ...ROW, path: "syntax/explicit_vr_le.dcm" };
  assert.equal(entryFor(RECORD, row, FAILURE), null);
});

test("the same boundary with another error is not accounted for", () => {
  const failure = { boundary: "read-back", error: "the canvas was the wrong size" };
  assert.equal(entryFor(RECORD, ROW, failure), null);
});

test("a missing error is not treated as a match", () => {
  assert.equal(entryFor(RECORD, ROW, { boundary: "read-back" }), null);
});

test("an empty record accounts for nothing", () => {
  assert.equal(entryFor({ entries: [] }, ROW, FAILURE), null);
  assert.equal(entryFor({}, ROW, FAILURE), null);
});

// ---------------------------------------------------------------------------
// The committed file, and the shape it has to keep
// ---------------------------------------------------------------------------

test("every field is required, because each carries part of the claim", () => {
  for (const field of [
    "transferSyntax",
    "feature",
    "boundary",
    "errorContains",
    "rows",
    "why",
  ]) {
    const entries = [{ ...ENTRY }];
    delete entries[0][field];
    assert.throws(
      () => validateRecord({ ...RECORD, entries }),
      new RegExp(field),
      `an entry missing ${field} was accepted`,
    );
  }
});

test("an entry that accounts for no row is refused", () => {
  assert.throws(
    () => validateRecord({ ...RECORD, entries: [{ ...ENTRY, rows: [] }] }),
    /no rows/,
  );
  assert.throws(
    () => validateRecord({ ...RECORD, entries: [{ ...ENTRY, rows: "a-string" }] }),
    /no rows/,
  );
});

// The empty string is contained by every string, so `String(error).includes("")`
// is always true. An entry with an empty `errorContains` would validate,
// account for its rows, and retire the fourth conjunct while looking complete.
test("an empty errorContains is refused, not treated as a wildcard", () => {
  assert.throws(
    () => validateRecord({ ...RECORD, entries: [{ ...ENTRY, errorContains: "" }] }),
    /empty errorContains/,
  );
  assert.throws(
    () => validateRecord({ ...RECORD, entries: [{ ...ENTRY, errorContains: 7 }] }),
    /empty errorContains/,
  );
  // And it really would match anything, which is why the refusal is there.
  assert.equal(
    entryFor(
      { entries: [{ ...ENTRY, errorContains: "" }] },
      ROW,
      { boundary: "read-back", error: "something else entirely" },
    )?.feature,
    ENTRY.feature,
  );
});

// A row path that is the empty string is in the list and accounts for nothing,
// which is the same failure as an empty list wearing a different shape.
test("an empty row path is refused", () => {
  assert.throws(
    () => validateRecord({ ...RECORD, entries: [{ ...ENTRY, rows: [""] }] }),
    /lists no rows, or lists an empty one/,
  );
  assert.throws(
    () =>
      validateRecord({
        ...RECORD,
        entries: [{ ...ENTRY, rows: ["synthetic/a.dcm", ""] }],
      }),
    /lists no rows, or lists an empty one/,
  );
});

test("claimedRows names every path any entry claims", () => {
  const record = {
    entries: [ENTRY, { ...ENTRY, rows: ["a.dcm", "b.dcm"] }],
  };
  assert.deepEqual(
    [...claimedRows(record)].sort(),
    ["a.dcm", "b.dcm", "synthetic/us_ybr_full_422.dcm"],
  );
  assert.equal(claimedRows({}).size, 0);
});

// The committed file itself, read through the same reader the run uses.
test("the committed record validates and names the version it describes", () => {
  const record = readUnsupported();
  assert.equal(record.cornerstone3DVersion, "5.8.2");
  assert.ok(record.entries.length > 0);
  for (const entry of record.entries) {
    assert.ok(entry.rows.length > 0);
    assert.match(entry.boundary, /^(reached|decoded|presented|read-back)$/);
  }
});

test("the committed record is what readUnsupported returns", () => {
  assert.deepEqual(
    readUnsupported(),
    JSON.parse(readFileSync(UNSUPPORTED_PATH, "utf8")),
  );
});
