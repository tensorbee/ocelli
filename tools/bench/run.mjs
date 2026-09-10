#!/usr/bin/env node
// The benchmark harness. HLD section 26: "Measure with the benchmark harness
// before optimising anything." This is the harness that sentence names.
//
// It is an INSTRUMENT, not a report. Most of its subjects have nothing to
// measure in this tree, because decision D7 puts the oracle and the instruments
// before the port code, and the harness says so and names the story a reader
// should go and read. `--list` is the authority on how many and which, because
// it reads each subject's blocking story out of the backlog and a count written
// here goes stale the first time a story lands, which is exactly what happened
// to the sentence this one replaces. It substitutes no proxy workload, times no stub and
// invents no number.
//
// The HLD states no performance target of any kind. Not a millisecond, not a
// frame rate, not a latency budget. `docs/hld/` was searched for it. The only
// budget-setting method written down in this repository is spike gate A7.3's,
// which is relative to the incumbent viewer and says in terms "Do not invent a
// number." So this harness compares a figure against a figure IT RECORDED, on
// the machine that recorded it, and against nothing else.
//
//   bin/ocelli.sh bench             run what can be run, write the record
//   bin/ocelli.sh bench --compare   compare against ci/bench-baseline.json
//   bin/ocelli.sh bench --accept    re-baseline, with a story and a reason
//   bin/ocelli.sh gate bench        the instrument's own integrity. No duration

import { existsSync } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  BASELINE_PATH,
  DEFAULT_OUT,
  isEntryPoint,
  runnerBasename,
  runnerPath,
} from "./src/paths.mjs";
import { loadRegistry } from "./src/registry.mjs";
import { INCOMPARABLE, MEASURED, subjectState } from "./src/state.mjs";
import { conditions, hostClass, hostClassKey } from "./src/hostclass.mjs";
import {
  acceptRecord,
  compareRecord,
  emptyBaseline,
  movedSubjects,
} from "./src/record.mjs";

const require = createRequire(import.meta.url);

export const USAGE = `bin/ocelli.sh bench [options]

  --list                 print each subject with its definition, the story that
                         gives it a subject and that story's backlog status,
                         then stop. It resolves no state and runs nothing
  --compare              compare the run against ci/bench-baseline.json for
                         this host class. Reports incomparable, never a guess,
                         on a machine that did not record the baseline
  --accept               re-baseline from this run. Needs --story and --why
  --subject <id>         restrict --accept to one subject
  --story <F-ID>         the story a re-baseline belongs to
  --why <text>           what moved, and why. Required, and not decoration
  --tolerance <fraction> the tolerance to record with a new baseline entry
  --tolerance-why <text> where that tolerance came from. Required with it, in
                         the shape ci/tier-thresholds.json states per figure:
                         measured, derived from something named, or absent
  --out <dir>            where the run record goes (default tools/bench/out)
  --no-build             do not rebuild the wasm artefact first. The record is
                         then marked pre-existing and --accept refuses it

The comparison is deliberately NOT in gate --floor or gate --sprint. A duration
comparison on a machine that did not record the baseline is either noise or a
skip, and a skipped gate is not a pass here.
`;

