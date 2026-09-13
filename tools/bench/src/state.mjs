// The states a subject can be in for one run, and the fourth that is refused.
//
//   measured      a runner exists, it ran, and the record carries a number and
//                 a host class
//   unavailable   there is no number, and the record says why. When a story
//                 blocks the subject the record names that F-ID, so a reader
//                 gets pointed at the story rather than at a blank
//   incomparable  a number exists and the recorded baseline was taken on a
//                 different host class or with a different instrument, so no
//                 comparison is made. A duration is not portable and this is
//                 where that is admitted rather than averaged over
//   refused       a number for a pending, archived or superseded subject. Not
//                 a state, an exception, and `scripts/bench_check.py` refuses
//                 the tracked half of the same mistake
//
// The fourth is the defect this whole story is most likely to produce. A
// benchmark harness under time pressure invents a workload, produces a
// plausible number, and that number sits in a tracked file describing nothing.
// `unavailable` is the correct output here and it is a useful one.

import { runnerBasename } from "./paths.mjs";

/** A number, and either no comparison was asked for or it was comparable. */
export const MEASURED = "measured";

/** No number. `reason` says which kind of absence. */
export const UNAVAILABLE = "unavailable";

/** A number, and nothing to compare it against on this machine. */
export const INCOMPARABLE = "incomparable";

/** The story that gives this subject something to measure has not landed. */
export const NO_SUBJECT = "no_subject";

/** The subject exists and no runner has been written for it yet. */
export const NO_RUNNER = "no_runner";

/** The runner ran and failed. The failure is carried, never swallowed. */
export const RUNNER_FAILED = "runner_failed";

/**
 * The state of one subject in one run.
 *
 * @param {object} resolved a row from `resolveSubjects`
 * @param {{hasRunner: boolean, value: number|null, detail: object|null,
 *          error: string|null}} outcome
 * @returns {object} the record entry for this subject
 */
export function subjectState(resolved, outcome) {
  const { subject, storyStatus, subjectExists } = resolved;
  const { hasRunner, value = null, detail = null, error = null } = outcome;

  const entry = {
    id: subject.id,
    title: subject.title,
    unit: subject.unit,
    tiers: subject.tiers,
    definition_hld: subject.definition_hld,
  };

  if (value !== null && !subjectExists) {
    throw new Error(
      `REFUSED: a value was produced for ${subject.id}, whose subject story ` +
        `${subject.subject_story} is ${storyStatus} rather than ` +
        `in-progress or done. A ` +
        `number recorded for a thing that does not exist describes nothing, ` +
        `and it is the one output this harness must never produce. Report ` +
        `unavailable and name the story instead.`,
    );
  }
  if (value !== null && !hasRunner) {
    throw new Error(
      `REFUSED: a value was produced for ${subject.id} and no runner file ` +
        `exists for it. A number with no instrument has no provenance.`,
    );
  }

  if (!subjectExists) {
    return {
      ...entry,
      state: UNAVAILABLE,
      reason: NO_SUBJECT,
      blocking_story: subject.subject_story,
      blocking_story_status: storyStatus,
      note: subject.note ?? null,
    };
  }
  if (!hasRunner) {
    return {
      ...entry,
      state: UNAVAILABLE,
      reason: NO_RUNNER,
      blocking_story: subject.subject_story,
      blocking_story_status: storyStatus,
      // `runnerBasename` and not a second `replaceAll(".", "_")` here. This
      // note tells a reader where to put a runner and `runnerPath` is what
      // decides whether the harness can see one, so two spellings of the
      // convention would let this file name a path the gate does not look at.
      // That is the failure `paths.mjs` describes and it was spelled twice
      // until the S03 review's eighth pass, in the module whose header says a
      // rule living on one side only is a defect.
      note:
        "the subject exists and no runner has been written for it. Add " +
        `tools/bench/src/runners/${runnerBasename(subject.id)}.mjs.`,
    };
  }
  if (error !== null) {
    return {
      ...entry,
      state: UNAVAILABLE,
      reason: RUNNER_FAILED,
      blocking_story: null,
      error,
    };
  }
  if (value === null) {
    throw new Error(
      `${subject.id} has a runner, reported no error, and produced no ` +
        `value. A runner that returns nothing and says nothing is worse ` +
        `than one that throws.`,
    );
  }
  return { ...entry, state: MEASURED, value, detail };
}
