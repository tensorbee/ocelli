// A subject whose story is not done resolves to `unavailable` and never to a
// number.
//
// This is the anti-fabrication rule at run time. `scripts/bench_check.py` holds
// the tracked half, refusing a runner file and a baseline entry, and this holds
// the half that decides what a record may say.

import assert from "node:assert/strict";
import test from "node:test";

import {
  MEASURED,
  NO_RUNNER,
  NO_SUBJECT,
  RUNNER_FAILED,
  UNAVAILABLE,
  subjectState,
} from "../src/state.mjs";

const subject = {
  id: "decode.frame",
  title: "One Decoder::decode call over one corpus frame",
  unit: "ms",
  tiers: ["n/a"],
  definition_hld: "18-codec-registry.md section 21",
  definition: "one call",
  subject_story: "F-023",
  note: "a note",
};

const pending = { subject, storyStatus: "pending", subjectExists: false };
const landed = {
  subject: { ...subject, subject_story: null },
  storyStatus: null,
  subjectExists: true,
};

test("a pending subject is unavailable and names the blocking story", () => {
  const entry = subjectState(pending, { hasRunner: false, value: null });
  assert.equal(entry.state, UNAVAILABLE);
  assert.equal(entry.reason, NO_SUBJECT);
  assert.equal(entry.blocking_story, "F-023");
  assert.equal(entry.blocking_story_status, "pending");
  assert.equal(Object.hasOwn(entry, "value"), false,
    "an unavailable subject carried a value field");
});

test("a number for a pending subject is REFUSED, not recorded", () => {
  assert.throws(
    () => subjectState(pending, { hasRunner: true, value: 1.5 }),
    /REFUSED: a value was produced for decode.frame/,
  );
});

test("a number with no runner is REFUSED, because it has no provenance", () => {
  assert.throws(
    () => subjectState(landed, { hasRunner: false, value: 1.5 }),
    /no runner file exists for it/,
  );
});

test("a subject that exists with no runner is unavailable, not measured", () => {
  const entry = subjectState(landed, { hasRunner: false, value: null });
  assert.equal(entry.state, UNAVAILABLE);
  assert.equal(entry.reason, NO_RUNNER);
  assert.match(entry.note, /src\/runners\/decode_frame\.mjs/);
});

test("a runner that threw is unavailable and carries the failure", () => {
  const entry = subjectState(landed, {
    hasRunner: true, value: null, error: "chromium would not launch",
  });
  assert.equal(entry.state, UNAVAILABLE);
  assert.equal(entry.reason, RUNNER_FAILED);
  assert.equal(entry.error, "chromium would not launch");
});

test("a runner that returned nothing and said nothing is refused", () => {
  assert.throws(
    () => subjectState(landed, { hasRunner: true, value: null }),
    /produced no\s+value/,
  );
});

test("a subject with a runner and a value is measured", () => {
  const entry = subjectState(landed, {
    hasRunner: true, value: 3.25, detail: { statistic: "median" },
  });
  assert.equal(entry.state, MEASURED);
  assert.equal(entry.value, 3.25);
  assert.equal(entry.unit, "ms");
  assert.deepEqual(entry.detail, { statistic: "median" });
});

test("an in-progress story is still not done, so still unavailable", () => {
  const entry = subjectState(
    { subject, storyStatus: "in-progress", subjectExists: false },
    { hasRunner: false, value: null },
  );
  assert.equal(entry.state, UNAVAILABLE);
  assert.equal(entry.blocking_story_status, "in-progress");
});
