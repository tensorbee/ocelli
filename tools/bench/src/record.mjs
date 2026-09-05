// The run record, the comparison against the recorded baseline, and --accept.
//
// Two files, and the split follows the size budget's exactly, which is the
// mechanism spike gate A7.3 names when it says to gate on regression beyond a
// tolerance "the same mechanism ci/wasm-size-budget.json already uses for
// binary size":
//
//   tools/bench/out/run.json   gitignored, one run, every subject with its
//                              state, the host class and the pinned versions
//   ci/bench-baseline.json     tracked, the recorded measurement per subject
//                              per host class, with a tolerance
//
// The comparison is deliberately NOT a gate. `bin/ocelli.sh gate bench` asserts
// the instrument's integrity, which is deterministic and needs no GPU, and
// `bin/ocelli.sh bench --compare` compares durations on the machine that owns
// the baseline. A duration comparison on a machine that did not record the
// baseline is either noise or a skip, and this project's rule is that a skip is
// not a pass, so putting it in the floor would produce a permanently amber
// gate, and an amber gate is a gate that gets disabled.

import { INCOMPARABLE, MEASURED } from "./state.mjs";
import { hostClassKey, sameHostClass, sameInstrument } from "./hostclass.mjs";

/** A comparison was made and the figure sits inside the tolerance. */
export const WITHIN = "within";

/** A comparison was made and the figure sits outside it, either way. */
export const MOVED = "moved";

/** Nothing recorded for this subject on this host class. */
export const NO_BASELINE = "no_baseline";

/** The figure went up, went down, or did not move. Arithmetic, no judgement. */
export const UP = "up";
export const DOWN = "down";
export const UNCHANGED = "unchanged";

/** What the movement MEANS, once the unit is known. */
export const BETTER = "better";
export const WORSE = "worse";

/**
 * What an INCREASE means, per unit in `tools/bench/subjects.json`.
 *
 * **Seven of the eleven registry rows are durations and four are not**, which
 * is why this table exists rather than a hardcoded rule. `direction` used to be
 * `delta > 0 ? "slower" : "faster"` for every subject, so a
 * `tier.startup_microbenchmark` figure in `pixels_per_second` that HALVED
 * would have been printed as `faster`, and `cine.frame_change_rate` in
 * `changed_frames_per_second` the same. That is the harness reporting a
 * regression as an improvement, in the one field a reader looks at first.
 *
 * `fraction_of_one_vcpu` is `worse` on an increase for the reason
 * `docs/spikes/A7-tier-c.md` section A7.3 gives, "Ocelli's idle cost must be
 * indistinguishable from zero", and it is listed rather than folded in with
 * the durations because it is not one and the words `slower` and `faster` were
 * never right for it either.
 *
 * **An unrecognised unit throws.** The registry is the authority on units and
 * it declares four. A fifth arriving with a new subject is a decision about
 * which way is better, and defaulting it to "an increase is worse" would make
 * that decision silently and in the wrong direction half the time.
 *
 * **The throw is not the guard, though, and it was being counted as one.** It
 * fires inside `withinTolerance`, which `--compare` reaches only for a subject
 * that already owns a baseline on the machine running it, so a fifth unit
 * lands in the tree with `gate bench` green and fails later on whichever
 * machine happens to own that baseline. `INCREASE_MEANS and the registry
 * declare the same units` in `tools/bench/tests/record_test.mjs` is the
 * binding, and it is the same one
 * `the_recorded_bands_match_the_checked_in_file` gives
 * `FillRateBands::RECORDED` and `ci/tier-thresholds.json`.
 */
export const INCREASE_MEANS = {
  ms: WORSE,
  fraction_of_one_vcpu: WORSE,
  changed_frames_per_second: BETTER,
  pixels_per_second: BETTER,
};

/** An empty baseline, in the shape the tracked file uses. */
export function emptyBaseline() {
  return { host_classes: {} };
}

/**
 * The baseline entry for one subject on one host class, or `null`.
 */
export function baselineEntry(baseline, key, subjectId) {
  const forHost = baseline.host_classes?.[key];
  if (!forHost) {
    return null;
  }
  return forHost.subjects?.[subjectId] ?? null;
}

