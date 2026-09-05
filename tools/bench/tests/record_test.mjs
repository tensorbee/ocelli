// The baseline round-trips, the tolerance is applied on both sides, and
// --accept rewrites only the named subject.

import assert from "node:assert/strict";
import test from "node:test";

import {
  acceptRecord,
  baselineEntry,
  compareRecord,
  emptyBaseline,
  movedSubjects,
  MOVED,
  NO_BASELINE,
  WITHIN,
  withinTolerance,
} from "../src/record.mjs";
import { hostClassKey } from "../src/hostclass.mjs";
import { INCOMPARABLE, MEASURED, UNAVAILABLE } from "../src/state.mjs";

const HOST = {
  platform: "darwin",
  release: "25.6.0",
  arch: "arm64",
  cpu_model: "Apple M5 Max",
  cpu_count: 16,
  memory_bytes: 68719476736,
};
const OTHER_HOST = { ...HOST, cpu_model: "Apple M4 Pro" };
const INSTRUMENT = { node: "v24.16.0", playwright: "1.62.1", chromium: "142" };

const record = (overrides = {}) => ({
  host_class: overrides.host ?? HOST,
  instrument: overrides.instrument ?? INSTRUMENT,
  conditions: { load_average: [0.1, 0.1, 0.1] },
  subjects: overrides.subjects ?? [
    { id: "wasm.cold_start", unit: "ms", state: MEASURED, value: 10,
      detail: { statistic: "median" } },
    { id: "decode.frame", unit: "ms", state: UNAVAILABLE,
      blocking_story: "F-023" },
  ],
});

const baselineWith = (value, tolerance, host = HOST) => ({
  host_classes: {
    [hostClassKey(host)]: {
      host_class: host,
      subjects: {
        "wasm.cold_start": {
          value, unit: "ms", tolerance, provenance: "measured",
          recorded: "2026-09-05", story: "F-006", why: "first measurement",
          host_class: host, instrument: INSTRUMENT,
        },
      },
    },
  },
});

test("the tolerance is applied on BOTH sides of the baseline", () => {
  assert.equal(withinTolerance(10, 10.9, 0.1).within, true);
  assert.equal(withinTolerance(10, 11.1, 0.1).within, false);
  assert.equal(withinTolerance(10, 9.1, 0.1).within, true);
  // An unexplained improvement fails as loudly as an unexplained regression.
  // A runner that started timing less work reads as an improvement.
  assert.equal(withinTolerance(10, 8.9, 0.1).within, false);
});

test("the direction and the fraction are reported", () => {
  const slower = withinTolerance(10, 12, 0.1);
  assert.equal(slower.direction, "slower");
  assert.equal(slower.delta, 2);
  assert.ok(Math.abs(slower.delta_fraction - 0.2) < 1e-12);
  assert.equal(withinTolerance(10, 8, 0.1).direction, "faster");
  assert.equal(withinTolerance(10, 10, 0.1).direction, "unchanged");
});

test("a baseline of zero cannot carry a fractional tolerance", () => {
  assert.throws(() => withinTolerance(0, 1, 0.1), /cannot carry a fractional/);
});

test("a matching host class and instrument yields a comparison", () => {
  const compared = compareRecord(record(), baselineWith(10, 0.3));
  const entry = compared.subjects[0];
  assert.equal(entry.state, MEASURED);
  assert.equal(entry.comparison.verdict, WITHIN);
  assert.equal(entry.comparison.baseline, 10);
  assert.equal(compared.compared_against, hostClassKey(HOST));
});

test("a figure outside the tolerance is MOVED, and the run says so", () => {
  const compared = compareRecord(record(), baselineWith(5, 0.1));
  assert.equal(compared.subjects[0].comparison.verdict, MOVED);
  assert.deepEqual(movedSubjects(compared).map((one) => one.id),
    ["wasm.cold_start"]);
});

