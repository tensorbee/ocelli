// The volume subjects, their committed declaration and their committed truth.
//
// F-X007. The stack pass renders one frame of one instance and asks a series
// nothing about its geometry. This module is the other half: it says which
// corpus directories are rendered as VOLUMES, what is true of each, and what
// the pinned cornerstone3D does with them where the two differ.
//
// Two committed files, and they are separate on purpose:
//
//  * `volume-params.json` declares what to render. It is the volume pass's
//    equivalent of `render-params.json`, and a change to it changes every
//    volume reference frame.
//  * `volume-truth.json` declares what is TRUE, hand-computed from PS3.3
//    C.7.6.2.1.1 and `scripts/corpus_synth.py`, plus the recorded fact of where
//    cornerstone3D 5.8.2 disagrees with it.
//
// Both are strict in both directions, in the manner `unsupported.json` sets:
// a claim that cannot be honoured fails the run, and a claim that is no longer
// true fails the run too. A file of excuses that grows to fit each new failure
// is not a record of anything.
//
// **This module decides nothing about what Ocelli should do with a non-uniform
// series.** No volume builder exists, decision D7 holds, and no HLD section
// states a refusal policy. What this makes is the reference output that makes
// that decision checkable.

import { readFileSync } from "node:fs";

import {
  GEOMETRY_TOLERANCE_MM,
  measureSubject,
  vectorsWithin,
  within,
} from "./geometry.mjs";
import { RESERVED_VOLUME_PREFIX } from "./manifest.mjs";
import { oraclePath } from "./paths.mjs";

export const VOLUME_PARAMS_PATH = oraclePath("volume-params.json");
export const VOLUME_TRUTH_PATH = oraclePath("volume-truth.json");

/**
 * The reserved output-name prefix for a volume frame.
 *
 * Declared in `src/manifest.mjs`, which is where it is enforced: that module
 * refuses a corpus path reducing to an id starting with it, so a future corpus
 * row cannot collide with a volume frame in the flat output directory. No
 * current row does. Re-exported here because this is the story that owns it.
 */
export { RESERVED_VOLUME_PREFIX };

/** The `category` token `corpus/manifest.tsv` puts on every series member. */
export const SERIES_CATEGORY_TOKEN = "series";

/**
 * Keys `render-params.json` owns, refused here.
 *
 * One declaration of each, not two that could drift. `src/params.mjs` refuses
 * a rule that sets `canvas` for the same reason and states it the same way: a
 * parameter recorded in a sidecar as having produced the frame, having
 * produced nothing, is worse than an undeclared one.
 */
const RENDER_PARAMS_KEYS = [
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
];

/** The only `expect` a frame pair may carry, because it is the only one checked. */
const PAIR_EXPECTATIONS = new Set(["identical"]);

/**
 * The spacing components `compareGeometry` actually compares against the
 * reference, and therefore the only ones a declared `referenceDivergence` may
 * name. A divergence nobody checks is a sentence, not a record.
 *
 * `spacing[2]` is the through-plane spacing, compared against the gaps this
 * harness measures from the files. `spacing[0]` and `spacing[1]` are the
 * in-plane pair, compared against PixelSpacing (0028,0030) CROSSWISE, and they
 * joined this list in the S03 sprint review: until then the harness compared
 * no in-plane spacing against the reference at all.
 */
const COMPARED_SPACING_FIELDS = ["spacing[0]", "spacing[1]", "spacing[2]"];

/** The subject id for a corpus directory. */
export function subjectIdFor(seriesDirectory) {
  return `${RESERVED_VOLUME_PREFIX}${seriesDirectory.replace(/\//g, "__")}`;
}

/** The frame id for one subject in one orientation. */
export function frameIdFor(subjectId, orientation) {
  return `${subjectId}__${orientation}`;
}

function requireText(value, field, where) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(
      `${where}: ${field} is ${JSON.stringify(value)}. Every declaration here ` +
        `carries its own reason, because a parameter nobody explained is a ` +
        `parameter nobody can review.`,
    );
  }
  return value;
}

/**
 * Refuse a declaration that cannot be honoured.
 *
 * Separate from the read so `tests/volume_test.mjs` exercises THIS function
 * rather than a copy of its rules, which is `src/unsupported.mjs`'s pattern and
 * its reason: a test that reimplemented the rules would pass whatever this file
 * said.
 */
