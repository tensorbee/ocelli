// Unit tests for the volume subject declaration and its two committed files.
//
// No browser and no corpus bytes. What these cover is the machinery that
// decides WHICH volumes are rendered and WHAT is claimed about them, which is
// the half of F-X007 that can be wrong without any pixel looking odd.
//
// Expected values come from PS3.3 C.7.6.2.1.1 and from
// `scripts/corpus_synth.py`'s constants, and the geometry arithmetic itself is
// covered by `geometry_test.mjs`. Where a test builds its own params object it
// is testing the rule, and where it reads the committed file it is testing the
// claim the repository actually ships.

import test from "node:test";
import assert from "node:assert/strict";

import { measureSubject } from "../src/geometry.mjs";
import { parseManifest } from "../src/manifest.mjs";
import { readFileSync } from "node:fs";
import { repoPath } from "../src/paths.mjs";
import {
  RESERVED_VOLUME_PREFIX,
  SERIES_CATEGORY_TOKEN,
  checkSubjectsAgainstManifest,
  checkZProfile,
  compareGeometry,
  comparePairs,
  frameIdFor,
  readVolumeParams,
  readVolumeTruth,
  selectSubjects,
  subjectIdFor,
  validateVolumeParams,
  validateVolumeTruth,
} from "../src/volume.mjs";

const MANIFEST = parseManifest(readFileSync(repoPath("corpus/manifest.tsv"), "utf8"));

/** A minimal valid declaration, for the rule tests. */
function params(overrides = {}) {
  return {
    version: 1,
    loadTimeoutMs: 1000,
    loadTimeoutWhy: "because",
    orientations: ["AXIAL"],
    orientationsWhy: "because",
    blendMode: "COMPOSITE",
    blendModeWhy: "because",
    slabThicknessMm: null,
    slabThicknessWhy: "because",
    cameraMode: "reset",
    cameraModeWhy: "because",
    subjects: [
      {
        id: "volume__a__b",
        seriesDirectory: "a/b",
        why: "because",
        members: ["a/b/one.dcm", "a/b/two.dcm"],
      },
    ],
    ...overrides,
  };
}

