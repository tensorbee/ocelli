// Hand checks for the folder viewer subjects. These numbers are computed
// here, not copied from a run. The file does not launch a browser.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  assertCanvasCheck,
  assertForwardCounters,
  assertNoPreviewBody,
  assertPageHooks,
  assertRestartRecovery,
  assertScrollDelivery,
  assertScrollStage,
  assertWindowSample,
  ensureReleaseBinaryArgs,
  ensureViewerBinary,
  filledPixels,
  foldHeapReadings,
  greyWord,
  HEAP_AGREE_BYTES,
  HEAP_READ_CAP,
  longEl,
  median,
  mono16Part10,
  pageHeapBound,
  profileFromHeld,
  readRetainedHeap,
  serverRssBound,
  shortEl,
} from "../src/viewer_work.mjs";

function refuses(fn, text) {
  assert.throws(fn, (error) => error instanceof Error && String(error.message).includes(text));
}

test("median is the middle value", () => {
  // Sorted 1, 5, 9. The middle value is 5.
  assert.equal(median([9, 1, 5]), 5);
  // Sorted 1, 2, 3, 4. The two middle values are 2 and 3. Their mean is 2.5.
  assert.equal(median([1, 2, 3, 4]), 2.5);
});

test("greyWord repeats one byte into a little-endian RGBA word", () => {
  // 64 is 0x40. The word is 0xFF404040.
  assert.equal(greyWord(64), 0xff404040);
});

test("serverRssBound matches the stated slack for the named radiograph", () => {
  // 4 * 17400516 = 69602064
  // 2 * 17400516 + 5 * 8700000 = 34801032 + 43500000 = 78301032
  // 2 * 78301032 = 156602064
  // 32 MiB = 33554432
  // 69602064 + 156602064 + 33554432 = 259758560
  assert.equal(serverRssBound(0, 17400516, 8700000), 259758560);
});

test("pageHeapBound counts five canvases plus the named allowances", () => {
  const dx = [
    [3000, 2900],
    [3000, 2900],
    [3000, 2900],
    [3000, 2900],
    [3000, 2900],
  ];
  // 5 * 4 * 3000 * 2900 = 174000000
  // 64 MiB = 67108864 and 16 MiB = 16777216
  // 174000000 + 67108864 + 16777216 = 257886080
  assert.equal(pageHeapBound(dx), 257886080);
  const ct = [
    [512, 512],
    [512, 512],
    [512, 512],
    [512, 512],
    [512, 512],
  ];
  // 5 * 4 * 512 * 512 = 5242880
  // 64 MiB + 16 MiB = 83886080
  // 5242880 + 83886080 = 89128960
  assert.equal(pageHeapBound(ct), 89128960);
});

test("a release viewer with no rebuild stays on the release profile", () => {
  const args = ensureReleaseBinaryArgs(false);
  assert.ok(args.includes("--release"));
  assert.ok(args.includes("--offline"));
  assert.equal(args.indexOf("--release") < args.indexOf("--offline"), true);
});

test("a recorded release subject builds the release binary", async () => {
  let seen = null;
  const path = await ensureViewerBinary(undefined, false, (args) => {
    seen = args;
    return "/tmp/viewer";
  });
  assert.equal(path, "/tmp/viewer");
  assert.ok(seen.includes("--release"));
  assert.ok(seen.includes("--offline"));
  assert.equal(seen.includes("debug"), false);
});

test("retained heap is the first pair that agrees", () => {
  // 64 KiB is the stated tolerance. A larger gap is a different reading.
  assert.equal(HEAP_AGREE_BYTES, 64 * 1024);
  assert.equal(HEAP_READ_CAP, 8);
  // The first pair differs by about 26 MiB. The next pair differs by 1140.
  const review = foldHeapReadings([124293603, 98124338, 98125478, 98125478], HEAP_AGREE_BYTES, HEAP_READ_CAP);
  assert.equal(review.ok, true);
  assert.equal(review.retained, 98125478);
  assert.equal(review.collections, 3);
  const equal = foldHeapReadings([5, 5], HEAP_AGREE_BYTES, HEAP_READ_CAP);
  assert.equal(equal.ok, true);
  assert.equal(equal.retained, 5);
  assert.equal(equal.collections, 2);
  const edge = foldHeapReadings([0, 64 * 1024], HEAP_AGREE_BYTES, HEAP_READ_CAP);
  assert.equal(edge.ok, true);
  assert.equal(edge.retained, 64 * 1024);
  assert.equal(edge.collections, 2);
  const over = foldHeapReadings([0, 64 * 1024 + 1], HEAP_AGREE_BYTES, HEAP_READ_CAP);
  assert.equal(over.ok, false);
  assert.equal(over.retained, null);
  assert.equal(over.collections, 2);
  const steps = foldHeapReadings([0, 1_000_000, 2_000_000, 3_000_000, 4_000_000], HEAP_AGREE_BYTES, 4);
  assert.equal(steps.ok, false);
  assert.equal(steps.retained, null);
  assert.equal(steps.collections, 4);
});