test("a baseline recorded on another machine is INCOMPARABLE", () => {
  const compared = compareRecord(record(), baselineWith(10, 0.3, OTHER_HOST));
  const entry = compared.subjects[0];
  assert.equal(entry.state, INCOMPARABLE);
  assert.equal(entry.comparison.verdict, INCOMPARABLE);
  assert.deepEqual(entry.comparison.recorded_on, [hostClassKey(OTHER_HOST)]);
  assert.equal(entry.value, 10, "an incomparable subject lost its number");
});

test("two machines that COLLIDE on the key are still INCOMPARABLE", () => {
  // The key is lossy: hostClassKey sanitises with [^A-Za-z0-9._-]+ to `_`, so
  // "Apple M5 Max" and "Apple/M5/Max" produce the same string. Without this the
  // mismatch branch inside compareRecord is unreachable by any test, because
  // every other case takes the `recorded === null` path instead, and an
  // unreachable branch is counted as coverage forever.
  const collidingHost = { ...HOST, cpu_model: "Apple/M5/Max" };
  assert.equal(hostClassKey(collidingHost), hostClassKey(HOST),
    "the sanitiser stopped colliding, so this test no longer reaches the " +
      "branch it exists for");

  const baseline = baselineWith(10, 0.3);
  baseline.host_classes[hostClassKey(HOST)].subjects["wasm.cold_start"]
    .host_class = collidingHost;
  const compared = compareRecord(record(), baseline);
  const entry = compared.subjects[0];
  assert.equal(entry.state, INCOMPARABLE,
    "a figure from a different CPU was compared because the keys collided");
  assert.deepEqual(entry.comparison.differing_host_fields, ["cpu_model"]);
  assert.deepEqual(entry.comparison.differing_instrument_fields, []);
});

test("a different instrument on the same machine is INCOMPARABLE", () => {
  const baseline = baselineWith(10, 0.3);
  baseline.host_classes[hostClassKey(HOST)].subjects["wasm.cold_start"]
    .instrument = { ...INSTRUMENT, chromium: "999" };
  const compared = compareRecord(record(), baseline);
  assert.equal(compared.subjects[0].state, INCOMPARABLE);
  assert.deepEqual(
    compared.subjects[0].comparison.differing_instrument_fields, ["chromium"]);
});

test("a baseline naming no host class is INCOMPARABLE, not a match", () => {
  // The hole this closes: comparing the observed host class against itself
  // when the recorded one is absent. That comparison always passes, and the
  // map's key cannot stand in for it, because the key is sanitised and lossy.
  for (const field of ["host_class", "instrument"]) {
    const baseline = baselineWith(10, 0.3);
    delete baseline.host_classes[hostClassKey(HOST)]
      .subjects["wasm.cold_start"][field];
    const compared = compareRecord(record(), baseline);
    assert.equal(compared.subjects[0].state, INCOMPARABLE,
      `an entry with no ${field} was compared anyway`);
    assert.match(compared.subjects[0].comparison.reason,
      /no host class or no instrument/);
  }
});

test("nothing recorded anywhere is NO_BASELINE, not a pass", () => {
  const compared = compareRecord(record(), emptyBaseline());
  assert.equal(compared.subjects[0].comparison.verdict, NO_BASELINE);
});

test("an unavailable subject is never compared", () => {
  const compared = compareRecord(record(), baselineWith(10, 0.3));
  assert.equal(compared.subjects[1].state, UNAVAILABLE);
  assert.equal(Object.hasOwn(compared.subjects[1], "comparison"), false);
});