export function validateVolumeParams(parsed) {
  const where = "volume-params.json";
  if (parsed?.version !== 1) {
    throw new Error(`${where}: version is ${JSON.stringify(parsed?.version)}, expected 1`);
  }
  for (const key of RENDER_PARAMS_KEYS) {
    if (Object.hasOwn(parsed, key)) {
      throw new Error(
        `${where} declares ${JSON.stringify(key)}, which ` +
          `render-params.json already declares. One declaration of each, ` +
          `because two that could drift would put a value in a volume ` +
          `sidecar as the parameter that produced the frame while the frame ` +
          `was produced by the other one.`,
      );
    }
  }
  if (typeof parsed.loadTimeoutMs !== "number" || !(parsed.loadTimeoutMs > 0)) {
    throw new Error(
      `${where}: loadTimeoutMs is ${JSON.stringify(parsed.loadTimeoutMs)}. A ` +
        `volume load is a different unit of work from a single frame, so the ` +
        `timeout is declared here rather than inherited, and a timeout is a ` +
        `failure and never a retry.`,
    );
  }
  requireText(parsed.loadTimeoutWhy, "loadTimeoutWhy", where);
  if (!Array.isArray(parsed.orientations) || parsed.orientations.length === 0) {
    throw new Error(
      `${where}: at least one orientation is needed, or the volume pass would ` +
        `build every volume and render nothing`,
    );
  }
  for (const orientation of parsed.orientations) {
    requireText(orientation, "an orientation", where);
  }
  requireText(parsed.orientationsWhy, "orientationsWhy", where);
  requireText(parsed.blendMode, "blendMode", where);
  requireText(parsed.blendModeWhy, "blendModeWhy", where);
  requireText(parsed.slabThicknessWhy, "slabThicknessWhy", where);
  requireText(parsed.cameraMode, "cameraMode", where);
  requireText(parsed.cameraModeWhy, "cameraModeWhy", where);
  if (parsed.slabThicknessMm !== null && typeof parsed.slabThicknessMm !== "number") {
    throw new Error(
      `${where}: slabThicknessMm is ${JSON.stringify(parsed.slabThicknessMm)}, ` +
        `and it is a number or null. Null is a declaration that the viewport ` +
        `keeps cornerstone3D's own default, and it is not the same as omitting ` +
        `the key.`,
    );
  }

  if (!Array.isArray(parsed.subjects) || parsed.subjects.length === 0) {
    throw new Error(
      `${where} declares no subjects. A declaration with nothing in it is not ` +
        `"nothing to do", it is a volume pass that reports success having ` +
        `rendered nothing.`,
    );
  }

  const seenIds = new Set();
  const seenMembers = new Map();
  for (const subject of parsed.subjects) {
    requireText(subject?.seriesDirectory, "seriesDirectory", where);
    requireText(subject?.why, "why", where);
    const derived = subjectIdFor(subject.seriesDirectory);
    if (subject.id !== derived) {
      throw new Error(
        `${where}: subject ${JSON.stringify(subject.id)} sits in ` +
          `${subject.seriesDirectory}, which derives the id ${derived}. The id ` +
          `is the output name F-011 reads, so it is written down and checked ` +
          `rather than only derived.`,
      );
    }
    if (seenIds.has(subject.id)) {
      throw new Error(`${where}: subject ${subject.id} is declared more than once`);
    }
    seenIds.add(subject.id);
    if (!Array.isArray(subject.members) || subject.members.length < 2) {
      throw new Error(
        `${where}: subject ${subject.id} declares ` +
          `${Array.isArray(subject.members) ? subject.members.length : 0} ` +
          `member(s), and a volume needs at least two to have a gap`,
      );
    }
    for (const member of subject.members) {
      requireText(member, "a member path", where);
      if (!member.startsWith(`${subject.seriesDirectory}/`)) {
        throw new Error(
          `${where}: member ${member} is not under ${subject.seriesDirectory}/, ` +
            `which subject ${subject.id} declares. A subject is one directory.`,
        );
      }
      if (seenMembers.has(member)) {
        throw new Error(
          `${where}: ${member} is declared more than once (${seenMembers.get(member)} ` +
            `and ${subject.id}). A row belongs to exactly one subject.`,
        );
      }
      seenMembers.set(member, subject.id);
    }
  }
  return parsed;
}

/** Read and validate the committed declaration. */
export function readVolumeParams() {
  return validateVolumeParams(JSON.parse(readFileSync(VOLUME_PARAMS_PATH, "utf8")));
}

/**
 * The declaration against the corpus, strict in both directions.
 *
 * Returns the problems rather than throwing, so the run reports every one at
 * once and reaches the single failure path that discards the output.
 */
