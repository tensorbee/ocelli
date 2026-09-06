// The baseline round-trips, the tolerance is applied on both sides, and
// --accept rewrites only the named subject.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  acceptRecord,
  baselineEntry,
  BETTER,
  compareRecord,
  DOWN,
  emptyBaseline,
  INCREASE_MEANS,
  movedSubjects,
  MOVED,
  NO_BASELINE,
  UNCHANGED,
  UP,
  WITHIN,
  withinTolerance,
  WORSE,
} from "../src/record.mjs";
import { SUBJECTS_PATH } from "../src/paths.mjs";
import { parseRegistry } from "../src/registry.mjs";
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
  assert.equal(withinTolerance(10, 10.9, 0.1, "ms").within, true);
  assert.equal(withinTolerance(10, 11.1, 0.1, "ms").within, false);
  assert.equal(withinTolerance(10, 9.1, 0.1, "ms").within, true);
  // An unexplained improvement fails as loudly as an unexplained regression.
  // A runner that started timing less work reads as an improvement.
  assert.equal(withinTolerance(10, 8.9, 0.1, "ms").within, false);
});

test("a figure sitting exactly on the tolerance is INSIDE it", () => {
  // The boundary itself, which nothing probed. The test above steps to either
  // side of a 0.1 tolerance, at 0.09 and 0.11, and never onto it, so `<=`
  // mutated to `<` left the suite green. Both neighbouring guards in this
  // function are covered and this was the one untested boundary in it.
  //
  // The boundary belongs to the inside, and that is not a preference. Spike
  // gate A7.3 names ci/wasm-size-budget.json's mechanism as the one to follow,
  // and `scripts/pin_and_size_check.py` refuses on `size > ceiling`, so a
  // figure exactly at the ceiling passes there. A tolerance stated as a
  // fraction that excluded its own value would mean a 10 per cent tolerance
  // admits less than 10 per cent.
  assert.equal(withinTolerance(10, 11, 0.1, "ms").within, true,
    "a figure exactly 10 per cent above a baseline was outside a 10 per cent " +
      "tolerance");
  // Both sides, because the tolerance is applied both ways.
  assert.equal(withinTolerance(10, 9, 0.1, "ms").within, true,
    "a figure exactly 10 per cent below a baseline was outside a 10 per cent " +
      "tolerance");
  // And the far side of the boundary, so the comparison cannot have collapsed
  // into one that admits everything.
  assert.equal(withinTolerance(10, 11.000001, 0.1, "ms").within, false);
  assert.equal(withinTolerance(10, 8.999999, 0.1, "ms").within, false);
});

test("the direction and the fraction are reported", () => {
  const up = withinTolerance(10, 12, 0.1, "ms");
  assert.equal(up.direction, UP);
  assert.equal(up.delta, 2);
  assert.ok(Math.abs(up.delta_fraction - 0.2) < 1e-12);
  assert.equal(withinTolerance(10, 8, 0.1, "ms").direction, DOWN);
  assert.equal(withinTolerance(10, 10, 0.1, "ms").direction, UNCHANGED);
});

test("a rate that FELL is worse, and a duration that fell is better", () => {
  // The defect this replaces: `direction` was `delta > 0 ? "slower" :
  // "faster"` for every unit, so tier.startup_microbenchmark, whose unit is
  // pixels_per_second and whose subject story is already done, would have
  // reported a halved fill rate as `faster`. Four of the eleven registry rows
  // are not durations.
  const halvedRate = withinTolerance(1000, 500, 0.1, "pixels_per_second");
  assert.equal(halvedRate.direction, DOWN);
  assert.equal(halvedRate.sense, WORSE,
    "a fill rate that halved was reported as an improvement");

  const fasterCine = withinTolerance(30, 60, 0.1, "changed_frames_per_second");
  assert.equal(fasterCine.direction, UP);
  assert.equal(fasterCine.sense, BETTER);

  const slower = withinTolerance(10, 12, 0.1, "ms");
  assert.equal(slower.sense, WORSE);
  assert.equal(withinTolerance(10, 8, 0.1, "ms").sense, BETTER);

  // A7.3: "Ocelli's idle cost must be indistinguishable from zero", so more
  // CPU is worse. The polarity matches a duration and the WORDS did not, which
  // is why the vocabulary is up and down rather than slower and faster.
  const moreCpu = withinTolerance(0.02, 0.05, 0.1, "fraction_of_one_vcpu");
  assert.equal(moreCpu.direction, UP);
  assert.equal(moreCpu.sense, WORSE);

  assert.equal(withinTolerance(10, 10, 0.1, "ms").sense, UNCHANGED);
});

