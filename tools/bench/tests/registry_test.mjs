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
  VALID_STATUS,
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

test("the Roadmap table is a status table too", () => {
  // `docs/sprints/BACKLOG.md` has eighteen `### M<n>,` headings and one
  // `### Roadmap, Phase 2 and Phase 3`, and the rows under the last one carry
  // a status like every other row. The heading rule is duplicated in
  // `scripts/bench_check.py` and `scripts/backlog_check.py`, and this module's
  // header says a rule that exists on one side only is a defect, so the clause
  // is load bearing rather than defensive.
  //
  // Measured in the S03 sprint review's ninth pass: dropping
  // `|| heading.startsWith("Roadmap")` left all sixty-six tests green, because
  // every fixture above uses an `M<n>` heading. The failure is loud, the
  // driver throws on an unknown F-ID rather than labelling one wrongly, which
  // is what caps the severity and not what makes the clause optional.
  //
  // The Roadmap table has a DIFFERENT column count from the milestone tables,
  // eight against eight with different headers, and the status is read as the
  // second-to-last cell in both. Both shapes are here so the reader is not
  // reading position off one example.
  const backlog = [
    "### M18, Whole-slide imaging",
    "",
    "| F-ID | Epic ref | Sprint | Story | Layer | Est | Depends on | Status |",
    "|------|----------|--------|-------|-------|-----|------------|--------|",
    "| F-149 | E35.7 | S72 | a milestone row | Rust | 8w | F-144 | pending |",
    "",
    "### Roadmap, Phase 2 and Phase 3",
    "",
    "Tracked, unscheduled. No sprint is assigned.",
    "",
    "| F-ID | Epic ref | Phase | Epic | Story | Layer | Est | Status |",
    "|------|----------|-------|------|-------|-------|-----|--------|",
    "| F-158 | E20.1 | P2 | E20 gateway | adopt one | Server | 6w | pending |",
    "| F-159 | E20.2 | P3 | E20 gateway | and another | Server | 4w | done |",
  ].join("\n");
  const statuses = backlogStatuses(backlog);
  assert.equal(statuses.get("F-149"), "pending");
  assert.equal(
    statuses.get("F-158"),
    "pending",
    "a Roadmap row was not read as a status row",
  );
  assert.equal(statuses.get("F-159"), "done");
  assert.equal(statuses.size, 3);
});

test("a heading that is neither a milestone nor the Roadmap is not a table", () => {
  // The other direction, so the clause above cannot be satisfied by reading
  // every `### ` heading as a status table. `docs/sprints/BACKLOG.md` carries
  // prose subsections and the recorded-defects table, and a row under one of
  // those has no status cell to read.
  const backlog = [
    "### Notes on the imported backlog",
    "",
    "| F-ID | Epic ref | Defect |",
    "| F-145 | E35.3 | declared dependency E4.9 does not exist |",
  ].join("\n");
  assert.equal(backlogStatuses(backlog).size, 0);
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

test("only a done story makes a subject exist, over every status", () => {
  // `resolveSubjects` was only ever driven with `done` and `pending`, so
  // `in-progress`, `archived` and `superseded` never reached it and
  // `status === "done"` mutated to `status !== "pending"` left the suite
  // green. The output that mutation produces is the one `src/state.mjs` calls
  // "the defect this whole story is most likely to produce": a plausible
  // number recorded beside a story that has not landed.
  //
  // The three that were never driven are the ones it matters most for.
  // `archived` and `superseded` are exactly the statuses a stale subject row
  // outlives, and a superseded story is one whose subject was REPLACED rather
  // than delivered, so reading either as delivered would let this harness time
  // a subject that no longer exists.
  //
  // The expectation is a table and deliberately not `status === "done"`
  // recomputed here, which would assert the implementation against itself.
  // Its authority is `scripts/bench_check.py`, which refuses a runner file and
  // a baseline entry on `status != "done"` twice over, and this module's own
  // header rule that a rule living on one side only is a defect.
  const DELIVERED = {
    pending: false,
    "in-progress": false,
    done: true,
    archived: false,
    superseded: false,
  };
  assert.deepEqual(Object.keys(DELIVERED), [...VALID_STATUS],
    "the accepted status set moved and this table no longer covers it");

  for (const [status, delivered] of Object.entries(DELIVERED)) {
    const subjects = parseRegistry(
      registryText([{ ...ROW, subject_story: "F-001" }]),
    ).subjects;
    const [resolved] = resolveSubjects(subjects, {
      allocationFids: new Set(["F-001"]),
      statuses: new Map([["F-001", status]]),
    });
    assert.equal(resolved.storyStatus, status);
    assert.equal(resolved.subjectExists, delivered,
      `a story that is ${JSON.stringify(status)} was read as ` +
        `${resolved.subjectExists ? "delivered" : "not delivered"}`);
  }
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
