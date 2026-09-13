import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const SCRIPT = new URL("../run_codec_wasm.mjs", import.meta.url);
const GOOD = Uint8Array.from([0,97,115,109,1,0,0,0,1,4,1,96,0,0,3,2,1,0,7,8,1,4,109,97,105,110,0,0,10,4,1,2,0,11]);
const EMPTY = Uint8Array.from([0,97,115,109,1,0,0,0]);

function run(...paths) {
  return spawnSync(process.execPath, [SCRIPT.pathname, ...paths], { encoding: "utf8" });
}

test("codec wasm runner refuses wrong arity and absent main", async () => {
  assert.match(run().stderr, /usage:/);
  const directory = await mkdtemp(join(tmpdir(), "ocelli-codec-wasm-"));
  try {
    const empty = join(directory, "empty.wasm");
    await writeFile(empty, EMPTY);
    assert.match(run(empty, empty).stderr, /does not export main/);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("codec wasm runner executes both modules", async () => {
  const directory = await mkdtemp(join(tmpdir(), "ocelli-codec-wasm-"));
  try {
    const good = join(directory, "good.wasm");
    await writeFile(good, GOOD);
    assert.equal(run(good, good).status, 0);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
