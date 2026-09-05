// Where things are, resolved once from this file's own location.
//
// The harness is run from the repository root by `bin/ocelli.sh oracle`, from
// `tools/oracle` by `npm start`, and from a test file by `node --test`. A
// relative path would mean three different corpora depending on the caller,
// which is the quiet kind of wrong this project is built to avoid.

import { realpathSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));

/** `tools/oracle`. */
export const ORACLE_ROOT = resolve(HERE, "..");

/** The repository root. */
export const REPO_ROOT = resolve(ORACLE_ROOT, "..", "..");

/** A path inside the repository, from a POSIX-style relative path. */
export function repoPath(...parts) {
  return join(REPO_ROOT, ...parts);
}

/** A path inside `tools/oracle`. */
export function oraclePath(...parts) {
  return join(ORACLE_ROOT, ...parts);
}

/**
 * Whether this module is the script node was asked to run.
 *
 * The obvious idiom, `import.meta.url === \`file://${process.argv[1]}\``, is
 * WRONG and it fails silently, which is the worst combination this project
 * has. `import.meta.url` percent-encodes and resolves symlinks. `argv[1]` does
 * neither. So a repository path containing a space, a `#`, a `%` or a
 * non-ASCII byte, or reached through a symlink, makes the two unequal, the
 * main block never runs, node exits 0, and `bin/ocelli.sh` reports the oracle
 * gate GREEN having rendered nothing.
 *
 * That is verbatim the defect this whole story exists to prevent, one level
 * up: not a page that starts and does nothing, but a harness that does not
 * start and says nothing. Measured on this machine through a symlink named
 * with a space: `node "<dir with space>/run.mjs" --help` printed nothing and
 * exited 0.
 *
 * `realpathSync` on both sides is what closes it. It decodes nothing, so
 * `fileURLToPath` does that half, and it resolves symlinks and `..` on both,
 * so the two are compared as the same kind of thing.
 */
export function isEntryPoint(importMetaUrl) {
  const invoked = process.argv[1];
  if (!invoked) {
    return false;
  }
  try {
    return realpathSync(invoked) === realpathSync(fileURLToPath(importMetaUrl));
  } catch {
    // Three things throw in here and all three mean the same thing. An argv[1]
    // that does not exist, an `import.meta.url` that is not a `file:` URL, and
    // a realpath that cannot be taken. None of them is this file.
    return false;
  }
}
