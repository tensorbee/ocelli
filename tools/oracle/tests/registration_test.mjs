// One list of test suites, asserted against the two places that copy it.
//
// The suite list is written by hand twice: in `run.mjs`'s `runUnitTests`, which
// the oracle gate runs, and in `package.json`'s `test` script, which a
// developer runs. Nothing compared them. A suite added to one and not the other
// is a suite the gate runs and `npm test` does not, or the reverse, and either
// way it is silent.
//
// `checkPins` in the same driver does exactly this cross-check for exactly this
// reason, and says so: "a pin recorded in one place and not the other is a pin
// that does not hold". The same is true of a test.
//
// The disk is the authority here rather than either list, so a suite file that
// is registered NOWHERE also goes red, which is the case neither list could
// ever catch on its own.

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";

import { codecPackages, packageNameOf } from "../build-page.mjs";
import { oraclePath } from "../src/paths.mjs";

/** Every `*_test.mjs` on disk, which is what a suite file looks like here. */
function suitesOnDisk() {
  return readdirSync(oraclePath("tests"))
    .filter((name) => name.endsWith("_test.mjs"))
    .sort();
}

/** The filenames a text names, in the `tests/` directory. */
function named(text) {
  return [...text.matchAll(/tests\/([A-Za-z0-9_.-]+_test\.mjs)/g)]
    .map((match) => match[1])
    .sort();
}

/**
 * The body of one function in a source file.
 *
 * Slicing to the end of the file would let a COMMENT naming a suite satisfy
 * the check while the list itself was short, and `run.mjs` does carry comment
 * references to files under `tests/`. The closing brace of the function is the
 * boundary that makes the check about the list.
 */
function functionBody(source, declaration) {
  const start = source.indexOf(declaration);
  assert.notEqual(start, -1, `${declaration} is not in the source`);
  const end = source.indexOf("\n}\n", start);
  assert.notEqual(end, -1, `${declaration} has no closing brace`);
  return source.slice(start, end);
}

test("there is at least one suite, so the comparisons below mean something", () => {
  assert.ok(suitesOnDisk().length > 0);
});

test("the driver's unit-test list names exactly the suites on disk", () => {
  const source = readFileSync(oraclePath("run.mjs"), "utf8");
  const listed = named(functionBody(source, "async function runUnitTests"));
  assert.deepEqual(
    listed,
    suitesOnDisk(),
    "run.mjs's runUnitTests and tools/oracle/tests/ disagree, so the oracle " +
      "gate runs a different set from the one that exists",
  );
});

test("the npm test script names exactly the suites on disk", () => {
  const manifest = JSON.parse(readFileSync(oraclePath("package.json"), "utf8"));
  assert.deepEqual(
    named(manifest.scripts.test),
    suitesOnDisk(),
    "package.json's test script and tools/oracle/tests/ disagree, so " +
      "`npm test` runs a different set from the gate",
  );
});

// The LLD's layout table counts the suites in words, and that count has now
// been stale three times: once on the `src/` row, twice on this one, the last
// time in the very round that incremented it. A number maintained by hand
// beside a directory that grows is a number that will be wrong again.
test("the LLD's suite count is the number of suites", () => {
  const lld = readFileSync(
    oraclePath("..", "..", "docs", "lld", "oracle.md"),
    "utf8",
  );
  const words = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight",
    "nine", "ten", "eleven", "twelve",
  ];
  const expected = words[suitesOnDisk().length];
  assert.ok(expected, "add a word to the list above");
  // Every table ROW of this shape, so a second stale copy elsewhere in the
  // file is caught rather than shadowed by the correct one. A count written as
  // a prose sentence rather than a row would not match, and the file carries
  // none.
  const rows = [...lld.matchAll(/\| (\w+) `node:test` suites/g)].map(
    (match) => match[1],
  );
  assert.deepEqual(
    rows,
    [expected],
    `docs/lld/oracle.md should say there are ${expected} node:test suites, ` +
      `exactly once, and there are ${suitesOnDisk().length}`,
  );
});

// This file is itself one of the suites, so it is in both lists, which is the
// point: a suite that registers itself nowhere cannot hide behind the fact that
// nothing runs it.
// `codecPackages` reduces a subpath specifier to a package name, and both
// halves of `checkPins`'s codec cross-check read it. The unscoped branch is
// unreachable from the committed `CODEC_WASM`, which is why it needs a test:
// taking two segments unconditionally would fail closed but name a package
// that does not exist.
test("a subpath specifier reduces to its package name, scoped or not", () => {
  assert.deepEqual(codecPackages(), [
    "@cornerstonejs/codec-charls",
    "@cornerstonejs/codec-libjpeg-turbo-8bit",
    "@cornerstonejs/codec-openjpeg",
    "@cornerstonejs/codec-openjph",
  ]);
  assert.deepEqual(packageNameOf("@scope/name/subpath"), "@scope/name");
  assert.deepEqual(packageNameOf("@scope/name"), "@scope/name");
  assert.deepEqual(packageNameOf("name/subpath"), "name");
  assert.deepEqual(packageNameOf("name"), "name");
});

test("this suite is registered in both places", () => {
  assert.ok(suitesOnDisk().includes("registration_test.mjs"));
});