export function checkSubjectsAgainstManifest(params, allRows) {
  const problems = [];
  const byPath = new Map(allRows.map((row) => [row.path, row]));
  const claimed = new Map();

  for (const subject of params.subjects) {
    for (const member of subject.members) {
      claimed.set(member, subject.id);
      if (!byPath.has(member)) {
        problems.push(
          `volume-params.json: subject ${subject.id} declares ${member}, which ` +
            `is not a row of corpus/manifest.tsv. Reference output is only ` +
            `meaningful against the corpus the manifest describes.`,
        );
      }
    }
  }

  for (const row of allRows) {
    if (!row.categories.includes(SERIES_CATEGORY_TOKEN)) {
      continue;
    }
    if (!claimed.has(row.path)) {
      problems.push(
        `corpus/manifest.tsv: ${row.path} carries the ` +
          `${JSON.stringify(SERIES_CATEGORY_TOKEN)} category token and belongs ` +
          `to no declared volume subject. A series row nobody assembles is a ` +
          `row this story covers as pixels and asks nothing as geometry, which ` +
          `is exactly the gap F-X007 exists to close.`,
      );
    }
  }
  return problems;
}

/**
 * Which subjects this run may attempt, given the rows it selected.
 *
 * A subject is attempted only when EVERY member is selected. A partially
 * selected one is refused BY NAME rather than skipped, so
 * `--rows synthetic/ct_series_uniform/slice_000.dcm` says what it did rather
 * than producing no volumes and reporting success.
 */
export function selectSubjects(params, selectedPaths) {
  const attempted = [];
  const problems = [];
  for (const subject of params.subjects) {
    const present = subject.members.filter((member) => selectedPaths.has(member));
    if (present.length === subject.members.length) {
      attempted.push(subject);
      continue;
    }
    if (present.length === 0) {
      continue;
    }
    problems.push(
      `volume subject ${subject.id}: ${present.length} of ` +
        `${subject.members.length} member(s) are in this run's row selection. ` +
        `A volume is assembled from all of its members or from none, and a ` +
        `partial subject is refused by name rather than skipped, because a ` +
        `run that quietly produced no volume would report success.`,
    );
  }
  return { attempted, problems };
}

function maxDeviationOf(gaps) {
  const mean = gaps.reduce((total, gap) => total + gap, 0) / gaps.length;
  return Math.max(...gaps.map((gap) => Math.abs(gap - mean)));
}

/**
 * Refuse a truth record that cannot say what it claims.
 *
 * The `params` argument is what makes this strict in both directions: every
 * declared subject has an entry here and every entry here names a declared
 * subject.
 */
