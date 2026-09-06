// Where things are, resolved once from this file's own location.
//
// The harness runs from the repository root through `bin/ocelli.sh bench`, from
// `tools/bench` through `npm start`, and from a test file through `node --test`.
// A relative path would mean three different registries depending on the
// caller.

import { realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));

/** `tools/bench`. */
export const BENCH_ROOT = resolve(HERE, "..");

/** The repository root. */
export const REPO_ROOT = resolve(BENCH_ROOT, "..", "..");

/** A path inside the repository, from a POSIX-style relative path. */
export function repoPath(...parts) {
  return join(REPO_ROOT, ...parts);
}

/** A path inside `tools/bench`. */
export function benchPath(...parts) {
  return join(BENCH_ROOT, ...parts);
}

/** The tracked subject registry. */
export const SUBJECTS_PATH = benchPath("subjects.json");

/** The tracked baseline, one recorded measurement per subject per host class. */
export const BASELINE_PATH = repoPath("ci", "bench-baseline.json");

/** The run record. Gitignored. One run. */
export const DEFAULT_OUT = benchPath("out");

/** Where the runner file for a subject lives, if it has one. */
export function runnerPath(subjectId) {
  return benchPath("src", "runners", `${runnerBasename(subjectId)}.mjs`);
}

/**
 * The runner file's basename for a subject id.
 *
 * One rule, in one place, because `scripts/bench_check.py` has to apply the
 * same one from Python. A second spelling of this convention would let a
 * runner exist that the gate could not see, and an unseen runner is exactly
 * the file the anti-fabrication rule is about.
 */
export function runnerBasename(subjectId) {
  return subjectId.replaceAll(".", "_");
}

/**
 * Whether this module is the script node was asked to run.
 *
 * The obvious idiom, comparing `import.meta.url` with `process.argv[1]`, is
 * wrong and it fails silently. `import.meta.url` percent-encodes and resolves
 * symlinks and `argv[1]` does neither, so a repository path holding a space, a
 * `%` or a non-ASCII byte, or reached through a symlink, makes the two unequal.
 * The main block then never runs, node exits 0, and `bin/ocelli.sh` reports the
 * gate green having measured nothing. Sprint worktrees in this project reach
 * the repository through a symlinked corpus already, so this is not
 * hypothetical.
 *
 * `realpathSync` on both sides closes it. It decodes nothing, so
 * `fileURLToPath` does that half, and it resolves symlinks on both, so the two
 * are compared as the same kind of thing.
 */
export function isEntryPoint(importMetaUrl) {
  const invoked = process.argv[1];
  if (!invoked) {
    return false;
  }
  try {
    return realpathSync(invoked) === realpathSync(fileURLToPath(importMetaUrl));
  } catch {
    // An argv[1] that does not exist, an import.meta.url that is not a file
    // URL, and a realpath that cannot be taken all mean the same thing here.
    return false;
  }
}