function truth(overrides = {}) {
  return {
    version: 1,
    cornerstone3DVersion: "5.8.2",
    toleranceMm: 1e-6,
    subjects: {
      volume__a__b: {
        citation: "PS3.3 C.7.6.2.1.1",
        why: "because",
        uniform: null,
        referenceDivergence: null,
      },
    },
    framePairs: [],
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Naming, which is the contract F-011 reads
// ---------------------------------------------------------------------------

test("a subject id is the reserved prefix and the directory, flattened", () => {
  assert.equal(RESERVED_VOLUME_PREFIX, "volume__");
  assert.equal(
    subjectIdFor("synthetic/ct_series_nonuniform"),
    "volume__synthetic__ct_series_nonuniform",
  );
  assert.equal(subjectIdFor("real/ct_cmb_mml"), "volume__real__ct_cmb_mml");
});

test("a frame id is the subject id and the orientation", () => {
  assert.equal(
    frameIdFor("volume__synthetic__ct_series_nonuniform", "SAGITTAL"),
    "volume__synthetic__ct_series_nonuniform__SAGITTAL",
  );
});

// Every frame id has to be a legal file name, because a volume frame writes
// `<id>.raw`, `<id>.png` and `<id>.json` into the same flat directory the
// stack frames use.
test("every committed frame id is a safe, flat file name", () => {
  const declared = readVolumeParams();
  for (const subject of declared.subjects) {
    for (const orientation of declared.orientations) {
      assert.match(frameIdFor(subject.id, orientation), /^[A-Za-z0-9_.-]+$/);
    }
  }
});

// ---------------------------------------------------------------------------
// The declaration refuses what it cannot honour
// ---------------------------------------------------------------------------

// The one rule that keeps two committed files from drifting. render-params.json
// owns the canvas, the background, the interpolation and the VOI policy, and a
// second declaration of any of them here would be a parameter recorded in a
// volume sidecar as having produced the frame while having produced nothing.
// This is `src/params.mjs`'s rule about `canvas`, one file up.
test("a key render-params.json already owns is refused here", () => {
  for (const key of [
    "canvas",
    "background",
    "interpolation",
    "voi",
    "camera",
    "allowUniform",
    "modalityVoiDefaults",
    "informationFloor",
    "base",
    "rules",
  ]) {
    assert.throws(
      () => validateVolumeParams(params({ [key]: {} })),
      /render-params.json already declares/,
      `${key} was accepted`,
    );
  }
});

test("a subject id that is not the one its directory derives is refused", () => {
  const bad = params();
  bad.subjects[0].id = "volume__something_else";
  assert.throws(() => validateVolumeParams(bad), /derives the id/);
});

test("a member outside its subject's own directory is refused", () => {
  const bad = params();
  bad.subjects[0].members = ["a/b/one.dcm", "a/c/two.dcm"];
  assert.throws(() => validateVolumeParams(bad), /is not under/);
});

test("a subject of fewer than two members is refused, because it has no gap", () => {
  const bad = params();
  bad.subjects[0].members = ["a/b/one.dcm"];
  assert.throws(() => validateVolumeParams(bad), /at least two/);
});

test("a member declared twice, in one subject or in two, is refused", () => {
  const repeated = params();
  repeated.subjects[0].members = ["a/b/one.dcm", "a/b/one.dcm"];
  assert.throws(() => validateVolumeParams(repeated), /more than once/);
});

test("a declaration with no subjects is refused, not treated as nothing to do", () => {
  assert.throws(() => validateVolumeParams(params({ subjects: [] })), /no subjects/);
});

test("a timeout with no stated reason is refused", () => {
  assert.throws(
    () => validateVolumeParams(params({ loadTimeoutWhy: "" })),
    /loadTimeoutWhy/,
  );
  assert.throws(
    () => validateVolumeParams(params({ loadTimeoutMs: 0 })),
    /loadTimeoutMs/,
  );
});

test("no orientations at all is refused, because the pass would render nothing", () => {
  assert.throws(
    () => validateVolumeParams(params({ orientations: [] })),
    /at least one orientation/,
  );
});

// ---------------------------------------------------------------------------
// Strict in both directions against the manifest
// ---------------------------------------------------------------------------

test("a declared member that is not a manifest row is refused", () => {
  const problems = checkSubjectsAgainstManifest(params(), MANIFEST);
  assert.ok(
    problems.some((problem) => problem.includes("a/b/one.dcm")),
    problems.join("\n"),
  );
});

// The direction that stops this file quietly stopping covering a directory
// somebody adds to the corpus.
test("a series row belonging to no subject is refused", () => {
  const declared = readVolumeParams();
  const shortened = {
    ...declared,
    subjects: declared.subjects.slice(0, 1),
  };
  const problems = checkSubjectsAgainstManifest(shortened, MANIFEST);
  assert.ok(
    problems.some((problem) => problem.includes("belongs to no declared volume subject")),
    problems.join("\n"),
  );
});

test("the committed declaration covers every series row and claims no other", () => {
  assert.deepEqual(checkSubjectsAgainstManifest(readVolumeParams(), MANIFEST), []);
});

// The corpus's own accounting, asserted rather than described, because the LLD
// states it in prose and prose does not go red.
test("sixty-two manifest rows carry the series token, in four directories", () => {
  const rows = MANIFEST.filter((row) =>
    row.categories.includes(SERIES_CATEGORY_TOKEN),
  );
  assert.equal(rows.length, 62);
  const directories = new Set(rows.map((row) => row.path.replace(/\/[^/]*$/, "")));
  assert.deepEqual([...directories].sort(), [
    "real/ct_cmb_mml",
    "real/mr_eay131",
    "synthetic/ct_series_nonuniform",
    "synthetic/ct_series_uniform",
  ]);
});

// ---------------------------------------------------------------------------
// Selection: a partial subject is refused by name, never skipped
// ---------------------------------------------------------------------------

test("a subject whose members are all selected is attempted", () => {
  const declared = readVolumeParams();
  const subject = declared.subjects[0];
  const { attempted, problems } = selectSubjects(declared, new Set(subject.members));
  assert.deepEqual(problems, []);
  assert.deepEqual(attempted.map((entry) => entry.id), [subject.id]);
});

// `--rows synthetic/ct_series_uniform/slice_000.dcm` must say what it did
// rather than producing no volumes and reporting success.
test("a partially selected subject is refused by name", () => {
  const declared = readVolumeParams();
  const subject = declared.subjects[0];
  const { attempted, problems } = selectSubjects(
    declared,
    new Set([subject.members[0]]),
  );
  assert.deepEqual(attempted, []);
  assert.equal(problems.length, 1);
  assert.match(problems[0], new RegExp(subject.id));
  assert.match(problems[0], /1 of \d+/);
});

test("a selection touching no subject attempts none and refuses none", () => {
  const declared = readVolumeParams();
  const { attempted, problems } = selectSubjects(
    declared,
    new Set(["syntax/reference_mono12.dcm"]),
  );
  assert.deepEqual(attempted, []);
  assert.deepEqual(problems, []);
});

// ---------------------------------------------------------------------------
// The truth file, strict in both directions against the params file
// ---------------------------------------------------------------------------

test("a subject with no truth entry is refused", () => {
  assert.throws(
    () => validateVolumeTruth(truth({ subjects: {} }), params()),
    /has no entry in volume-truth.json/,
  );
});

test("a truth entry naming no declared subject is refused", () => {
  const extra = truth();
  extra.subjects.volume__nobody = { citation: "x", why: "y", uniform: null, referenceDivergence: null };
  assert.throws(() => validateVolumeTruth(extra, params()), /names no declared subject/);
});

// A file that claimed uniformity and listed gaps contradicting it would be an
// authoritative-looking wrong answer, which is worse than no answer.
test("a truth entry whose declared uniform contradicts its own gaps is refused", () => {
  const bad = truth();
  bad.subjects.volume__a__b = {
    citation: "PS3.3 C.7.6.2.1.1",
    why: "because",
    normal: [0, 0, 1],
    projectionsMm: [0, 2.5, 6.25],
    gapsMm: [2.5, 3.75],
    uniform: true,
    referenceDivergence: null,
  };
  assert.throws(() => validateVolumeTruth(bad, params()), /declares uniform true/);
});

test("a truth entry whose gaps do not follow from its projections is refused", () => {
  const bad = truth();
  bad.subjects.volume__a__b = {
    citation: "PS3.3 C.7.6.2.1.1",
    why: "because",
    normal: [0, 0, 1],
    projectionsMm: [0, 2.5, 5],
    gapsMm: [2.5, 3.75],
    uniform: false,
    referenceDivergence: null,
  };
  assert.throws(() => validateVolumeTruth(bad, params()), /do not follow from/);
});

test("an unclassified subject may not also declare a reference divergence", () => {
  const bad = truth();
  bad.subjects.volume__a__b.referenceDivergence = {
    field: "spacing[2]",
    reference: 2.5,
    truth: null,
    attributedTo: "reference",
    why: "because",
  };
  assert.throws(() => validateVolumeTruth(bad, params()), /classifies nothing/);
});

test("a frame pair naming an undeclared subject is refused", () => {
  const bad = truth({
    framePairs: [
      { left: "volume__a__b", right: "volume__nobody", expect: "identical", why: "x" },
    ],
  });
  assert.throws(() => validateVolumeTruth(bad, params()), /names no declared subject/);
});

test("a frame pair expecting something the harness cannot check is refused", () => {
  const bad = truth({
    framePairs: [
      { left: "volume__a__b", right: "volume__a__b", expect: "similar", why: "x" },
    ],
  });
  assert.throws(() => validateVolumeTruth(bad, params()), /the only expectation/);
});

test("the committed truth file validates against the committed declaration", () => {
  const declared = readVolumeParams();
  assert.equal(validateVolumeTruth(readVolumeTruth(), declared).version, 1);
});

// The committed truth's synthetic numbers, checked against the harness's own
// geometry rather than taken on faith. This is the fixture the design plan
// asks for, sitting where the file it judges can be seen.
test("the committed truth's synthetic gaps are the ones PS3.3 and the generator give", () => {
  const declared = readVolumeTruth();
  assert.deepEqual(
    declared.subjects.volume__synthetic__ct_series_uniform.gapsMm,
    [2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 2.5],
  );
  assert.deepEqual(
    declared.subjects.volume__synthetic__ct_series_nonuniform.gapsMm,
    [2.5, 2.5, 2.5, 2.5, 2.5, 2.5, 3.75, 1.25, 2.5],
  );
  assert.deepEqual(
    declared.subjects.volume__synthetic__ct_series_nonuniform.projectionsMm[7],
    18.75,
  );
  assert.equal(declared.subjects.volume__real__ct_cmb_mml.uniform, null);
  assert.equal(declared.subjects.volume__real__mr_eay131.uniform, null);
});

// ---------------------------------------------------------------------------
// The z profile
// ---------------------------------------------------------------------------

const Z_EXPECTATION = {
  voxel: [10, 6],
  expectedFirstValue: 1056,
  expectedStepPerSlice: 16,
};

/** 1056 + 16k, the hand-computed ramp. See volume-truth.json's `why`. */
const Z_RAMP = Array.from({ length: 10 }, (_, k) => 1056 + 16 * k);

test("the hand-computed z ramp passes", () => {
  assert.deepEqual(checkZProfile("s", Z_RAMP, Z_EXPECTATION), []);
});

test("a dropped slice breaks the z ramp", () => {
  const dropped = [...Z_RAMP];
  dropped.splice(4, 1);
  const problems = checkZProfile("s", dropped, Z_EXPECTATION);
  assert.equal(problems.length, 1);
  assert.match(problems[0], /the z profile/);
});

test("a duplicated slice breaks the z ramp", () => {
  const duplicated = [...Z_RAMP];
  duplicated[5] = duplicated[4];
  assert.equal(checkZProfile("s", duplicated, Z_EXPECTATION).length, 1);
});

test("two swapped slices break the z ramp", () => {
  const swapped = [...Z_RAMP];
  const held = swapped[2];
  swapped[2] = swapped[7];
  swapped[7] = held;
  assert.equal(checkZProfile("s", swapped, Z_EXPECTATION).length, 1);
});

// A whole volume built from one slice repeated reads back a plausible frame
// that hashes stably, which is the degeneracy the stack path has no analogue
// for.
test("a volume of one slice repeated breaks the z ramp", () => {
  assert.equal(
    checkZProfile("s", new Array(10).fill(1056), Z_EXPECTATION).length,
    1,
  );
});

// The step is right and the origin is wrong, which is an off-by-one in the
// in-plane index rather than in the slice order. The step check alone would
// pass it.
test("a ramp at the wrong in-plane voxel is caught by the first value", () => {
  const shifted = Z_RAMP.map((value) => value + 16);
  const problems = checkZProfile("s", shifted, Z_EXPECTATION);
  assert.equal(problems.length, 1);
  assert.match(problems[0], /first value/);
});

// ---------------------------------------------------------------------------
// The reference against the truth
// ---------------------------------------------------------------------------

const NORMAL = [-0.6, 0.8, 0.0];
const IOP = [0.8, 0.6, 0.0, 0.0, 0.0, -1.0];

function membersAt(distances) {
  return distances.map((distance, index) => ({
    path: `synthetic/x/slice_${String(index).padStart(3, "0")}.dcm`,
    imagePositionPatient: NORMAL.map((axis) => Number((distance * axis).toFixed(6))),
    imageOrientationPatient: IOP,
    pixelSpacing: [0.5, 0.25],
  }));
}

const UNIFORM = measureSubject(membersAt([0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5, 20, 22.5]));
const NONUNIFORM = measureSubject(membersAt([0, 2.5, 5, 7.5, 10, 12.5, 15, 18.75, 20, 22.5]));

/** What cornerstone3D 5.8.2 resolves for both series. */
const REFERENCE = { dimensions: [20, 12, 10], spacing: [0.25, 0.5, 2.5], origin: [0, 0, 0] };

test("a uniform subject agrees with the reference and declares no divergence", () => {
  const committed = readVolumeTruth();
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_uniform",
    measured: UNIFORM,
    referenceGeometry: REFERENCE,
    truth: committed.subjects.volume__synthetic__ct_series_uniform,
    toleranceMm: committed.toleranceMm,
  });
  assert.deepEqual(result.problems, []);
  assert.equal(result.uniform, true);
  assert.equal(result.referenceAgreesWithTruth, true);
  assert.equal(result.referenceDivergence, null);
});