export function parseArgs(argv) {
  const options = {
    list: false,
    compare: false,
    accept: false,
    subject: null,
    story: null,
    why: null,
    tolerance: null,
    toleranceWhy: null,
    out: DEFAULT_OUT,
    build: true,
    help: false,
  };
  const value = (index, flag) => {
    if (index >= argv.length || argv[index] === "" ||
        argv[index].startsWith("--")) {
      throw new Error(`${flag} needs a value. Try --help.`);
    }
    return argv[index];
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--list": options.list = true; break;
      case "--compare": options.compare = true; break;
      case "--accept": options.accept = true; break;
      case "--subject": options.subject = value(++index, arg); break;
      case "--story": options.story = value(++index, arg); break;
      case "--why": options.why = value(++index, arg); break;
      case "--tolerance": {
        const raw = value(++index, arg);
        const parsed = Number(raw);
        if (!Number.isFinite(parsed) || parsed <= 0 || parsed > 1) {
          throw new Error(
            `--tolerance takes a fraction above 0 and at most 1, not ` +
              `${JSON.stringify(raw)}`,
          );
        }
        options.tolerance = parsed;
        break;
      }
      case "--tolerance-why": options.toleranceWhy = value(++index, arg); break;
      case "--out": options.out = value(++index, arg); break;
      case "--no-build": options.build = false; break;
      case "--help": case "-h": options.help = true; break;
      default:
        throw new Error(`unknown argument ${arg}. Try --help.`);
    }
  }
  if (options.accept && (!options.story || !options.why)) {
    throw new Error(
      "--accept needs --story and --why. ci/wasm-size-budget.json carries " +
        "the attribution for its one re-baseline and that is what makes the " +
        "number readable a year later.",
    );
  }
  if (options.tolerance !== null && !options.toleranceWhy) {
    throw new Error(
      "--tolerance needs --tolerance-why. A tolerance is a derived figure " +
        "and ci/tier-thresholds.json states the provenance of each figure " +
        "separately for exactly this reason.",
    );
  }
  if (options.toleranceWhy && options.tolerance === null) {
    throw new Error("--tolerance-why only means something with --tolerance");
  }
  if (options.accept && !options.build) {
    throw new Error(
      "--accept refuses --no-build. A baseline recorded against an artefact " +
        "this run did not build is a number about whatever happened to be in " +
        "crates/ocelli-wasm/pkg.",
    );
  }
  if (options.subject && !options.accept) {
    throw new Error("--subject only means something with --accept");
  }
  return options;
}

/** The tools the figures were taken with, compared alongside the host class. */
function instrument() {
  let playwright = null;
  try {
    playwright = require("playwright/package.json").version;
  } catch {
    // Absent. The record says so rather than claiming a version.
  }
  return { node: process.version, playwright, chromium: null };
}

/**
 * Run every subject that has a subject and a runner.
 *
 * A runner file for a subject whose story has not landed is REFUSED here as
 * well as by `scripts/bench_check.py`. The gate keeps it out of the tracked
 * tree and this keeps a local one from producing a record, which are two
 * different halves of the same rule: the tracked half is about what is
 * committed, this half is about what a number can ever be attached to.
 */
export async function runSubjects(resolved, options) {
  const entries = [];
  const collected = { chromium: null };
  for (const row of resolved) {
    const hasRunner = existsSync(runnerPath(row.subject.id));
    if (hasRunner && !row.subjectExists) {
      throw new Error(
        `REFUSED: ${runnerBasename(row.subject.id)}.mjs exists and ` +
          `${row.subject.id}'s subject story ${row.subject.subject_story} is ` +
          `${row.storyStatus} rather than done. A runner for a subject that ` +
          `does not exist can only be timing a stub.`,
      );
    }
    if (!hasRunner || !row.subjectExists) {
      entries.push(subjectState(row, { hasRunner, value: null }));
      continue;
    }
    let outcome;
    try {
      const module = await import(
        pathToFileURL(runnerPath(row.subject.id)).href
      );
      const result = await module.run({
        outDir: options.out,
        build: options.build,
      });
      if (result.instrument?.chromium) {
        collected.chromium = result.instrument.chromium;
      }
      outcome = { hasRunner, value: result.value, detail: result.detail };
    } catch (error) {
      outcome = {
        hasRunner,
        value: null,
        error: String(error?.stack ?? error?.message ?? error),
      };
    }
    entries.push(subjectState(row, outcome));
  }
  return { entries, collected };
}

function print(record) {
  const width = Math.max(...record.subjects.map((one) => one.id.length));
  for (const entry of record.subjects) {
    const name = entry.id.padEnd(width);
    if (entry.state === MEASURED || entry.state === INCOMPARABLE) {
      const comparison = entry.comparison
        ? `  ${entry.comparison.verdict}`
        : "";
      process.stdout.write(
        `  ${name}  ${entry.state}  ${entry.value} ${entry.unit}` +
          `${comparison}\n`,
      );
      continue;
    }
    const because = entry.blocking_story
      ? `blocked on ${entry.blocking_story} (${entry.blocking_story_status})`
      : entry.reason;
    process.stdout.write(`  ${name}  ${entry.state}  ${because}\n`);
  }
}

