// The subject registry: parse it, and resolve each row's blocking story.
//
// This module answers one question for the driver: for each subject, does a
// subject EXIST today, and if not, which story is a reader supposed to go and
// read. It does not decide what the repository is allowed to contain. That is
// `scripts/bench_check.py`, which is the gate, and the two are deliberately
// different jobs over the same data:
//
//   the gate    refuses a runner file or a baseline entry for a subject whose
//               story is not done, and fails CI
//   this module refuses to RUN a subject it cannot label, so that no record can
//               ever carry a number beside a story that has not landed
//
// THE OVERLAP IS LARGER THAN ONE FACT AND SAYING OTHERWISE WOULD HIDE THE COST.
// Both sides carry the required-field list, the valid tiers, the valid backlog
// statuses, the status-table parser with its `### M<n>,` and `Roadmap` heading
// rule, the duplicate-id rule and the story resolution. That is a real
// duplication in two languages, kept because the gate has to run under `python3`
// with no node install and the driver has to refuse before it produces a record.
//
// **The rules are kept identical deliberately**, and a rule that exists on one
// side only is a defect rather than a difference: a registry the driver refuses
// and the gate accepts would land in the tree and then fail for whoever next ran
// the harness. `runner_basename` here and `runner_basename` in
// `scripts/bench_check.py` are the model, and both say so at their site.
//
// The one asymmetry that is intended is the CONSEQUENCE. The gate reports a
// problem and fails CI. This module throws, because a row it cannot label is a
// row it cannot honestly put a state on, and a run that guessed would produce
// the record this whole story exists to refuse.

import { readFileSync } from "node:fs";

import { repoPath, SUBJECTS_PATH } from "./paths.mjs";

/** Fields every row must carry. An absent one is a refusal, not a default. */
export const REQUIRED_FIELDS = [
  "id",
  "title",
  "definition_hld",
  "definition",
  "unit",
  "dimensions",
  "tiers",
  "subject_story",
  "feeds",
];

/** Tiers a row may declare. `n/a` is a legitimate answer, per deviation D-07. */
export const VALID_TIERS = ["A", "B", "C", "n/a"];

/** The statuses `docs/sprints/BACKLOG.md` uses. Mirrors backlog_check.py. */
export const VALID_STATUS = [
  "pending",
  "in-progress",
  "done",
  "archived",
  "superseded",
];

const BACKLOG_ROW = /^\|\s*(F-X?\d{3}[a-z]?)\s*\|(.*)$/;

/**
 * Parse the registry text and check its shape.
 *
 * @param {string} text the contents of `subjects.json`
 * @returns {{subjects: object[]}}
 */
export function parseRegistry(text) {
  const parsed = JSON.parse(text);
  if (!Array.isArray(parsed.subjects)) {
    throw new Error("subjects.json has no `subjects` array");
  }
  const seen = new Set();
  for (const subject of parsed.subjects) {
    for (const field of REQUIRED_FIELDS) {
      if (!Object.hasOwn(subject, field)) {
        throw new Error(
          `subject ${JSON.stringify(subject.id ?? "<no id>")} is missing ` +
            `the ${field} field. Every field is required, because a row with ` +
            `no definition is a row a later story is free to redefine.`,
        );
      }
    }
    if (typeof subject.id !== "string" || subject.id === "") {
      throw new Error("a subject has a non-string or empty id");
    }
    if (seen.has(subject.id)) {
      throw new Error(`subject id ${subject.id} appears twice`);
    }
    seen.add(subject.id);
    if (subject.subject_story !== null &&
        typeof subject.subject_story !== "string") {
      throw new Error(
        `subject ${subject.id} has a subject_story that is neither null nor ` +
          `an F-ID`,
      );
    }
    if (!Array.isArray(subject.tiers) || subject.tiers.length === 0) {
      throw new Error(
        `subject ${subject.id} declares no tiers. Deviation D-07 makes "n/a" ` +
          `a legitimate answer and an omitted one not.`,
      );
    }
    for (const tier of subject.tiers) {
      if (!VALID_TIERS.includes(tier)) {
        throw new Error(
          `subject ${subject.id} declares tier ${JSON.stringify(tier)}, ` +
            `which is not one of ${VALID_TIERS.join(", ")}`,
        );
      }
    }
    if (!Array.isArray(subject.dimensions)) {
      throw new Error(`subject ${subject.id} has non-array dimensions`);
    }
    if (!Array.isArray(subject.feeds)) {
      throw new Error(`subject ${subject.id} has non-array feeds`);
    }
  }
  return { subjects: parsed.subjects };
}

