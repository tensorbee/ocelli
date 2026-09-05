// A host class that differs in any recorded field yields `incomparable`, and an
// identical one yields a comparison.
//
// Every case here drives a stand-in for `node:os`, so nothing asserts anything
// about the machine the test runs on. A test that read the real CPU model would
// pass on one laptop and fail on the next, which is the defect the host class
// exists to name rather than one to reproduce.

import assert from "node:assert/strict";
import test from "node:test";

import {
  conditions,
  hostClass,
  hostClassKey,
  HOST_CLASS_FIELDS,
  sameHostClass,
  sameInstrument,
} from "../src/hostclass.mjs";

const fakeOs = (overrides = {}) => ({
  platform: () => overrides.platform ?? "darwin",
  release: () => overrides.release ?? "25.6.0",
  arch: () => overrides.arch ?? "arm64",
  cpus: () => overrides.cpus ?? new Array(16).fill({ model: "Apple M5 Max" }),
  totalmem: () => overrides.totalmem ?? 68719476736,
  freemem: () => overrides.freemem ?? 12884901888,
  loadavg: () => overrides.loadavg ?? [1.5, 1.2, 1.1],
  uptime: () => overrides.uptime ?? 4321.7,
});

test("the class carries the three fields the oracle does not collect", () => {
  const hc = hostClass(fakeOs());
  assert.deepEqual(Object.keys(hc).sort(), [...HOST_CLASS_FIELDS].sort());
  assert.equal(hc.cpu_model, "Apple M5 Max");
  assert.equal(hc.cpu_count, 16);
  assert.equal(hc.memory_bytes, 68719476736);
});

test("a machine reporting no CPUs says unknown rather than throwing", () => {
  assert.equal(hostClass(fakeOs({ cpus: [] })).cpu_model, "unknown");
});

test("the key is stable and separates every field", () => {
  const key = hostClassKey(hostClass(fakeOs()));
  assert.equal(key, "darwin|25.6.0|arm64|Apple_M5_Max|16|68719476736");
  assert.equal(key, hostClassKey(hostClass(fakeOs())),
    "the key is not stable across two calls");
});

test("a difference in any single field makes two classes different", () => {
  const base = hostClass(fakeOs());
  for (const [field, override] of [
    ["platform", { platform: "linux" }],
    ["release", { release: "6.8.0" }],
    ["arch", { arch: "x64" }],
    ["cpu_model", { cpus: new Array(16).fill({ model: "Apple M4 Pro" }) }],
    ["cpu_count", { cpus: new Array(8).fill({ model: "Apple M5 Max" }) }],
    ["memory_bytes", { totalmem: 34359738368 }],
  ]) {
    const other = hostClass(fakeOs(override));
    const verdict = sameHostClass(base, other);
    assert.equal(verdict.same, false, `${field} did not make a difference`);
    assert.deepEqual(verdict.differing, [field]);
    assert.notEqual(hostClassKey(base), hostClassKey(other));
  }
});

test("an identical class compares equal", () => {
  const verdict = sameHostClass(hostClass(fakeOs()), hostClass(fakeOs()));
  assert.equal(verdict.same, true);
  assert.deepEqual(verdict.differing, []);
});

test("load average is recorded and is NOT part of the class", () => {
  const busy = fakeOs({ loadavg: [9.9, 8.8, 7.7], freemem: 1024 });
  const idle = fakeOs({ loadavg: [0.1, 0.1, 0.1], freemem: 68719476000 });
  assert.equal(hostClassKey(hostClass(busy)), hostClassKey(hostClass(idle)),
    "the machine's load moved it into a different host class, which would " +
      "make every run incomparable with every other");
  assert.deepEqual(conditions(busy).load_average, [9.9, 8.8, 7.7]);
  assert.equal(conditions(busy).free_memory_bytes, 1024);
  assert.equal(conditions(busy).uptime_seconds, 4322);
});

test("the instrument is compared separately from the machine", () => {
  const a = { node: "v24.16.0", playwright: "1.62.1", chromium: "142.0.1" };
  assert.equal(sameInstrument(a, { ...a }).same, true);
  const moved = sameInstrument(a, { ...a, chromium: "143.0.1" });
  assert.equal(moved.same, false);
  assert.deepEqual(moved.differing, ["chromium"]);
  // A field present on one side and absent on the other is a difference, not a
  // match by omission. A baseline recorded before a version was captured must
  // not silently compare equal to one recorded after.
  const partial = sameInstrument(a, { node: a.node, playwright: a.playwright });
  assert.equal(partial.same, false);
  assert.deepEqual(partial.differing, ["chromium"]);
  // **And the other direction, which is the one that reaches the key union.**
  // The case above is already seen by `Object.keys(a)` on its own, so dropping
  // `...Object.keys(b)` from the set left the suite green while the parallel
  // guard on `sameHostClass` was covered in both directions. The direction it
  // dropped is the one that actually happens: the baseline is the older
  // record, so it is the side that carries a field this run does not, and a
  // baseline naming a tool the run never captured would have compared as
  // identical and been given a verdict.
  const richerBaseline = sameInstrument(a, { ...a, gpu: "Apple M5 Max" });
  assert.equal(richerBaseline.same, false,
    "an instrument key the recorded side carries and this run does not was " +
      "read as a match");
  assert.deepEqual(richerBaseline.differing, ["gpu"]);
});
