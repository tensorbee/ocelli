// Unit tests for `isEntryPoint`, and one regression test for the whole driver.
//
// The idiom this replaced, `import.meta.url === `file://${process.argv[1]}``,
// is wrong on a repository path containing a space and wrong through a
// symlink, and it FAILS SILENTLY: the main block does not run, node exits 0,
// and `bin/ocelli.sh` reports the oracle gate green having rendered nothing.
// Measured before the fix, `node "<dir with space>/run.mjs" --help` printed
// nothing and exited 0.
//
// So this file does not only test the helper. It runs the real driver through
// a real symlink whose name contains a space and requires it to speak, which
// is the only test that would have caught the original.

import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { isEntryPoint, oraclePath, ORACLE_ROOT, REPO_ROOT, repoPath } from "../src/paths.mjs";

const HERE = fileURLToPath(import.meta.url);

test("the roots are the directories they claim to be", () => {
  assert.equal(oraclePath("run.mjs"), join(ORACLE_ROOT, "run.mjs"));
  assert.equal(repoPath("corpus", "manifest.tsv"), join(REPO_ROOT, "corpus", "manifest.tsv"));
  assert.ok(ORACLE_ROOT.endsWith(join("tools", "oracle")));
});

// `node --test <file>` sets argv[1] to the file, so this module IS the entry
// point while the suite runs. Both directions are asserted from that: itself
// yes, a sibling no.
test("a module that is not the invoked script is not the entry point", () => {
  assert.equal(isEntryPoint(import.meta.url), true);
  const sibling = pathToFileURL(oraclePath("run.mjs")).href;
  assert.equal(isEntryPoint(sibling), false);
});

test("a missing or absent argv[1] is not the entry point", () => {
  const saved = process.argv[1];
  try {
    process.argv[1] = undefined;
    assert.equal(isEntryPoint(import.meta.url), false);
    process.argv[1] = join(tmpdir(), "this-file-does-not-exist-ocelli");
    assert.equal(isEntryPoint(import.meta.url), false);
  } finally {
    process.argv[1] = saved;
  }
});

// The comparison is between real paths, so an argv[1] that names this file by
// any route is the entry point. `..` is the cheap version of the symlink case.
test("argv[1] naming this file by an indirect route is the entry point", () => {
  const saved = process.argv[1];
  try {
    process.argv[1] = HERE;
    assert.equal(isEntryPoint(import.meta.url), true);
    process.argv[1] = join(ORACLE_ROOT, "src", "..", "tests", "paths_test.mjs");
    assert.equal(isEntryPoint(import.meta.url), true);
  } finally {
    process.argv[1] = saved;
  }
});

function run(command, args) {
  return new Promise((done) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", (error) => done({ code: 127, stdout, stderr: String(error) }));
    child.on("close", (code) => done({ code: code ?? 1, stdout, stderr }));
  });
}

// The regression itself. A space in the path percent-encodes in
// `import.meta.url` and does not in `argv[1]`, and a symlink is resolved in one
// and not the other, so this route breaks both halves of the old comparison at
// once. `--help` is used because it is the driver's cheapest observable output.
test("the driver speaks when reached through a symlinked path with a space", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-paths-test-"));
  try {
    const link = join(base, "a dir with spaces");
    await symlink(ORACLE_ROOT, link);
    const result = await run(process.execPath, [join(link, "run.mjs"), "--help"]);
    assert.equal(result.code, 0, result.stderr);
    assert.match(
      result.stdout,
      /bin\/ocelli\.sh oracle/,
      "the driver printed nothing, which is how this failed silently before",
    );
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// The same route, and an argument the driver must REFUSE. Exiting 0 on
// nonsense is what a harness that never started looks like from the outside.
test("the driver refuses an unknown argument through that same path", async () => {
  const base = await mkdtemp(join(tmpdir(), "ocelli-paths-test-"));
  try {
    const link = join(base, "a dir with spaces");
    await symlink(ORACLE_ROOT, link);
    const result = await run(process.execPath, [join(link, "run.mjs"), "--nonsense"]);
    assert.equal(result.code, 1, "an unknown argument must be a failure");
    assert.match(result.stderr, /unknown argument --nonsense/);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// `build-page.mjs` and `tests/faults.mjs` carry the same guard, and a fix
// applied to one of three copies is how this class comes back.
test("every module with a main block uses the shared guard", async () => {
  const { readFile } = await import("node:fs/promises");
  for (const name of ["run.mjs", "build-page.mjs", "tests/faults.mjs"]) {
    const source = await readFile(oraclePath(name), "utf8");
    assert.match(
      source,
      /isEntryPoint\(import\.meta\.url\)/,
      `${name} does not use isEntryPoint`,
    );
    assert.doesNotMatch(
      source,
      /import\.meta\.url === /,
      `${name} still compares import.meta.url to a string`,
    );
  }
});
