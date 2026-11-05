// Hand checks for the folder viewer subjects. These numbers are computed
// here, not copied from a run. The file does not launch a browser.

import assert from "node:assert/strict";
import test from "node:test";

import {
  filledPixels,
  greyWord,
  median,
  mono16Part10,
  pageHeapBound,
  serverRssBound,
} from "../src/viewer_work.mjs";

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
