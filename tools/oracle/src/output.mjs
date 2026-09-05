// The output directory's lifecycle.
//
// `tools/oracle/out/` is where F-011 will read reference frames from, so the
// rule this module enforces is narrow and absolute: **the directory holds the
// output of one complete run that passed every boundary, or it holds nothing.**
//
// Nothing under it is ever committed. A reference frame of a real corpus row is
// a rendered picture of patient data and every real row in
// `corpus/manifest.tsv` carries `burned-in-unchecked`.

import { statSync } from "node:fs";
import { mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

import { rowId } from "./manifest.mjs";
import { assertFrameIntegrity, buildSidecar } from "./sidecar.mjs";

/** What marks a directory as this harness's own output. */
export const RUN_RECORD = "run.json";

/**
 * Empty the output directory before anything is rendered.
 *
 * Early, and not at the end, because the end is only reached by a run that got
 * that far. A pin that moved, a missing corpus row, a digest mismatch, a failed
 * unit suite, a browser that will not start: every one of those aborts before a
 * single frame exists, and emptying at the end would cover only the failures
 * that got past all of them.
 *
 * It will not empty a directory it did not write. `--rows` requires `--out`, so
 * an operator types a directory name on the normal path, and `rm -rf` on a
 * directory this harness did not create is not a thing to do quietly. Its own
 * output is recognised by holding `run.json`.
 */
export async function prepareOutput(out) {
  let existing;
  try {
    existing = await readdir(out);
  } catch (error) {
    if (error?.code === "ENOENT") {
      // Nothing there yet, which is the first run and is fine.
      return;
    }
    // ENOTDIR, EACCES, ELOOP. Swallowing these as "nothing there" is how a
    // `--out` pointing at a FILE renders the whole corpus and then dies at
    // `mkdir` with an unexplained EEXIST, half an hour later and nowhere near
    // the cause.
    throw new Error(
      `--out ${out} cannot be read as a directory ` +
        `(${error?.code ?? "unknown"}: ${String(error?.message ?? error)})`,
    );
  }
  if (existing.length > 0 && !existing.includes(RUN_RECORD)) {
    throw new Error(
      `--out ${out} exists, is not empty, and holds no ${RUN_RECORD}, so it is ` +
        `not an oracle output directory. This harness empties its output ` +
        `before it renders, and it will not empty a directory it did not write.`,
    );
  }
  await rm(out, { recursive: true, force: true });
}

/**
 * Remove the output after a run that failed.
 *
 * The frames of a run that went red are frames nobody stands behind, and F-011
 * has no way to tell them from good ones once the terminal has scrolled.
 *
 * **Precondition: `prepareOutput` accepted this path earlier in the same run.**
 * That is what makes an unguarded `rm -rf` safe here, because a directory this
 * harness did not write is refused there and the refusal aborts before any
 * caller of this function is reached. A future caller that has not been through
 * `prepareOutput` must not use this.
 */
export async function discardOutput(out) {
  await rm(out, { recursive: true, force: true });
}

/**
 * Whether `candidate` is `ancestor`, or anything beneath it.
 *
 * A string prefix test would have been the comparison `sameDirectory` exists
 * to avoid, one level up: `<out>/sub` is caught, and `<OUT>/sub` on a
 * case-insensitive filesystem, or `<symlink-to-out>/sub`, are not. So the walk
 * asks `sameDirectory` about each ancestor in turn, which puts the case and
 * symlink handling in one place rather than two.
 *
 * `dirname` strictly shortens an absolute path until the root, where it becomes
 * a fixed point, so the root check is what terminates the walk. There is no
 * depth cap: one was here and it FAILED OPEN, returning "not inside" once it
 * was exhausted, which at sixty-four levels down is a 332 character path well
 * inside PATH_MAX. A counter that cannot end a loop the root check would not
 * end, and can only produce a wrong answer, is worse than no counter.
 */
export function isInside(candidate, ancestor) {
  let directory = resolve(candidate);
  for (;;) {
    if (sameDirectory(directory, ancestor)) {
      return true;
    }
    const parent = dirname(directory);
    if (parent === directory) {
      return false;
    }
    directory = parent;
  }
}

/** Write one row's frame, PNG and sidecar. Returns the frame's digest. */
export async function writeRow(outDir, entry, environment, installed) {
  const { row, params, result } = entry;
  const id = rowId(row.path);
  const raw = Buffer.from(result.rawBase64, "base64");
  const digest = assertFrameIntegrity(row.path, raw, result);

  await writeFile(join(outDir, `${id}.raw`), raw);
  await writeFile(
    join(outDir, `${id}.png`),
    Buffer.from(result.pngBase64, "base64"),
  );
  await writeFile(
    join(outDir, `${id}.json`),
    `${JSON.stringify(buildSidecar({ row, params, result, environment, installed }), null, 2)}\n`,
  );
  return digest;
}

/** Create the output directory. Called only by a run with nothing wrong. */
export async function openOutput(out) {
  await mkdir(out, { recursive: true });
}

/**
 * Whether two paths name the same directory.
 *
 * String comparison is not enough, and each way it fails has already bitten
 * this harness once. `--out tools/oracle/out` from the repository root is the
 * canonical directory spelled relatively, which `resolve` fixes. `--out
 * tools/oracle/OUT` on a case-insensitive filesystem is the same directory
 * again, and `realpathSync` does NOT normalise case on macOS, so that one
 * survives resolving. A symlink is a third spelling.
 *
 * The device and inode pair is the filesystem's own answer to "is this the
 * same directory", so it is the one asked here.
 *
 * **Where either path does not exist there is nothing to compare, and this
 * falls back to the resolved strings.** That is a real weakening and it is the
 * right boundary: the identity test catches a case-only variant and a symlink,
 * and both of those can only name an EXISTING directory. If the canonical
 * output does not exist there is no reference render to protect, and the
 * resolved-string comparison still catches every spelling that names it
 * directly.
 */
export function sameDirectory(left, right) {
  try {
    const a = statSync(left);
    const b = statSync(right);
    return a.dev === b.dev && a.ino === b.ino;
  } catch {
    return resolve(left) === resolve(right);
  }
}