export function validateVolumeTruth(parsed, params) {
  const where = "volume-truth.json";
  if (parsed?.version !== 1) {
    throw new Error(`${where}: version is ${JSON.stringify(parsed?.version)}, expected 1`);
  }
  requireText(parsed.cornerstone3DVersion, "cornerstone3DVersion", where);
  if (typeof parsed.toleranceMm !== "number" || !(parsed.toleranceMm > 0)) {
    throw new Error(
      `${where}: toleranceMm is ${JSON.stringify(parsed.toleranceMm)}. HLD ` +
        `25.1 gives geometry as "world coordinates within 1e-6 mm" and this ` +
        `file sets no other.`,
    );
  }

  const declared = new Set(params.subjects.map((subject) => subject.id));
  const entries = parsed.subjects ?? {};
  for (const id of declared) {
    if (!Object.hasOwn(entries, id)) {
      throw new Error(
        `${where}: subject ${id} is declared in volume-params.json and has no ` +
          `entry in volume-truth.json. A subject nobody made a claim about is ` +
          `a subject whose reference output means nothing, and "we did not ` +
          `look" is a claim that has to be written down too.`,
      );
    }
  }
  for (const id of Object.keys(entries)) {
    if (!declared.has(id)) {
      throw new Error(
        `${where}: entry ${id} names no declared subject in ` +
          `volume-params.json. A claim about something nobody renders cannot ` +
          `go stale visibly.`,
      );
    }
    const entry = entries[id];
    requireText(entry?.citation, `subjects.${id}.citation`, where);
    requireText(entry?.why, `subjects.${id}.why`, where);
    if (entry.uniform !== null && typeof entry.uniform !== "boolean") {
      throw new Error(
        `${where}: subjects.${id}.uniform is ${JSON.stringify(entry.uniform)}, ` +
          `and it is true, false, or null for a subject this file classifies ` +
          `nothing about`,
      );
    }
    if (entry.expectedRefusal !== undefined) {
      // The volume pass's `unsupported.json`, per subject. A series that
      // CANNOT be one volume is declared and named with its reason, never
      // silently dropped, which is the same rule the stack pass applies to a
      // row the reference cannot decode. Strict in both directions: a declared
      // refusal that does not occur fails the run, and an undeclared one fails
      // the run.
      const refusal = entry.expectedRefusal;
      requireText(refusal?.boundary, `subjects.${id}.expectedRefusal.boundary`, where);
      requireText(refusal?.why, `subjects.${id}.expectedRefusal.why`, where);
      if (typeof refusal.errorContains !== "string" || refusal.errorContains === "") {
        throw new Error(
          `${where}: subjects.${id}.expectedRefusal has an empty ` +
            `errorContains. Every string contains the empty string, so that ` +
            `would accept any refusal at all while looking like a complete ` +
            `entry.`,
        );
      }
      if (entry.uniform !== null) {
        throw new Error(
          `${where}: subjects.${id} expects to be refused and also classifies ` +
            `its uniformity. A subject that never becomes a volume has no ` +
            `uniformity to classify.`,
        );
      }
    }
    if (entry.uniform === null) {
      for (const key of ["projectionsMm", "gapsMm", "normal", "zProfile"]) {
        if (entry[key] !== undefined) {
          throw new Error(
            `${where}: subjects.${id} classifies nothing and still declares ` +
              `${key}. An expectation nobody checks is not an expectation.`,
          );
        }
      }
      // A `referenceDivergence` IS allowed on an entry that classifies
      // nothing, and this rule used to forbid it. The reasoning it carried,
      // that a divergence is measured against a truth, conflates two
      // questions. Whether the reference's single through-plane spacing
      // describes the measured gaps is answered by comparing cornerstone3D's
      // own resolved `spacing[2]` against gaps this harness measured from the
      // files, and no declared truth enters it. What a truth would be needed
      // for is a UNIFORMITY verdict, which `uniform: null` declines to give,
      // and that is decision 3 of the S03 design round and stands.
      //
      // Found by the S03 sprint review. The same conflation sat in three
      // places: here, in the early return in `compareGeometry`, and in a unit
      // test that asserted the resulting silence was correct. Its consequence
      // was that `real/mr_eay131` published `referenceDivergence: null` while
      // its gaps ran 5 to 50 mm against a resolved 10 mm, and F-011's
      // attribution ladder reads that field at rung 3, so a null sent it to
      // rung 5 whose default is `ours`.
      //
      // What IS still required is that such an entry states a `truth` of null
      // rather than a number, because a number there would be a uniformity
      // claim wearing a divergence's clothes.
      if (entry.referenceDivergence !== null &&
          entry.referenceDivergence.truth !== null) {
        throw new Error(
          `${where}: subjects.${id} classifies nothing, so its ` +
            `referenceDivergence must state a truth of null. It states ` +
            `${JSON.stringify(entry.referenceDivergence.truth)}, which is a ` +
            `uniformity claim by another name.`,
        );
      }
      continue;
    }

    const projections = entry.projectionsMm;
    const gaps = entry.gapsMm;
    if (!Array.isArray(projections) || projections.length < 2 ||
        !Array.isArray(gaps) || gaps.length !== projections.length - 1) {
      throw new Error(
        `${where}: subjects.${id} declares ${JSON.stringify(projections?.length)} ` +
          `projection(s) and ${JSON.stringify(gaps?.length)} gap(s). N ` +
          `positions have N-1 gaps.`,
      );
    }
    if (!Array.isArray(entry.normal) || entry.normal.length !== 3) {
      throw new Error(`${where}: subjects.${id} declares no three-element normal`);
    }
    for (let index = 1; index < projections.length; index += 1) {
      const derived = projections[index] - projections[index - 1];
      if (!within(derived, gaps[index - 1], parsed.toleranceMm)) {
        throw new Error(
          `${where}: subjects.${id}'s gaps do not follow from its own ` +
            `projections. Gap ${index - 1} is declared ${gaps[index - 1]} and ` +
            `the projections give ${derived}. A file that contradicts itself ` +
            `is an authoritative-looking wrong answer.`,
        );
      }
    }
    const deviation = maxDeviationOf(gaps);
    const impliedUniform = deviation <= parsed.toleranceMm;
    if (impliedUniform !== entry.uniform) {
      throw new Error(
        `${where}: subjects.${id} declares uniform ${entry.uniform} and its own ` +
          `gaps deviate from their mean by ${deviation} mm, which against the ` +
          `declared tolerance of ${parsed.toleranceMm} mm makes it ` +
          `${impliedUniform}`,
      );
    }
    if (entry.zProfile !== undefined) {
      const profile = entry.zProfile;
      requireText(profile?.citation, `subjects.${id}.zProfile.citation`, where);
      requireText(profile?.why, `subjects.${id}.zProfile.why`, where);
      if (!Array.isArray(profile.voxel) || profile.voxel.length !== 2 ||
          profile.voxel.some((value) => !Number.isInteger(value) || value < 0)) {
        throw new Error(
          `${where}: subjects.${id}.zProfile.voxel is ` +
            `${JSON.stringify(profile.voxel)} and it is two non-negative ` +
            `integers, an in-plane column and row`,
        );
      }
      if (!Number.isFinite(profile.expectedFirstValue) ||
          !Number.isFinite(profile.expectedStepPerSlice) ||
          profile.expectedStepPerSlice === 0) {
        throw new Error(
          `${where}: subjects.${id}.zProfile needs a finite ` +
            `expectedFirstValue and a non-zero expectedStepPerSlice. A step of ` +
            `zero is what a volume built from one slice ten times produces, so ` +
            `it cannot also be the expectation.`,
        );
      }
    }
    if (entry.referenceDivergence !== null) {
      const divergence = entry.referenceDivergence;
      requireText(divergence?.field, `subjects.${id}.referenceDivergence.field`, where);
      requireText(divergence?.why, `subjects.${id}.referenceDivergence.why`, where);
      if (!COMPARED_SPACING_FIELDS.includes(divergence.field)) {
        throw new Error(
          `${where}: subjects.${id} declares a divergence in ` +
            `${JSON.stringify(divergence.field)}, and the fields the harness ` +
            `compares are ` +
            `${COMPARED_SPACING_FIELDS.map((field) => JSON.stringify(field)).join(", ")}. ` +
            `A divergence nobody checks is a sentence, not a record.`,
        );
      }
      if (divergence.attributedTo !== "reference" && divergence.attributedTo !== "ocelli") {
        throw new Error(
          `${where}: subjects.${id}'s divergence is attributed to ` +
            `${JSON.stringify(divergence.attributedTo)}. Decision D14 says a ` +
            `divergence is attributed to a side.`,
        );
      }
      if (typeof divergence.reference !== "number") {
        throw new Error(
          `${where}: subjects.${id}'s divergence declares no numeric reference ` +
            `value to compare the built volume against`,
        );
      }
    }
  }

  for (const pair of parsed.framePairs ?? []) {
    for (const side of ["left", "right"]) {
      if (!declared.has(pair?.[side])) {
        throw new Error(
          `${where}: frame pair ${side} ${JSON.stringify(pair?.[side])} names ` +
            `no declared subject`,
        );
      }
    }
    requireText(pair?.why, "a frame pair's why", where);
    if (!PAIR_EXPECTATIONS.has(pair.expect)) {
      throw new Error(
        `${where}: frame pair ${pair.left} against ${pair.right} expects ` +
          `${JSON.stringify(pair.expect)}, and ` +
          `${[...PAIR_EXPECTATIONS].map((value) => JSON.stringify(value)).join(", ")} ` +
          `is the only expectation the harness checks. An expectation the ` +
          `harness ignores is worse than none, because the file would read as ` +
          `having made a claim.`,
      );
    }
  }
  return parsed;
}