export async function main(argv) {
  let options;
  try {
    options = parseArgs(argv);
  } catch (error) {
    process.stderr.write(`${String(error?.message ?? error)}\n`);
    return 2;
  }
  if (options.help) {
    process.stdout.write(USAGE);
    return 0;
  }

  const resolved = loadRegistry();

  if (options.list) {
    for (const row of resolved) {
      const where = row.subject.subject_story === null
        ? "subject exists today"
        : `${row.subject.subject_story} (${row.storyStatus})`;
      process.stdout.write(
        `  ${row.subject.id}\n    ${row.subject.title}\n` +
          `    ${row.subject.definition_hld}\n    ${where}\n`,
      );
    }
    return 0;
  }

  const { entries, collected } = await runSubjects(resolved, options);
  const record = {
    tool: "tools/bench/run.mjs",
    story: "F-006",
    started: new Date().toISOString(),
    artefact_provenance: options.build ? "built by this run" : "pre-existing",
    host_class: hostClass(os),
    host_class_key: hostClassKey(hostClass(os)),
    conditions: conditions(os),
    instrument: { ...instrument(), chromium: collected.chromium },
    subjects: entries,
  };

  let baseline = emptyBaseline();
  if (existsSync(BASELINE_PATH)) {
    baseline = JSON.parse(await readFile(BASELINE_PATH, "utf8"));
  }

  let final = record;
  if (options.compare) {
    final = compareRecord(record, baseline);
  }

  await mkdir(options.out, { recursive: true });
  await writeFile(
    join(options.out, "run.json"),
    `${JSON.stringify(final, null, 2)}\n`,
    "utf8",
  );

  print(final);
  const measured = final.subjects.filter((one) => one.state === MEASURED);
  const unavailable = final.subjects.filter((one) => one.state !== MEASURED &&
    one.state !== INCOMPARABLE);
  process.stdout.write(
    `\n  ${measured.length} measured, ${unavailable.length} unavailable, ` +
      `record in ${join(options.out, "run.json")}\n`,
  );

  // BEFORE --accept, deliberately. A run in which one runner threw and another
  // measured must not write a baseline and exit 0 with the failure visible only
  // in `out/run.json`. With two runners from F-024 onward, a failure in one
  // must prevent accepting the successful result from the other.
  const failedRunners = final.subjects.filter((one) => one.reason ===
    "runner_failed");
  if (failedRunners.length > 0) {
    process.stderr.write(
      `\nFAILED: ${failedRunners.map((one) => one.id).join(", ")}\n`,
    );
    for (const entry of failedRunners) {
      process.stderr.write(`  ${entry.id}: ${entry.error}\n`);
    }
    if (options.accept) {
      process.stderr.write(
        "  Nothing was re-baselined. A run that could not take every figure " +
          "it has a runner for is not the run to record from.\n",
      );
    }
    return 1;
  }

  if (options.accept) {
    const next = acceptRecord(baseline, record, {
      subjectId: options.subject,
      story: options.story,
      why: options.why,
      tolerance: options.tolerance,
      toleranceWhy: options.toleranceWhy,
      today: new Date().toISOString().slice(0, 10),
    });
    await writeFile(
      BASELINE_PATH,
      `${JSON.stringify(next, null, 2)}\n`,
      "utf8",
    );
    process.stdout.write(`  re-baselined into ci/bench-baseline.json\n`);
    return 0;
  }

  if (options.compare) {
    const moved = movedSubjects(final);
    if (moved.length > 0) {
      process.stderr.write(
        `\nMOVED: ${moved.map((one) => `${one.id} ` +
          `${(one.comparison.delta_fraction * 100).toFixed(1)}% ` +
          `${one.comparison.direction} (${one.comparison.sense})`)
          .join(", ")}\n`,
      );
      process.stderr.write(
        "A figure that moved outside its tolerance is not automatically a " +
          "defect. It is a change that was not declared. Re-baseline with " +
          "--accept --story --why, or find what moved.\n",
      );
      return 1;
    }
  }
  return 0;
}

if (isEntryPoint(import.meta.url)) {
  main(process.argv.slice(2)).then(
    (code) => { process.exitCode = code; },
    (error) => {
      process.stderr.write(`${String(error?.stack ?? error)}\n`);
      process.exitCode = 1;
    },
  );
}
