// Unit tests for the driver's argument parsing, and for the one guard that
// stands between a typo and the canonical reference render.
//
// `tools/oracle/out/` holds the 89 frames F-011 reads, and `prepareOutput`
// empties its target before rendering. So `--rows` plus the canonical `--out`
// would replace the whole corpus render with a subset, and the only thing
// stopping that is one comparison in `parseArgs`. Before this file, nothing in
// the repository watched it: deleting the guard outright left every test green
// and `eslint` silent.

import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, statSync } from "node:fs";
import { mkdir, mkdtemp, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";

import { DEFAULT_OUT, parseArgs } from "../run.mjs";
import { FAULTS } from "../src/faults.mjs";
import { isInside, sameDirectory } from "../src/output.mjs";

test("with no arguments, every check is on and the output is canonical", () => {
  const options = parseArgs([]);
  assert.equal(options.out, DEFAULT_OUT);
  assert.equal(options.rows, null);
  assert.equal(options.inject, null);
  assert.equal(options.once, false);
  assert.equal(options.unit, true);
  assert.equal(options.metadataCheck, true);
  assert.equal(options.selfTest, true);
  assert.equal(options.reportUnsupported, false);
});

test("each --no- flag turns off exactly its own check", () => {
  assert.equal(parseArgs(["--no-unit"]).unit, false);
  assert.equal(parseArgs(["--no-unit"]).metadataCheck, true);
  assert.equal(parseArgs(["--no-metadata-check"]).metadataCheck, false);
  assert.equal(parseArgs(["--no-self-test"]).selfTest, false);
  assert.equal(parseArgs(["--once"]).once, true);
});

test("an unknown argument is refused rather than ignored", () => {
  assert.throws(() => parseArgs(["--nonsense"]), /unknown argument --nonsense/);
  assert.throws(() => parseArgs(["rows"]), /unknown argument rows/);
});

// A flag whose value went missing would otherwise become `undefined` and
// surface much later as an unrelated error.
test("a flag with a missing or empty value is refused", () => {
  for (const flag of ["--out", "--rows", "--inject"]) {
    assert.throws(() => parseArgs([flag]), new RegExp(`${flag} needs a value`));
    assert.throws(() => parseArgs([flag, ""]), new RegExp(`${flag} needs a value`));
    assert.throws(
      () => parseArgs([flag, "--once"]),
      new RegExp(`${flag} needs a value`),
    );
  }
});

test("--out is resolved, so a relative spelling is an absolute path", () => {
  assert.equal(parseArgs(["--out", "somewhere"]).out, resolve("somewhere"));
  assert.ok(parseArgs(["--out", "somewhere"]).out.startsWith(sep));
});

// ---------------------------------------------------------------------------
// The guard over the canonical output
// ---------------------------------------------------------------------------

test("--rows into the canonical output is refused, absolutely spelled", () => {
  assert.throws(
    () => parseArgs(["--rows", "syntax/", "--out", DEFAULT_OUT]),
    /must name a directory outside/,
  );
});

// The spelling that defeated the string comparison, and the one the usage text
// prints. It is a relative path from the repository root.
test("--rows into the canonical output is refused, relatively spelled", () => {
  const saved = process.cwd();
  try {
    process.chdir(resolve(DEFAULT_OUT, "..", "..", ".."));
    assert.throws(
      () => parseArgs(["--rows", "syntax/", "--out", "tools/oracle/out"]),
      /must name a directory outside/,
    );
    process.chdir(resolve(DEFAULT_OUT, ".."));
    assert.throws(
      () => parseArgs(["--rows", "syntax/", "--out", "out"]),
      /must name a directory outside/,
    );
    assert.throws(
      () => parseArgs(["--rows", "syntax/", "--out", "./out/"]),
      /must name a directory outside/,
    );
  } finally {
    process.chdir(saved);
  }
});

// `realpathSync` does not normalise case on macOS, so a case-only variant
// survives resolving and names the same directory on a case-insensitive
// filesystem. Skipped where the filesystem is case-sensitive, because there
// the two really are different directories.
/** Whether the filesystem itself says these are one directory. */
function namesTheSameDirectory(left, right) {
  try {
    const a = statSync(left);
    const b = statSync(right);
    return a.dev === b.dev && a.ino === b.ino;
  } catch {
    return false;
  }
}

test("--rows into the canonical output is refused, case-only variant", () => {
  const shouted = `${DEFAULT_OUT.slice(0, -3)}OUT`;
  if (!namesTheSameDirectory(DEFAULT_OUT, shouted)) {
    // A case-sensitive filesystem, where the two really are two directories.
    return;
  }
  assert.throws(
    () => parseArgs(["--rows", "syntax/", "--out", shouted]),
    /must name a directory outside/,
  );
});

// A subdirectory of the canonical output would not destroy the reference
// render, it would nest a partial one inside it.
test("--rows into a subdirectory of the canonical output is refused", () => {
  assert.throws(
    () => parseArgs(["--rows", "syntax/", "--out", join(DEFAULT_OUT, "sub")]),
    /must name a directory outside/,
  );
  assert.throws(
    () => parseArgs(["--rows", "syntax/", "--out", join(DEFAULT_OUT, "a", "b")]),
    /must name a directory outside/,
  );
});

// The two spellings a string prefix test would have missed, applied to the
// PARENT. These are the same case-only and symlink spellings the whole-
// directory tests cover, one level up, and they were accepted until the
// containment test started asking `sameDirectory` about every ancestor.
test("--rows into a subdirectory of a case-only spelling is refused", () => {
  const shouted = `${DEFAULT_OUT.slice(0, -3)}OUT`;
  if (!namesTheSameDirectory(DEFAULT_OUT, shouted)) {
    // A case-sensitive filesystem, where the two really are two directories.
    return;
  }
  assert.throws(
    () => parseArgs(["--rows", "syntax/", "--out", join(shouted, "sub")]),
    /must name a directory outside/,
  );
});

test("--rows into a subdirectory of a symlink is refused", async () => {
  if (!existsSync(DEFAULT_OUT)) {
    return;
  }
  const base = await mkdtemp(join(tmpdir(), "ocelli-args-test-"));
  try {
    const link = join(base, "link-to-out");
    await symlink(DEFAULT_OUT, link);
    assert.throws(
      () => parseArgs(["--rows", "syntax/", "--out", join(link, "sub")]),
      /must name a directory outside/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// A sibling that merely shares a name prefix is NOT inside.
test("--rows into a sibling sharing a name prefix is allowed", () => {
  const sibling = `${DEFAULT_OUT}-old`;
  assert.equal(
    parseArgs(["--rows", "syntax/", "--out", sibling]).out,
    sibling,
  );
});

test("--rows into a directory of its own is allowed", () => {
  const options = parseArgs(["--rows", "syntax/", "--out", join(tmpdir(), "oc")]);
  assert.equal(options.rows, "syntax/");
  assert.equal(options.out, resolve(join(tmpdir(), "oc")));
});

// A symlink is the third spelling of one directory, and the one neither
// resolving nor case folding would catch.
//
// Guarded on the canonical output EXISTING, and that is the honest boundary
// rather than a convenience. A symlink can only point at something, so the
// identity test needs a target, and if there is no canonical output there is
// no reference render to protect. The gate runs this suite AFTER
// `prepareOutput` has emptied the directory, so on a gate run this returns
// early, which is why the boundary is written down here and in
// `sameDirectory`'s own comment rather than discovered later.
test("--rows into a symlink to the canonical output is refused", async () => {
  if (!existsSync(DEFAULT_OUT)) {
    return;
  }
  const base = await mkdtemp(join(tmpdir(), "ocelli-args-test-"));
  try {
    const link = join(base, "link-to-out");
    await symlink(DEFAULT_OUT, link);
    assert.throws(
      () => parseArgs(["--rows", "syntax/", "--out", link]),
      /must name a directory outside/,
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// The same guard with no filesystem in it at all, so something asserts the
// symlink and case behaviour on every run including a gate run.
test("isInside walks by identity, not by string prefix", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-args-test-"));
  try {
    const real = join(base, "real");
    await mkdir(join(real, "deep", "deeper"), { recursive: true });
    const link = join(base, "link");
    await symlink(real, link);
    assert.equal(isInside(real, real), true);
    assert.equal(isInside(join(real, "deep", "deeper"), real), true);
    assert.equal(isInside(join(link, "deep"), real), true);
    assert.equal(isInside(base, real), false);
    // A sibling sharing a name prefix is not inside, which is what a string
    // prefix test would have got wrong in the other direction.
    await mkdir(`${real}-old`);
    assert.equal(isInside(`${real}-old`, real), false);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("sameDirectory answers by identity where both paths exist", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-args-test-"));
  try {
    const real = join(base, "real");
    await mkdir(real);
    const link = join(base, "link");
    await symlink(real, link);
    assert.equal(sameDirectory(real, link), true);
    assert.equal(sameDirectory(real, join(base, "real", "..", "real")), true);
    assert.equal(sameDirectory(real, base), false);
    // Neither exists, so the resolved strings are the answer.
    assert.equal(sameDirectory(join(base, "no"), join(base, "no")), true);
    assert.equal(sameDirectory(join(base, "no"), join(base, "other")), false);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("the canonical output without --rows is the normal case and is allowed", () => {
  assert.equal(parseArgs(["--out", DEFAULT_OUT]).out, DEFAULT_OUT);
  assert.equal(parseArgs([]).out, DEFAULT_OUT);
});

// ---------------------------------------------------------------------------
// Injection
// ---------------------------------------------------------------------------

test("an injected run turns off everything that would fail with it", () => {
  const options = parseArgs(["--inject", "truncate"]);
  assert.equal(options.inject, "truncate");
  assert.equal(options.once, true);
  assert.equal(options.unit, false);
  assert.equal(options.metadataCheck, false);
  assert.equal(options.selfTest, false);
});

test("every declared fault name parses", () => {
  for (const name of Object.keys(FAULTS)) {
    assert.equal(parseArgs(["--inject", name]).inject, name);
  }
});

// A plain `FAULTS[name]` lookup resolves through Object.prototype, so
// `constructor`, `toString` and `hasOwnProperty` were all truthy and all
// accepted. Each injects nothing, so the run rendered normally and reported
// green having broken nothing.
test("a fault name inherited from Object.prototype is refused", () => {
  for (const name of ["constructor", "toString", "hasOwnProperty", "__proto__"]) {
    assert.throws(
      () => parseArgs(["--inject", name]),
      /unknown fault/,
      `${name} was accepted as a fault name`,
    );
  }
});

test("an unknown fault name is refused, and the message names the real ones", () => {
  assert.throws(() => parseArgs(["--inject", "nonsense"]), /unknown fault/);
  assert.throws(
    () => parseArgs(["--inject", "nonsense"]),
    new RegExp(Object.keys(FAULTS)[0]),
  );
});