test("a non-uniform subject diverges from the reference, as declared", () => {
  const committed = readVolumeTruth();
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_nonuniform",
    measured: NONUNIFORM,
    referenceGeometry: REFERENCE,
    truth: committed.subjects.volume__synthetic__ct_series_nonuniform,
    toleranceMm: committed.toleranceMm,
  });
  assert.deepEqual(result.problems, []);
  assert.equal(result.uniform, false);
  assert.equal(result.referenceAgreesWithTruth, false);
  assert.equal(result.referenceDivergence.field, "spacing[2]");
  assert.equal(result.referenceDivergence.reference, 2.5);
});

// The direction that stops the file becoming a list of excuses.
test("a divergence that did not occur fails the run", () => {
  const committed = readVolumeTruth();
  const stale = {
    ...committed.subjects.volume__synthetic__ct_series_uniform,
    referenceDivergence: {
      field: "spacing[2]",
      reference: 2.5,
      truth: null,
      attributedTo: "reference",
      why: "stale",
    },
  };
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_uniform",
    measured: UNIFORM,
    referenceGeometry: REFERENCE,
    truth: stale,
    toleranceMm: committed.toleranceMm,
  });
  assert.equal(result.problems.length, 1);
  assert.match(result.problems[0], /declares a referenceDivergence.*did not/s);
});