/** Read and validate the committed truth. */
export function readVolumeTruth() {
  return JSON.parse(readFileSync(VOLUME_TRUTH_PATH, "utf8"));
}

/**
 * The assembled volume's z profile against its declared ramp.
 *
 * A volume built from the wrong slices, from one slice repeated, or with two
 * slices transposed renders a perfectly plausible frame that hashes stably.
 * That is the degeneracy the stack path has no analogue for, and the ramp
 * `scripts/corpus_synth.py` writes is what makes it visible.
 *
 * The step is compared exactly and not within a tolerance, because the values
 * are integers: `case_series` writes `trap_frame(probe=False) + index * 16`
 * into a 12-bit-in-16 unsigned container (PS3.3 C.7.6.3.1.4) and the reference
 * rescales it with an integer slope and intercept (PS3.3 C.11.1).
 */
export function checkZProfile(subjectId, values, expectation) {
  const problems = [];
  const { voxel, expectedFirstValue, expectedStepPerSlice } = expectation;
  if (!Array.isArray(values) || values.length < 2) {
    problems.push(
      `${subjectId}: the z profile at voxel ${JSON.stringify(voxel)} came back ` +
        `with ${Array.isArray(values) ? values.length : 0} value(s), and a ramp ` +
        `needs at least two`,
    );
    return problems;
  }
  if (values[0] !== expectedFirstValue) {
    problems.push(
      `${subjectId}: the z profile at voxel ${JSON.stringify(voxel)} starts at ` +
        `${values[0]} and volume-truth.json computes ${expectedFirstValue} by ` +
        `hand from PS3.3 C.11.1 and scripts/corpus_synth.py. The step can be ` +
        `right while the first value is wrong, which is an off-by-one in the ` +
        `in-plane index rather than in the slice order.`,
    );
  }
  for (let index = 1; index < values.length; index += 1) {
    const step = values[index] - values[index - 1];
    if (step !== expectedStepPerSlice) {
      problems.push(
        `${subjectId}: the z profile at voxel ${JSON.stringify(voxel)} steps by ` +
          `${step} from slice ${index - 1} to slice ${index}, and ` +
          `scripts/corpus_synth.py writes a ramp of exactly ` +
          `${expectedStepPerSlice} per slice. A volume that dropped, ` +
          `duplicated or reordered a slice renders a plausible frame and ` +
          `hashes stably, so this is what catches it. Values: ` +
          `${JSON.stringify(values)}`,
      );
      break;
    }
  }
  return problems;
}