test("--accept round-trips and rewrites only the named subject", () => {
  const before = {
    host_classes: {
      [hostClassKey(HOST)]: {
        host_class: HOST,
        subjects: {
          "wasm.cold_start": { value: 10, unit: "ms", tolerance: 0.3 },
          "some.other": { value: 99, unit: "ms", tolerance: 0.5 },
        },
      },
    },
  };
  const after = acceptRecord(before, record(), {
    subjectId: "wasm.cold_start",
    story: "F-006",
    why: "the first recorded cold start",
    tolerance: 0.25,
    toleranceWhy: "derived from the observed spread",
    today: "2026-09-05",
  });
  const key = hostClassKey(HOST);
  assert.equal(after.host_classes[key].subjects["wasm.cold_start"].value, 10);
  assert.equal(after.host_classes[key].subjects["wasm.cold_start"].tolerance,
    0.25);
  assert.equal(after.host_classes[key].subjects["wasm.cold_start"].replaced,
    10);
  assert.deepEqual(after.host_classes[key].subjects["some.other"],
    { value: 99, unit: "ms", tolerance: 0.5 },
    "--accept rewrote a subject it was not asked about");
  assert.deepEqual(before.host_classes[key].subjects["wasm.cold_start"],
    { value: 10, unit: "ms", tolerance: 0.3 },
    "--accept mutated the baseline it was given");
  assert.equal(
    baselineEntry(after, key, "wasm.cold_start").story, "F-006");
});

test("--accept keeps the previous tolerance and its reason when none is given",
  () => {
    const before = baselineWith(10, 0.3);
    before.host_classes[hostClassKey(HOST)].subjects["wasm.cold_start"]
      .tolerance_provenance = "derived from the observed spread";
    const after = acceptRecord(before, record(), {
      story: "F-006", why: "re-run", today: "2026-09-05",
    });
    const entry = baselineEntry(after, hostClassKey(HOST), "wasm.cold_start");
    assert.equal(entry.tolerance, 0.3);
    assert.equal(entry.tolerance_provenance,
      "derived from the observed spread",
      "the tolerance survived and the reason it was chosen did not");
  });

test("a new tolerance carries its own stated provenance", () => {
  const after = acceptRecord(emptyBaseline(), record(), {
    story: "F-006", why: "the artefact grew",
    tolerance: 0.25, toleranceWhy: "widened to cover the clock quantum",
    today: "2026-09-05",
  });
  const entry = baselineEntry(after, hostClassKey(HOST), "wasm.cold_start");
  assert.equal(entry.tolerance, 0.25);
  assert.equal(entry.tolerance_provenance,
    "widened to cover the clock quantum");
});

test("a new tolerance with no provenance is refused", () => {
  assert.throws(
    () => acceptRecord(emptyBaseline(), record(), {
      story: "F-006", why: "x", tolerance: 0.25, today: "2026-09-05",
    }),
    /a new tolerance needs its own reason/,
  );
});

test("--accept records the host class and the instrument beside the value",
  () => {
    const after = acceptRecord(emptyBaseline(), record(), {
      story: "F-006", why: "first", tolerance: 0.3,
      toleranceWhy: "derived from the observed spread", today: "2026-09-05",
    });
    const entry = baselineEntry(after, hostClassKey(HOST), "wasm.cold_start");
    assert.deepEqual(entry.host_class, HOST);
    assert.deepEqual(entry.instrument, INSTRUMENT);
    assert.equal(entry.provenance, "measured");
    assert.equal(entry.recorded, "2026-09-05");
  });

test("--accept refuses a re-baseline with no story and no reason", () => {
  assert.throws(
    () => acceptRecord(emptyBaseline(), record(),
      { why: "x", today: "2026-09-05" }),
    /needs both a story and a reason/,
  );
  assert.throws(
    () => acceptRecord(emptyBaseline(), record(),
      { story: "F-006", today: "2026-09-05" }),
    /needs both a story and a reason/,
  );
});

test("--accept refuses a subject this run did not measure", () => {
  assert.throws(
    () => acceptRecord(emptyBaseline(), record(), {
      subjectId: "decode.frame", story: "F-006", why: "x",
      today: "2026-09-05",
    }),
    /no measured figure for decode.frame/,
  );
});