// And the direction that stops one hiding.
test("a divergence nobody declared fails the run", () => {
  const committed = readVolumeTruth();
  const undeclared = {
    ...committed.subjects.volume__synthetic__ct_series_nonuniform,
    referenceDivergence: null,
  };
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_nonuniform",
    measured: NONUNIFORM,
    referenceGeometry: REFERENCE,
    truth: undeclared,
    toleranceMm: committed.toleranceMm,
  });
  assert.equal(result.problems.length, 1);
  assert.match(result.problems[0], /no referenceDivergence/);
});

// HLD 25.1's 1e-6 is the tolerance that acts, and a drift of 2e-6 is over it.
// This is the node-side half of the `volume-geometry-drift` fault.
test("a projected position 2e-6 mm out of place fails against the truth", () => {
  const committed = readVolumeTruth();
  const drifted = membersAt([0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5, 20, 22.5]);
  drifted[4] = {
    ...drifted[4],
    imagePositionPatient: drifted[4].imagePositionPatient.map(
      (value, index) => value + 2e-6 * NORMAL[index],
    ),
  };
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_uniform",
    measured: measureSubject(drifted),
    referenceGeometry: REFERENCE,
    truth: committed.subjects.volume__synthetic__ct_series_uniform,
    toleranceMm: committed.toleranceMm,
  });
  assert.ok(result.problems.length > 0);
  assert.match(result.problems.join("\n"), /volume-truth.json declares/);
});