/**
 * Story status by F-ID, read from `docs/sprints/BACKLOG.md`.
 *
 * Only the status tables are read, which is the same restriction
 * `scripts/backlog_check.py` applies and for the same reason: the file also
 * carries a "Recorded defects" table whose rows begin with an F-ID and carry
 * no status.
 *
 * @param {string} text the contents of BACKLOG.md
 * @returns {Map<string, string>}
 */
export function backlogStatuses(text) {
  const statuses = new Map();
  let inStatusTable = false;
  for (const line of text.split("\n")) {
    if (line.startsWith("### ")) {
      const heading = line.slice(4).trim();
      inStatusTable = /^M\d+,/.test(heading) || heading.startsWith("Roadmap");
      continue;
    }
    if (line.startsWith("## ")) {
      inStatusTable = false;
      continue;
    }
    if (!inStatusTable) {
      continue;
    }
    const match = BACKLOG_ROW.exec(line);
    if (!match) {
      continue;
    }
    const cells = match[2].split("|").map((cell) => cell.trim());
    const status = cells.length >= 2 ? cells[cells.length - 2] : "";
    statuses.set(match[1], status);
  }
  return statuses;
}

/**
 * Resolve every row's blocking story against the delivery record.
 *
 * @returns {object[]} one entry per subject, each carrying the row plus
 *   `storyStatus` (null when the row names no story) and `subjectExists`
 */
export function resolveSubjects(subjects, { allocationFids, statuses }) {
  return subjects.map((subject) => {
    const story = subject.subject_story;
    if (story === null) {
      return { subject, storyStatus: null, subjectExists: true };
    }
    if (!allocationFids.has(story)) {
      throw new Error(
        `subject ${subject.id} names ${story}, which is not an F-ID in ` +
          `docs/sprints/allocation.json. Every subject_story is resolved ` +
          `against the allocation rather than copied from a comment, ` +
          `because comments in this tree once named F-096 for work that is ` +
          `F-101's and were corrected in S03.`,
      );
    }
    const status = statuses.get(story);
    if (status === undefined) {
      throw new Error(
        `subject ${subject.id} names ${story}, which has no status row in ` +
          `docs/sprints/BACKLOG.md`,
      );
    }
    if (!VALID_STATUS.includes(status)) {
      throw new Error(
        `subject ${subject.id} names ${story}, whose BACKLOG.md status is ` +
          `${JSON.stringify(status)}`,
      );
    }
    return { subject, storyStatus: status, subjectExists: status === "done" };
  });
}

/** Every F-ID `docs/sprints/allocation.json` knows about. */
export function allocationFids(text) {
  const parsed = JSON.parse(text);
  if (!Array.isArray(parsed.stories)) {
    throw new Error("allocation.json has no `stories` array");
  }
  return new Set(parsed.stories.map((story) => story.fid));
}

/** Read and resolve the tracked registry against the tracked delivery record. */
export function loadRegistry() {
  const { subjects } = parseRegistry(
    readFileSync(SUBJECTS_PATH, "utf8"),
  );
  const fids = allocationFids(
    readFileSync(repoPath("docs", "sprints", "allocation.json"), "utf8"),
  );
  const statuses = backlogStatuses(
    readFileSync(repoPath("docs", "sprints", "BACKLOG.md"), "utf8"),
  );
  return resolveSubjects(subjects, { allocationFids: fids, statuses });
}