/**
 * Whether an observed figure sits within a fractional tolerance of a baseline.
 *
 * **Both sides.** An unexplained improvement fails as loudly as an unexplained
 * regression, which is what `ci/wasm-size-budget.json` already does for bytes
 * and for the same reason: during build-out the check does not mean "you
 * exceeded a budget", it means "the figure moved and the move was not
 * declared". A one-sided check would let a measurement quietly stop measuring
 * the thing it was recorded against, because a runner that started timing less
 * work reads as an improvement.
 *
 * `unit` is the recorded unit and it is required, because `direction` alone
 * says which way the number moved and only the unit says what that means. See
 * `INCREASE_MEANS`.
 *
 * **A missing tolerance is refused rather than treated as zero.** `null <= 0`
 * is what `Math.abs(fraction) <= null` reduces to in JavaScript, so an entry
 * with no tolerance used to admit only a bit-identical duration and report
 * every other figure as `moved`, which is a comparison that has quietly become
 * a different check. `scripts/bench_check.py` refuses a tracked entry without
 * a tolerance, and this is the arithmetic's own refusal for a baseline that
 * reached here some other way.
 */
export function withinTolerance(baselineValue, observed, tolerance, unit) {
  if (!(baselineValue > 0)) {
    throw new Error(
      `a baseline of ${baselineValue} cannot carry a fractional tolerance`,
    );
  }
  if (typeof tolerance !== "number" || !Number.isFinite(tolerance) ||
      tolerance <= 0 || tolerance > 1) {
    throw new Error(
      `a tolerance of ${JSON.stringify(tolerance)} is not a fraction above 0 ` +
        "and at most 1. Spike gate A7.3 names ci/wasm-size-budget.json's " +
        "mechanism, and null degenerates to an equality check rather than to " +
        "no check, which is the same comparison wearing a different meaning.",
    );
  }
  const meaning = INCREASE_MEANS[unit];
  if (meaning === undefined) {
    throw new Error(
      `the unit ${JSON.stringify(unit)} declares no direction, so this ` +
        "comparison cannot say whether the figure got better or worse. Add " +
        "it to INCREASE_MEANS in tools/bench/src/record.mjs with the reason, " +
        "which is a decision and not a default.",
    );
  }
  const delta = observed - baselineValue;
  const fraction = delta / baselineValue;
  const direction = delta === 0 ? UNCHANGED : delta > 0 ? UP : DOWN;
  // Written as two named steps rather than one nested conditional, because
  // the polarity is the whole point of the field and a reader has to be able
  // to check it by reading rather than by tracing.
  const wentTheWrongWay = meaning === WORSE
    ? direction === UP
    : direction === DOWN;
  return {
    within: Math.abs(fraction) <= tolerance,
    delta,
    delta_fraction: fraction,
    direction,
    sense: direction === UNCHANGED
      ? UNCHANGED
      : wentTheWrongWay ? WORSE : BETTER,
  };
}

/**
 * Compare one run against the recorded baseline.
 *
 * Returns a new record. Every measured subject gains a `comparison` block, and
 * a subject whose baseline was taken elsewhere becomes `incomparable`, which
 * keeps its number and refuses the verdict.
 */
export function compareRecord(record, baseline) {
  const key = hostClassKey(record.host_class);
  const subjects = record.subjects.map((entry) => {
    if (entry.state !== MEASURED) {
      return entry;
    }
    const recorded = baselineEntry(baseline, key, entry.id);
    if (recorded === null) {
      const anywhere = Object.entries(baseline.host_classes ?? {})
        .filter(([, forHost]) => forHost.subjects?.[entry.id] !== undefined)
        .map(([otherKey]) => otherKey);
      if (anywhere.length === 0) {
        return {
          ...entry,
          comparison: {
            verdict: NO_BASELINE,
            reason:
              "nothing is recorded for this subject on any host class. Run " +
              "--accept on the machine that should own the baseline.",
          },
        };
      }
      return {
        ...entry,
        state: INCOMPARABLE,
        comparison: {
          verdict: INCOMPARABLE,
          reason:
            "this subject is recorded, and on other host classes only. A " +
            "duration recorded on one machine says nothing about another.",
          recorded_on: anywhere,
        },
      };
    }
    // An absent recorded host class or instrument is INCOMPARABLE and never a
    // match by omission. Falling back to the observed values would have made
    // the entry compare equal to itself, which is a comparison that always
    // passes. The map's key is not enough on its own either: it is a lossy
    // string, sanitised with `[^A-Za-z0-9._-]+` to `_`, so two different CPU
    // models can produce one key. The per-entry block is the lossless check and
    // it has to be present to be one. `scripts/bench_check.py` requires both
    // fields on every tracked entry, and this is the same rule for a baseline
    // that reached the comparison some other way.
    if (!recorded.host_class || !recorded.instrument) {
      return {
        ...entry,
        state: INCOMPARABLE,
        comparison: {
          verdict: INCOMPARABLE,
          reason:
            "the recorded figure names no host class or no instrument, so " +
            "there is nothing to establish that it was taken on this machine " +
            "with these tools. The map's key is a lossy string and is not " +
            "enough on its own.",
        },
      };
    }
    const hostDiff = sameHostClass(record.host_class, recorded.host_class);
    const instrumentDiff = sameInstrument(record.instrument,
      recorded.instrument);
    if (!hostDiff.same || !instrumentDiff.same) {
      return {
        ...entry,
        state: INCOMPARABLE,
        comparison: {
          verdict: INCOMPARABLE,
          reason:
            "the recorded figure was taken on a different host class or with " +
            "a different instrument.",
          differing_host_fields: hostDiff.differing,
          differing_instrument_fields: instrumentDiff.differing,
        },
      };
    }
    // The RECORDED unit, not the run's. A subject whose unit changed under a
    // baseline that still names the old one is comparing two different
    // quantities, and taking the unit from the entry being compared against is
    // what makes that visible rather than silently converted.
    const outcome = withinTolerance(recorded.value, entry.value,
      recorded.tolerance, recorded.unit);
    return {
      ...entry,
      comparison: {
        verdict: outcome.within ? WITHIN : MOVED,
        baseline: recorded.value,
        tolerance: recorded.tolerance,
        recorded: recorded.recorded ?? null,
        ...outcome,
      },
    };
  });
  return { ...record, compared_against: key, subjects };
}