test("a retained heap that cannot be read is an invalid run", async () => {
  await assert.rejects(
    () => readRetainedHeap({ evaluate: async () => null }),
    /the page did not expose performance.memory/,
  );
  await assert.rejects(
    () => readRetainedHeap({
      evaluate: async (fn) => {
        const source = Function.prototype.toString.call(fn);
        if (!source.includes("globalThis.gc")) return 10;
        return null;
      },
    }),
    /the page did not expose gc or performance.memory/,
  );
  let step = 0;
  await assert.rejects(
    () => readRetainedHeap({
      evaluate: async (fn) => {
        const source = Function.prototype.toString.call(fn);
        if (!source.includes("globalThis.gc")) return 10;
        step += 1;
        return step * 1_000_000;
      },
    }),
    /retained heap readings did not agree/,
  );
});

test("profileFromHeld accepts only the server build", () => {
  assert.equal(profileFromHeld({ build: "release" }), "release");
  assert.equal(profileFromHeld({ build: "debug" }), "debug");
  refuses(() => profileFromHeld({}), "the viewer did not report its build");
  refuses(() => profileFromHeld({ build: "release-debug" }), "the viewer did not report its build");
});

test("viewer subjects record the build the server reports", () => {
  const source = readFileSync(new URL("../src/viewer_work.mjs", import.meta.url), "utf8");
  for (const name of ["measureOpen", "measureFirstImage", "measureScroll", "measureWindow", "measureResident"]) {
    const start = source.indexOf(`export async function ${name}`);
    assert.ok(start >= 0, name);
    const rest = source.slice(start);
    const end = rest.indexOf("\nexport async function ", 1);
    const body = end === -1 ? rest : rest.slice(0, end);
    assert.match(body, /reportedBuild\(/);
    assert.equal(body.includes("profile: chosen"), false);
    assert.equal(/profile:\s*"release"/.test(body), false);
    assert.equal(/profile:\s*"debug"/.test(body), false);
  }
});

test("assertPageHooks with no arguments uses the page globals", () => {
  const request = globalThis.requestGrey;
  const draw = globalThis.drawGrey;
  try {
    delete globalThis.requestGrey;
    delete globalThis.drawGrey;
    refuses(() => assertPageHooks(), "the page did not expose requestGrey and drawGrey");
    globalThis.requestGrey = () => {};
    globalThis.drawGrey = () => {};
    assert.doesNotThrow(() => assertPageHooks());
  } finally {
    if (request === undefined) delete globalThis.requestGrey;
    else globalThis.requestGrey = request;
    if (draw === undefined) delete globalThis.drawGrey;
    else globalThis.drawGrey = draw;
  }
});

test("openPage evaluates the real in-page hooks check", () => {
  const source = readFileSync(new URL("../src/viewer_work.mjs", import.meta.url), "utf8");
  const start = source.indexOf("async function openPage");
  assert.ok(start >= 0);
  const rest = source.slice(start);
  const end = rest.indexOf("\nasync function ", 1);
  const body = end === -1 ? rest : rest.slice(0, end);
  assert.match(body, /await page\.evaluate\(assertPageHooks\)/);
});

test("viewer work refusals name the condition they reject", () => {
  refuses(() => median([]), "median needs at least one value");
  refuses(() => shortEl(0x0010, 0x0010, "PN", Buffer.from([1])), "a short element has a bad length");
  refuses(() => longEl(0x7fe0, 0x0010, "OW", Buffer.from([1])), "a long element has a bad length");
  refuses(() => assertPageHooks(null, null), "the page did not expose requestGrey and drawGrey");
  refuses(() => assertScrollDelivery(10), "scroll delivered");
  refuses(() => assertScrollStage(0, 1), "the stage showed a fault during the scroll");
  refuses(() => assertScrollStage(2, 0), "the stage was blank during the scroll");
  refuses(() => assertForwardCounters(["2 / 3", "1 / 3"]), "the counter went backwards");
  refuses(() => assertWindowSample(null), "a window step did not reach drawGrey");
  refuses(() => assertCanvasCheck({ error: "preview status 500" }), "the canvas check failed");
  refuses(() => assertCanvasCheck({ points: 4, mismatches: [] }), "canvas check sampled");
  refuses(
    () => assertCanvasCheck({ points: 16, mismatches: [{ x: 1, y: 2 }] }),
    "canvas pixel",
  );
  refuses(() => assertRestartRecovery("the file is no longer held", 1), "a restart left the gone sentence");
  refuses(() => assertRestartRecovery("", 0), "the restart did not re-post the file");
  refuses(() => assertNoPreviewBody(2), "/preview carried a body");
});

test("mono16Part10 is a Part 10 preamble and constant stored samples", () => {
  // PS3.10: 128 preamble bytes, then DICM.
  // PS3.5: one OW sample is two little-endian bytes. Two pixels store 50.
  const pixels = filledPixels(2, 50);
  assert.equal(pixels.length, 4);
  assert.equal(pixels.readUInt16LE(0), 50);
  assert.equal(pixels.readUInt16LE(2), 50);
  const bytes = mono16Part10({
    rows: 1,
    columns: 2,
    sopClass: "1.2.840.10008.5.1.4.1.1.2",
    sopUid: "2.25.216",
    seriesUid: "2.25.2160001",
    modality: "CT",
    instanceNumber: "1",
    pixels,
  });
  assert.equal(bytes.subarray(128, 132).toString("ascii"), "DICM");
});