/**
 * The measured geometry against the declared truth, and the reference against
 * both.
 *
 * `measured` comes from `geometry.mjs`, which reads the files and never reads
 * a cornerstone3D module. `referenceGeometry` is what the reference itself
 * built. Comparing the two against a hand-computed truth is what lets the
 * output show the reference getting a series wrong, which is the case here.
 */
export function compareGeometry({
  subjectId,
  measured,
  referenceGeometry,
  truth,
  toleranceMm = GEOMETRY_TOLERANCE_MM,
}) {
  const problems = [];
  const record = {
    source: "volume-truth.json",
    uniform: truth?.uniform ?? null,
    citation: truth?.citation ?? null,
    toleranceMm,
  };

  // Whether the reference's single through-plane spacing describes every
  // measured gap needs NO truth. It compares cornerstone3D's own resolved
  // spacing[2] against gaps this harness measured from the files, so it is
  // answerable for a real series exactly as it is for a synthetic one.
  // Computed HERE, above the uniformity early return, and not below it.
  //
  // **This was a defect and the sprint review found it.** The early return
  // below skipped the whole block, so `real/mr_eay131` shipped
  // `referenceDivergence: null` while its gaps ran 5 to 50 mm against a
  // resolved 10 mm. F-011's attribution ladder reads that field at rung 3, so
  // a null sent the ladder to rung 5, whose default is "ours". The guard's own
  // message says why that matters: a divergence nobody wrote down is a
  // divergence F-011 would attribute to Ocelli. Declining to JUDGE a real
  // series' uniformity is decision 3 and stands. Declining to MEASURE the
  // reference's divergence was an accident of where the return sat.
  const referenceZ = referenceGeometry?.spacing?.[2];
  const referenceDescribesEveryGap =
    typeof referenceZ === "number" &&
    measured.gapsMm.every((gap) => within(gap, referenceZ, toleranceMm));
  const declaredDivergence = truth?.referenceDivergence ?? null;

  // A subject declares at most one divergence, so every branch below asks for
  // the one naming ITS field rather than for any divergence at all. Without
  // that, declaring an in-plane divergence would silently excuse a
  // through-plane one.
  const divergenceIn = (field) =>
    declaredDivergence !== null && declaredDivergence.field === field
      ? declaredDivergence
      : null;

  // **The in-plane spacing, against the reference's own. The S03 sprint
  // review's smell S18 is that nothing compared it at all.**
  //
  // PS3.3 C.7.6.2.1.1 gives PixelSpacing (0028,0030) as [between rows, between
  // columns]. cornerstone3D 5.8.2 builds its volume spacing as
  // `[PixelSpacing[1], PixelSpacing[0], zSpacing]` in
  // `generateVolumePropsFromImageIds`, so the two orderings are REVERSED with
  // respect to each other and the two arrays must agree crosswise. Its own
  // locals for that pair are named `rowSpacing` and `columnSpacing` in the
  // reversed sense, so the names in the vendored bundle are no guide.
  //
  // A transposition is invisible on a square-pixel series and renders a
  // plausible, stably hashing frame on any other, which is what the corpus's
  // non-square [0.5, 0.25] row exists to catch. Like the through-plane check
  // it needs no truth, so it sits above the uniformity early return and
  // answers for a real series too.
  const referenceInPlane = Array.isArray(referenceGeometry?.spacing)
    ? referenceGeometry.spacing
    : null;
  const measuredInPlane = Array.isArray(measured.pixelSpacing)
    ? measured.pixelSpacing
    : null;
  const inPlaneDeclared = divergenceIn("spacing[0]") ?? divergenceIn("spacing[1]");
  if (referenceInPlane === null || referenceInPlane.length < 2 ||
      measuredInPlane === null || measuredInPlane.length !== 2) {
    problems.push(
      `${subjectId}: the in-plane spacing cannot be compared. The reference ` +
        `published ${JSON.stringify(referenceGeometry?.spacing)} and the ` +
        `harness measured ${JSON.stringify(measured.pixelSpacing)}. A check ` +
        `that quietly does nothing is worse than no check.`,
    );
  } else {
    const inPlaneAgrees =
      within(referenceInPlane[0], measuredInPlane[1], toleranceMm) &&
      within(referenceInPlane[1], measuredInPlane[0], toleranceMm);
    const transposed =
      !inPlaneAgrees &&
      within(referenceInPlane[0], measuredInPlane[0], toleranceMm) &&
      within(referenceInPlane[1], measuredInPlane[1], toleranceMm);
    if (!inPlaneAgrees && inPlaneDeclared === null) {
      problems.push(
        `${subjectId}: cornerstone3D resolved the in-plane spacing as ` +
          `[${referenceInPlane[0]}, ${referenceInPlane[1]}] and the files ` +
          `declare PixelSpacing (0028,0030) as ` +
          `[${measuredInPlane[0]}, ${measuredInPlane[1]}]. PS3.3 C.7.6.2.1.1 ` +
          `makes PixelSpacing [between rows, between columns] and the ` +
          `reference emits [PixelSpacing[1], PixelSpacing[0], zSpacing], so ` +
          `the two must agree crosswise and volume-truth.json declares no ` +
          `referenceDivergence for it.` +
          (transposed
            ? ` They agree in order instead, which is the transposition ` +
              `itself rather than a rounding.`
            : ``),
      );
    }
    if (inPlaneAgrees && inPlaneDeclared !== null) {
      problems.push(
        `${subjectId}: volume-truth.json declares a referenceDivergence in ` +
          `${inPlaneDeclared.field} and it did not occur. The reference's ` +
          `in-plane spacing agrees with PixelSpacing (0028,0030). Remove the ` +
          `entry: a stale claim reads as a known limit and hides a reference ` +
          `that improved.`,
      );
    }
  }

  if (!truth || truth.uniform === null) {
    // Decision 3 of the design round: a real series is measured and judged by
    // nothing. HLD 25.1's 1e-6 mm is a comparison tolerance and is exact for
    // the synthetic subjects by construction. Applying it to a real series
    // would produce a verdict the corpus cannot support. That is about
    // UNIFORMITY. The reference divergence below is a different question and
    // is answered.
    if (!referenceDescribesEveryGap && divergenceIn("spacing[2]") === null) {
      problems.push(
        `${subjectId}: cornerstone3D resolved spacing[2] as ${referenceZ} mm ` +
          `and the measured gaps are ${JSON.stringify(measured.gapsMm)}, so ` +
          `the reference's single through-plane spacing does not describe ` +
          `this series, and volume-truth.json declares no referenceDivergence ` +
          `for it. A divergence nobody wrote down is a divergence F-011 would ` +
          `attribute to Ocelli. This subject declares uniform null, which ` +
          `declines to judge the series and does not excuse an undeclared ` +
          `divergence.`,
      );
    }
    return {
      truth: record,
      uniform: null,
      referenceAgreesWithTruth: referenceDescribesEveryGap,
      referenceDivergence: declaredDivergence,
      problems,
    };
  }

  const declared = [
    ["normal", measured.normal, truth.normal],
    ["projectionsMm", measured.projectionsMm, truth.projectionsMm],
    ["gapsMm", measured.gapsMm, truth.gapsMm],
  ];
  for (const [field, actual, expected] of declared) {
    if (!vectorsWithin(actual, expected, toleranceMm)) {
      problems.push(
        `${subjectId}: the harness measured ${field} as ` +
          `${JSON.stringify(actual)} and volume-truth.json declares ` +
          `${JSON.stringify(expected)}, outside HLD 25.1's ${toleranceMm} mm. ` +
          `The measurement comes from the files through ` +
          `tools/oracle/src/geometry.mjs and the expectation is hand-computed ` +
          `from PS3.3 C.7.6.2.1.1, so a disagreement is one of the two being ` +
          `wrong and not a tolerance to widen.`,
      );
    }
  }

  const uniform = measured.maxDeviationFromMeanMm <= toleranceMm;
  if (uniform !== truth.uniform) {
    problems.push(
      `${subjectId}: the harness measures the gaps as deviating from their ` +
        `mean by ${measured.maxDeviationFromMeanMm} mm, which against ` +
        `${toleranceMm} mm makes the series ${uniform ? "uniform" : "non-uniform"}, ` +
        `and volume-truth.json declares uniform ${truth.uniform}`,
    );
  }

  // The reference agrees with the truth only when its single through-plane
  // spacing describes EVERY gap. cornerstone3D 5.8.2 computes that number from
  // the endpoints alone, so on a series whose interior gaps differ it answers a
  // mean that no interior slice sits at.
  // Hoisted above the uniformity early return, so a subject that declines to
  // judge uniformity still answers this. Reused rather than recomputed.
  const referenceAgreesWithTruth = referenceDescribesEveryGap;
  const divergence = divergenceIn("spacing[2]");
  if (!referenceAgreesWithTruth && divergence === null) {
    problems.push(
      `${subjectId}: cornerstone3D resolved spacing[2] as ${referenceZ} mm and ` +
        `the measured gaps are ${JSON.stringify(measured.gapsMm)}, so the ` +
        `reference's single through-plane spacing does not describe this ` +
        `series, and volume-truth.json declares no referenceDivergence for it. ` +
        `A divergence nobody wrote down is a divergence F-011 would attribute ` +
        `to Ocelli.`,
    );
  }
  if (referenceAgreesWithTruth && divergence !== null) {
    problems.push(
      `${subjectId}: volume-truth.json declares a referenceDivergence in ` +
        `${divergence.field} and it did not occur. cornerstone3D resolved ` +
        `spacing[2] as ${referenceZ} mm, which describes every measured gap. ` +
        `Remove the entry: a stale claim reads as a known limit and hides a ` +
        `reference that improved.`,
    );
  }
  if (divergence !== null && typeof referenceZ === "number" &&
      !within(referenceZ, divergence.reference, toleranceMm)) {
    problems.push(
      `${subjectId}: volume-truth.json records the reference's ` +
        `${divergence.field} as ${divergence.reference} and cornerstone3D ` +
        `resolved ${referenceZ}. The recorded fact has moved, so the reasoning ` +
        `written beside it may not hold either.`,
    );
  }

  return {
    truth: record,
    uniform,
    referenceAgreesWithTruth,
    // The DECLARED divergence, whatever spacing component it names, and not
    // just the through-plane one the branches above ask for. F-011's rung 3
    // attributes a geometry difference to the reference on this field, and an
    // in-plane divergence explains one exactly as a through-plane divergence
    // does. The early return above publishes the same thing.
    referenceDivergence: declaredDivergence,
    problems,
  };
}

