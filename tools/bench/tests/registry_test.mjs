// The registry parses, and every subject_story resolves in allocation.json.
//
// These are negative cases first. A parser that accepted everything would pass
// a test that only fed it the real file, and the real file is the one input
// that is known to be well formed.

import assert from "node:assert/strict";
import test from "node:test";

import {
  allocationFids,
  backlogStatuses,
  loadRegistry,
  parseRegistry,
  REQUIRED_FIELDS,
  resolveSubjects,
} from "../src/registry.mjs";

const ROW = {
  id: "example.subject",
  title: "an example",
  definition_hld: "23-performance-rules.md section 26",
  definition: "starts here, stops there",
  unit: "ms",
  dimensions: ["rows"],
  tiers: ["n/a"],
  subject_story: null,
  feeds: [],
};

const registryText = (subjects) => JSON.stringify({ subjects });

test("a well formed registry parses", () => {
  const parsed = parseRegistry(registryText([ROW]));
  assert.equal(parsed.subjects.length, 1);
  assert.equal(parsed.subjects[0].id, "example.subject");
});

test("every required field is required", () => {
  for (const field of REQUIRED_FIELDS) {
    const row = { ...ROW };
    delete row[field];
    assert.throws(
      () => parseRegistry(registryText([row])),
      new RegExp(`missing the ${field} field`),
      `an absent ${field} was accepted`,
    );
  }
});

test("a duplicate id is refused", () => {
  assert.throws(
    () => parseRegistry(registryText([ROW, { ...ROW }])),
    /appears twice/,
  );
});

test("a row declaring no tier is refused, and n/a is accepted", () => {
  assert.throws(
    () => parseRegistry(registryText([{ ...ROW, tiers: [] }])),
    /declares no tiers/,
  );
  assert.throws(
    () => parseRegistry(registryText([{ ...ROW, tiers: ["D"] }])),
    /which is not one of/,
  );
  assert.doesNotThrow(
    () => parseRegistry(registryText([{ ...ROW, tiers: ["n/a"] }])),
  );
});

test("backlog statuses come from the status tables only", () => {
  const backlog = [
    "## Recorded defects in the imported backlog",
    "",
    "| F-ID | Epic ref | Defect |",
    "| F-145 | E35.3 | declared dependency E4.9 does not exist |",
    "",
    "### M1, Foundations",
    "",
    "| F-ID | Epic ref | Sprint | Story | Layer | Est | Depends on | Status |",
    "|------|----------|--------|-------|-------|-----|------------|--------|",
    "| F-001 | E1.1 | S01 | a thing | Build | 2w | - | done |",
    "| F-004 | E1.4 | S03 | another | Build | 2w | F-002 | pending |",
  ].join("\n");
  const statuses = backlogStatuses(backlog);
  assert.equal(statuses.get("F-001"), "done");
  assert.equal(statuses.get("F-004"), "pending");
  assert.equal(statuses.has("F-145"), false,
    "a recorded-defect row was read as a status row");
});

test("an unknown F-ID is rejected rather than guessed at", () => {
  const subjects = parseRegistry(
    registryText([{ ...ROW, subject_story: "F-999" }]),
  ).subjects;
  assert.throws(
    () => resolveSubjects(subjects, {
      allocationFids: new Set(["F-001"]),
      statuses: new Map([["F-001", "done"]]),
    }),
    /which is not an F-ID in/,
  );
});

test("an F-ID with no backlog row is rejected", () => {
  const subjects = parseRegistry(
    registryText([{ ...ROW, subject_story: "F-001" }]),
  ).subjects;
  assert.throws(
    () => resolveSubjects(subjects, {
      allocationFids: new Set(["F-001"]),
      statuses: new Map(),
    }),
    /has no status row/,
  );
});

test("resolution reports whether the subject exists", () => {
  const subjects = parseRegistry(registryText([
    { ...ROW, id: "a", subject_story: null },
    { ...ROW, id: "b", subject_story: "F-001" },
    { ...ROW, id: "c", subject_story: "F-002" },
  ])).subjects;
  const resolved = resolveSubjects(subjects, {
    allocationFids: new Set(["F-001", "F-002"]),
    statuses: new Map([["F-001", "done"], ["F-002", "pending"]]),
  });
  assert.deepEqual(resolved.map((one) => one.subjectExists),
    [true, true, false]);
  assert.deepEqual(resolved.map((one) => one.storyStatus),
    [null, "done", "pending"]);
});

test("allocationFids reads the tracked allocation", () => {
  const fids = allocationFids(JSON.stringify({
    stories: [{ fid: "F-001" }, { fid: "F-X003" }],
  }));
  assert.equal(fids.has("F-X003"), true);
});

test("the tracked registry loads against the tracked delivery record", () => {
  const resolved = loadRegistry();
  assert.ok(resolved.length >= 11,
    "the registry lost rows. Subjects are defined once and kept.");
  const coldStart = resolved.find(
    (one) => one.subject.id === "wasm.cold_start",
  );
  assert.ok(coldStart, "wasm.cold_start left the registry");
  assert.equal(coldStart.subject.subject_story, null,
    "wasm.cold_start has a subject today and must name no blocking story");
  // Deliberately NOT an assertion that exactly one subject is measurable. That
  // would go red the day an unrelated story is marked done, which is a floor
  // gate failing for a reason nobody in that story can act on. What the tree
  // is allowed to contain is `scripts/bench_check.py`'s job.
  for (const row of resolved) {
    assert.ok(row.subject.definition.length > 0,
      `${row.subject.id} has an empty definition`);
  }
});