// And 5e-7 mm, which is under it, does not. Without this the test above would
// pass against any tolerance at all, including none.
test("a projected position 5e-7 mm out of place does not", () => {
  const committed = readVolumeTruth();
  const drifted = membersAt([0, 2.5, 5, 7.5, 10, 12.5, 15, 17.5, 20, 22.5]);
  drifted[4] = {
    ...drifted[4],
    imagePositionPatient: drifted[4].imagePositionPatient.map(
      (value, index) => value + 5e-7 * NORMAL[index],
    ),
  };
  const result = compareGeometry({
    subjectId: "volume__synthetic__ct_series_uniform",
    measured: measureSubject(drifted),
    referenceGeometry: REFERENCE,
    truth: committed.subjects.volume__synthetic__ct_series_uniform,
    toleranceMm: committed.toleranceMm,
  });
  assert.deepEqual(result.problems, []);
});

// An unclassified subject is asked nothing, which is decision 3 of the design
// round applied. A real series would otherwise be reported as non-uniform
// against a tolerance that was never meant for it.
test("an unclassified subject is measured and judged by nothing", () => {
  const committed = readVolumeTruth();
  const result = compareGeometry({
    subjectId: "volume__real__ct_cmb_mml",
    measured: NONUNIFORM,
    referenceGeometry: REFERENCE,
    truth: committed.subjects.volume__real__ct_cmb_mml,
    toleranceMm: committed.toleranceMm,
  });
  assert.deepEqual(result.problems, []);
  assert.equal(result.uniform, null);
  assert.equal(result.referenceAgreesWithTruth, null);
});

// ---------------------------------------------------------------------------
// The frame pairs
// ---------------------------------------------------------------------------