/**
 * Re-baseline one subject, or every measured subject in the run.
 *
 * `why` is required and it is not decoration. `ci/wasm-size-budget.json`
 * carries the attribution for its one re-baseline and that is what makes the
 * number readable a year later. A re-baseline with no stated cause is a number
 * replaced by a newer number for no recorded reason.
 */
export function acceptRecord(baseline, record, { subjectId = null, story,
  why, tolerance = null, toleranceWhy = null, today }) {
  if (!story || !why) {
    throw new Error(
      "--accept needs both a story and a reason. A re-baseline with no " +
        "stated cause replaces a measurement with a newer number and loses " +
        "why it moved.",
    );
  }
  if (tolerance !== null && !toleranceWhy) {
    throw new Error(
      "a new tolerance needs its own reason. ci/tier-thresholds.json states " +
        "the provenance of each figure separately, saying which is measured, " +
        "which is derived and which is absent rather than guessed, and a " +
        "tolerance is a derived figure like any other.",
    );
  }
  const key = hostClassKey(record.host_class);
  const next = {
    ...baseline,
    host_classes: { ...(baseline.host_classes ?? {}) },
  };
  const forHost = next.host_classes[key] ?? {
    host_class: record.host_class,
    subjects: {},
  };
  next.host_classes[key] = {
    ...forHost,
    host_class: record.host_class,
    subjects: { ...forHost.subjects },
  };

  const measured = record.subjects.filter(
    (entry) => entry.state === MEASURED || entry.state === INCOMPARABLE,
  );
  const chosen = subjectId === null
    ? measured
    : measured.filter((entry) => entry.id === subjectId);
  if (chosen.length === 0) {
    throw new Error(
      subjectId === null
        ? "this run measured nothing, so there is nothing to record"
        : `this run has no measured figure for ${subjectId}`,
    );
  }
  for (const entry of chosen) {
    const previous = forHost.subjects?.[entry.id] ?? null;
    next.host_classes[key].subjects[entry.id] = {
      value: entry.value,
      unit: entry.unit,
      tolerance: tolerance ?? previous?.tolerance ?? null,
      // A new tolerance is a decision and it carries its own reason. Carrying
      // the old one over carries its old reason with it, so a tolerance can
      // never end up in the file with nothing beside it saying where the
      // number came from.
      tolerance_provenance: tolerance === null
        ? previous?.tolerance_provenance ?? null
        : toleranceWhy,
      provenance: "measured",
      recorded: today,
      story,
      why,
      host_class: record.host_class,
      instrument: record.instrument,
      conditions: record.conditions,
      detail: entry.detail ?? null,
      ...(previous === null ? {} : { replaced: previous.value }),
    };
  }
  return next;
}

/** Every subject in a compared record whose figure moved outside tolerance. */
export function movedSubjects(record) {
  return record.subjects.filter(
    (entry) => entry.comparison?.verdict === MOVED,
  );
}
