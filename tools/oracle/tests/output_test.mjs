// Unit tests for the output directory's lifecycle.
//
// `prepareOutput` calls `rm -rf` on a path an operator typed, because `--rows`
// requires `--out`. The guard between a typo and somebody's files is one
// condition, and it is not reached on any normal run, so without these tests it
// would be a guard nobody has watched fail.

import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { prepareOutput, RUN_RECORD } from "../src/output.mjs";

async function scratch() {
  return mkdtemp(join(tmpdir(), "ocelli-output-test-"));
}

test("a directory that does not exist yet is left alone", async () => {
  const base = await scratch();
  try {
    const target = join(base, "never-created");
    await prepareOutput(target);
    await assert.rejects(() => readdir(target), { code: "ENOENT" });
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("an empty directory is removed", async () => {
  const base = await scratch();
  try {
    const target = join(base, "empty");
    await mkdir(target);
    await prepareOutput(target);
    await assert.rejects(() => readdir(target), { code: "ENOENT" });
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test("a previous run's output is removed, because it holds run.json", async () => {
  const base = await scratch();
  try {
    const target = join(base, "out");
    await mkdir(target);
    await writeFile(join(target, RUN_RECORD), "{}\n");
    await writeFile(join(target, "row.raw"), "pixels");
    await prepareOutput(target);
    await assert.rejects(() => readdir(target), { code: "ENOENT" });
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// The one that matters. An operator's directory, named by a typo, survives.
test("a non-empty directory with no run.json is refused, and survives", async () => {
  const base = await scratch();
  try {
    const target = join(base, "notes");
    await mkdir(target);
    await writeFile(join(target, "important.md"), "do not delete me");
    await assert.rejects(
      () => prepareOutput(target),
      /not an oracle output directory/,
    );
    assert.deepEqual(await readdir(target), ["important.md"]);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

// Swallowing ENOTDIR as "nothing there yet" is how a `--out` pointing at a file
// renders the whole corpus and then dies at `mkdir` with an unexplained EEXIST,
// nowhere near the cause.
test("an --out that is a file is refused, naming the reason", async () => {
  const base = await scratch();
  try {
    const target = join(base, "a-file");
    await writeFile(target, "not a directory");
    await assert.rejects(() => prepareOutput(target), /cannot be read as a directory/);
    await assert.rejects(() => prepareOutput(target), /ENOTDIR/);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});