test("a unit that declares no direction is refused, not defaulted", () => {
  assert.throws(() => withinTolerance(10, 12, 0.1, "furlongs"),
    /declares no direction/);
  assert.throws(() => withinTolerance(10, 12, 0.1, undefined),
    /declares no direction/);
});

test("INCREASE_MEANS and the registry declare the same units", () => {
  // The test above proves the THROW. It does not prove that the throw is ever
  // reached, and it is not reached by anything in the floor: `withinTolerance`
  // runs under `bench --compare` and only for a subject that already owns a
  // baseline on the machine running it. Measured on this tree, changing
  // decode.frame's unit to "furlongs" in tools/bench/subjects.json left
  // `bin/ocelli.sh gate bench` at exit 0, so a unit with no stated direction
  // lands green here and fails later on whichever machine owns that baseline.
  //
  // This is the binding, and its shape is `caps.rs`'s
  // `the_recorded_bands_match_the_checked_in_file`: a constant in source and
  // the checked-in file it claims to follow, asserted equal. Both directions,
  // because a unit the registry declares with no direction is a comparison
  // that cannot report better or worse, and a direction for a unit no subject
  // declares is a row that has outlived the subject it was added for.
  const { subjects } = parseRegistry(readFileSync(SUBJECTS_PATH, "utf8"));
  const declared = new Set(subjects.map((subject) => subject.unit));
  const stated = new Set(Object.keys(INCREASE_MEANS));

  for (const unit of declared) {
    assert.ok(
      stated.has(unit),
      `tools/bench/subjects.json declares the unit ${JSON.stringify(unit)} ` +
        "and INCREASE_MEANS states no direction for it, so a comparison of " +
        "that subject cannot say whether the figure got better or worse. Add " +
        "it with the reason, which is a decision and not a default.",
    );
  }
  for (const unit of stated) {
    assert.ok(
      declared.has(unit),
      `INCREASE_MEANS states a direction for ${JSON.stringify(unit)} and no ` +
        "subject declares it. The registry is the authority on units, so the " +
        "row is stale rather than the registry being incomplete.",
    );
  }
});

test("a baseline of zero cannot carry a fractional tolerance", () => {
  assert.throws(() => withinTolerance(0, 1, 0.1, "ms"),
    /cannot carry a fractional/);
});

test("a missing tolerance is refused rather than read as zero", () => {
  // `Math.abs(fraction) <= null` is `<= 0` in JavaScript, so a null tolerance
  // used to admit only a bit-identical duration and report everything else as
  // moved. That is a different check wearing the same name.
  for (const bad of [null, undefined, 0, -0.1, 1.5, NaN, "0.25"]) {
    assert.throws(() => withinTolerance(10, 10.1, bad, "ms"),
      /is not a fraction above 0 and at most 1/,
      `a tolerance of ${JSON.stringify(bad)} was accepted`);
  }
});

test("a baseline entry with no tolerance is refused by the comparison", () => {
  assert.throws(() => compareRecord(record(), baselineWith(10, null)),
    /is not a fraction above 0 and at most 1/);
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

test("the comparison reads the RECORDED unit and not the run's", () => {
  // The comment at the call site states the decision and nothing held it:
  // every other case here builds a run whose unit already equals its
  // baseline's, so `recorded.unit` and `entry.unit` were indistinguishable and
  // swapping them left the suite green.
  //
  // A subject whose unit changed under a baseline that still names the old one
  // is comparing two different quantities, and the unit is the only thing that
  // says which way is better. So the two units here have OPPOSITE polarity in
  // INCREASE_MEANS: read against the recorded `ms` a figure that rose is
  // `worse`, and read against the run's `pixels_per_second` the same rise is
  // `better`. That is a regression printed as an improvement in the field a
  // reader looks at first, which is the defect `INCREASE_MEANS` was added for
  // arriving by a different route.
  const changedUnit = record({
    subjects: [
      { id: "wasm.cold_start", unit: "pixels_per_second", state: MEASURED,
        value: 12 },
    ],
  });
  const entry = compareRecord(changedUnit, baselineWith(10, 0.3)).subjects[0];
  assert.equal(entry.comparison.direction, UP);
  assert.equal(entry.comparison.sense, WORSE,
    "the run's own unit set the polarity, so a duration that rose by 20 per " +
      "cent was reported as an improvement against a baseline recorded in ms");
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