/**
 * The declared frame pairs, against the digests the run produced.
 *
 * This is the sharpest claim in the file. The uniform and the non-uniform
 * subjects differ only in slice 7's `ImagePositionPatient`, and cornerstone3D
 * 5.8.2 averages that difference away, so it is PREDICTED to render the two to
 * bit-identical reformats. Recording that as pairs of equal digests is
 * stronger and cheaper than recording it as a sentence, and if the reference
 * ever stops averaging this is what goes red.
 */
export function comparePairs(framePairs, framesBySubject) {
  const records = [];
  const problems = [];
  for (const pair of framePairs ?? []) {
    const left = framesBySubject[pair.left];
    const right = framesBySubject[pair.right];
    if (!left || !right) {
      problems.push(
        `frame pair ${pair.left} against ${pair.right}: ` +
          `${!left ? pair.left : pair.right} produced no frames, so the pair ` +
          `would pass by having compared nothing`,
      );
      continue;
    }
    const orientations = [];
    for (const orientation of Object.keys(left)) {
      const leftSha256 = left[orientation] ?? null;
      const rightSha256 = right[orientation] ?? null;
      orientations.push({
        orientation,
        leftSha256,
        rightSha256,
        identical: leftSha256 !== null && leftSha256 === rightSha256,
      });
    }
    records.push({ ...pair, orientations });
    if (pair.expect === "identical") {
      const differing = orientations.filter((entry) => !entry.identical);
      if (differing.length > 0) {
        problems.push(
          `frame pair ${pair.left} against ${pair.right} expects identical ` +
            `frames and ${differing.map((entry) => entry.orientation).join(", ")} ` +
            `differ (${differing
              .map((entry) => `${entry.orientation}: ${entry.leftSha256} then ${entry.rightSha256}`)
              .join("; ")}). ${pair.why}`,
        );
      }
    }
  }
  return { records, problems };
}

/** Measure one subject from its members' own attributes. Re-exported for the driver. */
export { measureSubject };