const PAIR = [
  {
    left: "volume__synthetic__ct_series_uniform",
    right: "volume__synthetic__ct_series_nonuniform",
    expect: "identical",
    why: "the reference averages",
  },
];

test("equal digests in every orientation satisfy an identical pair", () => {
  const frames = {
    volume__synthetic__ct_series_uniform: { AXIAL: "a", SAGITTAL: "b", CORONAL: "c" },
    volume__synthetic__ct_series_nonuniform: { AXIAL: "a", SAGITTAL: "b", CORONAL: "c" },
  };
  const { records, problems } = comparePairs(PAIR, frames);
  assert.deepEqual(problems, []);
  assert.equal(records[0].orientations.length, 3);
  assert.ok(records[0].orientations.every((entry) => entry.identical));
});

// The falsifiable half. If cornerstone3D ever stops averaging, this is what
// goes red, and it names the orientation.
test("one differing orientation fails the pair and names it", () => {
  const frames = {
    volume__synthetic__ct_series_uniform: { AXIAL: "a", SAGITTAL: "b", CORONAL: "c" },
    volume__synthetic__ct_series_nonuniform: { AXIAL: "a", SAGITTAL: "z", CORONAL: "c" },
  };
  const { problems } = comparePairs(PAIR, frames);
  assert.equal(problems.length, 1);
  assert.match(problems[0], /SAGITTAL/);
});

test("a pair whose frames are missing fails rather than passing vacuously", () => {
  const { problems } = comparePairs(PAIR, {
    volume__synthetic__ct_series_uniform: { AXIAL: "a" },
  });
  assert.ok(problems.length > 0);
  assert.match(problems.join("\n"), /no frames/);
});

// ---------------------------------------------------------------------------
// A subject that cannot be a volume is declared, never dropped
// ---------------------------------------------------------------------------

// `real/ct_cmb_mml` is twenty-seven real TCIA instances at only nine distinct
// positions on the slice normal, four deep at six of them, across three values
// of AcquisitionNumber. It is not one spatial volume, and the harness refuses
// it by name. That refusal is declared here rather than worked around, which is
// the same rule `unsupported.json` applies to a row the reference cannot
// decode.
test("the committed truth declares the one subject that cannot be a volume", () => {
  const entry = readVolumeTruth().subjects.volume__real__ct_cmb_mml;
  assert.equal(entry.uniform, null);
  assert.equal(entry.expectedRefusal.boundary, "volume-geometry");
  assert.match(entry.expectedRefusal.errorContains, /same position/);
  assert.ok(entry.expectedRefusal.why.length > 0);
});

// Every string contains the empty string, so an empty `errorContains` would
// accept any refusal at all while looking like a complete entry.
test("an expected refusal with an empty errorContains is refused", () => {
  const bad = truth();
  bad.subjects.volume__a__b.expectedRefusal = {
    boundary: "volume-geometry",
    errorContains: "",
    why: "because",
  };
  assert.throws(() => validateVolumeTruth(bad, params()), /empty[\s\S]*errorContains/);
});

test("an expected refusal with no stated boundary or reason is refused", () => {
  for (const missing of ["boundary", "why"]) {
    const bad = truth();
    bad.subjects.volume__a__b.expectedRefusal = {
      boundary: "volume-geometry",
      errorContains: "x",
      why: "because",
    };
    bad.subjects.volume__a__b.expectedRefusal[missing] = "";
    assert.throws(() => validateVolumeTruth(bad, params()), /expectedRefusal/, missing);
  }
});

// A subject that never becomes a volume has no uniformity to classify, so a
// file claiming both would be making a claim it cannot support.
test("a subject cannot both expect refusal and classify its uniformity", () => {
  const bad = truth();
  bad.subjects.volume__a__b.uniform = true;
  bad.subjects.volume__a__b.normal = [0, 0, 1];
  bad.subjects.volume__a__b.projectionsMm = [0, 2.5];
  bad.subjects.volume__a__b.gapsMm = [2.5];
  bad.subjects.volume__a__b.expectedRefusal = {
    boundary: "volume-geometry",
    errorContains: "x",
    why: "because",
  };
  assert.throws(() => validateVolumeTruth(bad, params()), /has no[\s\S]*uniformity/);
});
